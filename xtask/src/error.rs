//! The failures an xtask command reports: which step failed, on what, and why.

use std::fmt;
use std::path::PathBuf;

/// A failed xtask step, classified so the caller sees the operation and the offending value.
#[derive(Debug)]
pub enum XtaskError {
    /// The command line did not match any xtask command or flag.
    Usage(String),
    /// A child process could not start or exited unsuccessfully.
    Command {
        /// The command line as it was run.
        command: String,
        /// Why it failed: the exit status or the start error.
        cause: String,
    },
    /// A repository file could not be read or written.
    Io {
        /// The file that failed.
        path: PathBuf,
        /// The underlying I/O error.
        cause: std::io::Error,
    },
    /// A version string is not `MAJOR.MINOR.PATCH`.
    Version {
        /// The offending text.
        text: String,
        /// What was wrong with it.
        reason: String,
    },
    /// `Cargo.toml` or `CHANGELOG.md` does not have the shape the release rules read.
    Shape(String),
    /// The release would break a release rule (bump size, version mismatch, dirty tree).
    Release(String),
}

impl fmt::Display for XtaskError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Usage(msg) => write!(f, "usage error: {msg}"),
            Self::Command { command, cause } => write!(f, "`{command}` failed: {cause}"),
            Self::Io { path, cause } => write!(f, "{}: {cause}", path.display()),
            Self::Version { text, reason } => {
                write!(
                    f,
                    "invalid version `{text}`: {reason}; expected MAJOR.MINOR.PATCH"
                )
            }
            Self::Shape(msg) => write!(f, "unexpected file shape: {msg}"),
            Self::Release(msg) => write!(f, "release refused: {msg}"),
        }
    }
}

impl std::error::Error for XtaskError {}
