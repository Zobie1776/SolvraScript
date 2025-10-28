//=============================================
// utils/src/logging.rs
//=============================================
// Author: Nova Shell Team
// License: MIT
// Goal: Tracing helpers shared across Nova Shell processes
// Objective: Offer a consistent subscriber configuration with component labels
//=============================================

use std::sync::OnceLock;
use tracing::Level;
use tracing_subscriber::fmt::SubscriberBuilder;
use tracing_subscriber::EnvFilter;

static INIT: OnceLock<()> = OnceLock::new();

/// Initialize tracing with a component label.
pub fn init(component: &str) {
    INIT.get_or_init(|| {
        SubscriberBuilder::default()
            .with_env_filter(EnvFilter::from_default_env().add_directive(Level::INFO.into()))
            .with_target(true)
            .with_ansi(true)
            .compact()
            .init();
    });
    tracing::info!(component, "tracing initialised");
}
