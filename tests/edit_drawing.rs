//! Drawing edits (`corliss::edit::drawing`) against the reference writer's verified drawing
//! jobs: each edited document must hold the same streams, after decompression, as the
//! reference output in `tests/fixtures/edits`.

mod support;

use corliss::edit::drawing;
use support::edits;

fn assert_job(name: &str, got: corliss::Document) {
    let want = edits::expected(name);
    assert_eq!(edits::diff(&got, &want), None, "{name}");
}

#[test]
fn changing_a_note_text_matches_the_reference() {
    let mut ed = edits::editor("R1_note.idw", "spec/dw_note.idw");
    drawing::set_note_text(&mut ed, 1, "HELLO NOTE", "HELLO WRITER").unwrap();
    assert_job("R1_note.idw", ed.finish().unwrap());
}

#[test]
fn renaming_a_sheet_matches_the_reference() {
    let mut ed = edits::editor("R2_rename.idw", "spec/dw_proj.idw");
    drawing::rename_sheet(&mut ed, 1, "MAIN").unwrap();
    assert_job("R2_rename.idw", ed.finish().unwrap());
}

#[test]
fn moving_a_view_matches_the_reference() {
    let mut ed = edits::editor("R3_move.idw", "spec/dw_proj.idw");
    drawing::move_view(&mut ed, 1, "VIEW1", 21.59, 30.742193571192765).unwrap();
    assert_job("R3_move.idw", ed.finish().unwrap());
}

#[test]
fn scaling_a_view_matches_the_reference() {
    let mut ed = edits::editor("R4_scale.idw", "spec/dw_proj.idw");
    drawing::set_view_scale(&mut ed, 1, "VIEW1", 1.0).unwrap();
    assert_job("R4_scale.idw", ed.finish().unwrap());
}

#[test]
fn adding_a_note_matches_the_reference() {
    let mut ed = edits::editor("R6_addnote.idw", "spec/dw_proj.idw");
    let template = support::open_spec("dw_note.idw");
    drawing::add_note(&mut ed, 1, "WRITER NOTE", 5.0, 20.0, &template).unwrap();
    assert_job("R6_addnote.idw", ed.finish().unwrap());
}

#[test]
fn deleting_a_note_matches_the_reference() {
    let mut ed = edits::editor("R7_delnote.idw", "spec/dw_note.idw");
    drawing::delete_note(&mut ed, 1, "HELLO NOTE").unwrap();
    assert_job("R7_delnote.idw", ed.finish().unwrap());
}
