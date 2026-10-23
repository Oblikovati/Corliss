//! Reads a release's section of `CHANGELOG.md` (Keep a Changelog) and the version step its
//! headings require.
//!
//! Heading rules: `Fixed` and `Security` need a patch release; `Added`, `Changed` and
//! `Deprecated` need a minor one; `Removed`, a `Breaking` heading, or the word `BREAKING`
//! anywhere in the section need a major one. Any other heading is rejected rather than guessed.

use crate::error::XtaskError;
use crate::version::{BumpLevel, Version};

/// The body of the `## [X.Y.Z]` section for `version`: the lines up to the next `## ` heading.
///
/// ```text
/// let body = release_section(&changelog, Version::parse("0.2.0")?)?;
/// ```
pub fn release_section(changelog: &str, version: Version) -> Result<String, XtaskError> {
    let wanted = version.to_string();
    let mut lines = changelog.lines();
    if !lines
        .by_ref()
        .any(|line| section_name(line) == Some(wanted.as_str()))
    {
        return Err(XtaskError::Shape(format!(
            "CHANGELOG.md has no `## [{wanted}] - YYYY-MM-DD` section; move the release's \
             entries out of `## [Unreleased]` into it"
        )));
    }
    let body: Vec<&str> = lines.take_while(|line| !line.starts_with("## ")).collect();
    Ok(body.join("\n").trim().to_owned())
}

/// The smallest version step that the section's headings allow.
///
/// ```text
/// assert_eq!(required_bump("### Fixed\n- a fix")?, BumpLevel::Patch);
/// ```
pub fn required_bump(section: &str) -> Result<BumpLevel, XtaskError> {
    let mut required: Option<BumpLevel> = None;
    for line in section.lines() {
        let level = match line.strip_prefix("### ") {
            Some(heading) => Some(heading_level(heading.trim())?),
            None => line.contains("BREAKING").then_some(BumpLevel::Major),
        };
        required = required.max(level);
    }
    required.ok_or_else(|| {
        XtaskError::Shape(format!(
            "the release section has no `### ` change headings; found:\n{section}"
        ))
    })
}

/// The section as GitHub release notes, without the link-reference definitions
/// (`[0.1.0]: https://...`) that Keep a Changelog puts after the last section.
///
/// ```text
/// let notes = release_notes(&body);
/// ```
pub fn release_notes(section: &str) -> String {
    let kept: Vec<&str> = section
        .lines()
        .filter(|line| !is_link_definition(line))
        .collect();
    format!("{}\n", kept.join("\n").trim())
}

fn section_name(line: &str) -> Option<&str> {
    let rest = line.strip_prefix("## [")?;
    rest.split_once(']').map(|(name, _)| name.trim())
}

fn heading_level(heading: &str) -> Result<BumpLevel, XtaskError> {
    match heading {
        "Fixed" | "Security" => Ok(BumpLevel::Patch),
        "Added" | "Changed" | "Deprecated" => Ok(BumpLevel::Minor),
        "Removed" | "Breaking" => Ok(BumpLevel::Major),
        other => Err(XtaskError::Shape(format!(
            "unknown changelog heading `### {other}`; expected Added, Changed, Deprecated, \
             Removed, Fixed, Security or Breaking"
        ))),
    }
}

fn is_link_definition(line: &str) -> bool {
    line.starts_with('[') && line.contains("]: ")
}

#[cfg(test)]
mod tests {
    use super::*;

    const CHANGELOG: &str = "# Changelog\n\n## [Unreleased]\n\n### Added\n\n- next\n\n\
        ## [0.2.0] - 2026-10-01\n\n### Fixed\n\n- a fix\n\n### Added\n\n- a feature\n\n\
        ## [0.1.0] - 2026-09-01\n\n### Added\n\n- first\n\n\
        [0.2.0]: https://example.invalid/v0.2.0\n";

    fn v(text: &str) -> Version {
        Version::parse(text).expect("test version parses")
    }

    #[test]
    fn extracts_only_the_requested_section() {
        let body = release_section(CHANGELOG, v("0.2.0")).expect("section exists");
        assert!(body.starts_with("### Fixed") && body.ends_with("- a feature"));
        assert!(!body.contains("first") && !body.contains("next"));
    }

    #[test]
    fn last_section_runs_to_end_of_file() {
        let body = release_section(CHANGELOG, v("0.1.0")).expect("section exists");
        assert!(body.contains("- first") && body.contains("[0.2.0]: "));
    }

    #[test]
    fn missing_section_names_the_version() {
        let err = release_section(CHANGELOG, v("0.3.0")).expect_err("no 0.3.0 section");
        assert!(err.to_string().contains("## [0.3.0]"), "{err}");
    }

    #[test]
    fn heading_rules_pick_the_largest_requirement() {
        let level = |text: &str| required_bump(text).expect("valid section");
        assert_eq!(level("### Fixed\n- x\n### Security\n- y"), BumpLevel::Patch);
        assert_eq!(level("### Fixed\n- x\n### Added\n- y"), BumpLevel::Minor);
        assert_eq!(level("### Changed\n- x"), BumpLevel::Minor);
        assert_eq!(level("### Deprecated\n- x"), BumpLevel::Minor);
        assert_eq!(level("### Added\n- x\n### Removed\n- y"), BumpLevel::Major);
        assert_eq!(level("### Breaking\n- x"), BumpLevel::Major);
    }

    #[test]
    fn breaking_marker_requires_major() {
        let section = "### Changed\n- **BREAKING:** renamed `Document::open`";
        assert_eq!(required_bump(section).ok(), Some(BumpLevel::Major));
    }

    #[test]
    fn rejects_unknown_and_missing_headings() {
        assert!(required_bump("### Misc\n- x").is_err());
        assert!(required_bump("- an entry without a heading").is_err());
    }

    #[test]
    fn notes_drop_link_definitions() {
        let notes = release_notes("### Added\n\n- [docs](https://x.invalid)\n\n[0.1.0]: https://x");
        assert_eq!(notes, "### Added\n\n- [docs](https://x.invalid)\n");
    }
}
