#![allow(dead_code)]

use crate::ast::{ExportItem, ImportSource, Program, Stmt};
use crate::interpreter::Value;
use crate::parser::{ParseError, Parser};
use crate::stdlib_registry::{StdlibContext, StdlibRegistry};
use crate::tokenizer::Tokenizer;
use crate::vm::compiler as vm_compiler;
use std::collections::{HashMap, HashSet};
use std::env;
use std::ffi::OsStr;
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

#[derive(Debug, Clone, PartialEq, Eq)]
enum ModuleStatus {
    Initializing,
    Ready,
}

#[derive(Debug)]
pub struct ModuleCacheEntry {
    descriptor: Option<ModuleDescriptor>,
    exports: Option<HashMap<String, Value>>,
    status: ModuleStatus,
}

#[derive(Debug, Clone)]
enum CachedStandardModule {
    Script(PathBuf),
    Compiled(PathBuf),
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
    cache_dir: PathBuf,
    hot_reload: bool,
    stdx_root: PathBuf,
    compat_root: PathBuf,
    standard_resolution_cache: HashMap<String, CachedStandardModule>,
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
        let src_root = manifest_dir.join("src");
        let stdx_root = src_root.join("stdx");
        println!("[debug] ModuleLoader stdx_root: {}", stdx_root.display());
        let stdx_core_root = stdx_root.join("core");
        println!("[debug] ModuleLoader stdx_core_root: {}", stdx_core_root.display());
        let compat_root = manifest_dir.join("compat").join("legacy_shims");
        let stdlib_registry = StdlibRegistry::with_defaults(&manifest_dir);
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
            script_paths: vec![current_dir.clone(), src_root.clone(), stdx_root.clone(),
            stdx_core_root.clone()],
            stdlib_paths: vec![stdx_root.clone(), compat_root.clone()],
            compiled_paths: vec![stdx_root.clone(), compat_root.clone(), cache_dir.clone()],
            stdlib,
            cache: HashMap::new(),
            cache_dir,
            hot_reload,
            stdx_root,
            compat_root,
            standard_resolution_cache: HashMap::new(),
        }
    }

    pub fn add_script_path<P: Into<PathBuf>>(&mut self, path: P) {
        let path = path.into();
        if !self.script_paths.contains(&path) {
            self.script_paths.push(path);
        }
    }

    pub fn add_stdlib_path<P: Into<PathBuf>>(&mut self, path: P) {
        let path = path.into();
        self.stdlib_paths.push(path.clone());
        self.compiled_paths.push(path);
    }

    pub fn script_search_paths(&self) -> Vec<PathBuf> {
        self.script_paths.clone()
        
    }

    pub fn preload_standard_modules(&mut self) {
        for name in self.stdlib.module_names() {
            let _ = self.prepare_module(&ImportSource::StandardModule(name), None);
        }
    }

    pub fn preload_compat_shims(&mut self) {
        if let Ok(entries) = fs::read_dir(&self.compat_root) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() && path.extension().and_then(|ext| ext.to_str()).map(|ext| ext == "svs").unwrap_or(false) {
                    if let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) {
                        let module = format!("compat.legacy_shims.{}", stem);
                        let _ = self.prepare_module(&ImportSource::BareModule(module), None);
                    }
                }
            }
        }
    }

	pub fn get_key(&self, source: &ImportSource, base_dir: Option<&Path>) -> Result<String, ModuleError> {
		match source {
			ImportSource::ScriptPath(path) => {
				let resolved = self.resolve_script_path(path, base_dir)?;
				Ok(self.canonical_path_buf(&resolved).display().to_string())
			}
			ImportSource::StandardModule(name) | ImportSource::BareModule(name) => {
				if name.starts_with("stdx") || name.starts_with("std") || name.starts_with("compat") {
					Ok(format!("std::{}", Self::normalise_module_name(name).to_string_lossy()))
				} else {
					let candidate = format!("{}.svs", name);
					let resolved = self.resolve_script_path(&candidate, base_dir)?;
					Ok(self.canonical_path_buf(&resolved).display().to_string())
				}
			}
		}
	}


    pub fn prepare_module(
        &mut self,
        source: &ImportSource,
        base_dir: Option<&Path>,
    ) -> Result<ModuleDescriptor, ModuleError> {
        let key = self.get_key(source, base_dir)?;

        if self.hot_reload {
            self.cache.remove(&key);
        }

        if let Some(entry) = self.cache.get(&key) {
            if entry.status == ModuleStatus::Initializing {
                return Err(ModuleError::Cyclic { module: key });
            }
            if let Some(descriptor) = &entry.descriptor {
                return Ok(descriptor.clone());
            }
        }

        self.cache.insert(
            key.clone(),
            ModuleCacheEntry {
                descriptor: None,
                exports: None,
                status: ModuleStatus::Initializing,
            },
        );

        let descriptor = match source {
            ImportSource::ScriptPath(path) => {
                let resolved = self.resolve_script_path(path, base_dir)?;
                self.load_script_descriptor(source.clone(), resolved, key.clone())
            }
            ImportSource::StandardModule(name) => {
                self.load_standard_descriptor(source.clone(), name)
            }
            ImportSource::BareModule(name) => {
                if name.starts_with("stdx") || name.starts_with("std") || name.starts_with("compat") {
                    self.load_standard_descriptor(source.clone(), name)
                } else {
                    let candidate = format!("{}.svs", name);
                    let resolved = self.resolve_script_path(&candidate, base_dir)?;
                    self.load_script_descriptor(source.clone(), resolved, key.clone())
                }
            }
        } ?;

        let program_clone = match &descriptor.artifact {
            ModuleArtifact::Script { program, .. } => Some(program.clone()),
            _ => None,
        };

        self.cache.insert(
            key.clone(),
            ModuleCacheEntry {
                descriptor: Some(descriptor.clone()),
                exports: None,
                status: ModuleStatus::Initializing,
            },
        );

        if let Some(program) = program_clone {
            for import in program.find_imports() {
                self.prepare_module(&import.source, Some(&descriptor.base_dir))?;
            }
        }

        if let Some(entry) = self.cache.get_mut(&key) {
            entry.status = ModuleStatus::Ready;
        }

        Ok(descriptor)
    }

    fn resolve_script_path(
        &self,
        module_path: &str,
        base_dir: Option<&Path>,
    ) -> Result<PathBuf, ModuleError> {
        println!(
            "[debug] resolve_script_path: '{}' from base {:?}",
            module_path, base_dir
        );
        let direct = PathBuf::from(module_path);
        if direct.is_absolute() && direct.exists() {
            println!(
                "[debug] Resolved absolute script '{}' -> {}",
                module_path,
                direct.display()
            );
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
                println!(
                    "[debug] Found script '{}' -> {}",
                    module_path,
                    candidate.display()
                );
                if candidate.is_file() {
                    return Ok(candidate);
                }
                if candidate.is_dir() {
                    let mod_svs = candidate.join("mod.svs");
                    if mod_svs.exists() && mod_svs.is_file() {
                        return Ok(mod_svs);
                    }
                    let with_ext = candidate.with_extension("svs");
                    if with_ext.exists() {
                        return Ok(with_ext);
                    }
                }
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
        println!("[debug] load_standard_descriptor: {}", name);
        let module_id = format!("std::{}", name);
        if let Some(resolution) = self.standard_resolution_cache.get(name).cloned() {
            return self.build_standard_descriptor_from_resolution(source, &module_id, resolution);
        }
        if let Some(script_path) = self.stdlib.resolve(name) {
            self.standard_resolution_cache.insert(
                name.to_string(),
                CachedStandardModule::Script(script_path.clone()),
            );
            return self.build_standard_script_descriptor(source, &module_id, script_path);
        }

        let module_path = Self::normalise_module_name(name);
        for std_path in &self.stdlib_paths {
            let search_path = if std_path == &self.stdx_root {
                let components: Vec<_> = module_path.iter().collect();
                if let Some(first) = components.first() {
                    if first == &OsStr::new("std") || first == &OsStr::new("stdx") {
                        Self::strip_prefix_segments(&module_path, &[first.to_str().unwrap_or("")])
                    } else {
                        module_path.clone()
                    }
                } else {
                    module_path.clone()
                }
            } else if std_path == &self.compat_root {
                Self::strip_prefix_segments(&module_path, &["compat", "legacy_shims"])
            } else {
                module_path.clone()
            };
            let base = std_path.join(&search_path);
            let module_leaf = search_path
                .file_name()
                .and_then(|leaf| leaf.to_str())
                .unwrap_or("")
                .to_string();
            for ext in ["svc", "nvc"] {
                let compiled = base.with_extension(ext);
                if compiled.exists() {
                    self.standard_resolution_cache.insert(
                        name.to_string(),
                        CachedStandardModule::Compiled(compiled.clone()),
                    );
                    return self.build_standard_compiled_descriptor(source, &module_id, compiled);
                }
            }

            let script = base.with_extension("svs");
            if script.exists() {
                self.standard_resolution_cache.insert(
                    name.to_string(),
                    CachedStandardModule::Script(script.clone()),
                );
                return self.build_standard_script_descriptor(source, &module_id, script);
            }

            let mod_script = base.join("mod.svs");
            if mod_script.exists() {
                self.standard_resolution_cache.insert(
                    name.to_string(),
                    CachedStandardModule::Script(mod_script.clone()),
                );
                return self.build_standard_script_descriptor(source, &module_id, mod_script);
            }

            if !module_leaf.is_empty() {
                let leaf_script = base.join(format!("{module_leaf}.svs"));
                if leaf_script.exists() {
                    self.standard_resolution_cache.insert(
                        name.to_string(),
                        CachedStandardModule::Script(leaf_script.clone()),
                    );
                    return self.build_standard_script_descriptor(source, &module_id, leaf_script);
                }
            }
        }

        Err(ModuleError::NotFound {
            module: format!("std::{}", name),
        })
    }

    fn build_standard_descriptor_from_resolution(
        &self,
        source: ImportSource,
        module_id: &str,
        cached: CachedStandardModule,
    ) -> Result<ModuleDescriptor, ModuleError> {
        match cached {
            CachedStandardModule::Script(path) => {
                self.build_standard_script_descriptor(source, module_id, path)
            }
            CachedStandardModule::Compiled(path) => {
                self.build_standard_compiled_descriptor(source, module_id, path)
            }
        }
    }

    fn build_standard_script_descriptor(
        &self,
        source: ImportSource,
        module_id: &str,
        script_path: PathBuf,
    ) -> Result<ModuleDescriptor, ModuleError> {
        let parsed = self.parse_script(&script_path)?;
        let fingerprint = Self::compute_fingerprint(&parsed.source);
        let declared_exports = self.collect_declared_exports(&parsed.program);
        let compiled =
            self.compile_script_if_needed(module_id, &script_path, &parsed.program, &fingerprint)?;
        let base_dir = script_path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));
        Ok(ModuleDescriptor {
            id: module_id.to_string(),
            source,
            origin: ModuleOrigin::Standard(script_path.clone()),
            artifact: ModuleArtifact::Script {
                program: parsed.program,
                path: script_path,
                fingerprint,
                compiled,
            },
            base_dir,
            declared_exports,
        })
    }

    fn build_standard_compiled_descriptor(
        &self,
        source: ImportSource,
        module_id: &str,
        compiled_path: PathBuf,
    ) -> Result<ModuleDescriptor, ModuleError> {
        let bytes = match fs::read(&compiled_path) {
            Ok(buf) => buf,
            Err(error) => {
                return Err(ModuleError::Io {
                    path: compiled_path.clone(),
                    error,
                });
            }
        };
        let fingerprint = Self::compute_fingerprint_bytes(&bytes);
        let base_dir = compiled_path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));
        Ok(ModuleDescriptor {
            id: module_id.to_string(),
            source,
            origin: ModuleOrigin::Compiled(compiled_path.clone()),
            artifact: ModuleArtifact::Compiled {
                path: compiled_path,
                bytes,
                fingerprint,
            },
            base_dir,
            declared_exports: Vec::new(),
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
        self.canonical_path_buf(path).display().to_string()
    }

    fn canonical_path_buf(&self, path: &Path) -> PathBuf {
        path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
    }

    fn normalise_module_name(name: &str) -> PathBuf {
        name.replace("::", "/")
            .split('.')
            .collect::<PathBuf>()
    }

    fn strip_prefix_segments(path: &Path, prefixes: &[&str]) -> PathBuf {
        let components: Vec<_> = path.iter().collect();
        if components.len() >= prefixes.len()
            && prefixes
                .iter()
                .enumerate()
                .all(|(i, prefix)| components[i] == OsStr::new(prefix))
        {
            let mut remainder = PathBuf::new();
            for component in &components[prefixes.len()..] {
                remainder.push(component);
            }
            remainder
        } else {
            let mut fallback = PathBuf::new();
            for component in components {
                fallback.push(component);
            }
            fallback
        }
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
        self.cache.get(id).and_then(|entry| entry.descriptor.as_ref())
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

        let bytes = match vm_compiler::compile_program(&compile_program) {
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
        let mut result = Program::new(statements, program.position.clone());
        result.ensure_entry_point();
        result
    }

    fn cache_file_for(&self, canonical: &str, fingerprint: &str) -> PathBuf {
        let hash = Self::hash_bytes(canonical.as_bytes().iter().copied());
        self.cache_dir.join(format!("{}-{}", hash, fingerprint))
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
        format!("{:016x}", hash)
    }

    fn collect_declared_exports(&self, program: &Program) -> Vec<String> {
        let mut names: HashSet<String> = HashSet::new();
        for decl in program.find_exports() {
            match &decl.item {
                ExportItem::Function(func) => {
                    names.insert(func.name.to_string());
                }
                ExportItem::Variable(var) => {
                    names.insert(var.name.to_string());
                }
                ExportItem::Class(class) => {
                    names.insert(class.name.to_string());
                }
                ExportItem::Interface(iface) => {
                    names.insert(iface.name.to_string());
                }
                ExportItem::Type(ty) => {
                    names.insert(ty.name.to_string());
                }
                ExportItem::Module(name) => {
                    names.insert(name.clone());
                }
                ExportItem::Symbol { name, alias } => {
                    let export_name = alias
                        .as_ref()
                        .map(|sym| sym.to_string())
                        .unwrap_or_else(|| name.to_string());
                    names.insert(export_name);
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
        loader
            .add_script_path(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("stdx_tests/modules"));
        loader
    }

    #[test]
    fn resolve_script_module() {
        //eprintln!("[RESOLVE PATH] import={} base={:?}", import, base);
        let mut loader = loader();
        let desc = loader
            .prepare_module(
                //eprintln!("[MODULE PREPARE] {}", name);
                &ImportSource::ScriptPath("stdx_tests/modules/sample_module.svs".to_string()),
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
            other => panic!("expected script artifact, got {:?}", other),
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
            other => panic!("expected script artifact, got {:?}", other),
        }
    }

    #[test]
    fn resolve_std_io_import() {
        let mut loader = ModuleLoader::new();
        loader
            .prepare_module(&ImportSource::BareModule("std.io".to_string()), None)
            .expect("std.io module");
    }

    #[test]
    fn resolve_std_core_string() {
        let mut loader = ModuleLoader::new();
        loader
            .prepare_module(
                &ImportSource::BareModule("std.core.string".to_string()),
                None,
            )
            .expect("std.core.string module");
    }

    #[test]
    fn resolve_std_core_prelude() {
        let mut loader = ModuleLoader::new();
        loader
            .prepare_module(
                &ImportSource::BareModule("std.core.prelude".to_string()),
                None,
            )
            .expect("std.core.prelude module");
    }

    #[test]
    fn resolve_stdx_ai_module() {
        let mut loader = ModuleLoader::new();
        loader
            .prepare_module(&ImportSource::BareModule("stdx.ai".to_string()), None)
            .expect("stdx.ai module");
    }
    /*
        #[test]
        fn resolve_compat_shim_module() {
            let mut loader = ModuleLoader::new();
            loader
                .prepare_module(
                    &ImportSource::BareModule("compat.legacy_shims.io".to_string()),
                    None,
                )
                .expect("compat shim module");
        }
    */
    #[test]
    fn resolve_std_option_module() {
        let mut loader = ModuleLoader::new();
        loader
            .prepare_module(&ImportSource::StandardModule("option".to_string()), None)
            .expect("option module");
    }

    #[test]
    fn resolve_std_result_module() {
        let mut loader = ModuleLoader::new();
        loader
            .prepare_module(&ImportSource::StandardModule("result".to_string()), None)
            .expect("result module");
    }

    #[test]
    fn test_canonical_path_variations() {
        let tmp_dir = tempfile::tempdir().expect("create temp dir");
        let file_a_path = tmp_dir.path().join("a");
        fs::create_dir(&file_a_path).expect("create file_a_path");
        let target_file = file_a_path.join("b.svs");
        fs::write(&target_file, "fn main() {}").expect("create target file");

        let loader = ModuleLoader::new(); // Use a loader instance for canonical_path_buf

        let canonical_target = loader.canonical_path_buf(&target_file).display().to_string();

        // Test with ./
        let path_with_dot = tmp_dir.path().join("a/./b.svs");
        let canonical_dot = loader.canonical_path_buf(&path_with_dot).display().to_string();
        assert_eq!(canonical_target, canonical_dot, "Path with '.' should canonicalize correctly");

        // Test with ../
        let path_with_double_dot = tmp_dir.path().join("a/c/../b.svs");
        fs::create_dir(&tmp_dir.path().join("a/c")).expect("create a/c"); // Create intermediate directory
        let canonical_double_dot = loader.canonical_path_buf(&path_with_double_dot).display().to_string();
        assert_eq!(canonical_target, canonical_double_dot, "Path with '..' should canonicalize correctly");

        // Test with just the direct relative path
        let direct_relative = tmp_dir.path().join("a/b.svs");
        let canonical_direct_relative = loader.canonical_path_buf(&direct_relative).display().to_string();
        assert_eq!(canonical_target, canonical_direct_relative, "Direct relative path should canonicalize correctly");
    }
}
