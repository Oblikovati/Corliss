//! JSON rendering of the remaining feature kinds, and of a whole feature.

use corliss::model::part::*;
use serde_json::{Value, json};

use super::feature_json::*;

pub(crate) fn solid_kind(k: &FeatureKind) -> Option<Obj> {
    Some(match k {
        FeatureKind::Extrusion(e) => extrusion(e),
        FeatureKind::Revolve(r) => revolve(r),
        FeatureKind::Coil(c) => coil(c),
        FeatureKind::Loft(l) => loft(l),
        FeatureKind::Sweep(s) => sweep(s),
        FeatureKind::Emboss(e) => emboss(e),
        FeatureKind::Rib(r) => rib(r),
        FeatureKind::Hole(h) => hole(h),
        FeatureKind::Fillet(f) => fillet(f),
        FeatureKind::Chamfer(c) => chamfer(c),
        FeatureKind::CircularPattern(c) => circular(c),
        FeatureKind::SketchDrivenPattern(p) => sketch_pattern(p),
        _ => return None,
    })
}

pub(crate) fn placed_kind(k: &FeatureKind) -> Option<Value> {
    Some(match k {
        FeatureKind::Thread(t) => json!({"faces": t.faces, "full_length": t.full_length,
            "length": param(&t.length), "offset": param(&t.offset), "thread": thread(&t.thread)}),
        FeatureKind::MoveFace(m) => {
            json!({"faces": m.faces, "mode": opt_raw(&m.mode), "transform": m.transform})
        }
        FeatureKind::Shell(s) => {
            json!({"direction": code(&s.direction), "removed_faces": s.removed_faces,
            "thickness": param(&s.thickness), "approximation": [opt_raw(&s.approximation[0]), opt_raw(&s.approximation[1])]})
        }
        FeatureKind::FaceDraft(f) => json!({"pull_direction": f.pull_direction, "faces": f.faces,
            "fixed_edges": f.fixed_edges, "angle": param(&f.angle), "mode": code(&f.mode),
            "fixed_plane": f.fixed_plane, "parting_tool": f.parting_tool}),
        FeatureKind::Unwrap(u) => {
            json!({"faces": u.faces, "body": u.body, "merge_result": u.merge_result,
            "auto_face_chain": u.auto_face_chain, "transform": u.transform})
        }
        FeatureKind::DeleteFace { faces, heal } => json!({"faces": faces, "heal": heal}),
        FeatureKind::ReplaceFace { existing, new } => json!({"existing": existing, "new": new}),
        FeatureKind::Combine {
            operation,
            tool_bodies,
            keep_toolbodies,
        } => json!({"operation": code(operation),
            "tool_bodies": tool_bodies, "keep_toolbodies": keep_toolbodies}),
        FeatureKind::Split {
            method,
            all_faces,
            tool,
        } => json!({"method": code(method), "all_faces": all_faces, "tool": tool}),
        FeatureKind::Thicken {
            operation,
            distance,
            face_sets,
        } => {
            let sets: Vec<Value> = face_sets
                .iter()
                .map(|s| json!({"faces": s.faces, "offset": param(&s.offset)}))
                .collect();
            json!({"operation": code(operation), "distance": param(distance), "face_sets": sets})
        }
        _ => return None,
    })
}

pub(crate) fn sheet_kind(k: &FeatureKind) -> Option<Value> {
    Some(match k {
        FeatureKind::SheetMetalGroup { members, .. } => json!({"members": members}),
        FeatureKind::SheetMetalPlate {
            thickness,
            direction,
            dimensions,
            operation,
        } => json!({
            "thickness": param(thickness), "direction": direction, "dimensions": dims(dimensions), "operation": code(operation)}),
        FeatureKind::CornerSeam {
            gap_type,
            thickness,
            gap,
            overlap,
            edges,
            relief_size,
            relief_type,
            percent,
        } => json!({
            "gap_type": opt_raw(gap_type), "thickness": param(thickness), "gap": param(gap), "overlap": param(overlap),
            "edges": edges, "relief_size": param(relief_size), "relief_type": opt_raw(relief_type), "percent": param(percent)}),
        FeatureKind::SheetMetalBend {
            radius,
            thickness,
            edges,
        } => json!({"radius": param(radius),
            "thickness": param(thickness), "edges": edges}),
        FeatureKind::SheetMetalCorner {
            gap,
            relief_size,
            relief_type,
            gap_type,
        } => json!({"gap": param(gap),
            "relief_size": param(relief_size), "relief_type": opt_raw(relief_type), "gap_type": opt_raw(gap_type)}),
        FeatureKind::Hem {
            thickness,
            edges,
            dimensions,
        } => json!({"thickness": param(thickness), "edges": edges,
            "dimensions": dims(dimensions)}),
        FeatureKind::FlatPattern {
            thickness,
            flat_body,
            folded_body,
        } => json!({"thickness": param(thickness),
            "flat_body": flat_body, "folded_body": folded_body}),
        FeatureKind::BendPart {
            radius,
            angle,
            flags,
            bend_line,
        } => {
            let mut d =
                obj(json!({"radius": param(radius), "angle": param(angle), "flags": flags}));
            if let Some(w) = bend_line {
                d.insert("bend_line".into(), wire(w));
            }
            Value::Object(d)
        }
        FeatureKind::PunchTool {
            parameters,
            transform,
        } => json!({"parameters": params(parameters), "transform": transform}),
        _ => return None,
    })
}

pub(crate) fn surface_kind(k: &FeatureKind) -> Option<Value> {
    Some(match k {
        FeatureKind::BoundaryPatch {
            loops,
            parameter,
            surface,
        } => {
            let edge = |(e, f): &(Option<FeatureNaming>, Option<FeatureNaming>)| json!({"edge": e, "face": f});
            let lp = |l: &FeaturePatchLoop| json!({"weight": param(&l.weight), "edges": l.edges.iter().map(edge).collect::<Vec<_>>()});
            json!({"loops": loops.iter().map(lp).collect::<Vec<_>>(), "parameter": param(parameter), "surface": surface})
        }
        FeatureKind::Stitch {
            surfaces,
            tolerance,
            flags,
        } => json!({"surfaces": surfaces, "tolerance": param(tolerance), "flags": flags}),
        FeatureKind::Sculpt { tools, mode } => {
            let t: Vec<Value> = tools
                .iter()
                .map(|t| json!({"tool": t.tool, "side": opt_raw(&t.side)}))
                .collect();
            let mode = match mode {
                FeatureSculptMode::NewSolid => "new solid",
                FeatureSculptMode::Add => "add",
                FeatureSculptMode::Remove => "remove",
            };
            json!({"tools": t, "mode": mode})
        }
        FeatureKind::RuledSurface(r) => {
            json!({"ruled_type": code(&r.ruled_type), "distance": param(&r.distance),
            "angle": param(&r.angle), "surface": r.surface, "vector": r.vector, "flags": r.flags})
        }
        FeatureKind::Trim { tool, surface } => json!({"tool": tool, "surface": surface}),
        FeatureKind::Extend {
            edges,
            extent,
            distance,
            surface,
        } => json!({"edges": edges, "extent": code(extent),
            "distance": param(distance), "surface": surface}),
        _ => return None,
    })
}

pub(crate) fn body_kind(k: &FeatureKind) -> Option<Value> {
    Some(match k {
        FeatureKind::Tagging { bodies } => json!({"bodies": bodies}),
        FeatureKind::Mark { flags } => json!({"flags": flags}),
        FeatureKind::PatternResultBody { ids, body } => json!({"ids": ids, "body": body}),
        FeatureKind::MoveBodies { bodies, transforms } => {
            json!({"bodies": bodies, "transforms": transforms})
        }
        FeatureKind::CopyObject {
            source_ref,
            result,
            flag,
            ..
        } => json!({"source_ref": source_ref, "result": result, "flag": flag}),
        FeatureKind::BaseBody { body, source } => json!({"body": body, "source": source}),
        FeatureKind::MeshFeature { meshes } => json!({"meshes": meshes}),
        FeatureKind::RepairBodies => json!({}),
        FeatureKind::DerivedPart {
            file,
            scale,
            transform,
        } => json!({"file": file, "scale": param(scale), "transform": transform}),
        FeatureKind::Import { file } => json!({"file": file}),
        _ => return None,
    })
}

pub(crate) fn source_kind(k: &FeatureKind) -> Option<Value> {
    Some(match k {
        FeatureKind::Decal {
            sketch,
            images,
            face,
            wrap_to_face,
        } => {
            let im: Vec<Value> = images.iter().map(|i| json!({"width": i.width, "height": i.height, "pos": i.pos, "scale": i.scale})).collect();
            json!({"sketch": sketch, "images": im, "face": face, "wrap_to_face": wrap_to_face})
        }
        FeatureKind::Finish {
            faces: f,
            parameters,
        } => {
            let mut d = Obj::new();
            if let Some(f) = f {
                d.insert("faces".into(), faces(f));
            }
            if let Some(ps) = parameters {
                let key =
                    |p: &FeatureFinishParameter| p.name.clone().unwrap_or_else(|| "None".into());
                d.insert(
                    "parameters".into(),
                    Value::Object(ps.iter().map(|p| (key(p), opt_raw(&p.value))).collect()),
                );
            }
            Value::Object(d)
        }
        FeatureKind::RectangularPattern {
            base,
            direction_a,
            direction_b,
            new_solid,
        } => {
            let mut d = base_pattern(base);
            d.insert("direction_a".into(), rect_direction(direction_a));
            d.insert("direction_b".into(), rect_direction(direction_b));
            if let Some(n) = new_solid {
                d.insert("new_solid".into(), json!(n));
            }
            Value::Object(d)
        }
        FeatureKind::Mirror {
            base,
            plane_tool,
            plane,
            flags,
        } => {
            let mut d = base_pattern(base);
            let plane = plane
                .as_ref()
                .map(|p| json!({"origin": p.origin, "normal": p.normal}));
            d.extend(obj(
                json!({"plane_tool": plane_tool, "plane": plane, "flags": flags}),
            ));
            Value::Object(d)
        }
        _ => return None,
    })
}

pub(crate) fn kind_fields(k: &FeatureKind) -> Obj {
    if let Some(d) = solid_kind(k) {
        return d;
    }
    let readers = [
        placed_kind,
        sheet_kind,
        surface_kind,
        body_kind,
        source_kind,
    ];
    readers
        .iter()
        .find_map(|r| r(k))
        .map(obj)
        .expect("every kind has an adapter")
}

pub(crate) fn feature_json(f: &Feature) -> Value {
    let mut d = kind_fields(&f.kind);
    d.insert("index".into(), json!(f.index));
    d.insert("name".into(), json!(f.name));
    d.insert("type".into(), json!(f.kind.type_name()));
    if let Some(o) = &f.direct_edit {
        d.insert("direct_edit".into(), json!(o.name));
    }
    if let Some(o) = &f.parent {
        d.insert("parent".into(), json!(o.name));
    }
    if let Some(i) = &f.ifeature {
        d.insert(
            "ifeature".into(),
            json!({"instance": i.instance, "parameters": i.parameters}),
        );
    }
    Value::Object(d)
}
