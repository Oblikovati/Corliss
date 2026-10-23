//! Child-process helpers. Every gate step is an ordinary command, printed before it runs, so a
//! failing CI step can be reproduced by copying the printed line.

use std::path::Path;
use std::process::{Command, Stdio};

use crate::error::XtaskError;

/// The cargo that launched xtask. Using it keeps every step on the toolchain pinned by
/// `rust-toolchain.toml`, even when another `cargo`/`rustc` (e.g. a GNU install) is on PATH.
///
/// ```text
/// run(root, &cargo_program(), &["build", "--release"])
/// ```
pub fn cargo_program() -> String {
    std::env::var("CARGO").unwrap_or_else(|_| String::from("cargo"))
}

/// Runs `program args` in `root` with inherited output; fails unless it exits with status 0.
///
/// ```text
/// run(root, "git", &["status"])?;
/// ```
pub fn run(root: &Path, program: &str, args: &[&str]) -> Result<(), XtaskError> {
    let command = render(program, args);
    println!("$ {command}");
    let status = Command::new(program)
        .args(args)
        .current_dir(root)
        .status()
        .map_err(|err| start_failure(&command, &err))?;
    if status.success() {
        return Ok(());
    }
    Err(XtaskError::Command {
        command,
        cause: format!("exited with {status}, expected exit code 0"),
    })
}

/// Runs `program args` in `root` and returns its stdout; stderr is folded into the error.
///
/// ```text
/// let tags = capture(root, "git", &["tag", "--list", "v*"])?;
/// ```
pub fn capture(root: &Path, program: &str, args: &[&str]) -> Result<String, XtaskError> {
    let command = render(program, args);
    let output = Command::new(program)
        .args(args)
        .current_dir(root)
        .output()
        .map_err(|err| start_failure(&command, &err))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let cause = format!("exited with {}: {}", output.status, stderr.trim());
        return Err(XtaskError::Command { command, cause });
    }
    String::from_utf8(output.stdout).map_err(|err| XtaskError::Command {
        command,
        cause: format!("stdout is not UTF-8: {err}"),
    })
}

/// Whether `program args` starts and exits 0; probes optional tools such as `cargo deny`.
///
/// ```text
/// if succeeds(root, "cargo", &["deny", "--version"]) { /* run it */ }
/// ```
pub fn succeeds(root: &Path, program: &str, args: &[&str]) -> bool {
    Command::new(program)
        .args(args)
        .current_dir(root)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

fn start_failure(command: &str, err: &std::io::Error) -> XtaskError {
    XtaskError::Command {
        command: command.to_owned(),
        cause: format!("could not start: {err}"),
    }
}

fn render(program: &str, args: &[&str]) -> String {
    let mut parts = vec![program];
    parts.extend_from_slice(args);
    parts.join(" ")
}
