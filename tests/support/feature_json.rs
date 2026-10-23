//! JSON rendering of part features in the reference reader's dict shape (shared by the part
//! and assembly feature tests): helpers and the solid-feature kinds.

use corliss::model::Matrix4;
use corliss::model::part::*;
use serde_json::{Map, Value, json};

pub(crate) type Obj = Map<String, Value>;

/// A code-table enum: its label, or the raw value for an unnamed code.
pub(crate) trait Code {
    fn json(&self) -> Value;
}

macro_rules! code {
    ($($t:ident),*) => {$(
        impl Code for $t {
            fn json(&self) -> Value {
                match (self.label(), self) {
                    (Some(l), _) => json!(l),
                    (None, $t::Other(v)) => raw(v),
                    (None, _) => Value::Null,
                }
            }
        }
    )*};
}

code!(
    FeatureOperation,
    FeatureExtent,
    FeatureFilletType,
    FeatureDraftMode,
    FeatureRuledType,
    FeatureCreationMethod,
    FeatureHoleSeat,
    FeatureSplitMethod,
    FeatureChamferType,
    FeatureShellDirection,
    FeatureEmbossOperation,
    FeatureRevolveExtent,
    FeatureRevolveDirection,
    FeatureCoilMethod,
    FeatureLoftType,
    FeatureSweepOrientation
);

pub(crate) fn code<C: Code>(c: &Option<C>) -> Value {
    c.as_ref().map_or(Value::Null, Code::json)
}

pub(crate) fn raw(v: &FeatureValue) -> Value {
    match v {
        FeatureValue::Bool(b) => json!(b),
        FeatureValue::Int(i) => json!(i),
        FeatureValue::Float(f) => json!(f),
        FeatureValue::Text(s) => json!(s),
        FeatureValue::Bytes(b) => json!(b.iter().map(|x| format!("{x:02x}")).collect::<String>()),
        FeatureValue::List(l) => Value::Array(l.iter().map(raw).collect()),
    }
}

pub(crate) fn opt_raw(v: &Option<FeatureValue>) -> Value {
    v.as_ref().map_or(Value::Null, raw)
}

pub(crate) fn param(p: &Option<ParamValue>) -> Value {
    p.as_ref()
        .map_or(Value::Null, |p| json!({"name": p.name, "value": p.value}))
}

pub(crate) fn params(ps: &[Option<ParamValue>]) -> Value {
    Value::Array(ps.iter().map(param).collect())
}

pub(crate) fn dims(ps: &[ParamValue]) -> Value {
    Value::Array(
        ps.iter()
            .map(|p| json!({"name": p.name, "value": p.value}))
            .collect(),
    )
}

pub(crate) fn matrix(m: &Option<Matrix4>) -> Value {
    json!(m)
}

pub(crate) fn faces(f: &[Option<FeatureNaming>]) -> Value {
    json!(f)
}

pub(crate) fn wire(w: &FeatureWire) -> Value {
    json!({"wire_asm": w.wire_asm, "wire_index": w.wire_index})
}

pub(crate) fn axis(a: &Option<FeatureAxis>) -> Value {
    a.as_ref()
        .map_or(Value::Null, |a| json!({"pos": a.pos, "dir": a.dir}))
}

pub(crate) fn thread(t: &Option<FeatureThreadInfo>) -> Value {
    t.as_ref().map_or(Value::Null, |t| {
        json!({"size": t.size, "designation": t.designation, "standard": t.standard, "class": t.class,
               "flags": t.flags, "right_hand": t.right_hand, "full_depth": t.full_depth})
    })
}

pub(crate) fn obj(v: Value) -> Obj {
    match v {
        Value::Object(m) => m,
        _ => Obj::new(),
    }
}

pub(crate) fn add_profile(d: &mut Obj, p: &Option<FeatureProfile>) {
    if let Some(p) = p {
        d.insert("sketch".into(), json!(p.sketch));
        d.insert("profile_loops".into(), json!(p.loops));
    }
}

pub(crate) fn add_base(d: &mut Obj, b: &FeatureSolidBase) {
    d.insert("operation".into(), code(&b.operation));
    if let Some(body) = &b.body {
        d.insert("body".into(), json!(body));
    }
}

pub(crate) fn extrusion(e: &FeatureExtrusion) -> Obj {
    let mut d = obj(json!({
        "operation": code(&e.operation), "extent": code(&e.extent), "distance": param(&e.distance),
        "taper": param(&e.taper), "flipped": e.flipped, "symmetric": e.symmetric, "direction": e.direction,
        "from_face": e.from_face, "to_face": e.to_face, "body": e.body}));
    if let Some(b) = &e.direction_b {
        let side = json!({"distance": param(&b.distance), "taper": param(&b.taper), "extent": code(&b.extent)});
        d.insert("direction_b".into(), side);
    }
    add_profile(&mut d, &e.profile);
    d
}

pub(crate) fn revolve(r: &FeatureRevolve) -> Obj {
    let mut d = obj(
        json!({"extent": code(&r.extent), "angle": param(&r.angle), "direction": code(&r.direction)}),
    );
    add_base(&mut d, &r.base);
    if r.axis.is_some() {
        d.insert("axis".into(), axis(&r.axis));
    }
    add_profile(&mut d, &r.profile);
    d
}

pub(crate) fn coil_end(e: &FeatureCoilEnd) -> Value {
    json!({"closed": e.closed, "transition": param(&e.transition), "flat": param(&e.flat)})
}

pub(crate) fn coil(c: &FeatureCoil) -> Obj {
    let mut d = obj(json!({
        "method": code(&c.method), "rotation_flag": c.rotation_flag, "pitch": param(&c.pitch),
        "height": param(&c.height), "revolutions": param(&c.revolutions), "taper": param(&c.taper),
        "start": coil_end(&c.start), "end": coil_end(&c.end)}));
    add_base(&mut d, &c.base);
    if c.axis.is_some() {
        d.insert("axis".into(), axis(&c.axis));
    }
    add_profile(&mut d, &c.profile);
    d
}

pub(crate) fn loft(l: &FeatureLoft) -> Obj {
    let section = |s: &FeatureLoftSection| {
        let mut o = obj(wire(&s.wire));
        o.insert("weight".into(), param(&s.weight));
        o.insert("angle".into(), param(&s.angle));
        o.insert("reversed".into(), json!(s.reversed));
        o.insert(
            "condition".into(),
            json!(if s.free { "free" } else { "direction" }),
        );
        Value::Object(o)
    };
    let mut d = obj(json!({
        "loft_type": code(&l.loft_type), "closed_loop": l.closed_loop,
        "merge_tangent_faces": l.merge_tangent_faces,
        "sections": l.sections.iter().map(section).collect::<Vec<_>>()}));
    add_base(&mut d, &l.base);
    d
}

pub(crate) fn sweep(s: &FeatureSweep) -> Obj {
    let mut d = obj(json!({"taper": param(&s.taper), "twist": param(&s.twist),
        "orientation": code(&s.orientation), "path": wire(&s.path)}));
    add_base(&mut d, &s.base);
    add_profile(&mut d, &s.profile);
    d
}

pub(crate) fn emboss(e: &FeatureEmboss) -> Obj {
    let mut d = obj(json!({
        "operation": code(&e.operation), "from_plane": e.from_plane, "depth": param(&e.depth),
        "taper": param(&e.taper), "direction": e.direction, "flipped": e.flipped,
        "wrap_to_face": e.wrap_to_face, "wrap_faces": e.wrap_faces, "flags": e.flags}));
    add_profile(&mut d, &e.profile);
    d
}

pub(crate) fn rib(r: &FeatureRib) -> Obj {
    let rib_type = if r.normal {
        "normal to sketch plane"
    } else {
        "parallel to sketch plane"
    };
    let mut d = obj(json!({
        "rib_type": rib_type, "thickness": param(&r.thickness), "extent": opt_raw(&r.extent),
        "extent_distance": param(&r.extent_distance), "symmetric_thickness": r.symmetric_thickness,
        "direction": r.direction, "taper": param(&r.taper), "boss": params(&r.boss), "flags": r.flags}));
    add_profile(&mut d, &r.profile);
    if r.profile.is_some() {
        d.insert("open_profile".into(), json!(true));
    }
    d
}

pub(crate) fn hole(h: &FeatureHole) -> Obj {
    let mut d = obj(json!({
        "seat": code(&h.seat), "diameter": param(&h.diameter), "depth": param(&h.depth),
        "seat_diameter": param(&h.seat_diameter), "seat_depth": param(&h.seat_depth),
        "countersink_angle": param(&h.countersink_angle), "drill_point_angle": param(&h.drill_point_angle),
        "tap_or_major": param(&h.tap_or_major), "extent": code(&h.extent), "direction": h.direction,
        "placement": matrix(&h.placement), "thread": thread(&h.thread)}));
    if let Some(t) = &h.taper_thread {
        d.insert("taper_thread".into(), params(t));
    }
    if let Some(l) = &h.slot_length {
        d.insert("slot_length".into(), param(l));
    }
    d
}

pub(crate) fn fillet_set(s: &FeatureFilletSet) -> Value {
    match s {
        FeatureFilletSet::Constant { edges, radius, g1 } => {
            json!({"kind": "constant", "edges": edges, "radius": param(radius), "g1": g1})
        }
        FeatureFilletSet::Variable { edges, radii } => {
            let pts: Vec<Value> = radii
                .iter()
                .map(|p| json!({"point": p.point, "radius": param(&p.radius)}))
                .collect();
            json!({"kind": "variable", "edges": edges, "radii": pts})
        }
        FeatureFilletSet::Faces { faces, radius } => {
            let fs: Obj = faces
                .iter()
                .map(|(k, v)| (k.to_string(), json!(v)))
                .collect();
            json!({"kind": "faces", "faces": fs, "radius": param(radius)})
        }
    }
}

pub(crate) fn fillet(f: &FeatureFillet) -> Obj {
    let setback = |s: &FeatureFilletSetback| json!({"edges": s.edges, "distances": params(&s.distances), "flags": s.flags});
    obj(json!({
        "fillet_type": code(&f.fillet_type), "sets": f.sets.iter().map(fillet_set).collect::<Vec<_>>(),
        "setbacks": f.setbacks.iter().map(setback).collect::<Vec<_>>(), "body": f.body}))
}

pub(crate) fn chamfer(c: &FeatureChamfer) -> Obj {
    let mut d = obj(
        json!({"method": code(&c.method), "edges": c.edges, "distance1": param(&c.distance1),
        "distance2": param(&c.distance2), "angle": param(&c.angle)}),
    );
    if let Some(f) = &c.face {
        d.insert("face".into(), faces(f));
    }
    d
}

pub(crate) fn base_pattern(b: &FeaturePatternBase) -> Obj {
    let mut d = obj(json!({"features": b.features, "bodies": b.bodies,
        "creation_method": code(&b.creation_method), "solids": b.solids}));
    if let Some(r) = &b.result_features {
        d.insert("result_features".into(), json!(r));
    }
    if let Some(s) = &b.suppressed {
        d.insert(
            "suppressed".into(),
            Value::Array(s.iter().map(raw).collect()),
        );
    }
    d
}

pub(crate) fn rect_direction(d: &FeaturePatternDirection) -> Value {
    let path = d
        .path
        .as_ref()
        .map(|p| json!({"start": p.start, "curve": p.curve, "flag": p.flag}));
    json!({"count": param(&d.count), "spacing": param(&d.spacing), "dir": d.dir, "path": path,
           "distribution": if d.fit == Some(true) { "fit" } else { "spacing" },
           "midplane": d.midplane, "flags": d.flags})
}

pub(crate) fn circular(c: &FeatureCircularPattern) -> Obj {
    let mut d = base_pattern(&c.base);
    d.extend(obj(json!({"count": param(&c.count), "angle": param(&c.angle), "axis": axis(&c.axis),
        "distribution": if c.incremental { "incremental" } else { "fitted" }, "midplane": c.midplane,
        "orientation": if c.fixed_orientation { "fixed" } else { "rotational" }, "flag1": c.flag1})));
    d
}

pub(crate) fn sketch_pattern(p: &FeatureSketchPattern) -> Obj {
    obj(
        json!({"features": p.features, "bodies": p.bodies, "creation_method": code(&p.creation_method),
        "points": p.points, "base_point": p.base_point}),
    )
}
