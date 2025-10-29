//=============================================
// nova_compositor/src/main.rs
//=============================================
// Author: Nova Shell Team
// License: MIT
// Goal: Binary entry point for the Nova Shell compositor
// Objective: Initialize tracing and drive the smithay event loop scaffold
//=============================================

use anyhow::Result;
use nova_compositor::Compositor;
use std::{thread, time::Duration};

fn main() -> Result<()> {
    nova_compositor::init_tracing();
    let mut compositor = Compositor::build()?;

    //Temp: keeps the compositor alive (60 FPS-ish) until calloop/smithay loop is in
    loop {
        compositor.tick();
        thread::sleep(Duration::from_millis(16));
    }
}
