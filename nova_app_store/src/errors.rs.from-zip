use crate::app::{AppDependency, AppId};
use crate::sandbox::SandboxPermission;
use semver::{Version, VersionReq};
use thiserror::Error;

/// Errors raised when interacting with the application catalog.
#[derive(Debug, Error)]
pub enum CatalogError {
    /// Attempted to register metadata for an app that already exists.
    #[error("application {0} is already registered in the catalog")]
    AlreadyExists(AppId),
    /// Requested app could not be found.
    #[error("application {0} is not registered in the catalog")]
    UnknownApp(AppId),
    /// Requested version does not exist for the app.
    #[error("version {1} of application {0} is not available")]
    UnknownVersion(AppId, Version),
    /// No versions satisfy the provided semantic requirement.
    #[error("no available version of application {0} satisfies {1}")]
    NoMatchingVersion(AppId, VersionReq),
    /// The application has not published any versions yet.
    #[error("application {0} does not have any published versions")]
    NoVersions(AppId),
}

/// Errors raised when installing, updating, or uninstalling applications.
#[derive(Debug, Error)]
pub enum InstallationError {
    /// General catalog related error.
    #[error(transparent)]
    Catalog(#[from] CatalogError),
    /// Unable to satisfy dependency constraints.
    #[error("failed to satisfy dependency {0:?}")]
    UnmetDependency(AppDependency),
    /// Sandbox rejected an operation because a permission is missing.
    #[error("missing sandbox permission: {0}")]
    Sandbox(SandboxPermission),
    /// Filesystem interaction failed.
    #[error("filesystem error: {0}")]
    Io(#[from] std::io::Error),
    /// Serialisation error while reading or writing manifests.
    #[error("manifest error: {0}")]
    Manifest(#[from] serde_json::Error),
    /// Attempted to remove an application that is not installed.
    #[error("application {0} is not currently installed")]
    NotInstalled(AppId),
    /// The OS data directory could not be determined.
    #[error("unable to determine NovaOS data directory for installations")]
    DataDirectoryUnavailable,
}
