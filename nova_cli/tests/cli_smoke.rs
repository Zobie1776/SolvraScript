use std::process::Command;

use assert_cmd::cargo::cargo_bin;
use expectrl::{session::Session, Eof, Expect, Regex};
use tempfile::tempdir;

#[test]
fn cli_smoke() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempdir()?;
    let bin = cargo_bin("nova_cli");
    let mut command = Command::new(bin);
    command.current_dir(temp.path());
    command.env("HOME", temp.path());
    command.env("XDG_CONFIG_HOME", temp.path().join("config"));
    command.env("XDG_DATA_HOME", temp.path().join("data"));

    let mut session = Session::spawn(command)?;
    session.expect(Regex("nova.*>"))?;

    session.send_line("echo hello")?;
    session.expect("hello")?;
    session.expect(Regex("nova.*>"))?;

    session.send_line("pwd")?;
    session.expect(temp.path().to_string_lossy().as_ref())?;
    session.expect(Regex("nova.*>"))?;

    session.send_line("!!")?;
    session.expect("pwd")?;
    session.expect(temp.path().to_string_lossy().as_ref())?;
    session.expect(Regex("nova.*>"))?;

    session.send_line("exit")?;
    session.expect(Eof)?;
    Ok(())
}
