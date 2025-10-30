//=============================================
// solvra_compositor/src/main.rs
//=============================================
// Author: Solvra GUI Team
// License: MIT
// Goal: Binary entry point for the Solvra GUI compositor
// Objective: Initialize tracing and drive the smithay event loop scaffold
//=============================================

use anyhow::Result;
use solvra_compositor::Compositor;

fn main() -> Result<()> {
    solvra_compositor::init_tracing();
    let mut compositor = Compositor::build()?;
    compositor.tick();
    Ok(())
}
