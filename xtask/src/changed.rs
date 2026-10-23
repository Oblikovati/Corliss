//! Selects the Rust files a change touches, so the PR gate's format check covers only them.

use std::path::Path;

use crate::error::XtaskError;
use crate::process::capture;

/// Rust sources among `paths` (repository-relative, `/`-separated), sorted and deduplicated.
/// Build output under `target/` is never selected.
///
/// ```text
/// assert_eq!(rust_sources(["src/a.rs", "README.md"]), ["src/a.rs"]);
/// ```
pub fn rust_sources<'a>(paths: impl IntoIterator<Item = &'a str>) -> Vec<String> {
    let mut files: Vec<String> = paths
        .into_iter()
        .map(str::trim)
        .filter(|path| path.ends_with(".rs") && !path.starts_with("target/"))
        .map(String::from)
        .collect();
    files.sort();
    files.dedup();
    files
}

/// Rust files added, copied, modified or renamed since the merge base of `base` and `HEAD`,
/// including uncommitted and untracked files, so a local run sees work not yet committed.
/// Fails when `base` cannot be resolved (no remote, shallow clone); the caller then checks
/// every file instead.
///
/// ```text
/// let files = changed_rust_files(root, "origin/main")?;
/// ```
pub fn changed_rust_files(root: &Path, base: &str) -> Result<Vec<String>, XtaskError> {
    let merge_base = git(root, &["merge-base", base, "HEAD"])?;
    let diff = git(
        root,
        &[
            "diff",
            "--name-only",
            "--diff-filter=ACMR",
            merge_base.trim(),
        ],
    )?;
    let untracked = git(root, &["ls-files", "--others", "--exclude-standard"])?;
    Ok(rust_sources(diff.lines().chain(untracked.lines())))
}

fn git(root: &Path, args: &[&str]) -> Result<String, XtaskError> {
    // Unquoted paths: git otherwise escapes non-ASCII names, which rustfmt cannot open.
    let mut full = vec!["-c", "core.quotepath=off"];
    full.extend_from_slice(args);
    capture(root, "git", &full)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_only_rust_sources() {
        let paths = [
            "src/lib.rs",
            "README.md",
            "xtask/src/main.rs",
            "src/grammar/grammars.bin.zst",
        ];
        assert_eq!(rust_sources(paths), ["src/lib.rs", "xtask/src/main.rs"]);
    }

    #[test]
    fn skips_build_output_and_duplicates() {
        let paths = ["target/debug/build/x/out.rs", " src/a.rs", "src/a.rs", ""];
        assert_eq!(rust_sources(paths), ["src/a.rs"]);
    }

    #[test]
    fn empty_input_selects_nothing() {
        assert!(rust_sources(std::iter::empty()).is_empty());
    }
}
