//! Structural edits (writer class W3) against the reference implementation's verified edit
//! jobs: the documents Corliss writes must hold the same streams, after decompression, as the
//! reference outputs in `tests/fixtures/edits`.

mod support;

use support::edits;

fn assert_job(name: &str, got: corliss::Document) {
    let want = edits::expected(name);
    assert_eq!(edits::diff(&got, &want), None, "{name}");
}

#[test]
fn rebuilding_an_untouched_segment_matches_the_reference() {
    let mut ed = edits::editor("P0_touch.ipt", "spec/ex_rect20.ipt");
    ed.segment("PmDCSegment").unwrap();
    assert_job("P0_touch.ipt", ed.finish().unwrap());
}

#[test]
fn adding_a_user_parameter_matches_the_reference() {
    let mut ed = edits::editor("P1_userparam.ipt", "fixture_part.ipt");
    corliss::edit::add_user_parameter(&mut ed, "my_len2", 3.0).unwrap();
    assert_job("P1_userparam.ipt", ed.finish().unwrap());
}

#[test]
fn the_comparison_reports_an_unedited_document() {
    let doc = support::open_label("fixture_part.ipt");
    assert!(edits::diff(&doc, &edits::expected("P1_userparam.ipt")).is_some());
}
