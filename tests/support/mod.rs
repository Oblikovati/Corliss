//! Fixture access shared by the integration tests.

// Each test binary compiles this module and uses only some of its helpers.
#![allow(dead_code)]

use std::path::{Path, PathBuf};

use corliss::Document;

/// Root of the Inventor 2027 fixtures copied from the reference implementation.
pub fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
}

/// Every fixture document, sorted by path.
pub fn all_fixtures() -> Vec<PathBuf> {
    let mut out = Vec::new();
    for dir in [fixtures_dir(), fixtures_dir().join("spec")] {
        let entries = std::fs::read_dir(&dir).expect("fixture directory is readable");
        out.extend(
            entries
                .map(|e| e.expect("fixture entry").path())
                .filter(|p| is_document(p)),
        );
    }
    out.sort();
    out
}

fn is_document(path: &Path) -> bool {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default();
    matches!(ext, "ipt" | "iam" | "idw" | "ipn" | "ide")
}

/// Opens the named fixture under `tests/fixtures/spec`.
pub fn open_spec(name: &str) -> Document {
    Document::open(fixtures_dir().join("spec").join(name)).expect("fixture opens")
}

/// The file name of a fixture path, for assertion messages.
pub fn label(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

pub mod oracle;

/// Opens the fixture at `label` (relative to `tests/fixtures`).
pub fn open_label(label: &str) -> Document {
    Document::open(fixtures_dir().join(label)).expect("fixture opens")
}
pub mod edits;
pub mod feature_json;
pub mod feature_kinds_json;
pub mod roots;
