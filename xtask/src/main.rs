//! `cargo xtask`: the Corliss CI gates and release check, runnable locally exactly as the
//! `.github/workflows/corliss.yml` workflow runs them. See `cargo xtask help`.

mod changed;
mod changelog;
mod cli;
mod error;
mod gates;
mod manifest;
mod process;
mod release;
mod version;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use cli::{USAGE, XtaskCommand, parse_command};
use error::XtaskError;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match parse_command(&args).and_then(|command| execute(&command)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err @ XtaskError::Usage(_)) => {
            eprintln!("xtask: {err}\n\n{USAGE}");
            ExitCode::from(2)
        }
        Err(err) => {
            eprintln!("xtask: {err}");
            ExitCode::FAILURE
        }
    }
}

fn execute(command: &XtaskCommand) -> Result<(), XtaskError> {
    let root = repository_root()?;
    match command {
        XtaskCommand::Pr { base } => gates::pr_gate(&root, base),
        XtaskCommand::Full { allow_dirty } => gates::full_gate(&root, *allow_dirty),
        XtaskCommand::Release(request) => release::release(&root, request),
        XtaskCommand::Help => {
            println!("{USAGE}");
            Ok(())
        }
    }
}

/// The workspace root, so the gates behave the same from any working directory.
fn repository_root() -> Result<PathBuf, XtaskError> {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest_dir.parent().map(Path::to_path_buf).ok_or_else(|| {
        XtaskError::Shape(format!(
            "xtask manifest directory {} has no parent; expected <workspace>/xtask",
            manifest_dir.display()
        ))
    })
}
