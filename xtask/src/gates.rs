//! The CI gates. Each is a fixed sequence of commands, so `cargo xtask <gate>` on a laptop runs
//! exactly what the workflow runs.

use std::path::Path;

use crate::changed::changed_rust_files;
use crate::error::XtaskError;
use crate::process::{capture, cargo_program, run, succeeds};

// Release profile throughout: the grammar interpreter runs ~570,000 fixture nodes per test run
// and is far too slow unoptimized. `--locked` makes a stale Cargo.lock fail instead of drift.
const CLIPPY: &[&str] = &[
    "clippy",
    "--workspace",
    "--all-targets",
    "--release",
    "--locked",
    "--",
    "-D",
    "warnings",
];
const BUILD: &[&str] = &["build", "--workspace", "--release", "--locked"];
const TEST: &[&str] = &[
    "test",
    "--workspace",
    "--all-targets",
    "--release",
    "--locked",
];
// `--all-targets` leaves doc tests out, so they run as their own step.
const DOC_TEST: &[&str] = &["test", "--workspace", "--doc", "--release", "--locked"];
const FMT_ALL: &[&str] = &["fmt", "--all", "--", "--check"];

/// The crate that ships; `xtask` is never packaged.
const PUBLISHED_PACKAGE: &str = "corliss";
/// Files the published crate cannot build or render without; `grammars.bin.zst` is pulled in
/// by `include_bytes!` in `src/grammar/mod.rs`.
pub const PACKAGE_REQUIRED_FILES: &[&str] = &["src/grammar/grammars.bin.zst", "README.md"];

// Windows caps a command line at 32,767 characters; batches keep rustfmt well below it.
const RUSTFMT_BATCH: usize = 100;

/// The pull-request gate: format check of the Rust files changed since `base`, then clippy,
/// build, tests and doc tests.
///
/// ```text
/// pr_gate(root, "origin/main")?;
/// ```
pub fn pr_gate(root: &Path, base: &str) -> Result<(), XtaskError> {
    check_changed_formatting(root, base)?;
    lint_build_test(root)
}

/// The full-matrix gate: format check of every file, clippy, build, tests, doc tests, a
/// verified `cargo package`, then `cargo deny` and `cargo audit` when they are installed.
/// `allow_dirty` lets a local run package uncommitted work.
///
/// ```text
/// full_gate(root, false)?;
/// ```
pub fn full_gate(root: &Path, allow_dirty: bool) -> Result<(), XtaskError> {
    cargo(root, FMT_ALL)?;
    lint_build_test(root)?;
    package(root, allow_dirty)?;
    optional_cargo_tool(root, &["deny", "--all-features", "check"])?;
    optional_cargo_tool(root, &["audit"])
}

/// Entries of `required` absent from a `cargo package --list` listing.
///
/// ```text
/// assert!(missing_package_files("README.md\n", &["README.md"]).is_empty());
/// ```
pub fn missing_package_files<'a>(listing: &str, required: &[&'a str]) -> Vec<&'a str> {
    let listed: Vec<String> = listing
        .lines()
        .map(|line| line.trim().replace('\\', "/"))
        .collect();
    required
        .iter()
        .copied()
        .filter(|file| !listed.iter().any(|entry| entry == file))
        .collect()
}

fn lint_build_test(root: &Path) -> Result<(), XtaskError> {
    for args in [CLIPPY, BUILD, TEST, DOC_TEST] {
        cargo(root, args)?;
    }
    Ok(())
}

fn check_changed_formatting(root: &Path, base: &str) -> Result<(), XtaskError> {
    let files = match changed_rust_files(root, base) {
        Ok(files) => files,
        Err(err) => {
            println!("no usable git base `{base}` ({err}); checking every file's formatting");
            return cargo(root, FMT_ALL);
        }
    };
    if files.is_empty() {
        println!("no Rust files changed since {base}; skipping the format check");
    }
    for batch in files.chunks(RUSTFMT_BATCH) {
        let mut args = vec!["--check", "--edition", "2024"];
        args.extend(batch.iter().map(String::as_str));
        run(root, "rustfmt", &args)?;
    }
    Ok(())
}

fn package(root: &Path, allow_dirty: bool) -> Result<(), XtaskError> {
    let mut args = vec!["package", "--package", PUBLISHED_PACKAGE, "--locked"];
    if allow_dirty {
        args.push("--allow-dirty");
    }
    let listing = capture(
        root,
        &cargo_program(),
        &[args.as_slice(), &["--list"]].concat(),
    )?;
    let missing = missing_package_files(&listing, PACKAGE_REQUIRED_FILES);
    if !missing.is_empty() {
        return Err(XtaskError::Shape(format!(
            "the {PUBLISHED_PACKAGE} package lacks {missing:?}; check `exclude` in Cargo.toml"
        )));
    }
    cargo(root, &args)
}

fn optional_cargo_tool(root: &Path, args: &[&str]) -> Result<(), XtaskError> {
    let tool = args.first().copied().unwrap_or_default();
    if !succeeds(root, &cargo_program(), &[tool, "--version"]) {
        println!("skipping `cargo {tool}`: not installed (not a CI gate yet)");
        return Ok(());
    }
    cargo(root, args)
}

fn cargo(root: &Path, args: &[&str]) -> Result<(), XtaskError> {
    run(root, &cargo_program(), args)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_listing_with_every_required_file_passes() {
        let listing = "Cargo.toml\nREADME.md\nsrc/grammar/grammars.bin.zst\nsrc/lib.rs\n";
        assert!(missing_package_files(listing, PACKAGE_REQUIRED_FILES).is_empty());
    }

    #[test]
    fn package_listing_reports_missing_files() {
        let listing = "Cargo.toml\nREADME.md\nsrc/lib.rs\n";
        let missing = missing_package_files(listing, PACKAGE_REQUIRED_FILES);
        assert_eq!(missing, ["src/grammar/grammars.bin.zst"]);
    }

    #[test]
    fn package_listing_accepts_windows_separators() {
        let listing = "src\\grammar\\grammars.bin.zst\r\nREADME.md\r\n";
        assert!(missing_package_files(listing, PACKAGE_REQUIRED_FILES).is_empty());
    }
}
