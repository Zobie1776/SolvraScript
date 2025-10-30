use crate::app::{AppId, InstallationManifest};
use crate::errors::InstallationError;
use directories::ProjectDirs;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Filesystem layout for the Solvra App Store installation directories.
#[derive(Debug, Clone)]
pub struct AppStorePaths {
    root: PathBuf,
    apps_dir: PathBuf,
    manifests_dir: PathBuf,
}

impl AppStorePaths {
    /// Resolve the default store locations inside the user's data directory.
    pub fn discover() -> Result<Self, InstallationError> {
        let dirs = ProjectDirs::from("dev", "Solvra", "solvra")
            .ok_or(InstallationError::DataDirectoryUnavailable)?;
        let root = dirs.data_dir().join("app_store");
        Ok(Self::from_root(root)?)
    }

    /// Create a layout from an explicit root directory. Used in tests and offline mode.
    pub fn from_root(root: PathBuf) -> std::io::Result<Self> {
        let apps_dir = root.join("apps");
        let manifests_dir = root.join("manifests");
        fs::create_dir_all(&apps_dir)?;
        fs::create_dir_all(&manifests_dir)?;
        Ok(Self {
            root,
            apps_dir,
            manifests_dir,
        })
    }

    /// Path to the manifest describing the installation.
    pub fn manifest_path(&self, id: &AppId) -> PathBuf {
        self.manifests_dir
            .join(format!("{}.json", id.as_str().replace('/', "_")))
    }

    /// Root directory for application payloads.
    pub fn app_root(&self, id: &AppId) -> PathBuf {
        self.apps_dir.join(id.as_str())
    }

    /// Root directory of the store.
    pub fn root(&self) -> &Path {
        &self.root
    }
}

/// Tracks installed applications and persists manifests to disk.
#[derive(Debug)]
pub struct InstallationRegistry {
    paths: AppStorePaths,
    records: BTreeMap<AppId, InstallationManifest>,
}

impl InstallationRegistry {
    /// Load installations from disk using the default directory layout.
    pub fn load() -> Result<Self, InstallationError> {
        let paths = AppStorePaths::discover()?;
        Self::load_from_paths(paths)
    }

    /// Load installations from disk using the provided layout. Primarily used for testing.
    pub fn load_from_paths(paths: AppStorePaths) -> Result<Self, InstallationError> {
        let mut registry = Self {
            paths,
            records: BTreeMap::new(),
        };
        registry.sync_from_disk()?;
        Ok(registry)
    }

    /// Persist a manifest to disk and update the in-memory registry.
    pub fn upsert(&mut self, manifest: InstallationManifest) -> Result<(), InstallationError> {
        let path = self.paths.manifest_path(&manifest.id);
        let payload_dir = self.paths.app_root(&manifest.id);
        fs::create_dir_all(&payload_dir)?;
        let json = serde_json::to_string_pretty(&manifest)?;
        fs::write(&path, json)?;
        self.records.insert(manifest.id.clone(), manifest);
        Ok(())
    }

    /// Remove an application manifest and associated payload directory.
    pub fn remove(&mut self, id: &AppId) -> Result<InstallationManifest, InstallationError> {
        let manifest = self
            .records
            .remove(id)
            .ok_or_else(|| InstallationError::NotInstalled(id.clone()))?;
        let manifest_path = self.paths.manifest_path(id);
        if manifest_path.exists() {
            fs::remove_file(manifest_path)?;
        }
        let payload_dir = self.paths.app_root(id);
        if payload_dir.exists() {
            fs::remove_dir_all(payload_dir)?;
        }
        Ok(manifest)
    }

    /// Retrieve a manifest for an installed application.
    pub fn get(&self, id: &AppId) -> Option<&InstallationManifest> {
        self.records.get(id)
    }

    /// List all installed applications.
    pub fn list(&self) -> impl Iterator<Item = (&AppId, &InstallationManifest)> {
        self.records.iter()
    }

    /// Expose underlying filesystem layout.
    pub fn paths(&self) -> &AppStorePaths {
        &self.paths
    }

    fn sync_from_disk(&mut self) -> Result<(), InstallationError> {
        self.records.clear();
        for entry in fs::read_dir(&self.paths.manifests_dir)? {
            let entry = entry?;
            if !entry.file_type()?.is_file() {
                continue;
            }
            let data = fs::read_to_string(entry.path())?;
            let manifest: InstallationManifest = serde_json::from_str(&data)?;
            self.records.insert(manifest.id.clone(), manifest);
        }
        Ok(())
    }
}

/// Utility that clears the registry for tests while keeping the directory structure intact.
pub fn clear_registry_dir(paths: &AppStorePaths) -> std::io::Result<()> {
    if paths.root().exists() {
        for entry in fs::read_dir(paths.root())? {
            let entry = entry?;
            if entry.path().is_dir() {
                fs::remove_dir_all(entry.path())?;
            } else {
                fs::remove_file(entry.path())?;
            }
        }
    }
    fs::create_dir_all(&paths.apps_dir)?;
    fs::create_dir_all(&paths.manifests_dir)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sandbox::{SandboxPermission, SandboxPolicy};
    use chrono::Utc;
    use semver::Version;
    use tempfile::tempdir;

    struct TempStore {
        paths: AppStorePaths,
        _dir: tempfile::TempDir,
    }

    fn temp_paths() -> TempStore {
        let dir = tempdir().unwrap();
        let root = dir.path().join("app_store");
        let paths = AppStorePaths::from_root(root).unwrap();
        TempStore { paths, _dir: dir }
    }

    #[test]
    fn roundtrip_manifest() {
        let temp = temp_paths();
        let mut registry = InstallationRegistry::load_from_paths(temp.paths.clone()).unwrap();
        let manifest = InstallationManifest {
            id: AppId::new("dev.solvra.writer").unwrap(),
            version: Version::new(1, 0, 0),
            installed_at: Utc::now(),
            sandbox: SandboxPolicy::new().allow_permission(SandboxPermission::FileWrite),
            capabilities: Vec::new(),
        };
        registry.upsert(manifest.clone()).unwrap();
        assert!(registry.get(&manifest.id).is_some());
        drop(registry);
        let registry = InstallationRegistry::load_from_paths(temp.paths.clone()).unwrap();
        assert_eq!(
            registry.get(&manifest.id).unwrap().version,
            Version::new(1, 0, 0)
        );
    }

    #[test]
    fn removing_manifest_cleans_up() {
        let temp = temp_paths();
        let mut registry = InstallationRegistry::load_from_paths(temp.paths.clone()).unwrap();
        let manifest = InstallationManifest {
            id: AppId::new("dev.solvra.writer").unwrap(),
            version: Version::new(1, 0, 0),
            installed_at: Utc::now(),
            sandbox: SandboxPolicy::new(),
            capabilities: Vec::new(),
        };
        registry.upsert(manifest.clone()).unwrap();
        let payload_dir = registry.paths().app_root(&manifest.id);
        assert!(payload_dir.exists());
        registry.remove(&manifest.id).unwrap();
        assert!(registry.get(&manifest.id).is_none());
        assert!(!payload_dir.exists());
    }
}
