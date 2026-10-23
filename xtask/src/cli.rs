//! Parses the `cargo xtask` command line into a typed command.

use std::path::PathBuf;

use crate::error::XtaskError;
use crate::release::ReleaseRequest;
use crate::version::Version;

/// The base a pull request is compared with when `--base` is not given.
pub const DEFAULT_BASE: &str = "origin/main";

/// Usage text printed by `cargo xtask help` and on a usage error.
pub const USAGE: &str = "\
usage: cargo xtask <command>

commands:
  pr [--base <ref>]      PR gate: rustfmt --check on Rust files changed since <ref>
                         (default origin/main; every file when <ref> is unusable),
                         clippy -D warnings, build, tests, doc tests (all --release)
  full [--allow-dirty]   full-matrix gate: cargo fmt --check, clippy, build, tests,
                         doc tests, cargo package (verified), cargo deny and
                         cargo audit when installed
  release <X.Y.Z> [--bump] [--tag] [--notes-file <path>]
                         check the release against CHANGELOG.md, Cargo.toml and
                         the v* tags; --bump sets Cargo.toml's version, --tag
                         creates the local tag vX.Y.Z, --notes-file writes the
                         CHANGELOG section; nothing is pushed or published
  help                   print this text";

/// One `cargo xtask` invocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum XtaskCommand {
    /// The pull-request gate, comparing with `base` for the format check.
    Pr {
        /// Git ref the changed files are computed against.
        base: String,
    },
    /// The full-matrix gate.
    Full {
        /// Pass `--allow-dirty` to `cargo package` for uncommitted local work.
        allow_dirty: bool,
    },
    /// The release check, with optional side effects.
    Release(ReleaseRequest),
    /// Print the usage text.
    Help,
}

/// Parses the arguments after `cargo xtask`.
///
/// ```text
/// assert_eq!(parse_command(&["full".into()])?, XtaskCommand::Full { allow_dirty: false });
/// ```
pub fn parse_command(args: &[String]) -> Result<XtaskCommand, XtaskError> {
    let Some((name, rest)) = args.split_first() else {
        return Ok(XtaskCommand::Help);
    };
    match name.as_str() {
        "pr" => parse_pr(rest),
        "full" => parse_full(rest),
        "release" => parse_release(rest),
        "help" | "--help" | "-h" => Ok(XtaskCommand::Help),
        other => Err(usage(format!(
            "unknown command `{other}`; expected pr, full, release or help"
        ))),
    }
}

fn parse_pr(rest: &[String]) -> Result<XtaskCommand, XtaskError> {
    match rest {
        [] => Ok(XtaskCommand::Pr {
            base: DEFAULT_BASE.to_owned(),
        }),
        [flag, base] if flag == "--base" && !base.is_empty() => {
            Ok(XtaskCommand::Pr { base: base.clone() })
        }
        other => Err(usage(format!(
            "pr takes only `--base <ref>`, got {other:?}"
        ))),
    }
}

fn parse_full(rest: &[String]) -> Result<XtaskCommand, XtaskError> {
    match rest {
        [] => Ok(XtaskCommand::Full { allow_dirty: false }),
        [flag] if flag == "--allow-dirty" => Ok(XtaskCommand::Full { allow_dirty: true }),
        other => Err(usage(format!(
            "full takes only `--allow-dirty`, got {other:?}"
        ))),
    }
}

fn parse_release(rest: &[String]) -> Result<XtaskCommand, XtaskError> {
    let mut iter = rest.iter();
    let Some(version) = iter.next() else {
        return Err(usage(
            "release needs a version, e.g. `release 0.2.0`".to_owned(),
        ));
    };
    let mut request = ReleaseRequest {
        version: Version::parse(version)?,
        bump: false,
        tag: false,
        notes_file: None,
    };
    apply_release_flags(&mut request, iter)?;
    Ok(XtaskCommand::Release(request))
}

fn apply_release_flags<'a>(
    request: &mut ReleaseRequest,
    mut iter: impl Iterator<Item = &'a String>,
) -> Result<(), XtaskError> {
    while let Some(flag) = iter.next() {
        match flag.as_str() {
            "--bump" => request.bump = true,
            "--tag" => request.tag = true,
            "--notes-file" => request.notes_file = Some(notes_path(iter.next())?),
            other => {
                return Err(usage(format!(
                    "unknown release flag `{other}`; expected --bump, --tag or --notes-file <path>"
                )));
            }
        }
    }
    Ok(())
}

fn notes_path(value: Option<&String>) -> Result<PathBuf, XtaskError> {
    match value {
        Some(path) if !path.is_empty() => Ok(PathBuf::from(path)),
        _ => Err(usage("--notes-file needs a path".to_owned())),
    }
}

fn usage(message: String) -> XtaskError {
    XtaskError::Usage(message)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Result<XtaskCommand, XtaskError> {
        let owned: Vec<String> = args.iter().map(|arg| (*arg).to_owned()).collect();
        parse_command(&owned)
    }

    #[test]
    fn pr_defaults_to_origin_main() {
        let base = DEFAULT_BASE.to_owned();
        assert_eq!(parse(&["pr"]).ok(), Some(XtaskCommand::Pr { base }));
        let base = "origin/release".to_owned();
        assert_eq!(
            parse(&["pr", "--base", "origin/release"]).ok(),
            Some(XtaskCommand::Pr { base })
        );
    }

    #[test]
    fn full_accepts_allow_dirty() {
        assert_eq!(
            parse(&["full"]).ok(),
            Some(XtaskCommand::Full { allow_dirty: false })
        );
        let dirty = parse(&["full", "--allow-dirty"]).ok();
        assert_eq!(dirty, Some(XtaskCommand::Full { allow_dirty: true }));
    }

    #[test]
    fn release_defaults_to_check_only() {
        let Ok(XtaskCommand::Release(request)) = parse(&["release", "v0.2.0"]) else {
            panic!("release parses");
        };
        assert_eq!(request.version.to_string(), "0.2.0");
        assert!(!request.bump && !request.tag && request.notes_file.is_none());
    }

    #[test]
    fn release_flags_set_side_effects() {
        let parsed = parse(&[
            "release",
            "0.2.0",
            "--bump",
            "--tag",
            "--notes-file",
            "n.md",
        ]);
        let Ok(XtaskCommand::Release(request)) = parsed else {
            panic!("release parses");
        };
        assert!(request.bump && request.tag);
        assert_eq!(request.notes_file, Some(PathBuf::from("n.md")));
    }

    #[test]
    fn rejects_bad_command_lines() {
        for args in [
            &["deploy"][..],
            &["pr", "--base"],
            &["full", "-x"],
            &["release"],
        ] {
            assert!(parse(args).is_err(), "{args:?} should be rejected");
        }
        assert!(parse(&["release", "0.2"]).is_err());
        assert!(parse(&["release", "0.2.0", "--notes-file"]).is_err());
        assert!(parse(&["release", "0.2.0", "--push"]).is_err());
    }

    #[test]
    fn no_arguments_prints_help() {
        assert_eq!(parse(&[]).ok(), Some(XtaskCommand::Help));
    }
}
