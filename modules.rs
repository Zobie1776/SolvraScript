#![allow(dead_code)]

use crate::ast::{ExportItem, ImportSource, Program, Stmt};
use crate::interpreter::Value;
use crate::parser::{ParseError, Parser};
use crate::stdlib_registry::{StdlibContext, StdlibRegistry};
use crate::tokenizer::Tokenizer;
use crate::vm::compiler;
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

pub mod core_vm;

const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x1000_0000_01b3;

static GLOBAL_HOT_RELOAD: AtomicBool = AtomicBool::new(false);

pub fn set_global_hot_reload(enabled: bool) {
    GLOBAL_HOT_RELOAD.store(enabled, Ordering::Relaxed);
}

#[derive(Debug, Clone)]
pub enum ModuleOrigin {
    Script(PathBuf),
    Standard(PathBuf),
    Compiled(PathBuf),
}

#[derive(Debug, Clone)]
pub struct CompiledModule {
    pub path: PathBuf,
    pub fingerprint: String,
}

#[derive(Debug, Clone)]
pub enum ModuleArtifact {
    Script {
        program: Program,
        path: PathBuf,
        fingerprint: String,
        compiled: Option<CompiledModule>,
    },
    Compiled {
        path: PathBuf,
        bytes: Vec<u8>,
        fingerprint: String,
    },
}

#[derive(Debug, Clone)]
pub struct ModuleDescriptor {
    pub id: String,
    pub source: ImportSource,
    pub origin: ModuleOrigin,
    pub artifact: ModuleArtifact,
    pub base_dir: PathBuf,
    pub declared_exports: Vec<String>,
}

#[derive(Debug)]
pub struct ModuleCacheEntry {
    descriptor: ModuleDescriptor,
    exports: Option<HashMap<String, Value>>,
}

struct ParsedScript {
    program: Program,
    source: String,
}

#[derive(Debug)]
pub enum ModuleError {
    NotFound { module: String },
    Io { path: PathBuf, error: io::Error },
    Tokenize { path: PathBuf, error: String },
    Parse { path: PathBuf, error: ParseError },
    Cyclic { module: String },
    Compile { path: PathBuf, error: String },
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
            ModuleError::Compile { path, error } => {
                write!(f, "Compilation failed for '{}': {}", path.display(), error)
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
    stdlib: StdlibContext,
    cache: HashMap<String, ModuleCacheEntry>,
    loading: HashSet<String>,
    cache_dir: PathBuf,
    hot_reload: bool,
}

impl Default for ModuleLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl ModuleLoader {
    pub fn new() -> Self {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let current_dir = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let stdlib_root = manifest_dir.join("lib").join("std");
        let stdlib_registry = StdlibRegistry::with_defaults(&stdlib_root);
        let stdlib = StdlibContext::new(stdlib_registry);
        let cache_dir = manifest_dir
            .parent()
            .map(|parent| parent.join("target/solvra_modules"))
            .unwrap_or_else(|| manifest_dir.join("target/solvra_modules"));
        let env_hot_reload = env::var("SOLVRA_HOT_RELOAD")
            .map(|value| {
                let lower = value.to_ascii_lowercase();
                matches!(lower.as_str(), "1" | "true" | "yes" | "on")
            })
            .unwrap_or(false);
        let hot_reload = GLOBAL_HOT_RELOAD.load(Ordering::Relaxed) || env_hot_reload;
        Self {
            script_paths: vec![current_dir.clone()],
            stdlib_paths: vec![stdlib_root.clone(), manifest_dir.join("stdlib")],
            compiled_paths: vec![
                stdlib_root.clone(),
                manifest_dir.join("stdlib"),
                cache_dir.clone(),
            ],
            stdlib,
            cache: HashMap::new(),
            loading: HashSet::new(),
            cache_dir,
            hot_reload,
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
                if self.hot_reload {
                    self.cache.remove(&canonical);
                }
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
                if self.hot_reload {
                    self.cache.remove(&key);
                }
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
                let candidate = format!("{}.svs", name);
                let resolved = self.resolve_script_path(&candidate, base_dir)?;
                let canonical = self.canonical_path(&resolved);
                if self.hot_reload {
                    self.cache.remove(&canonical);
                }
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
                let with_ext = candidate.with_extension("svs");
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
        let parsed = self.parse_script(&path)?;
        let fingerprint = Self::compute_fingerprint(&parsed.source);
        let declared_exports = self.collect_declared_exports(&parsed.program);
        let compiled = self.compile_script_if_needed(&key, &path, &parsed.program, &fingerprint)?;
        let base_dir = path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));
        Ok(ModuleDescriptor {
            id: key,
            source,
            origin: ModuleOrigin::Script(path.clone()),
            artifact: ModuleArtifact::Script {
                program: parsed.program,
                path: path.clone(),
                fingerprint: fingerprint.clone(),
                compiled: compiled.clone(),
            },
            base_dir,
            declared_exports,
        })
    }

    fn load_standard_descriptor(
        &mut self,
        source: ImportSource,
        name: &str,
    ) -> Result<ModuleDescriptor, ModuleError> {
        let module_id = format!("std::{name}");
        if let Some(script_path) = self.stdlib.resolve(name) {
            let parsed = self.parse_script(&script_path)?;
            let fingerprint = Self::compute_fingerprint(&parsed.source);
            let declared_exports = self.collect_declared_exports(&parsed.program);
            let compiled = self.compile_script_if_needed(
                &module_id,
                &script_path,
                &parsed.program,
                &fingerprint,
            )?;
            let base_dir = script_path
                .parent()
                .map(Path::to_path_buf)
                .unwrap_or_else(|| PathBuf::from("."));
            let origin_path = PathBuf::from(&script_path);
            return Ok(ModuleDescriptor {
                id: module_id.clone(),
                source,
                origin: ModuleOrigin::Standard(origin_path.clone()),
                artifact: ModuleArtifact::Script {
                    program: parsed.program,
                    path: origin_path,
                    fingerprint,
                    compiled,
                },
                base_dir,
                declared_exports,
            });
        }

        let module_path = Self::normalise_module_name(name);
        for std_path in &self.stdlib_paths {
            let base = std_path.join(&module_path);
            for ext in ["svc", "nvc"] {
                let compiled = base.with_extension(ext);
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
                    let fingerprint = Self::compute_fingerprint_bytes(&bytes);
                    let base_dir = compiled
                        .parent()
                        .map(Path::to_path_buf)
                        .unwrap_or_else(|| std_path.clone());
                    return Ok(ModuleDescriptor {
                        id: module_id.clone(),
                        source: source.clone(),
                        origin: ModuleOrigin::Compiled(compiled.clone()),
                        artifact: ModuleArtifact::Compiled {
                            path: compiled,
                            bytes,
                            fingerprint,
                        },
                        base_dir,
                        declared_exports: Vec::new(),
                    });
                }
            }

            let script = base.with_extension("svs");
            if script.exists() {
                let parsed = self.parse_script(&script)?;
                let fingerprint = Self::compute_fingerprint(&parsed.source);
                let declared_exports = self.collect_declared_exports(&parsed.program);
                let compiled = self.compile_script_if_needed(
                    &module_id,
                    &script,
                    &parsed.program,
                    &fingerprint,
                )?;
                let base_dir = script
                    .parent()
                    .map(Path::to_path_buf)
                    .unwrap_or_else(|| std_path.clone());
                return Ok(ModuleDescriptor {
                    id: module_id.clone(),
                    source,
                    origin: ModuleOrigin::Standard(script.clone()),
                    artifact: ModuleArtifact::Script {
                        program: parsed.program,
                        path: script,
                        fingerprint,
                        compiled,
                    },
                    base_dir,
                    declared_exports,
                });
            }
        }

        Err(ModuleError::NotFound {
            module: format!("std::{}", name),
        })
    }

    fn parse_script(&self, path: &Path) -> Result<ParsedScript, ModuleError> {
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
        let program = parser.parse().map_err(|error| ModuleError::Parse {
            path: path.to_path_buf(),
            error,
        })?;
        Ok(ParsedScript {
            program,
            source: content,
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

    fn compile_script_if_needed(
        &self,
        canonical: &str,
        _path: &Path,
        program: &Program,
        fingerprint: &str,
    ) -> Result<Option<CompiledModule>, ModuleError> {
        let cache_path = self.cache_file_for(canonical, fingerprint);
        if !self.hot_reload && cache_path.exists() {
            return Ok(Some(CompiledModule {
                path: cache_path,
                fingerprint: fingerprint.to_string(),
            }));
        }

        let compile_program = Self::prepare_program_for_compilation(program);
        if compile_program.find_functions().is_empty() {
            return Ok(None);
        }

        let bytes = match compiler::compile_program(&compile_program) {
            Ok(bytes) => bytes,
            Err(_) => {
                return Ok(None);
            }
        };

        if let Some(parent) = cache_path.parent() {
            fs::create_dir_all(parent).map_err(|error| ModuleError::Io {
                path: parent.to_path_buf(),
                error,
            })?;
        }

        fs::write(&cache_path, &bytes).map_err(|error| ModuleError::Io {
            path: cache_path.clone(),
            error,
        })?;

        Ok(Some(CompiledModule {
            path: cache_path,
            fingerprint: fingerprint.to_string(),
        }))
    }

    fn prepare_program_for_compilation(program: &Program) -> Program {
        let mut statements = Vec::new();
        for stmt in &program.statements {
            match stmt {
                Stmt::ExportDecl { decl } => match &decl.item {
                    ExportItem::Function(func) => {
                        statements.push(Stmt::FunctionDecl { decl: func.clone() });
                    }
                    ExportItem::Variable(var) => {
                        statements.push(Stmt::VariableDecl { decl: var.clone() });
                    }
                    _ => {}
                },
                other => statements.push(other.clone()),
            }
        }
        Program::new(statements, program.position.clone())
    }

    fn cache_file_for(&self, canonical: &str, fingerprint: &str) -> PathBuf {
        let hash = Self::hash_bytes(canonical.as_bytes().iter().copied());
        self.cache_dir.join(format!("{hash}-{fingerprint}.svc"))
    }

    fn compute_fingerprint(source: &str) -> String {
        Self::hash_bytes(
            source
                .as_bytes()
                .iter()
                .copied()
                .chain(env!("CARGO_PKG_VERSION").as_bytes().iter().copied()),
        )
    }

    fn compute_fingerprint_bytes(bytes: &[u8]) -> String {
        Self::hash_bytes(
            bytes
                .iter()
                .copied()
                .chain(env!("CARGO_PKG_VERSION").as_bytes().iter().copied()),
        )
    }

    fn hash_bytes<I: IntoIterator<Item = u8>>(iter: I) -> String {
        let mut hash = FNV_OFFSET_BASIS;
        for byte in iter {
            hash ^= byte as u64;
            hash = hash.wrapping_mul(FNV_PRIME);
        }
        format!("{hash:016x}")
    }

    fn collect_declared_exports(&self, program: &Program) -> Vec<String> {
        let mut names: HashSet<String> = HashSet::new();
        for decl in program.find_exports() {
            match &decl.item {
                ExportItem::Function(func) => {
                    names.insert(func.name.clone());
                }
                ExportItem::Variable(var) => {
                    names.insert(var.name.clone());
                }
                ExportItem::Class(class) => {
                    names.insert(class.name.clone());
                }
                ExportItem::Interface(iface) => {
                    names.insert(iface.name.clone());
                }
                ExportItem::Type(ty) => {
                    names.insert(ty.name.clone());
                }
                ExportItem::Module(name) => {
                    names.insert(name.clone());
                }
            }
        }
        let mut list: Vec<String> = names.into_iter().collect();
        list.sort();
        list
    }

    pub fn set_hot_reload(&mut self, enabled: bool) {
        self.hot_reload = enabled;
        if enabled {
            self.cache.clear();
        }
    }

    pub fn invalidate(&mut self, id: &str) {
        self.cache.remove(id);
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
                &ImportSource::ScriptPath("tests/modules/sample_module.svs".to_string()),
                None,
            )
            .expect("module to load");
        assert!(matches!(desc.origin, ModuleOrigin::Script(_)));
        match desc.artifact {
            ModuleArtifact::Script { compiled, .. } => {
                assert!(
                    compiled.is_some(),
                    "expected compiled artifact to be present"
                )
            }
            other => panic!("expected script artifact, got {other:?}"),
        }
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

    #[test]
    fn export_declarations_are_recorded() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let module_path = dir.path().join("export_mod.svs");
        fs::write(
            &module_path,
            "export fn greet(name) { return name; }\nexport value;\nlet value = 42;\n",
        )
        .expect("write module");

        let mut loader = ModuleLoader::new();
        loader.add_script_path(dir.path().to_path_buf());
        let descriptor = loader
            .prepare_module(
                &ImportSource::ScriptPath(module_path.to_string_lossy().to_string()),
                None,
            )
            .expect("load export module");

        assert_eq!(
            descriptor.declared_exports,
            vec!["greet".to_string(), "value".to_string()]
        );

        match descriptor.artifact {
            ModuleArtifact::Script { compiled, .. } => {
                assert!(
                    compiled.is_some(),
                    "expected compiled artifact for export module"
                );
            }
            other => panic!("expected script artifact, got {other:?}"),
        }
    }
}
