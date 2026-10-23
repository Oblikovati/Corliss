//! `cargo xtask release <X.Y.Z>`: checks that a release is allowed. By default it only reads;
//! `--bump` rewrites the Cargo.toml version and `--tag` creates the local `vX.Y.Z` tag. It
//! never pushes, publishes or creates a GitHub release.

use std::path::{Path, PathBuf};

use crate::changelog::{release_notes, release_section, required_bump};
use crate::error::XtaskError;
use crate::manifest::{package_version, with_package_version};
use crate::process::{capture, cargo_program, run};
use crate::version::{BumpLevel, Version, bump_level};

/// What `cargo xtask release` was asked to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseRequest {
    /// The version being released.
    pub version: Version,
    /// Rewrite Cargo.toml's version to `version` instead of refusing a mismatch.
    pub bump: bool,
    /// Create the annotated local tag `v<version>` on a clean HEAD.
    pub tag: bool,
    /// Where to write the CHANGELOG section as release notes.
    pub notes_file: Option<PathBuf>,
}

/// Checks the release against CHANGELOG.md, Cargo.toml and the existing `v*` tags, then
/// performs only the side effects the request's flags ask for.
///
/// ```text
/// release(root, &ReleaseRequest { version, bump: false, tag: false, notes_file: None })?;
/// ```
pub fn release(root: &Path, request: &ReleaseRequest) -> Result<(), XtaskError> {
    let section = release_section(&read(&root.join("CHANGELOG.md"))?, request.version)?;
    let required = required_bump(&section)?;
    let manifest = read(&root.join("Cargo.toml"))?;
    let current = package_version(&manifest)?;
    let baseline = previous_release(&released_versions(root)?, current, request.version);
    check_bump(baseline, request.version, required)?;
    sync_manifest(root, &manifest, current, request)?;
    if let Some(path) = &request.notes_file {
        write(path, &release_notes(&section))?;
    }
    if request.tag {
        create_tag(root, request.version)?;
    }
    report(baseline, request.version, required);
    Ok(())
}

/// The release a new `target` is compared with: the highest `v*` tag below it, else the
/// Cargo.toml version when that is below it, else none (the first release).
///
/// ```text
/// assert_eq!(previous_release(&[v("0.1.0")], v("0.2.0"), v("0.2.0")), Some(v("0.1.0")));
/// ```
pub fn previous_release(tags: &[Version], manifest: Version, target: Version) -> Option<Version> {
    let tagged = tags.iter().copied().filter(|tag| *tag < target).max();
    tagged.or((manifest < target).then_some(manifest))
}

/// Refuses a step from `baseline` to `target` smaller than `required`; a first release
/// (no baseline) has nothing to compare with.
///
/// ```text
/// check_bump(Some(v("0.1.0")), v("0.1.1"), BumpLevel::Major)?; // refused
/// ```
pub fn check_bump(
    baseline: Option<Version>,
    target: Version,
    required: BumpLevel,
) -> Result<(), XtaskError> {
    let Some(previous) = baseline else {
        return Ok(());
    };
    let actual = bump_level(previous, target)?;
    if actual >= required {
        return Ok(());
    }
    Err(XtaskError::Release(format!(
        "{previous} -> {target} is a {actual} step, but the CHANGELOG headings need {required}"
    )))
}

/// Versions of the `v*` tags in `git tag --list` output; other tags are ignored.
///
/// ```text
/// assert_eq!(tag_versions("v0.1.0\nnightly\n"), [v("0.1.0")]);
/// ```
pub fn tag_versions(listing: &str) -> Vec<Version> {
    listing
        .lines()
        .filter_map(|tag| tag.trim().strip_prefix('v'))
        .filter_map(|text| Version::parse(text).ok())
        .collect()
}

fn released_versions(root: &Path) -> Result<Vec<Version>, XtaskError> {
    Ok(tag_versions(&capture(
        root,
        "git",
        &["tag", "--list", "v*"],
    )?))
}

fn sync_manifest(
    root: &Path,
    manifest: &str,
    current: Version,
    request: &ReleaseRequest,
) -> Result<(), XtaskError> {
    if current == request.version {
        return Ok(());
    }
    if !request.bump {
        return Err(XtaskError::Release(format!(
            "Cargo.toml is at {current} but the release is {}; rerun with --bump to set it",
            request.version
        )));
    }
    write(
        &root.join("Cargo.toml"),
        &with_package_version(manifest, request.version)?,
    )?;
    // Refresh the workspace's own Cargo.lock entries so `--locked` builds keep passing.
    run(
        root,
        &cargo_program(),
        &["update", "--workspace", "--offline"],
    )
}

fn create_tag(root: &Path, version: Version) -> Result<(), XtaskError> {
    let status = capture(root, "git", &["status", "--porcelain"])?;
    if !status.trim().is_empty() {
        return Err(XtaskError::Release(format!(
            "refusing to tag v{version} with uncommitted changes:\n{}",
            status.trim_end()
        )));
    }
    let tag = format!("v{version}");
    let message = format!("corliss {version}");
    run(
        root,
        "git",
        &["tag", "--annotate", &tag, "--message", &message],
    )
}

fn report(baseline: Option<Version>, target: Version, required: BumpLevel) {
    match baseline {
        Some(previous) => println!(
            "release {target} ok: step from {previous} satisfies the CHANGELOG's {required} changes"
        ),
        None => println!("release {target} ok: first release, CHANGELOG needs {required}"),
    }
}

fn read(path: &Path) -> Result<String, XtaskError> {
    std::fs::read_to_string(path).map_err(|cause| XtaskError::Io {
        path: path.to_owned(),
        cause,
    })
}

fn write(path: &Path, text: &str) -> Result<(), XtaskError> {
    std::fs::write(path, text).map_err(|cause| XtaskError::Io {
        path: path.to_owned(),
        cause,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(text: &str) -> Version {
        Version::parse(text).expect("test version parses")
    }

    #[test]
    fn baseline_is_the_highest_tag_below_the_target() {
        let tags = [v("0.1.0"), v("0.3.0"), v("0.2.1"), v("0.2.0")];
        assert_eq!(
            previous_release(&tags, v("0.3.0"), v("0.3.0")),
            Some(v("0.2.1"))
        );
    }

    #[test]
    fn baseline_falls_back_to_a_lower_manifest_version() {
        assert_eq!(
            previous_release(&[], v("0.1.0"), v("0.2.0")),
            Some(v("0.1.0"))
        );
        assert_eq!(previous_release(&[], v("0.1.0"), v("0.1.0")), None);
    }

    #[test]
    fn first_release_accepts_any_requirement() {
        assert!(check_bump(None, v("0.1.0"), BumpLevel::Major).is_ok());
    }

    #[test]
    fn refuses_a_step_smaller_than_the_headings_need() {
        let err = check_bump(Some(v("1.2.0")), v("1.2.1"), BumpLevel::Minor).expect_err("refused");
        assert!(
            err.to_string().contains("1.2.0 -> 1.2.1 is a patch step"),
            "{err}"
        );
        assert!(check_bump(Some(v("1.2.0")), v("1.3.0"), BumpLevel::Major).is_err());
    }

    #[test]
    fn accepts_a_step_at_or_above_the_requirement() {
        assert!(check_bump(Some(v("1.2.0")), v("1.2.1"), BumpLevel::Patch).is_ok());
        assert!(check_bump(Some(v("1.2.0")), v("2.0.0"), BumpLevel::Minor).is_ok());
        assert!(check_bump(Some(v("0.1.0")), v("0.2.0"), BumpLevel::Major).is_ok());
    }

    #[test]
    fn refuses_a_target_below_the_baseline() {
        assert!(check_bump(Some(v("0.2.0")), v("0.1.5"), BumpLevel::Patch).is_err());
    }

    #[test]
    fn tag_listing_keeps_only_version_tags() {
        let listing = "v0.1.0\nv0.2.0-rc.1\nvnext\nv0.10.2\n";
        assert_eq!(tag_versions(listing), [v("0.1.0"), v("0.10.2")]);
    }
}
