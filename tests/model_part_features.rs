//! `Part::features` against the reference part reader's feature output.
//!
//! The adapter below renders the typed features as the reference reader's dicts: the same keys
//! per feature type, `None` as null, tuples as arrays.

mod support;

use corliss::model::part::*;
use corliss::model::{Model, Part};
use serde_json::Value;

use support::feature_kinds_json::feature_json;

fn features_json(label: &str) -> Value {
    let doc = support::open_label(label);
    let model = Model::load(&doc).expect("model decodes");
    let part = Part::new(&model).expect("part model");
    Value::Array(part.features().iter().map(feature_json).collect())
}

#[test]
fn features_match_reference_reader() {
    support::oracle::check_all("ipt", "features", features_json);
}

#[test]
fn two_direction_extrusion_reads_both_sides() {
    let doc = support::open_label("spec/ex_two_dir.ipt");
    let model = Model::load(&doc).expect("model decodes");
    let part = Part::new(&model).expect("part model");
    let features = part.features();
    let cut = features
        .iter()
        .find(|f| f.name.as_deref() == Some("Extrusion2"))
        .expect("second extrusion");
    let FeatureKind::Extrusion(e) = &cut.kind else {
        panic!("not an extrusion: {:?}", cut.kind)
    };
    assert_eq!(e.operation, Some(FeatureOperation::Cut));
    let side = e.direction_b.as_ref().expect("second direction");
    assert_eq!(side.extent, Some(FeatureExtent::Distance));
}
