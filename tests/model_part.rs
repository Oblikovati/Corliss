//! `corliss::model::Part` against the reference part reader: every method's output
//! for every part fixture, converted to the reference's JSON shape, must equal
//! `tests/fixtures/models.json`.

mod support;

use std::collections::{BTreeMap, HashMap};
use std::sync::OnceLock;

use corliss::model::part::{
    AssetKind, FaceOverride, IMate, IPartTable, MaterialAsset, ParamValue, Parameter,
    PartMaterials, PhysicalProperties, SheetMetal, SheetMetalSettings, Sketch,
    UserCoordinateSystem, WorkAxis, WorkGeometryKind, WorkPlane, WorkPoint,
};
use corliss::model::{Model, Part};
use serde_json::{Map, Value, json};

/// The part reader methods the oracle records.
const METHODS: [&str; 10] = [
    "parameters",
    "work_planes",
    "work_axes",
    "work_points",
    "user_coordinate_systems",
    "sketches",
    "imates",
    "sheet_metal",
    "materials",
    "ipart_table",
];

type Outputs = HashMap<String, BTreeMap<&'static str, Value>>;

/// Every part fixture's method outputs, computed once (each document is decoded once) and
/// shared by the tests.
fn outputs() -> &'static Outputs {
    static OUTPUTS: OnceLock<Outputs> = OnceLock::new();
    OUTPUTS.get_or_init(|| {
        let labels = support::oracle::labels("ipt");
        let workers = std::thread::available_parallelism().map_or(4, |n| n.get());
        let chunk = labels.len().div_ceil(workers).max(1);
        std::thread::scope(|s| {
            let handles: Vec<_> = labels
                .chunks(chunk)
                .map(|c| s.spawn(move || c.iter().map(|l| (l.clone(), run(l))).collect::<Vec<_>>()))
                .collect();
            handles
                .into_iter()
                .flat_map(|h| h.join().expect("worker finishes"))
                .collect()
        })
    })
}

fn run(label: &str) -> BTreeMap<&'static str, Value> {
    let doc = support::open_label(label);
    let model = Model::load(&doc).expect("model loads");
    let part = Part::new(&model).expect("part model");
    METHODS.iter().map(|&m| (m, call(&part, m))).collect()
}

fn call(part: &Part<'_>, method: &str) -> Value {
    match method {
        "parameters" => Value::Array(part.parameters().iter().map(parameter).collect()),
        "work_planes" => Value::Array(part.work_planes().iter().map(work_plane).collect()),
        "work_axes" => Value::Array(part.work_axes().iter().map(work_axis).collect()),
        "work_points" => Value::Array(part.work_points().iter().map(work_point).collect()),
        "user_coordinate_systems" => {
            Value::Array(part.user_coordinate_systems().iter().map(ucs).collect())
        }
        "sketches" => Value::Array(part.sketches().iter().map(sketch).collect()),
        "imates" => Value::Array(part.imates().iter().map(imate).collect()),
        "sheet_metal" => part.sheet_metal().as_ref().map_or(Value::Null, sheet_metal),
        "materials" => materials(&part.materials()),
        "ipart_table" => part.ipart_table().as_ref().map_or(Value::Null, ipart),
        other => panic!("unknown method {other}"),
    }
}

fn check(method: &str) {
    support::oracle::check_all("ipt", method, |label| outputs()[label][method].clone());
}

#[test]
fn parameters_match_reference() {
    check("parameters");
}

#[test]
fn work_planes_match_reference() {
    check("work_planes");
}

#[test]
fn work_axes_match_reference() {
    check("work_axes");
}

#[test]
fn work_points_match_reference() {
    check("work_points");
}

#[test]
fn user_coordinate_systems_match_reference() {
    check("user_coordinate_systems");
}

#[test]
fn sketches_match_reference() {
    check("sketches");
}

#[test]
fn imates_match_reference() {
    check("imates");
}

#[test]
fn sheet_metal_matches_reference() {
    check("sheet_metal");
}

#[test]
fn materials_match_reference() {
    check("materials");
}

#[test]
fn ipart_table_matches_reference() {
    check("ipart_table");
}

// ---------------------------------------------------------------- adapters to the reference shape

fn obj(pairs: Vec<(&str, Value)>) -> Map<String, Value> {
    pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect()
}

fn param_value(p: &Option<ParamValue>) -> Value {
    p.as_ref()
        .map_or(Value::Null, |p| json!({"name": p.name, "value": p.value}))
}

fn parameter(p: &Parameter) -> Value {
    let mut d = obj(vec![
        ("name", json!(p.name)),
        ("value", json!(p.value)),
        ("user", json!(p.user)),
        ("driven", json!(p.driven)),
        ("equation", json!(p.equation)),
    ]);
    if let Some(r) = &p.relation {
        d.insert(
            "relation".into(),
            json!({"terms": r.terms, "constant": r.constant}),
        );
    }
    Value::Object(d)
}

/// The work geometry kind, and for defined geometry its definition, inputs and parameter.
fn add_kind(d: &mut Map<String, Value>, kind: &WorkGeometryKind) {
    d.insert("kind".into(), json!(kind.label()));
    let WorkGeometryKind::Defined(def) = kind else {
        return;
    };
    d.insert("definition".into(), json!(def.type_id.to_string()));
    d.insert("inputs".into(), json!(def.inputs));
    if def.parameter.is_some() {
        d.insert("parameter".into(), param_value(&def.parameter));
    }
}

fn work_plane(p: &WorkPlane) -> Value {
    let mut d = obj(vec![
        ("index", json!(p.index)),
        ("name", json!(p.name)),
        ("origin", json!(p.origin)),
        ("u", json!(p.u)),
        ("v", json!(p.v)),
        ("normal", json!(p.normal)),
    ]);
    add_kind(&mut d, &p.kind);
    Value::Object(d)
}

fn work_axis(a: &WorkAxis) -> Value {
    let mut d = obj(vec![
        ("index", json!(a.index)),
        ("name", json!(a.name)),
        ("origin", json!(a.origin)),
        ("segment_end", json!(a.segment_end)),
        ("direction", json!(a.direction)),
    ]);
    add_kind(&mut d, &a.kind);
    Value::Object(d)
}

fn work_point(p: &WorkPoint) -> Value {
    let mut d = obj(vec![
        ("index", json!(p.index)),
        ("name", json!(p.name)),
        ("position", json!(p.position)),
    ]);
    add_kind(&mut d, &p.kind);
    Value::Object(d)
}

fn ucs(u: &UserCoordinateSystem) -> Value {
    let mut d = obj(vec![("index", json!(u.index)), ("name", json!(u.name))]);
    if u.transformation.is_some() {
        d.insert("transform".into(), json!(u.transform));
    }
    for (k, v) in &u.geometry {
        d.insert(k.to_string(), json!(v));
    }
    if let Some(v) = &u.values {
        let params: Map<String, Value> = v
            .parameters
            .iter()
            .map(|(k, p)| (k.clone(), param_value(p)))
            .collect();
        d.insert("parameters".into(), Value::Object(params));
        d.insert("defining_points".into(), json!(v.defining_points));
    }
    Value::Object(d)
}

fn sketch(s: &Sketch) -> Value {
    let mut d = obj(vec![
        ("index", json!(s.index)),
        ("name", json!(s.name)),
        ("plane", json!(s.plane)),
        ("matrix", json!(s.matrix)),
        ("normal", json!(s.normal)),
    ]);
    d.extend(sketch_geometry(s));
    d.extend(sketch_annotations(s));
    Value::Object(d)
}

fn sketch_geometry(s: &Sketch) -> Map<String, Value> {
    let points = s.points.iter().map(
        |p| json!({"index": p.index, "pos": p.pos, "model": p.model, "projected": p.projected}),
    );
    let lines = s.lines.iter().map(|l| {
        json!({"index": l.index, "start": l.start, "end": l.end, "mid": l.mid, "dir": l.dir, "reference": l.reference})
    });
    let circles = s
        .circles
        .iter()
        .map(|c| json!({"index": c.index, "center": c.center, "radius": c.radius}));
    let arcs = s.arcs.iter().map(|a| {
        json!({"index": a.index, "center": a.center, "radius": a.radius, "start": a.start, "end": a.end,
               "start_angle": a.start_angle, "sweep": a.sweep})
    });
    let ellipses = s.ellipses.iter().map(|e| {
        json!({"index": e.index, "center": e.center, "major_dir": e.major_dir,
               "major_radius": e.major_radius, "minor_radius": e.minor_radius})
    });
    let splines =
        s.splines.iter().map(|p| json!({"index": p.index, "fit_points": p.fit_points, "poles": p.poles, "knots": p.knots}));
    obj(vec![
        ("points", points.collect()),
        ("lines", lines.collect()),
        ("circles", circles.collect()),
        ("arcs", arcs.collect()),
        ("ellipses", ellipses.collect()),
        ("splines", splines.collect()),
    ])
}

fn sketch_annotations(s: &Sketch) -> Map<String, Value> {
    let texts = s.texts.iter().map(
        |t| json!({"index": t.index, "text": t.text, "anchor": t.anchor, "box": t.box_corners}),
    );
    let patterns = s
        .patterns
        .iter()
        .map(|p| json!({"index": p.index, "name": p.name, "type": p.pattern, "occurrences": p.occurrences}));
    let constraints = s
        .constraints
        .iter()
        .map(|c| json!({"index": c.index, "type": c.constraint, "entities": c.entities}));
    let dimensions = s.dimensions.iter().map(|c| {
        json!({"index": c.index, "type": c.dimension, "entities": c.entities,
               "parameter": param_value(&c.parameter), "driven": c.driven})
    });
    obj(vec![
        ("texts", texts.collect()),
        ("patterns", patterns.collect()),
        ("constraints", constraints.collect()),
        ("dimensions", dimensions.collect()),
    ])
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn imate(m: &IMate) -> Value {
    let mut d = obj(vec![
        ("name", json!(m.name)),
        ("uid", json!(m.uid.as_deref().map(hex))),
    ]);
    if let Some(p) = &m.plane {
        d.insert("plane_center".into(), json!(p.center));
        d.insert("plane_normal".into(), json!(p.normal));
    }
    Value::Object(d)
}

fn sheet_metal(sm: &SheetMetal) -> Value {
    let rules = sm.rules.iter().map(|r| {
        let unfold =
            r.unfold.as_ref().map(|u| json!({"name": u.name, "k_factor": u.k_factor, "spline_factor": u.spline_factor}));
        json!({"name": r.name, "thickness": r.thickness, "expressions": r.expressions, "unfold": unfold})
    });
    let mut d = obj(vec![("rules", rules.collect())]);
    if let Some(s) = &sm.settings {
        d.insert("settings".into(), sheet_metal_settings(s));
    }
    Value::Object(d)
}

/// A dict of parameter name → value, plus `enums` when there are any; Python's `str(None)`
/// keys an unnamed parameter.
fn sheet_metal_settings(s: &SheetMetalSettings) -> Value {
    let mut params: Map<String, Value> = s
        .parameters
        .iter()
        .map(|(k, v)| (k.clone().unwrap_or_else(|| "None".into()), json!(v)))
        .collect();
    if !s.enums.is_empty() {
        let enums = s
            .enums
            .iter()
            .map(|e| json!([e.type_id.to_string(), e.kind, e.value]));
        params.insert("enums".into(), enums.collect());
    }
    Value::Object(params)
}

fn asset(a: Option<&MaterialAsset>) -> Value {
    let Some(a) = a else { return Value::Null };
    let mut d = obj(vec![
        ("name", json!(a.name)),
        ("library_id", json!(a.library_id)),
        ("comment", json!(a.comment)),
    ]);
    match &a.kind {
        AssetKind::Material {
            physical: p,
            appearance,
        } => {
            d.insert("physical".into(), physical(p));
            d.insert("appearance".into(), asset(appearance.as_deref()));
        }
        AssetKind::Appearance { diffuse, texture } => {
            d.insert("diffuse".into(), json!(diffuse));
            d.insert("texture".into(), json!(texture));
        }
    }
    Value::Object(d)
}

/// The reference zips its eight keys with the stored values, so absent values have no key.
fn physical(p: &PhysicalProperties) -> Value {
    let keys = [
        ("density_g_cm3", p.density_g_cm3),
        ("youngs_modulus_GPa", p.youngs_modulus_gpa),
        ("poisson_ratio", p.poisson_ratio),
        ("yield_strength_MPa", p.yield_strength_mpa),
        ("ultimate_tensile_MPa", p.ultimate_tensile_mpa),
        ("thermal_conductivity_W_mK", p.thermal_conductivity_w_mk),
        ("linear_expansion_um_mC", p.linear_expansion_um_mc),
        ("specific_heat_J_gC", p.specific_heat_j_gc),
    ];
    Value::Object(
        keys.iter()
            .filter_map(|(k, v)| Some((k.to_string(), json!((*v)?))))
            .collect(),
    )
}

fn face_override(f: &FaceOverride) -> Value {
    let mut d = obj(vec![("appearance", json!(f.appearance))]);
    if let Some(id) = f.owner_feature_id {
        d.insert("owner_feature_id".into(), json!(id));
    }
    Value::Object(d)
}

fn materials(m: &PartMaterials) -> Value {
    let mut d = Map::new();
    if let Some(s) = &m.document_style {
        d.insert("material".into(), asset(s.material.as_ref()));
        d.insert("appearance".into(), asset(s.appearance.as_ref()));
        d.insert(
            "appearance_from_material".into(),
            json!(s.appearance_from_material),
        );
    }
    let libraries = [
        ("library_materials", &m.library_materials),
        ("library_colors", &m.library_colors),
    ];
    for (key, list) in libraries {
        if let Some(l) = list {
            d.insert(key.into(), json!(l));
        }
    }
    let features = m
        .feature_overrides
        .iter()
        .map(|f| json!({"feature": f.feature, "appearance": f.appearance}));
    d.insert("feature_overrides".into(), features.collect());
    d.insert(
        "face_overrides".into(),
        m.face_overrides.iter().map(face_override).collect(),
    );
    Value::Object(d)
}

fn ipart(t: &IPartTable) -> Value {
    let members = t
        .members
        .iter()
        .map(|m| json!({"name": m.name, "row": m.row, "active": m.active}));
    let mut d = obj(vec![("members", members.collect())]);
    if let Some(wb) = &t.workbook {
        if let Some(sheet) = &wb.sheet {
            d.insert("columns".into(), json!(sheet.columns));
            d.insert("rows".into(), json!(sheet.rows));
        }
        d.insert("factory".into(), json!(wb.factory));
    }
    Value::Object(d)
}
