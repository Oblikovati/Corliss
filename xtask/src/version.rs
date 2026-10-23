//! Release versions and the size of the step between two of them.

use std::fmt;

use crate::error::XtaskError;

/// A `MAJOR.MINOR.PATCH` release version. Pre-release and build suffixes are not used by
/// Corliss releases and are rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Version {
    /// Incompatible API changes.
    pub major: u64,
    /// Additive changes.
    pub minor: u64,
    /// Fixes.
    pub patch: u64,
}

/// How large a version step is, ordered so a larger step satisfies a smaller requirement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum BumpLevel {
    /// Fixes only.
    Patch,
    /// Additive, backward-compatible changes.
    Minor,
    /// Breaking changes.
    Major,
}

impl Version {
    /// Parses `X.Y.Z`, also accepting a tag's leading `v` (`v0.2.0`).
    ///
    /// ```text
    /// assert_eq!(Version::parse("v1.2.3")?.minor, 2);
    /// ```
    pub fn parse(text: &str) -> Result<Self, XtaskError> {
        let bare = text.trim().strip_prefix('v').unwrap_or(text.trim());
        let parts: Vec<&str> = bare.split('.').collect();
        let [major, minor, patch] = parts.as_slice() else {
            return Err(invalid(
                text,
                format!("found {} dot-separated parts", parts.len()),
            ));
        };
        Ok(Self {
            major: component(text, major)?,
            minor: component(text, minor)?,
            patch: component(text, patch)?,
        })
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl fmt::Display for BumpLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::Patch => "patch",
            Self::Minor => "minor",
            Self::Major => "major",
        };
        f.write_str(name)
    }
}

/// The compatibility level of the step `from -> to`, using Cargo's rules: below 1.0.0 the
/// leftmost non-zero component is the breaking one, so `0.1.x -> 0.2.0` is a major step and
/// `0.1.0 -> 0.1.1` may carry additive changes. Fails unless `to` is greater than `from`.
///
/// ```text
/// assert_eq!(bump_level(v("0.1.0"), v("0.2.0"))?, BumpLevel::Major);
/// ```
pub fn bump_level(from: Version, to: Version) -> Result<BumpLevel, XtaskError> {
    if to <= from {
        return Err(XtaskError::Release(format!(
            "version {to} is not greater than the previous release {from}"
        )));
    }
    if to.major != from.major || (from.major == 0 && (to.minor != from.minor || from.minor == 0)) {
        return Ok(BumpLevel::Major);
    }
    if from.major == 0 || to.minor != from.minor {
        return Ok(BumpLevel::Minor);
    }
    Ok(BumpLevel::Patch)
}

fn component(text: &str, part: &str) -> Result<u64, XtaskError> {
    let digits_only = !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit());
    if !digits_only || (part.len() > 1 && part.starts_with('0')) {
        return Err(invalid(
            text,
            format!("component `{part}` is not a plain number"),
        ));
    }
    part.parse()
        .map_err(|err| invalid(text, format!("component `{part}`: {err}")))
}

fn invalid(text: &str, reason: String) -> XtaskError {
    XtaskError::Version {
        text: text.to_owned(),
        reason,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(text: &str) -> Version {
        Version::parse(text).expect("test version parses")
    }

    #[test]
    fn parses_plain_and_tag_versions() {
        assert_eq!(
            v("1.2.3"),
            Version {
                major: 1,
                minor: 2,
                patch: 3
            }
        );
        assert_eq!(
            v("v0.10.0"),
            Version {
                major: 0,
                minor: 10,
                patch: 0
            }
        );
    }

    #[test]
    fn rejects_malformed_versions() {
        for bad in [
            "",
            "1.2",
            "1.2.3.4",
            "1.2.x",
            "1.02.3",
            "1.2.3-rc.1",
            "+1.2.3",
        ] {
            assert!(Version::parse(bad).is_err(), "{bad} should be rejected");
        }
    }

    #[test]
    fn orders_numerically_not_lexically() {
        assert!(v("0.10.0") > v("0.9.9"));
        assert_eq!(v("1.0.0").to_string(), "1.0.0");
    }

    #[test]
    fn classifies_steps_at_and_above_one() {
        assert_eq!(
            bump_level(v("1.2.3"), v("2.0.0")).ok(),
            Some(BumpLevel::Major)
        );
        assert_eq!(
            bump_level(v("1.2.3"), v("1.3.0")).ok(),
            Some(BumpLevel::Minor)
        );
        assert_eq!(
            bump_level(v("1.2.3"), v("1.2.4")).ok(),
            Some(BumpLevel::Patch)
        );
    }

    #[test]
    fn classifies_steps_below_one_with_cargo_rules() {
        assert_eq!(
            bump_level(v("0.1.0"), v("0.2.0")).ok(),
            Some(BumpLevel::Major)
        );
        assert_eq!(
            bump_level(v("0.1.0"), v("0.1.1")).ok(),
            Some(BumpLevel::Minor)
        );
        assert_eq!(
            bump_level(v("0.9.0"), v("1.0.0")).ok(),
            Some(BumpLevel::Major)
        );
        assert_eq!(
            bump_level(v("0.0.1"), v("0.0.2")).ok(),
            Some(BumpLevel::Major)
        );
    }

    #[test]
    fn refuses_steps_that_do_not_increase() {
        assert!(bump_level(v("0.2.0"), v("0.2.0")).is_err());
        assert!(bump_level(v("0.2.0"), v("0.1.9")).is_err());
    }

    #[test]
    fn larger_steps_satisfy_smaller_requirements() {
        assert!(BumpLevel::Major > BumpLevel::Minor && BumpLevel::Minor > BumpLevel::Patch);
    }
}
