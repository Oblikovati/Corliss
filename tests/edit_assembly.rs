//! Assembly edits against the reference writer's verified assembly jobs: the documents Corliss
//! writes must hold the same streams, after decompression, as the reference outputs in
//! `tests/fixtures/edits`.

mod support;

use corliss::edit::assembly::{self, IDENTITY, Placement};
use support::edits;

fn assert_job(name: &str, got: corliss::Document) {
    let want = edits::expected(name);
    assert_eq!(edits::diff(&got, &want), None, "{name}");
}

#[test]
fn setting_a_mate_offset_matches_the_reference() {
    let mut ed = edits::editor("W1_mate20.iam", "spec/as_mate.iam");
    assert_eq!(
        assembly::set_constraint_value(&mut ed, "Mate:1", 2.0, 0, true).unwrap(),
        "d0"
    );
    assert_job("W1_mate20.iam", ed.finish().unwrap());
}

#[test]
fn grounding_an_occurrence_matches_the_reference() {
    let mut ed = edits::editor("W2_ground2.iam", "spec/as_place2.iam");
    assembly::set_grounded(&mut ed, "ex_rect20:2", true).unwrap();
    assert_job("W2_ground2.iam", ed.finish().unwrap());
}

#[test]
fn moving_an_occurrence_matches_the_reference() {
    let mut ed = edits::editor("W3_move2.iam", "spec/as_place2.iam");
    let m: Placement = [
        [0.0, -1.0, 0.0, 10.0],
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 3.0],
        [0.0, 0.0, 0.0, 1.0],
    ];
    assert_eq!(
        assembly::set_occurrence_transform(&mut ed, "ex_rect20:2", &m).unwrap(),
        2
    );
    assert_job("W3_move2.iam", ed.finish().unwrap());
}

#[test]
fn an_unknown_occurrence_is_reported_with_the_known_labels() {
    let mut ed = edits::editor("W2_ground2.iam", "spec/as_place2.iam");
    let err = assembly::set_grounded(&mut ed, "nope:1", true)
        .unwrap_err()
        .to_string();
    assert!(
        err.contains("nope:1") && err.contains("ex_rect20:2"),
        "{err}"
    );
}

#[test]
fn the_identity_placement_is_the_unit_matrix() {
    let unit: Placement =
        std::array::from_fn(|r| std::array::from_fn(|c| if r == c { 1.0 } else { 0.0 }));
    assert_eq!(IDENTITY, unit);
}

#[test]
fn removing_a_mate_matches_the_reference() {
    let mut ed = edits::editor("W8_delmate.iam", "spec/as_mate.iam");
    assembly::remove_constraint(&mut ed, "Mate:1").unwrap();
    assert_job("W8_delmate.iam", ed.finish().unwrap());
}

#[test]
fn removing_an_occurrence_matches_the_reference() {
    let mut ed = edits::editor("W6_del2.iam", "spec/as_place3.iam");
    assert_eq!(
        assembly::remove_occurrence(&mut ed, "ex_rect20:2").unwrap(),
        2
    );
    assert_job("W6_del2.iam", ed.finish().unwrap());
}

#[test]
fn adding_an_occurrence_matches_the_reference() {
    let mut ed = edits::editor("W5_add3.iam", "spec/as_place2.iam");
    let mut t2 = IDENTITY;
    t2[2][3] = 6.0;
    let (label, id) =
        assembly::add_occurrence(&mut ed, "ex_rect20:2", &t2, Some("ex_rect20:3")).unwrap();
    assert_eq!((label.as_str(), id), ("ex_rect20:3", 3));
    assert_job("W5_add3.iam", ed.finish().unwrap());
}

const PAIR: [(&str, &str); 2] = [
    ("ex_rect20:1", "ex_rect20:1"),
    ("ex_rect20:2", "ex_rect20:2"),
];

fn copy_constraint(job: &str, source: &str, template: &str, name: &str, pairs: &[(&str, &str)]) {
    let mut ed = edits::editor(job, source);
    let template = support::open_label(template);
    assembly::add_constraint_from(&mut ed, &template, name, pairs).unwrap();
    let opened: Vec<String> = ed.opened().map(str::to_string).collect();
    let got = ed.finish().unwrap();
    // Regression (a reference bug, fixed): the class-list insertion shifted root_a.
    assert_eq!(
        support::roots::wrong_root_classes(&got, &opened),
        Vec::<String>::new(),
        "{job}"
    );
    assert_job(job, got);
}

#[test]
fn copying_a_mate_back_matches_the_reference() {
    copy_constraint(
        "W10_addmate.iam",
        "spec/as_delmate.iam",
        "spec/as_mate.iam",
        "Mate:1",
        &PAIR,
    );
}

#[test]
fn copying_a_flush_next_to_a_mate_matches_the_reference() {
    copy_constraint(
        "W13_flush.iam",
        "spec/as_mate.iam",
        "spec/as_flush.iam",
        "Flush:1",
        &PAIR,
    );
}

#[test]
fn copying_an_angle_onto_remapped_occurrences_matches_the_reference() {
    let pairs = [
        ("ex_rect20:1", "ex_rect20:1"),
        ("ex_rect20:3", "ex_rect20:2"),
    ];
    copy_constraint(
        "W14_angle.iam",
        "spec/as_place2.iam",
        "spec/as_angle.iam",
        "Angle:1",
        &pairs,
    );
}
