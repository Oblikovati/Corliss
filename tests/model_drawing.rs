//! `corliss::model::Drawing` against the reference drawing reader's output for every drawing
//! fixture.

mod support;

use corliss::model::drawing::{
    Dimension, DimensionKind, DimensionText, DrawingReference, DrawingView, Note, Orientation,
    Sheet, SheetFormat, SheetSize, TitleBlockField, plain,
};
use corliss::model::{Drawing, Model};
use serde_json::{Value, json};
use support::oracle::check_all;

/// Opens the fixture at `label` and maps its drawing with `f`.
fn with_drawing(label: &str, f: impl Fn(&Drawing<'_>) -> Value) -> Value {
    let doc = support::open_label(label);
    let model = Model::load(&doc).expect("model loads");
    let drawing = Drawing::new(&model).expect("drawing reads");
    f(&drawing)
}

fn size_json(size: &SheetSize) -> Value {
    match size {
        SheetSize::Named(name) => json!(name),
        SheetSize::Code(code) => json!(code),
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// A position as the reference reader reports it: empty when the node has no `loc`.
fn pos_json(pos: Option<[f64; 2]>) -> Value {
    pos.map_or(json!([]), |p| json!(p))
}

fn sheet_json(s: &Sheet) -> Value {
    json!({"index": s.index, "segment": s.segment, "name": s.name, "base_name": s.base_name,
           "size": s.size.as_ref().map(size_json), "width": s.width, "height": s.height})
}

fn orientation_json(o: Orientation) -> Value {
    match o {
        Orientation::Landscape => json!("landscape"),
        Orientation::Portrait => json!("portrait"),
        Orientation::Other(code) => json!(code),
    }
}

fn sheet_format_json(f: &SheetFormat) -> Value {
    let preview = f.preview.map(|p| {
        json!({"format": p.format, "mapping_mode": p.mapping_mode, "width": p.width, "height": p.height})
    });
    json!({"width": f.width, "height": f.height, "size": size_json(&f.size),
           "orientation": orientation_json(f.orientation), "version": f.version,
           "sheet_name": f.sheet_name, "flag": f.flag, "base_views": f.base_views,
           "projected_views": f.projected_views, "extra": f.extra, "preview": preview,
           "png": f.png.as_deref().map(hex), "pid": f.pid, "name": f.name})
}

fn reference_json(r: &DrawingReference) -> Value {
    let f = &r.file;
    json!({"kind": "inventor", "path": f.path, "relative_path": f.relative_path,
           "library_id": r.library_id, "library_name": f.library_name,
           "internal_name": f.internal_name.to_string(), "db_revision": f.db_revision.to_string(),
           "ref_id": f.ref_id, "n_occurrences": f.n_occurrences, "flags": r.flags})
}

fn view_json(v: &DrawingView) -> Value {
    json!({"sheet": v.sheet, "name": v.name, "model": v.model, "centre": v.centre,
           "scale": v.scale, "scale_text": v.scale_text, "frame": v.frame, "label": v.label,
           "nodes": {"scale": v.nodes.scale, "centre": v.nodes.centre, "frame": v.nodes.frame}})
}

fn title_block_json(f: &TitleBlockField) -> Value {
    json!({"text": f.text, "pos": f.pos, "node": f.node})
}

fn note_json(n: &Note) -> Value {
    json!({"text": n.text, "raw": n.raw, "pos": pos_json(n.pos), "node": n.node})
}

fn dimension_json(d: &Dimension) -> Value {
    let kind = match d.kind {
        DimensionKind::Linear => "linear",
    };
    json!({"node": d.node, "type": kind, "points": d.points, "text_pos": d.text_pos,
           "text": d.text, "view": d.view, "measured": d.measured})
}

fn list<T>(items: &[T], f: fn(&T) -> Value) -> Value {
    Value::Array(items.iter().map(f).collect())
}

#[test]
fn sheets_match_the_reference() {
    check_all("idw", "sheets", |label| {
        with_drawing(label, |d| list(&d.sheets(), sheet_json))
    });
}

#[test]
fn sheet_formats_match_the_reference() {
    check_all("idw", "sheet_formats", |label| {
        with_drawing(label, |d| {
            list(
                &d.sheet_formats().expect("sheet formats parse"),
                sheet_format_json,
            )
        })
    });
}

#[test]
fn references_match_the_reference() {
    check_all("idw", "references", |label| {
        with_drawing(label, |d| {
            list(&d.references().expect("UFRxDoc parses"), reference_json)
        })
    });
}

#[test]
fn views_match_the_reference() {
    check_all("idw", "views", |label| {
        with_drawing(label, |d| list(&d.views(None), view_json))
    });
}

#[test]
fn title_block_matches_the_reference() {
    check_all("idw", "title_block", |label| {
        with_drawing(label, |d| {
            list(&d.title_block(1).expect("sheet 1 exists"), title_block_json)
        })
    });
}

#[test]
fn notes_match_the_reference() {
    check_all("idw", "notes", |label| {
        with_drawing(label, |d| {
            list(&d.notes(1).expect("sheet 1 exists"), note_json)
        })
    });
}

#[test]
fn dimensions_match_the_reference() {
    check_all("idw", "dimensions", |label| {
        with_drawing(label, |d| {
            list(&d.dimensions(1).expect("sheet 1 exists"), dimension_json)
        })
    });
}

#[test]
fn dimension_texts_are_the_free_texts_without_view_labels() {
    let doc = support::open_spec("dw_dim.idw");
    let model = Model::load(&doc).unwrap();
    let drawing = Drawing::new(&model).unwrap();
    let texts: Vec<DimensionText> = drawing.dimension_texts(1).unwrap();
    assert!(texts.iter().any(|t| t.text == "40.00"), "{texts:?}");
    assert!(texts.iter().all(|t| !t.text.contains("\\Y")));
}

#[test]
fn sheet_prefixes_follow_creation_order_and_view_entries_split_the_browser_name() {
    let doc = support::open_spec("dw_asm.idw");
    let model = Model::load(&doc).unwrap();
    let drawing = Drawing::new(&model).unwrap();
    assert_eq!(drawing.sheet_prefixes(), ["DLSheet21", "DLSheet22"]);
    let entries = drawing.view_entries();
    assert!(
        entries.contains(&("VIEW3".to_string(), "as_mate.iam".to_string())),
        "{entries:?}"
    );
}

#[test]
fn a_missing_sheet_is_an_error_naming_the_sheet() {
    let doc = support::open_spec("dw_dim.idw");
    let model = Model::load(&doc).unwrap();
    let drawing = Drawing::new(&model).unwrap();
    let err = drawing.notes(7).unwrap_err().to_string();
    assert!(err.contains("sheet 7"), "{err}");
    assert!(drawing.views(Some(7)).is_empty());
}

#[test]
fn plain_strips_format_codes_like_the_reference_regex() {
    assert_eq!(plain("\\F29;HELLO NOTE"), "HELLO NOTE");
    assert_eq!(plain("\\Y0;\\nSCALE \\Y1;"), "\\nSCALE ");
    assert_eq!(plain("\\H-0.5;x\\;y\\Q;"), "x\\;y");
    assert_eq!(plain("\\F12x;"), "\\F12x;");
    assert_eq!(plain(""), "");
}
