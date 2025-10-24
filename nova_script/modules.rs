#![allow(dead_code)]

use crate::ast::{ImportSource, Program};
use crate::interpreter::Value;
use crate::parser::{ParseError, Parser};
use crate::tokenizer::Tokenizer;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub enum ModuleOrigin {
    Script(PathBuf),
    Standard(PathBuf),
    Compiled(PathBuf),
}

#[derive(Debug, Clone)]
pub enum ModuleArtifact {
    Script { program: Program, path: PathBuf },
    Compiled { path: PathBuf, bytes: Vec<u8> },
}

#[derive(Debug, Clone)]
pub struct ModuleDescriptor {
    pub id: String,
    pub source: ImportSource,
    pub origin: ModuleOrigin,
    pub artifact: ModuleArtifact,
    pub base_dir: PathBuf,
}

#[derive(Debug)]
pub struct ModuleCacheEntry {
    descriptor: ModuleDescriptor,
    exports: Option<HashMap<String, Value>>,
}

#[derive(Debug)]
pub enum ModuleError {
    NotFound { module: String },
    Io { path: PathBuf, error: io::Error },
    Tokenize { path: PathBuf, error: String },
    Parse { path: PathBuf, error: ParseError },
    Cyclic { module: String },
}

impl std::fmt::Display for ModuleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModuleError::NotFound { module } => {
                write!(f, "Module '{}' could not be located", module)
            }
            ModuleError::Io { path, error } => {
                write!(f, "Failed reading module '{}': {}", path.display(), error)
            }
            ModuleError::Tokenize { path, error } => {
                write!(
                    f,
                    "Tokenizer error while loading '{}': {}",
                    path.display(),
                    error
                )
            }
            ModuleError::Parse { path, error } => {
                write!(
                    f,
                    "Parse error while loading '{}': {}",
                    path.display(),
                    error
                )
            }
            ModuleError::Cyclic { module } => {
                write!(f, "Cyclic module import detected for '{}'", module)
            }
        }
    }
}

impl std::error::Error for ModuleError {}

#[derive(Debug)]
pub struct ModuleLoader {
    script_paths: Vec<PathBuf>,
    stdlib_paths: Vec<PathBuf>,
    compiled_paths: Vec<PathBuf>,
    cache: HashMap<String, ModuleCacheEntry>,
    loading: HashSet<String>,
}

impl ModuleLoader {
    pub fn new() -> Self {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        Self {
            script_paths: vec![current_dir.clone()],
            stdlib_paths: vec![manifest_dir.join("stdlib")],
            compiled_paths: vec![manifest_dir.join("stdlib")],
            cache: HashMap::new(),
            loading: HashSet::new(),
        }
    }

    pub fn add_script_path<P: Into<PathBuf>>(&mut self, path: P) {
        self.script_paths.push(path.into());
    }

    pub fn add_stdlib_path<P: Into<PathBuf>>(&mut self, path: P) {
        let path = path.into();
        self.stdlib_paths.push(path.clone());
        self.compiled_paths.push(path);
    }

    pub fn prepare_module(
        &mut self,
        source: &ImportSource,
        base_dir: Option<&Path>,
    ) -> Result<ModuleDescriptor, ModuleError> {
        match source {
            ImportSource::ScriptPath(path) => {
                let resolved = self.resolve_script_path(path, base_dir)?;
                let canonical = self.canonical_path(&resolved);
                if let Some(entry) = self.cache.get(&canonical) {
                    return Ok(entry.descriptor.clone());
                }
                if !self.loading.insert(canonical.clone()) {
                    return Err(ModuleError::Cyclic {
                        module: canonical.clone(),
                    });
                }
                let descriptor = self.load_script_descriptor(
                    source.clone(),
                    resolved.clone(),
                    canonical.clone(),
                )?;
                self.finish_loading(descriptor, canonical)
            }
            ImportSource::StandardModule(name) => {
                let key = format!("std::{name}");
                if let Some(entry) = self.cache.get(&key) {
                    return Ok(entry.descriptor.clone());
                }
                if !self.loading.insert(key.clone()) {
                    return Err(ModuleError::Cyclic { module: key });
                }
                let descriptor = self.load_standard_descriptor(source.clone(), name)?;
                self.finish_loading(descriptor, key)
            }
            ImportSource::BareModule(name) => {
                // Treat bare module names like script modules with optional extension.
                let candidate = format!("{}.ns", name);
                let resolved = self.resolve_script_path(&candidate, base_dir)?;
                let canonical = self.canonical_path(&resolved);
                if let Some(entry) = self.cache.get(&canonical) {
                    return Ok(entry.descriptor.clone());
                }
                if !self.loading.insert(canonical.clone()) {
                    return Err(ModuleError::Cyclic {
                        module: canonical.clone(),
                    });
                }
                let descriptor = self.load_script_descriptor(
                    ImportSource::BareModule(name.clone()),
                    resolved.clone(),
                    canonical.clone(),
                )?;
                self.finish_loading(descriptor, canonical)
            }
        }
    }

    fn finish_loading(
        &mut self,
        descriptor: ModuleDescriptor,
        key: String,
    ) -> Result<ModuleDescriptor, ModuleError> {
        let program_clone = match &descriptor.artifact {
            ModuleArtifact::Script { program, .. } => Some(program.clone()),
            ModuleArtifact::Compiled { .. } => None,
        };

        self.cache.insert(
            key.clone(),
            ModuleCacheEntry {
                descriptor: descriptor.clone(),
                exports: None,
            },
        );
        self.loading.remove(&key);

        if let Some(program) = program_clone {
            for import in program.find_imports() {
                let base = descriptor.base_dir.as_path();
                self.prepare_module(&import.source, Some(base))?;
            }
        }

        Ok(descriptor)
    }

    fn resolve_script_path(
        &self,
        module_path: &str,
        base_dir: Option<&Path>,
    ) -> Result<PathBuf, ModuleError> {
        let direct = PathBuf::from(module_path);
        if direct.is_absolute() && direct.exists() {
            return Ok(direct);
        }

        let mut candidates = Vec::new();
        if let Some(base) = base_dir {
            candidates.push(base.join(module_path));
        }
        for root in &self.script_paths {
            candidates.push(root.join(module_path));
        }

        for candidate in candidates {
            if candidate.exists() {
                return Ok(candidate);
            }
            if candidate.extension().is_none() {
                let with_ext = candidate.with_extension("ns");
                if with_ext.exists() {
                    return Ok(with_ext);
                }
            }
        }

        Err(ModuleError::NotFound {
            module: module_path.to_string(),
        })
    }

    fn load_script_descriptor(
        &self,
        source: ImportSource,
        path: PathBuf,
        key: String,
    ) -> Result<ModuleDescriptor, ModuleError> {
        let program = self.parse_script(&path)?;
        let base_dir = path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));
        Ok(ModuleDescriptor {
            id: key,
            source,
            origin: ModuleOrigin::Script(path.clone()),
            artifact: ModuleArtifact::Script {
                program,
                path: path.clone(),
            },
            base_dir,
        })
    }

    fn load_standard_descriptor(
        &self,
        source: ImportSource,
        name: &str,
    ) -> Result<ModuleDescriptor, ModuleError> {
        let module_path = Self::normalise_module_name(name);
        for std_path in &self.stdlib_paths {
            let base = std_path.join(&module_path);
            let compiled = base.with_extension("nvc");
            if compiled.exists() {
                let bytes = match fs::read(&compiled) {
                    Ok(buf) => buf,
                    Err(error) => {
                        return Err(ModuleError::Io {
                            path: compiled.clone(),
                            error,
                        });
                    }
                };
                let base_dir = compiled
                    .parent()
                    .map(Path::to_path_buf)
                    .unwrap_or_else(|| std_path.clone());
                return Ok(ModuleDescriptor {
                    id: format!("std::{}", name),
                    source,
                    origin: ModuleOrigin::Compiled(compiled.clone()),
                    artifact: ModuleArtifact::Compiled {
                        path: compiled,
                        bytes,
                    },
                    base_dir,
                });
            }

            let script = base.with_extension("ns");
            if script.exists() {
                let program = self.parse_script(&script)?;
                let base_dir = script
                    .parent()
                    .map(Path::to_path_buf)
                    .unwrap_or_else(|| std_path.clone());
                return Ok(ModuleDescriptor {
                    id: format!("std::{}", name),
                    source,
                    origin: ModuleOrigin::Standard(script.clone()),
                    artifact: ModuleArtifact::Script {
                        program,
                        path: script,
                    },
                    base_dir,
                });
            }
        }

        Err(ModuleError::NotFound {
            module: format!("std::{}", name),
        })
    }

    fn parse_script(&self, path: &Path) -> Result<Program, ModuleError> {
        let content = fs::read_to_string(path).map_err(|error| ModuleError::Io {
            path: path.to_path_buf(),
            error,
        })?;
        let mut tokenizer = Tokenizer::new(&content);
        let tokens = tokenizer
            .tokenize()
            .map_err(|error| ModuleError::Tokenize {
                path: path.to_path_buf(),
                error,
            })?;
        let mut parser = Parser::new(tokens);
        parser.parse().map_err(|error| ModuleError::Parse {
            path: path.to_path_buf(),
            error,
        })
    }

    fn canonical_path(&self, path: &Path) -> String {
        fs::canonicalize(path)
            .unwrap_or_else(|_| path.to_path_buf())
            .display()
            .to_string()
    }

    fn normalise_module_name(name: &str) -> PathBuf {
        let normalised = name.replace("::", "/");
        let mut path = PathBuf::new();
        for segment in normalised.split(&['/', '.'][..]) {
            if segment.is_empty() {
                continue;
            }
            path.push(segment);
        }
        path
    }

    pub fn exports_cloned(&self, id: &str) -> Option<HashMap<String, Value>> {
        self.cache
            .get(id)
            .and_then(|entry| entry.exports.as_ref().cloned())
    }

    pub fn store_exports(&mut self, id: &str, exports: HashMap<String, Value>) {
        if let Some(entry) = self.cache.get_mut(id) {
            entry.exports = Some(exports);
        }
    }

    pub fn descriptor(&self, id: &str) -> Option<&ModuleDescriptor> {
        self.cache.get(id).map(|entry| &entry.descriptor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn loader() -> ModuleLoader {
        let mut loader = ModuleLoader::new();
        loader.add_script_path(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/modules"));
        loader
    }

    #[test]
    fn resolve_script_module() {
        let mut loader = loader();
        let desc = loader
            .prepare_module(
                &ImportSource::ScriptPath("tests/modules/sample_module.ns".to_string()),
                None,
            )
            .expect("module to load");
        assert!(matches!(desc.origin, ModuleOrigin::Script(_)));
        assert!(matches!(desc.artifact, ModuleArtifact::Script { .. }));
    }

    #[test]
    fn resolve_stdlib_module() {
        let mut loader = ModuleLoader::new();
        let desc = loader
            .prepare_module(&ImportSource::StandardModule("vector".to_string()), None)
            .expect("vector stdlib module");
        match desc.origin {
            ModuleOrigin::Standard(_) | ModuleOrigin::Compiled(_) => {}
            ModuleOrigin::Script(_) => {}
        }
    }
}
