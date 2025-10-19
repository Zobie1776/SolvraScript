//! Nova App Store crate providing catalog, sandbox, and installation management.

pub mod app;
pub mod apps;
pub mod catalog;
pub mod errors;
pub mod installation;
pub mod sandbox;
pub mod store;

pub use app::{
    AppCapability, AppCategory, AppDependency, AppId, AppIdError, AppMetadata, AppPackage,
    InstallationManifest, ReviewPlaceholder, UiComponent, UiComponentKind,
};
pub use catalog::Catalog;
pub use errors::{CatalogError, InstallationError};
pub use installation::{AppStorePaths, InstallationRegistry};
pub use sandbox::{SandboxPermission, SandboxPolicy, SandboxViolation};
pub use store::AppStore;
