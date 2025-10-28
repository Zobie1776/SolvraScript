use std::error::Error;

#[cfg(feature = "gui")]
use anyhow::Context;

type ShellResult<T> = Result<T, Box<dyn Error>>;

/// Launches the Nova Shell TTY stub.
pub fn start_tty() -> ShellResult<()> {
    println!("Nova Shell (TTY) starting...");
    Ok(())
}

#[cfg(feature = "gui")]
/// Launches the Nova Shell GUI stub when windowing backends are available.
pub fn start_gui() -> ShellResult<()> {
    use winit::event::{Event, WindowEvent};
    use winit::event_loop::EventLoop;
    use winit::window::Window;

    let event_loop = EventLoop::new()
        .context("failed to build event loop")
        .map_err(|err| Box::<dyn Error>::from(err))?;

    #[allow(deprecated)]
    let window = event_loop
        .create_window(Window::default_attributes().with_title("Nova Shell"))
        .context("failed to create Nova Shell window")
        .map_err(|err| Box::<dyn Error>::from(err))?;

    #[allow(deprecated)]
    event_loop
        .run(move |event, elwt| match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => elwt.exit(),
            Event::AboutToWait => window.request_redraw(),
            _ => {}
        })
        .context("event loop error")
        .map_err(|err| Box::<dyn Error>::from(err))?;

    Ok(())
}
