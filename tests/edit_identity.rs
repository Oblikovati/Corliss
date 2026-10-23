//! Rebuilding without edits: opening every segment of a document in the editor and finishing
//! must give back the document's streams, after decompression (M header and body, B blocks and
//! end table, RSeSegInfo). Every derived field the commit rewrites is then shown to equal what
//! Inventor wrote, over the whole corpus.

mod support;

use corliss::edit::Editor;
use support::edits;

/// Debug builds check every fifth fixture (the grammar interpreter is slow unoptimised);
/// release builds and `CORLISS_FIXTURE=<part of a name>` check every matching fixture.
#[test]
fn every_segment_rebuilds_to_what_inventor_wrote() {
    let only = std::env::var("CORLISS_FIXTURE").ok();
    let sample = cfg!(debug_assertions) && only.is_none();
    let only = only.unwrap_or_default();
    let mut failures = Vec::new();
    let mut checked = 0;
    for (k, path) in support::all_fixtures().iter().enumerate() {
        let label = support::label(path);
        if !label.contains(only.as_str()) || (sample && k % 5 != 0) {
            continue;
        }
        checked += 1;
        if let Some(d) = rebuild_difference(path) {
            failures.push(format!("{label}: {d}"));
        }
    }
    assert!(checked > 0, "no fixture matched");
    assert!(
        failures.is_empty(),
        "{} of {checked} documents differ:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

/// The first difference between a fixture and its rebuild with every segment opened.
fn rebuild_difference(path: &std::path::Path) -> Option<String> {
    let doc = corliss::Document::open(path).unwrap();
    let names: Vec<String> = doc
        .seg_info()
        .unwrap()
        .segments
        .iter()
        .map(|e| e.name.clone())
        .filter(|n| !n.is_empty())
        .collect();
    let mut ed = Editor::new(&doc).unwrap();
    for name in &names {
        if let Err(e) = ed.segment(name) {
            return Some(format!("{name} does not open: {e}"));
        }
    }
    match ed.finish() {
        Ok(rebuilt) => edits::diff(&rebuilt, &doc),
        Err(e) => Some(format!("commit fails: {e}")),
    }
}
