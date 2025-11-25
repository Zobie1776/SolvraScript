//==================================================
// File: stdlib_registry.rs
//==================================================
// Author: ZobieLabs
// License: Duality Public License (DPL v1.0)
// Goal: Track SolvraScript stdlib module locations for angled import syntax
// Objective: Resolve <module> imports to canonical lib/std paths with caching
//==================================================

use std::collections::HashMap;
use std::path::{Path, PathBuf};

//==================================================
// Section 1.0 - Registry Types
//==================================================
// @TODO[StdlibPhase3]: Register <net>/<toml> once migration reaches networking tier.
// @ZNOTE[Resolver]: Registry stays small so lookups remain predictable and testable.

#[derive(Debug, Clone)]
pub struct StdlibRegistry {
    modules: HashMap<String, PathBuf>,
}

impl StdlibRegistry {
    pub fn with_defaults(root: impl AsRef<Path>) -> Self {
        let root = root.as_ref();
        let mut registry = Self {
            modules: HashMap::new(),
        };
        registry.register("io", root.join("io.svs"));
        registry.register("math", root.join("math.svs"));
        registry.register("string", root.join("string.svs"));
        registry.register("time", root.join("time.svs"));
        registry.register("fs", root.join("fs.svs"));
        registry.register("json", root.join("json.svs"));
        registry.register("sys", root.join("sys.svs"));

        // AI Modules
        registry.register("ai_tensor", root.join("ai/tensor.svs"));
        registry.register("ai_nn", root.join("ai/nn.svs"));
        registry.register("ai_data", root.join("ai/data.svs"));
        registry.register("ai_model", root.join("ai/model.svs"));
        registry.register("ai_utils", root.join("ai/utils.svs"));
        registry.register("ai_bridge", root.join("ai/bridge.svs"));

        registry
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
