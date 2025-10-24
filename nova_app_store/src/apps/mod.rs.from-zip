//! Built-in applications shipped with the Nova App Store.

pub mod novasheets;
pub mod novaslides;
pub mod novastream;
pub mod novawriter;

use crate::app::AppMetadata;

/// Return metadata for the built-in Nova applications bundled with the OS.
pub fn builtin_catalog() -> Vec<AppMetadata> {
    vec![
        novawriter::metadata(),
        novaslides::metadata(),
        novasheets::metadata(),
        novastream::metadata(),
    ]
}
