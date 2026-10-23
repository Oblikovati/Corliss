//! Node decoding conformance: every fixture segment decodes to the same fields, names, values
//! and bytes as the reference grammar engine (whose fingerprints are in
//! `tests/fixtures/grammar-digests.tsv`, written by the reference implementation's export
//! tooling).

mod support;

use std::collections::BTreeMap;

use corliss::grammar::{DecodeStatus, fingerprint};
use corliss::{DecodedSegment, Document};

/// One line of the digest file: (node count, partial count, fingerprint) by (file, segment).
type Expected = BTreeMap<(String, String), (usize, usize, u64)>;

fn expected() -> Expected {
    let text = std::fs::read_to_string(support::fixtures_dir().join("grammar-digests.tsv"))
        .expect("digest file");
    text.lines()
        .filter_map(|line| {
            let cols: Vec<&str> = line.split('\t').collect();
            let [file, seg, n, partial, digest] = cols[..] else {
                return None;
            };
            let value = (
                n.parse().ok()?,
                partial.parse().ok()?,
                u64::from_str_radix(digest, 16).ok()?,
            );
            Some(((file.to_string(), seg.to_string()), value))
        })
        .collect()
}

fn segment_key(seg: &DecodedSegment, stream_id: &str) -> String {
    if seg.segment_type == "TemplateSegment" {
        format!("Templates/{stream_id}")
    } else {
        seg.name.clone()
    }
}

/// Decodes every segment of `label` and returns (key, nodes, partial, fingerprint, first error).
fn decode_file(label: &str) -> Vec<(String, usize, usize, u64, Option<String>)> {
    let doc = Document::open(support::fixtures_dir().join(label)).expect("fixture opens");
    let mut out = Vec::new();
    for loc in doc.segments().expect("segments") {
        let seg = doc
            .decode_nodes(&loc)
            .unwrap_or_else(|e| panic!("{label} {}: {e}", loc.name));
        let partial = seg
            .nodes
            .iter()
            .filter(|n| n.status != DecodeStatus::Full)
            .count();
        let error = seg
            .nodes
            .iter()
            .find_map(|n| n.error.clone().map(|e| format!("{} {e}", n.type_id)));
        out.push((
            segment_key(&seg, &loc.stream_id),
            seg.nodes.len(),
            partial,
            fingerprint(&seg.nodes),
            error,
        ));
    }
    out
}

fn check(mut filter: impl FnMut(&str) -> bool) {
    let expected = expected();
    let files: Vec<String> = expected
        .keys()
        .map(|(f, _)| f.clone())
        .filter(|f| filter(f))
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    assert!(!files.is_empty(), "no fixture matched");
    let mut failures = Vec::new();
    let mut segments = 0;
    for file in &files {
        for (key, n, partial, digest, error) in decode_file(file) {
            segments += 1;
            let want = expected.get(&(file.clone(), key.clone()));
            if want != Some(&(n, partial, digest)) {
                failures.push(format!(
                    "{file} {key}: got ({n}, {partial}, {digest:016x}) want {want:?} {}",
                    error.unwrap_or_default()
                ));
            }
        }
    }
    let shown: Vec<&String> = failures.iter().take(40).collect();
    assert!(
        failures.is_empty(),
        "{} of {segments} segments differ:\n{}",
        failures.len(),
        shown
            .iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    );
}

/// Debug builds check every fifth fixture (the interpreter is slow unoptimised); release builds
/// and `CORLISS_FIXTURE=<part of a name>` check every matching fixture.
#[test]
fn every_fixture_segment_matches_the_reference_engine() {
    let only = std::env::var("CORLISS_FIXTURE").ok();
    let sample = cfg!(debug_assertions) && only.is_none();
    let only = only.unwrap_or_default();
    let mut seen = 0;
    check(|f| {
        seen += 1;
        f.contains(only.as_str()) && (!sample || seen % 5 == 1)
    });
}
