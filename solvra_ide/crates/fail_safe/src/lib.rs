#![forbid(unsafe_code)]

//! SolvraFailSafe – lightweight publish protection for SolvraIDE.
//!
//! The module stores a hashed passphrase in memory and validates requests before
//! performing sensitive actions such as publishing or deploying a workspace.

use anyhow::{anyhow, Result};
use argon2::{password_hash::SaltString, Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use once_cell::sync::Lazy;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use thiserror::Error;

// ===========================================================================
// Errors
// ===========================================================================

/// Error type returned by the fail-safe gatekeeper.
#[derive(Debug, Error)]
pub enum FailSafeError {
    #[error("no passphrase registered")]
    MissingPassphrase,
    #[error("passphrase mismatch")]
    InvalidPassphrase,
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl PartialEq for FailSafeError {
    fn eq(&self, other: &Self) -> bool {
        use FailSafeError::*;

        match (self, other) {
            (MissingPassphrase, MissingPassphrase) | (InvalidPassphrase, InvalidPassphrase) => true,
            (Other(lhs), Other(rhs)) => lhs.to_string() == rhs.to_string(),
            _ => false,
        }
    }
}

impl Eq for FailSafeError {}

// ===========================================================================
// Configuration
// ===========================================================================

/// Serialisable configuration for persisting the hashed passphrase.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FailSafeConfig {
    /// Base64 encoded Argon2 password hash.
    pub hash: String,
    /// Random salt used when the hash was generated.
    pub salt: String,
}

impl FailSafeConfig {
    /// Serialises the config into TOML for easy storage inside SolvraIDE.
    pub fn to_toml(&self) -> Result<String> {
        Ok(toml::to_string_pretty(self)?)
    }

    /// Deserialises the config from TOML.
    pub fn from_toml(input: &str) -> Result<Self> {
        Ok(toml::from_str(input)?)
    }
}

/// Global guard that retains the hashed passphrase in memory.
static FAIL_SAFE_STATE: Lazy<RwLock<Option<FailSafeConfig>>> = Lazy::new(|| RwLock::new(None));

// ===========================================================================
// Public API
// ===========================================================================

/// Registers a new passphrase, hashing it with Argon2id.
pub fn register_passphrase(passphrase: &str) -> Result<FailSafeConfig> {
    let salt = SaltString::generate(&mut rand::thread_rng());
    let argon2 = Argon2::default();
    let hash = argon2
        .hash_password(passphrase.as_bytes(), &salt)
        .map_err(|err| anyhow!(err.to_string()))?
        .to_string();
    let config = FailSafeConfig {
        hash,
        salt: salt.as_str().to_owned(),
    };
    *FAIL_SAFE_STATE.write() = Some(config.clone());
    Ok(config)
}

/// Loads a previously stored configuration into memory.
pub fn load_config(config: FailSafeConfig) {
    *FAIL_SAFE_STATE.write() = Some(config);
}

/// Clears the stored passphrase.
pub fn clear() {
    *FAIL_SAFE_STATE.write() = None;
}

/// Verifies an incoming passphrase request.
pub fn verify(passphrase: &str) -> Result<(), FailSafeError> {
    let guard = FAIL_SAFE_STATE.read();
    let Some(config) = guard.as_ref() else {
        return Err(FailSafeError::MissingPassphrase);
    };

    let parsed_hash = PasswordHash::new(&config.hash)
        .map_err(|err| FailSafeError::Other(anyhow!(err.to_string())))?;
    Argon2::default()
        .verify_password(passphrase.as_bytes(), &parsed_hash)
        .map_err(|_| FailSafeError::InvalidPassphrase)
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_and_verify() {
        let config = register_passphrase("solvra").expect("can register");
        assert!(verify("solvra").is_ok());
        assert_eq!(
            verify("wrong").unwrap_err(),
            FailSafeError::InvalidPassphrase
        );

        let restored = FailSafeConfig::from_toml(&config.to_toml().unwrap()).unwrap();
        clear();
        load_config(restored);
        assert!(verify("solvra").is_ok());
    }

    #[test]
    fn missing_passphrase() {
        clear();
        assert_eq!(
            verify("solvra").unwrap_err(),
            FailSafeError::MissingPassphrase
        );
    }
}
