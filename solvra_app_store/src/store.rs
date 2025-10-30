use crate::app::{AppId, AppMetadata, InstallationManifest};
use crate::apps;
use crate::catalog::Catalog;
use crate::errors::{CatalogError, InstallationError};
use crate::installation::{AppStorePaths, InstallationRegistry};
use semver::Version;
use std::collections::BTreeSet;

/// Primary entry point for interacting with the Solvra App Store runtime.
#[derive(Debug)]
pub struct AppStore {
    catalog: Catalog,
    installations: InstallationRegistry,
}

impl AppStore {
    /// Bootstrap an [`AppStore`] with the default registry paths and built-in apps.
    pub fn bootstrap() -> Result<Self, InstallationError> {
        let mut store = Self {
            catalog: Catalog::new(),
            installations: InstallationRegistry::load()?,
        };
        store.register_builtin_apps()?;
        Ok(store)
    }

    /// Register core Solvra applications so the catalog is immediately useful.
    pub fn register_builtin_apps(&mut self) -> Result<(), InstallationError> {
        for metadata in apps::builtin_catalog() {
            // Ignore duplicates so repeated bootstrap calls are idempotent.
            if let Err(CatalogError::AlreadyExists(_)) = self.catalog.register(metadata) {
                continue;
            }
        }
        Ok(())
    }

    /// Register a new application in the catalog.
    pub fn register_app(&mut self, metadata: AppMetadata) -> Result<(), CatalogError> {
        self.catalog.register(metadata)
    }

    /// List all applications currently published in the catalog.
    pub fn list_available(&self) -> impl Iterator<Item = (&AppId, &AppMetadata)> {
        self.catalog.iter()
    }

    /// Retrieve metadata for a single application.
    pub fn metadata(&self, id: &AppId) -> Option<&AppMetadata> {
        self.catalog.get(id)
    }

    /// List all installed applications.
    pub fn list_installed(&self) -> impl Iterator<Item = (&AppId, &InstallationManifest)> {
        self.installations.list()
    }

    /// Install or update an application. Dependencies are automatically resolved.
    pub fn install(
        &mut self,
        id: &AppId,
        version: Option<&Version>,
    ) -> Result<InstallationManifest, InstallationError> {
        let mut visited = BTreeSet::new();
        self.install_inner(id, version, &mut visited)
    }

    fn install_inner(
        &mut self,
        id: &AppId,
        version: Option<&Version>,
        visited: &mut BTreeSet<AppId>,
    ) -> Result<InstallationManifest, InstallationError> {
        if !visited.insert(id.clone()) {
            // Circular dependency detected.
            return Err(InstallationError::UnmetDependency(
                crate::app::AppDependency {
                    id: id.clone(),
                    requirement: semver::VersionReq::STAR,
                    optional: false,
                },
            ));
        }

        if let Some(installed) = self.installations.get(id) {
            if let Some(target_version) = version {
                if &installed.version == target_version {
                    visited.remove(id);
                    return Ok(installed.clone());
                }
            } else {
                visited.remove(id);
                return Ok(installed.clone());
            }
        }

        let metadata = self
            .catalog
            .get(id)
            .ok_or_else(|| CatalogError::UnknownApp(id.clone()))?
            .clone();
        let package = match version {
            Some(version) => metadata
                .package(version)
                .cloned()
                .ok_or_else(|| CatalogError::UnknownVersion(id.clone(), version.clone()))?,
            None => metadata
                .latest_package()
                .cloned()
                .ok_or_else(|| CatalogError::NoVersions(id.clone()))?,
        };

        // Resolve dependencies before installing the package itself.
        for dependency in &package.dependencies {
            if dependency.optional {
                continue;
            }
            let already_installed = self
                .installations
                .get(&dependency.id)
                .map(|manifest| dependency.requirement.matches(&manifest.version))
                .unwrap_or(false);
            if already_installed {
                continue;
            }
            let resolved_version = match self
                .catalog
                .resolve_version(&dependency.id, Some(&dependency.requirement))
            {
                Ok(pkg) => pkg.version.clone(),
                Err(err) => {
                    return Err(match err {
                        CatalogError::UnknownApp(_) | CatalogError::NoMatchingVersion(_, _) => {
                            InstallationError::UnmetDependency(dependency.clone())
                        }
                        other => InstallationError::from(other),
                    });
                }
            };
            self.install_inner(&dependency.id, Some(&resolved_version), visited)?;
        }

        let manifest = InstallationManifest::from_package(&metadata, &package);
        self.installations.upsert(manifest.clone())?;
        visited.remove(id);
        Ok(manifest)
    }

    /// Remove an installed application and its payload.
    pub fn uninstall(&mut self, id: &AppId) -> Result<InstallationManifest, InstallationError> {
        self.installations.remove(id)
    }

    /// Access the underlying installation registry paths (primarily for UI integration).
    pub fn paths(&self) -> &AppStorePaths {
        self.installations.paths()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::AppId;
    use crate::installation::AppStorePaths;
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
    fn installs_builtin_applications() {
        let temp = temp_paths();
        let registry = InstallationRegistry::load_from_paths(temp.paths.clone()).unwrap();
        let mut store = AppStore {
            catalog: Catalog::new(),
            installations: registry,
        };
        store.register_builtin_apps().unwrap();
        let writer_id = AppId::new("dev.solvra.writer").unwrap();
        let manifest = store.install(&writer_id, None).unwrap();
        assert_eq!(manifest.id, writer_id);
        assert!(store.installations.get(&writer_id).is_some());
    }
}
