//=============================================
// nova_compositor/src/main.rs
//=============================================
// Author: Nova GUI Team
// License: MIT
// Goal: Binary entry point for the Nova GUI compositor
// Objective: Initialize tracing and drive the smithay event loop scaffold
//=============================================

use anyhow::Result;
use nova_compositor::Compositor;

fn main() -> Result<()> {
    nova_compositor::init_tracing();
    let mut compositor = Compositor::build()?;
    compositor.tick();
    Ok(())
}
