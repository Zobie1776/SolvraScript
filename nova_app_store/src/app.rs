use crate::sandbox::SandboxPolicy;
use chrono::{DateTime, Utc};
use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::str::FromStr;

/// Identifier used to uniquely reference an application in the store catalog.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct AppId(String);

impl AppId {
    /// Construct an [`AppId`] after validating it matches the expected format.
    pub fn new(id: impl Into<String>) -> Result<Self, AppIdError> {
        let id = id.into();
        validate_id(&id)?;
        Ok(Self(id))
    }

    /// Borrow the identifier as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for AppId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for AppId {
    type Err = AppIdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

impl AsRef<str> for AppId {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

/// Error raised when an app identifier does not satisfy validation rules.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum AppIdError {
    /// The identifier contained invalid characters.
    #[error("app identifiers may only contain lowercase ASCII letters, digits, '.', '-' or '_'")]
    InvalidCharacters,
    /// The identifier was empty.
    #[error("app identifier cannot be empty")]
    Empty,
}

fn validate_id(id: &str) -> Result<(), AppIdError> {
    if id.is_empty() {
        return Err(AppIdError::Empty);
    }
    if !id
        .chars()
        .all(|c| matches!(c, 'a'..='z' | '0'..='9' | '.' | '-' | '_'))
    {
        return Err(AppIdError::InvalidCharacters);
    }
    Ok(())
}

/// Human-friendly category used to group applications in listings and search results.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AppCategory {
    Productivity,
    Creativity,
    Development,
    Multimedia,
    Utilities,
    System,
}

impl Default for AppCategory {
    fn default() -> Self {
        AppCategory::Utilities
    }
}

/// Metadata describing a single version of an application.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppPackage {
    /// Semantic version of the package.
    pub version: Version,
    /// Release notes summarising the most important changes.
    pub release_notes: Vec<String>,
    /// Dependencies required for the package to operate.
    pub dependencies: Vec<AppDependency>,
    /// Sandbox policy that governs access to privileged resources.
    pub sandbox: SandboxPolicy,
    /// Capabilities exposed by the application to the rest of NovaOS.
    pub capabilities: Vec<AppCapability>,
    /// UI components that can be embedded into NovaShell / NovaIDE.
    pub ui_components: Vec<UiComponent>,
    /// Optional checksum for the package payload.
    pub checksum: Option<String>,
    /// Timestamp when this package was published.
    pub published_at: DateTime<Utc>,
}

impl AppPackage {
    /// Create a new package with sensible defaults.
    pub fn new(version: Version) -> Self {
        Self {
            version,
            release_notes: Vec::new(),
            dependencies: Vec::new(),
            sandbox: SandboxPolicy::default(),
            capabilities: Vec::new(),
            ui_components: Vec::new(),
            checksum: None,
            published_at: Utc::now(),
        }
    }

    /// Add a dependency requirement to this package.
    pub fn with_dependency(mut self, dependency: AppDependency) -> Self {
        self.dependencies.push(dependency);
        self
    }

    /// Add a capability description to the package metadata.
    pub fn with_capability(mut self, capability: AppCapability) -> Self {
        self.capabilities.push(capability);
        self
    }

    /// Attach sandbox permissions to the package.
    pub fn with_sandbox(mut self, sandbox: SandboxPolicy) -> Self {
        self.sandbox = sandbox;
        self
    }

    /// Register a UI component contributed by the application.
    pub fn with_ui_component(mut self, component: UiComponent) -> Self {
        self.ui_components.push(component);
        self
    }
}

/// Declares a dependency on another app and constrains the acceptable version range.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppDependency {
    /// Identifier of the dependency.
    pub id: AppId,
    /// Required version range.
    pub requirement: VersionReq,
    /// Whether the dependency is optional.
    pub optional: bool,
}

impl AppDependency {
    /// Construct a new dependency definition.
    pub fn new(id: AppId, requirement: VersionReq) -> Self {
        Self {
            id,
            requirement,
            optional: false,
        }
    }

    /// Mark the dependency as optional.
    pub fn optional(mut self) -> Self {
        self.optional = true;
        self
    }
}

/// Declares a capability or service provided by an application.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppCapability {
    /// Machine readable identifier for the capability.
    pub id: String,
    /// Human readable description of the capability.
    pub description: String,
    /// Tags that make the capability easier to discover.
    pub tags: BTreeSet<String>,
}

impl AppCapability {
    /// Helper to construct a capability descriptor.
    pub fn new(id: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            description: description.into(),
            tags: BTreeSet::new(),
        }
    }

    /// Attach a tag for classification.
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.insert(tag.into());
        self
    }
}

/// Describes a UI component that can be embedded into NovaOS surfaces.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UiComponent {
    /// Unique identifier for the component.
    pub id: String,
    /// Type of integration offered by the component.
    pub kind: UiComponentKind,
    /// Optional path to the component entry point.
    pub entry_point: Option<String>,
    /// Short description displayed in integration pickers.
    pub description: String,
}

impl UiComponent {
    /// Construct a new UI component registration.
    pub fn new(
        id: impl Into<String>,
        kind: UiComponentKind,
        description: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            kind,
            entry_point: None,
            description: description.into(),
        }
    }

    /// Provide a concrete entry point (such as a NovaScript or Web component).
    pub fn with_entry_point(mut self, entry: impl Into<String>) -> Self {
        self.entry_point = Some(entry.into());
        self
    }
}

/// Enumerates supported UI integration points in NovaOS surfaces.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum UiComponentKind {
    /// A dedicated workspace tab inside NovaIDE.
    IdeView,
    /// A widget rendered inside NovaShell dashboards.
    ShellWidget,
    /// A command palette provider.
    CommandProvider,
    /// A standalone immersive view (e.g. presentation player).
    Immersive,
}

/// Rich metadata describing an application available in the store.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppMetadata {
    /// Unique identifier for the application.
    pub id: AppId,
    /// Human friendly display name.
    pub name: String,
    /// Short summary shown in catalog listings.
    pub summary: String,
    /// Detailed description displayed on the app detail page.
    pub description: String,
    /// Category used to group the application in catalogs.
    pub category: AppCategory,
    /// Publisher name.
    pub publisher: String,
    /// Tags aiding search discovery.
    pub tags: BTreeSet<String>,
    /// Optional project website or documentation link.
    pub homepage: Option<String>,
    /// All available versions of the application.
    pub versions: BTreeMap<Version, AppPackage>,
    /// Screenshot URLs (can be local paths) displayed in the store UI.
    pub screenshots: Vec<String>,
}

impl AppMetadata {
    /// Create metadata with mandatory fields and an initial package.
    pub fn new(
        id: AppId,
        name: impl Into<String>,
        summary: impl Into<String>,
        description: impl Into<String>,
        category: AppCategory,
        publisher: impl Into<String>,
        initial_package: AppPackage,
    ) -> Self {
        let mut versions = BTreeMap::new();
        versions.insert(initial_package.version.clone(), initial_package);
        Self {
            id,
            name: name.into(),
            summary: summary.into(),
            description: description.into(),
            category,
            publisher: publisher.into(),
            tags: BTreeSet::new(),
            homepage: None,
            versions,
            screenshots: Vec::new(),
        }
    }

    /// Attach an additional package version.
    pub fn with_package(mut self, package: AppPackage) -> Self {
        self.versions.insert(package.version.clone(), package);
        self
    }

    /// Attach a homepage link to the metadata.
    pub fn with_homepage(mut self, homepage: impl Into<String>) -> Self {
        self.homepage = Some(homepage.into());
        self
    }

    /// Append a tag for search and filtering.
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.insert(tag.into());
        self
    }

    /// Register a screenshot asset.
    pub fn with_screenshot(mut self, screenshot: impl Into<String>) -> Self {
        self.screenshots.push(screenshot.into());
        self
    }

    /// Retrieve the latest available package based on semantic version ordering.
    pub fn latest_package(&self) -> Option<&AppPackage> {
        self.versions.values().rev().next()
    }

    /// Retrieve a specific package version.
    pub fn package(&self, version: &Version) -> Option<&AppPackage> {
        self.versions.get(version)
    }
}

/// Record representing a review placeholder. It is used for future AI-enhanced ratings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewPlaceholder {
    /// Identifier for the user that left the review.
    pub user_id: String,
    /// Free-form review text.
    pub comment: String,
    /// Rating out of five stars.
    pub rating: u8,
    /// Time when the review was created.
    pub created_at: DateTime<Utc>,
}

/// Manifest describing an installed application on disk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallationManifest {
    /// Application identifier.
    pub id: AppId,
    /// Installed version.
    pub version: Version,
    /// Timestamp when installation completed.
    pub installed_at: DateTime<Utc>,
    /// Sandbox policy enforced for the installation.
    pub sandbox: SandboxPolicy,
    /// Capabilities registered at install time.
    pub capabilities: Vec<AppCapability>,
}

impl InstallationManifest {
    /// Construct a manifest entry from a package reference.
    pub fn from_package(metadata: &AppMetadata, package: &AppPackage) -> Self {
        Self {
            id: metadata.id.clone(),
            version: package.version.clone(),
            installed_at: Utc::now(),
            sandbox: package.sandbox.clone(),
            capabilities: package.capabilities.clone(),
        }
    }
}

