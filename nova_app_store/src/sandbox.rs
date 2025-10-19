use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fmt;

/// Individual permission scopes that can be granted to an application sandbox.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum SandboxPermission {
    /// Read-only access to user documents explicitly granted by the user.
    FileRead,
    /// Ability to create or modify files inside the sandbox storage area.
    FileWrite,
    /// Access to network resources as a client.
    NetworkClient,
    /// Permission to bind to local sockets.
    NetworkServer,
    /// Access to camera devices.
    Camera,
    /// Access to microphone devices.
    Microphone,
    /// Capture of screen contents.
    ScreenCapture,
    /// Capture of application windows.
    WindowCapture,
}

/// Policy defining the permissions and resource scopes for an application.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxPolicy {
    permissions: BTreeSet<SandboxPermission>,
    storage_roots: Vec<String>,
    network_hosts: Vec<String>,
}

impl Default for SandboxPolicy {
    fn default() -> Self {
        Self {
            permissions: BTreeSet::new(),
            storage_roots: Vec::new(),
            network_hosts: Vec::new(),
        }
    }
}

impl SandboxPolicy {
    /// Construct a new empty policy denying all permissions.
    pub fn new() -> Self {
        Self::default()
    }

    /// Grant a specific permission to the policy.
    pub fn allow_permission(mut self, permission: SandboxPermission) -> Self {
        self.permissions.insert(permission);
        self
    }

    /// Register a storage root that the application may access.
    pub fn allow_storage_root(mut self, path: impl Into<String>) -> Self {
        self.storage_roots.push(path.into());
        self
    }

    /// Register a network host that the application may contact.
    pub fn allow_network_host(mut self, host: impl Into<String>) -> Self {
        self.network_hosts.push(host.into());
        self
    }

    /// Returns true when the permission has been granted.
    pub fn permits(&self, permission: &SandboxPermission) -> bool {
        self.permissions.contains(permission)
    }

    /// Enumerate all granted permissions.
    pub fn permissions(&self) -> impl Iterator<Item = &SandboxPermission> {
        self.permissions.iter()
    }

    /// Enumerate whitelisted storage roots.
    pub fn storage_roots(&self) -> &[String] {
        &self.storage_roots
    }

    /// Enumerate whitelisted network hosts.
    pub fn network_hosts(&self) -> &[String] {
        &self.network_hosts
    }

    /// Validate that the requested permission has been granted.
    pub fn ensure(&self, permission: &SandboxPermission) -> Result<(), SandboxViolation> {
        if self.permits(permission) {
            Ok(())
        } else {
            Err(SandboxViolation::PermissionDenied(permission.clone()))
        }
    }
}

/// Error describing why a sandbox permission request failed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SandboxViolation {
    /// The permission was not granted by the policy.
    #[error("permission {0} is not granted")]
    PermissionDenied(SandboxPermission),
}

impl fmt::Display for SandboxPermission {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            SandboxPermission::FileRead => "file:read",
            SandboxPermission::FileWrite => "file:write",
            SandboxPermission::NetworkClient => "network:client",
            SandboxPermission::NetworkServer => "network:server",
            SandboxPermission::Camera => "device:camera",
            SandboxPermission::Microphone => "device:microphone",
            SandboxPermission::ScreenCapture => "capture:screen",
            SandboxPermission::WindowCapture => "capture:window",
        };
        f.write_str(label)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grants_and_checks_permissions() {
        let policy = SandboxPolicy::new()
            .allow_permission(SandboxPermission::FileRead)
            .allow_permission(SandboxPermission::NetworkClient)
            .allow_storage_root("~/Documents")
            .allow_network_host("api.novaos.dev");

        assert!(policy.permits(&SandboxPermission::FileRead));
        assert!(policy.ensure(&SandboxPermission::FileRead).is_ok());
        assert!(policy.ensure(&SandboxPermission::NetworkClient).is_ok());
        assert_eq!(policy.storage_roots(), ["~/Documents"]);
        assert_eq!(policy.network_hosts(), ["api.novaos.dev"]);
        assert_eq!(
            policy.ensure(&SandboxPermission::Camera),
            Err(SandboxViolation::PermissionDenied(
                SandboxPermission::Camera
            ))
        );
    }
}
