//! Reads and rewrites the `[package] version` of the root `Cargo.toml`, line by line so every
//! other byte (comments, key order) is kept.

use crate::error::XtaskError;
use crate::version::Version;

/// The version declared in the manifest's `[package]` table.
///
/// ```text
/// let current = package_version(&std::fs::read_to_string("Cargo.toml")?)?;
/// ```
pub fn package_version(manifest: &str) -> Result<Version, XtaskError> {
    let index = version_line(manifest)?;
    let line = manifest.lines().nth(index).unwrap_or_default();
    Version::parse(quoted_value(line)?)
}

/// The manifest with its `[package]` version replaced by `version`.
///
/// ```text
/// let bumped = with_package_version(&manifest, Version::parse("0.2.0")?)?;
/// ```
pub fn with_package_version(manifest: &str, version: Version) -> Result<String, XtaskError> {
    let index = version_line(manifest)?;
    let newline = if manifest.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let mut lines: Vec<String> = manifest.lines().map(String::from).collect();
    if let Some(slot) = lines.get_mut(index) {
        *slot = format!("version = \"{version}\"");
    }
    let mut text = lines.join(newline);
    if manifest.ends_with('\n') {
        text.push_str(newline);
    }
    Ok(text)
}

fn version_line(manifest: &str) -> Result<usize, XtaskError> {
    let mut table = "";
    for (index, line) in manifest.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            table = trimmed;
        } else if table == "[package]" && key_of(trimmed) == Some("version") {
            return Ok(index);
        }
    }
    Err(XtaskError::Shape(
        "Cargo.toml has no `version = \"X.Y.Z\"` line in its [package] table".to_owned(),
    ))
}

fn key_of(line: &str) -> Option<&str> {
    line.split_once('=').map(|(key, _)| key.trim())
}

fn quoted_value(line: &str) -> Result<&str, XtaskError> {
    let value = line.split_once('=').map(|(_, value)| value.trim());
    value
        .and_then(|v| v.strip_prefix('"'))
        .and_then(|v| v.split_once('"'))
        .map(|(inner, _)| inner)
        .ok_or_else(|| {
            XtaskError::Shape(format!(
                "Cargo.toml version line `{line}` is not `version = \"X.Y.Z\"`"
            ))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    const MANIFEST: &str = "[package]\nname = \"corliss\"\n# keep me\nversion = \"0.1.0\"\n\n\
        [dependencies]\nversion = \"9.9.9\"\n";

    #[test]
    fn reads_the_package_version_only() {
        let version = package_version(MANIFEST).expect("manifest has a version");
        assert_eq!(version.to_string(), "0.1.0");
    }

    #[test]
    fn rewrites_only_the_version_line() {
        let next = Version::parse("0.2.0").expect("valid");
        let bumped = with_package_version(MANIFEST, next).expect("manifest has a version");
        assert_eq!(bumped, MANIFEST.replace("\"0.1.0\"", "\"0.2.0\""));
    }

    #[test]
    fn keeps_crlf_line_endings() {
        let crlf = MANIFEST.replace('\n', "\r\n");
        let next = Version::parse("0.1.1").expect("valid");
        let bumped = with_package_version(&crlf, next).expect("manifest has a version");
        assert_eq!(bumped, crlf.replace("\"0.1.0\"", "\"0.1.1\""));
    }

    #[test]
    fn rejects_manifests_without_a_literal_version() {
        assert!(package_version("[package]\nname = \"x\"\n").is_err());
        assert!(package_version("[package]\nversion.workspace = true\n").is_err());
    }
}
