//! Built-in applications shipped with the Solvra App Store.

pub mod solvrasheets;
pub mod solvraslides;
pub mod solvrastream;
pub mod solvrawriter;

use crate::app::AppMetadata;

/// Return metadata for the built-in Solvra applications bundled with the OS.
pub fn builtin_catalog() -> Vec<AppMetadata> {
    vec![
        solvrawriter::metadata(),
        solvraslides::metadata(),
        solvrasheets::metadata(),
        solvrastream::metadata(),
    ]
}
