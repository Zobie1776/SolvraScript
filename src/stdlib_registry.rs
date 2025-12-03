//==================================================
// File: stdlib_registry.rs
//==================================================
// Author: ZobieLabs
// License: Duality Public License (DPL v1.0)
// Goal: Track SolvraScript stdlib module locations for angled import syntax
// Objective: Resolve <module> imports to canonical stdx paths with caching
//==================================================

use std::collections::HashMap;
use std::path::{Path, PathBuf};

//==================================================
// Section 1.0 - Registry Types
//==================================================
// @TODO[StdlibPhase3]: Register additional modules once implementation stabilizes.
// @ZNOTE[Resolver]: Registry stays small so lookups remain predictable and testable.

#[derive(Debug, Clone)]
pub struct StdlibRegistry {
    modules: HashMap<String, PathBuf>,
}

impl StdlibRegistry {
    pub fn with_defaults(manifest_dir: impl AsRef<Path>) -> Self {
        let manifest_dir = manifest_dir.as_ref();
        let stdx_root = manifest_dir.join("src").join("stdx");
        let mut registry = Self {
            modules: HashMap::new(),
        };
        registry.register("core", stdx_root.join("core.svs"));
        registry.register("string", stdx_root.join("string.svs"));
        registry.register("vector", stdx_root.join("core/vector.svs"));
        registry.register("option", stdx_root.join("core/option.svs"));
        registry.register("result", stdx_root.join("core/result.svs"));
        registry.register("iter", stdx_root.join("core/iter.svs"));
        registry.register("math", stdx_root.join("math.svs"));

        registry.register("io", stdx_root.join("io/io.svs"));
        registry.register("fs", stdx_root.join("fs.svs"));
        registry.register("os", stdx_root.join("os/mod.svs"));
        registry.register("sys", stdx_root.join("sys/mod.svs"));
        registry.register("system", stdx_root.join("system/mod.svs"));
        registry.register("path", stdx_root.join("path.svs"));
        registry.register("datetime", stdx_root.join("datetime/mod.svs"));
        registry.register("stdx/datetime", stdx_root.join("datetime/mod.svs"));
        registry.register("random", stdx_root.join("random.svs"));
        registry.register("time", stdx_root.join("time/mod.svs"));
        registry.register("json", stdx_root.join("json/mod.svs"));

        registry.register("web", stdx_root.join("web/mod.svs"));
        registry.register("networking", stdx_root.join("networking/mod.svs"));
        registry.register("net", stdx_root.join("networking/mod.svs"));
        registry.register("game", stdx_root.join("game/mod.svs"));
        registry.register("crypto", stdx_root.join("crypto/mod.svs"));

        // AI Modules (stdx)
        registry.register("ai_tensor", stdx_root.join("ai/tensor.svs"));
        registry.register("ai_nn", stdx_root.join("ai/nn.svs"));
        registry.register("ai_data", stdx_root.join("ai/data.svs"));
        registry.register("ai_model", stdx_root.join("ai/model.svs"));
        registry.register("ai_utils", stdx_root.join("ai/utils.svs"));
        registry.register("ai_bridge", stdx_root.join("ai/bridge.svs"));

        registry
    }

    pub fn module_names(&self) -> Vec<String> {
        self.modules.keys().cloned().collect()
    }

    pub fn register(&mut self, name: &str, path: PathBuf) {
        self.modules.insert(name.to_string(), path);
    }

    pub fn resolve(&self, name: &str) -> Option<PathBuf> {
        self.modules.get(name).cloned()
    }
}

#[derive(Debug, Clone)]
pub struct StdlibContext {
    registry: StdlibRegistry,
    cache: HashMap<String, PathBuf>,
}

impl StdlibContext {
    pub fn new(registry: StdlibRegistry) -> Self {
        Self {
            registry,
            cache: HashMap::new(),
        }
    }

    pub fn module_names(&self) -> Vec<String> {
        self.registry.module_names()
    }

    pub fn resolve(&mut self, name: &str) -> Option<PathBuf> {
        if let Some(path) = self.cache.get(name) {
            return Some(path.clone());
        }

        let resolved = self.registry.resolve(name)?;
        if resolved.as_path().exists() {
            self.cache.insert(name.to_string(), resolved.clone());
            Some(resolved)
        } else {
            None
        }
    }
}
