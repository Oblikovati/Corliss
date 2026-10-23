//! `corliss::model::Assembly` against the reference assembly reader: every method's
//! output, converted to the reference's JSON shape, must equal `tests/fixtures/models.json`
//! for every .iam fixture.

mod support;

use corliss::model::assembly::{
    AssemblyFeature, BomStructure, Constraint, ConstraintKind, ConstraintValue, DesignViews,
    Geometry, Occurrence, Pattern, PatternLayout, PositionalOverride, Representations, Selection,
    ValueKind,
};
use corliss::model::{Assembly, Matrix4, Model};
use serde_json::{Map, Value, json};
use support::oracle;

/// Runs `read` on the assembly of fixture `label`.
fn with_assembly(label: &str, read: impl Fn(&Assembly<'_>) -> Value) -> Value {
    let doc = support::open_label(label);
    let model = Model::load(&doc).expect("model loads");
    let assembly = Assembly::new(&model).expect("assembly reads");
    read(&assembly)
}

fn matrix(m: &Option<Matrix4>) -> Value {
    m.map_or(Value::Null, |m| json!(m))
}

fn occurrence(o: &Occurrence) -> Value {
    let bom = o.bom_structure.map_or(Value::Null, |b| match b {
        BomStructure::Default => json!("default"),
        BomStructure::Reference => json!("reference"),
        BomStructure::Other(code) => json!(code),
    });
    let mut v = json!({
        "index": o.index, "name": o.name, "id": o.id, "grounded": o.grounded,
        "pattern_member": o.pattern_member, "hidden": o.hidden, "suppressed": o.suppressed,
        "bom_structure": bom, "transform": matrix(&o.transform),
    });
    // The reference adds file and instance only when the UFRxDoc has a record.
    if let Some(instance) = o.instance {
        v["file"] = json!(o.file);
        v["instance"] = json!(instance);
    }
    v
}

fn design_views(d: &DesignViews) -> Value {
    let views: Vec<Value> = d
        .views
        .iter()
        .map(|v| {
            let mut out = json!({"name": v.name, "hidden": v.hidden});
            if let Some(c) = &v.camera {
                out["camera"] = json!({"target": c.target, "eye": c.eye});
            }
            out
        })
        .collect();
    json!({"active": d.active, "views": views})
}

fn positional_override(p: &PositionalOverride) -> Value {
    let mut values = Map::new();
    for (name, value) in &p.values {
        // Python's dict key None serialises as "null".
        values.insert(name.clone().unwrap_or_else(|| "null".into()), json!(value));
    }
    json!({"item": p.item, "master": p.master, "values": values})
}

fn representations(r: &Representations) -> Value {
    json!({
        "model_states": r.model_states,
        "design_views": design_views(&r.design_views),
        "positional": r.positional,
        "positional_overrides": r.positional_overrides.iter().map(positional_override).collect::<Vec<_>>(),
    })
}

fn pattern(p: &Pattern) -> Value {
    let elements: Vec<Value> = p
        .elements
        .iter()
        .map(|e| json!({"element": e.element, "occurrences": e.occurrences}))
        .collect();
    let mut v =
        json!({"index": p.index, "name": p.name, "sources": p.sources, "elements": elements});
    match &p.layout {
        Some(PatternLayout::Rectangular {
            counts,
            spacings,
            directions,
        }) => {
            v["type"] = json!("rectangular");
            v["counts"] = json!(counts);
            v["spacings"] = json!(spacings);
            v["directions"] = json!(directions);
        }
        Some(PatternLayout::Circular { count, angle, axis }) => {
            v["type"] = json!("circular");
            v["count"] = json!(count);
            v["angle"] = json!(angle);
            v["axis"] = axis.as_ref().map_or(
                Value::Null,
                |a| json!({"point": a.point, "direction": a.direction}),
            );
        }
        None => {}
    }
    v
}

fn geometry(g: &Geometry) -> Value {
    match g {
        Geometry::Plane { origin, normal } => {
            json!({"kind": "plane", "origin": origin, "normal": normal})
        }
        Geometry::Vector { direction } => json!({"kind": "vector", "direction": direction}),
        Geometry::Circle {
            center,
            normal,
            radius,
        } => {
            json!({"kind": "circle", "center": center, "normal": normal, "radius": radius})
        }
        Geometry::Cylinder {
            radius,
            axis_point,
            axis,
        } => {
            json!({"kind": "cylinder", "radius": radius, "axis_point": axis_point, "axis": axis})
        }
        Geometry::JointOrigin { frame } => json!({"kind": "joint origin", "frame": matrix(frame)}),
        Geometry::Axis => json!({"kind": "axis"}),
        Geometry::Point => json!({"kind": "point"}),
        Geometry::Other { type_id } => json!({"kind": format!("{type_id:08X}")}),
    }
}

fn selection(s: &Selection) -> Value {
    let g = match s {
        Selection::Unsupported { type_id } => return json!({"node": format!("{type_id:08X}")}),
        Selection::Geometry(g) => g,
    };
    let mut v = json!({"occurrence": g.occurrence, "geometry": g.geometry.as_ref().map(geometry)});
    if let Some(uid) = &g.imate_uid {
        v["imate_uid"] = json!(uid);
    }
    if let Some(p) = &g.pick_point {
        v["pick_point"] = json!(p);
    }
    v
}

fn constraint_type(k: ConstraintKind) -> Value {
    json!(match k {
        ConstraintKind::Mate => "mate",
        ConstraintKind::Flush => "flush",
        ConstraintKind::Angle => "angle",
        ConstraintKind::Tangent => "tangent",
        ConstraintKind::Insert => "insert",
        ConstraintKind::JointRotational => "joint: rotational",
        ConstraintKind::JointSlider => "joint: slider",
        ConstraintKind::JointRigid => "joint: rigid",
        ConstraintKind::JointBall => "joint: ball",
        ConstraintKind::JointCylindrical => "joint: cylindrical",
        ConstraintKind::JointPlanar => "joint: planar",
        ConstraintKind::Other(style) => return json!(style),
    })
}

fn constraint_value(v: &ConstraintValue) -> Value {
    let kind = match v.kind {
        ValueKind::Offset => json!("offset"),
        ValueKind::Angle => json!("angle"),
        ValueKind::JointAngle => json!("joint angle"),
        ValueKind::JointPosition => json!("joint position"),
        ValueKind::Other(n0, n1) => json!([n0, n1]),
    };
    json!({"kind": kind, "parameter": v.parameter, "value": v.value})
}

fn constraint(c: &Constraint) -> Value {
    json!({
        "index": c.index, "name": c.name, "type": constraint_type(c.kind), "flags": c.flags,
        "from_imates": c.from_imates,
        "selections": c.selections.iter().map(selection).collect::<Vec<_>>(),
        "values": c.values.iter().map(constraint_value).collect::<Vec<_>>(),
    })
}

fn list<T>(items: Vec<T>, convert: fn(&T) -> Value) -> Value {
    Value::Array(items.iter().map(convert).collect())
}

#[test]
fn files_match_reference() {
    oracle::check_all("iam", "files", |label| {
        with_assembly(label, |a| {
            let files = a
                .files()
                .into_iter()
                .map(|(id, path)| (id.to_string(), json!(path)));
            Value::Object(files.collect())
        })
    });
}

#[test]
fn occurrences_match_reference() {
    oracle::check_all("iam", "occurrences", |label| {
        with_assembly(label, |a| list(a.occurrences(), occurrence))
    });
}

#[test]
fn representations_match_reference() {
    oracle::check_all("iam", "representations", |label| {
        with_assembly(label, |a| representations(&a.representations()))
    });
}

#[test]
fn patterns_match_reference() {
    oracle::check_all("iam", "patterns", |label| {
        with_assembly(label, |a| list(a.patterns(), pattern))
    });
}

#[test]
fn constraints_match_reference() {
    oracle::check_all("iam", "constraints", |label| {
        with_assembly(label, |a| list(a.constraints(), constraint))
    });
}

/// The reference's `features` are the part feature records of AmDcSegment, each tied feature
/// with the `participants` (occurrence labels) it acts on.
#[test]
fn features_match_reference() {
    oracle::check_all("iam", "features", |label| {
        with_assembly(label, |a| {
            let features = a.features().expect("assembly features");
            Value::Array(features.iter().map(assembly_feature).collect())
        })
    });
}

fn assembly_feature(f: &AssemblyFeature) -> Value {
    let mut v = support::feature_kinds_json::feature_json(&f.feature);
    if let (Some(p), Value::Object(d)) = (&f.participants, &mut v) {
        d.insert("participants".into(), json!(p));
    }
    v
}
