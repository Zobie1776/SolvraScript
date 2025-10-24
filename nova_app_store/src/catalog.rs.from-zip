use crate::app::{AppId, AppMetadata, AppPackage};
use crate::errors::CatalogError;
use semver::{Version, VersionReq};
use std::collections::BTreeMap;

/// In-memory catalog containing metadata for all known applications.
#[derive(Debug, Default)]
pub struct Catalog {
    entries: BTreeMap<AppId, AppMetadata>,
}

impl Catalog {
    /// Create an empty catalog.
    pub fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
        }
    }

    /// Register metadata for a new application.
    pub fn register(&mut self, metadata: AppMetadata) -> Result<(), CatalogError> {
        let id = metadata.id.clone();
        if self.entries.contains_key(&id) {
            return Err(CatalogError::AlreadyExists(id));
        }
        self.entries.insert(id, metadata);
        Ok(())
    }

    /// Retrieve metadata for a specific application.
    pub fn get(&self, id: &AppId) -> Option<&AppMetadata> {
        self.entries.get(id)
    }

    /// Iterate over all registered metadata entries.
    pub fn iter(&self) -> impl Iterator<Item = (&AppId, &AppMetadata)> {
        self.entries.iter()
    }

    /// Mutably access metadata for an application.
    pub fn get_mut(&mut self, id: &AppId) -> Option<&mut AppMetadata> {
        self.entries.get_mut(id)
    }

    /// Resolve a package version that matches the provided requirement.
    pub fn resolve_version(
        &self,
        id: &AppId,
        requirement: Option<&VersionReq>,
    ) -> Result<&AppPackage, CatalogError> {
        let metadata = self
            .get(id)
            .ok_or_else(|| CatalogError::UnknownApp(id.clone()))?;
        if let Some(req) = requirement {
            metadata
                .versions
                .iter()
                .rev()
                .find(|(version, _)| req.matches(version))
                .map(|(_, package)| package)
                .ok_or_else(|| CatalogError::NoMatchingVersion(id.clone(), req.clone()))
        } else {
            metadata
                .latest_package()
                .ok_or_else(|| CatalogError::NoVersions(id.clone()))
        }
    }

    /// Resolve a specific version.
    pub fn resolve_exact(
        &self,
        id: &AppId,
        version: &Version,
    ) -> Result<&AppPackage, CatalogError> {
        let metadata = self
            .get(id)
            .ok_or_else(|| CatalogError::UnknownApp(id.clone()))?;
        metadata
            .package(version)
            .ok_or_else(|| CatalogError::UnknownVersion(id.clone(), version.clone()))
    }
}
