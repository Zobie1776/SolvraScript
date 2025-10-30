/// Binary entry for Solvra Shell.
/// Use `--features gui` to run a minimal windowed stub.
/// Otherwise, fall back to TTY mode.
#[cfg(feature = "gui")]
fn main() {
    if let Err(err) = solvra_shell_v0_1_0::start_gui() {
        eprintln!("GUI failed: {err}");
        #[cfg(target_os = "linux")]
        eprintln!("Hint: ensure Wayland/X11 backends are available (try installing Vulkan/Mesa or setting `WINIT_UNIX_BACKEND`).");
        std::process::exit(1);
    }
}

#[cfg(not(feature = "gui"))]
fn main() {
    if let Err(err) = solvra_shell_v0_1_0::start_tty() {
        eprintln!("Shell failed: {err}");
        std::process::exit(1);
    }
}
