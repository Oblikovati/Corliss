//! ACIS bodies (spec 13.6): the record split agrees with the reference parser
//! (`tests/fixtures/acis.json`, written by the reference implementation's export tooling), and
//! the decoded topology and geometry satisfy invariants that need no reference: vertices lie on
//! their edge curves and on the surfaces of the faces they bound, and closed bodies are
//! manifold.

mod support;

use std::collections::BTreeMap;
use std::sync::OnceLock;

use corliss::acis::{Brep, Curve, SabStream, Surface, eval};
use serde_json::Value;

/// Positions are written to 1e-6 cm (SPAresabs); allow a little more for rounding.
const ON_GEOMETRY: f64 = 1e-5;

fn reference() -> &'static Value {
    static REF: OnceLock<Value> = OnceLock::new();
    REF.get_or_init(|| {
        let path = support::fixtures_dir().join("acis.json");
        let text = std::fs::read_to_string(path).expect("acis.json");
        serde_json::from_str(&text).expect("acis.json parses")
    })
}

/// A stream with where it came from.
struct Found {
    context: String,
    segment: String,
    node: usize,
    stream: SabStream,
}

/// Every SAB stream of a document, in segment-name order.
fn streams(label: &str) -> Vec<Found> {
    let doc = support::open_label(label);
    let mut locs: Vec<_> = doc
        .segments()
        .unwrap()
        .into_iter()
        .filter(|l| !l.template)
        .collect();
    locs.sort_by(|a, b| a.name.cmp(&b.name));
    let mut out = Vec::new();
    for loc in locs {
        let seg = doc.decode_nodes(&loc).unwrap();
        for (k, n) in seg.nodes.iter().enumerate() {
            let Some(f) = n
                .field("sab")
                .filter(|_| n.type_id.0 == corliss::acis::T_ACIS_BODY)
            else {
                continue;
            };
            let context = format!("{label} {} #{}", loc.name, k + 1);
            let stream = SabStream::parse(&f.raw).unwrap_or_else(|e| panic!("{context}: {e}"));
            out.push(Found {
                context,
                segment: loc.name.clone(),
                node: k + 1,
                stream,
            });
        }
    }
    out
}

/// Every stream of every fixture, decoded once for all tests.
fn all_streams() -> &'static Vec<(String, Vec<Found>)> {
    static ALL: OnceLock<Vec<(String, Vec<Found>)>> = OnceLock::new();
    ALL.get_or_init(|| {
        let labels: Vec<String> = reference().as_object().unwrap().keys().cloned().collect();
        labels
            .into_iter()
            .map(|l| (l.clone(), streams(&l)))
            .collect()
    })
}

fn census(s: &SabStream) -> BTreeMap<String, u64> {
    let mut m = BTreeMap::new();
    for r in s.records.iter().filter(|r| r.live) {
        *m.entry(r.class.clone()).or_insert(0) += 1;
    }
    m
}

#[test]
fn record_census_matches_the_reference_parser() {
    let mut checked = 0;
    for (label, got) in all_streams() {
        let want = reference()[label].as_array().unwrap();
        assert_eq!(got.len(), want.len(), "{label}: stream count");
        for (f, w) in got.iter().zip(want) {
            let ctx = &f.context;
            assert_eq!(w["segment"].as_str(), Some(f.segment.as_str()), "{ctx}");
            assert_eq!(w["node"].as_u64(), Some(f.node as u64), "{ctx}");
            assert_eq!(
                w["version"].as_i64(),
                Some(i64::from(f.stream.header.version)),
                "{ctx}"
            );
            assert_eq!(
                w["records"].as_u64(),
                Some(f.stream.records.len() as u64),
                "{ctx}"
            );
            let live: BTreeMap<String, u64> = serde_json::from_value(w["live"].clone()).unwrap();
            assert_eq!(census(&f.stream), live, "{ctx}");
            checked += 1;
        }
    }
    assert!(checked > 400, "{checked} streams");
}

fn faces(brep: &Brep) -> impl Iterator<Item = &corliss::acis::Face> {
    brep.bodies
        .iter()
        .flat_map(|b| &b.lumps)
        .flat_map(|l| &l.shells)
        .flat_map(|s| &s.faces)
}

/// (context, B-rep) of every stream; every live face must be reached from a body.
fn breps() -> &'static Vec<(String, Brep)> {
    static ALL: OnceLock<Vec<(String, Brep)>> = OnceLock::new();
    ALL.get_or_init(|| {
        let mut out = Vec::new();
        for f in all_streams().iter().flat_map(|(_, s)| s) {
            let brep = Brep::read(&f.stream).unwrap_or_else(|e| panic!("{}: {e}", f.context));
            let reached = faces(&brep).count() as u64;
            let live = census(&f.stream).get("face").copied().unwrap_or(0);
            assert_eq!(reached, live, "{}: every live face is reached", f.context);
            out.push((f.context.clone(), brep));
        }
        out
    })
}

fn distance(p: [f64; 3], q: [f64; 3]) -> f64 {
    ((p[0] - q[0]).powi(2) + (p[1] - q[1]).powi(2) + (p[2] - q[2]).powi(2)).sqrt()
}

/// Distances from each edge's vertices to its curve at the stored parameters.
fn vertex_gaps(brep: &Brep, e: &corliss::acis::Edge) -> Vec<f64> {
    let sign = if e.reversed { -1.0 } else { 1.0 };
    [(e.start, e.start_param), (e.end, e.end_param)]
        .into_iter()
        .filter_map(|(v, t)| {
            Some(distance(
                eval::curve_point(&e.curve, sign * t)?,
                brep.vertices[v?].point,
            ))
        })
        .collect()
}

#[test]
fn vertices_lie_on_their_edge_curves() {
    let mut checked = 0;
    for (ctx, brep) in breps() {
        for e in brep
            .edges
            .iter()
            .filter(|e| matches!(e.curve, Curve::Straight { .. } | Curve::Ellipse(_)))
        {
            for d in vertex_gaps(brep, e) {
                assert!(
                    d < ON_GEOMETRY,
                    "{ctx}: edge record {} vertex off its curve by {d}",
                    e.record
                );
                checked += 1;
            }
        }
    }
    assert!(checked > 5000, "{checked} checks");
}

#[test]
fn vertices_lie_on_the_surfaces_of_their_faces() {
    let mut checked = 0;
    for (ctx, brep) in breps() {
        for f in faces(brep) {
            let edges = f
                .loops
                .iter()
                .flat_map(|l| &l.coedges)
                .filter_map(|c| c.edge);
            for v in edges
                .flat_map(|e| [brep.edges[e].start, brep.edges[e].end])
                .flatten()
            {
                let Some(d) = eval::surface_distance(&f.surface, brep.vertices[v].point) else {
                    continue;
                };
                let kind = f.surface.kind();
                assert!(
                    d < ON_GEOMETRY,
                    "{ctx}: face record {} ({kind}) vertex off by {d}",
                    f.record
                );
                checked += 1;
            }
        }
    }
    assert!(checked > 10000, "{checked} checks");
}

#[test]
fn intcurve_approximations_pass_through_their_vertices() {
    let mut checked = 0;
    for (ctx, brep) in breps() {
        for e in &brep.edges {
            let Curve::Intcurve {
                definition: Some(d),
                ..
            } = &e.curve
            else {
                continue;
            };
            let Some(tol) = d.fit_tolerance.filter(|_| d.approximation.is_some()) else {
                continue;
            };
            for gap in vertex_gaps(brep, e) {
                assert!(
                    gap < tol.max(ON_GEOMETRY) * 10.0,
                    "{ctx}: intcurve edge {} misses a vertex by {gap}",
                    e.record
                );
                checked += 1;
            }
        }
    }
    assert!(checked > 50, "{checked} checks");
}

#[test]
fn closed_bodies_use_every_edge_twice() {
    for (ctx, brep) in breps() {
        let mut uses = vec![0usize; brep.edges.len()];
        for c in faces(brep).flat_map(|f| &f.loops).flat_map(|l| &l.coedges) {
            if let Some(e) = c.edge {
                uses[e] += 1;
            }
        }
        if uses.iter().all(|u| *u != 1) {
            assert!(
                uses.iter().all(|u| *u == 0 || *u == 2),
                "{ctx}: non-manifold edges {uses:?}"
            );
        }
    }
}

/// Subtypes whose spl_sur part the specification still leaves open (OI-196).
const OPEN_SPLINE_SUBTYPES: [&str; 1] = ["VBL_SURF"];

/// A helix surface carries its exact helix in place of an approximation.
#[test]
fn every_spline_face_of_a_specified_subtype_has_its_approximation() {
    let mut counts: std::collections::BTreeMap<(String, bool), usize> = Default::default();
    for (_, brep) in breps() {
        for f in faces(brep) {
            let Surface::Spline {
                definition: Some(d),
                ..
            } = &f.surface
            else {
                continue;
            };
            *counts
                .entry((
                    d.subtype.clone(),
                    d.approximation.is_some() || d.helix.is_some(),
                ))
                .or_default() += 1;
        }
    }
    let missing: Vec<_> = counts
        .iter()
        .filter(|((s, has), _)| !has && !OPEN_SPLINE_SUBTYPES.contains(&s.as_str()))
        .collect();
    assert!(
        missing.is_empty(),
        "spline faces without an approximation: {missing:?}"
    );
    // Regression: ortho_spl_sur (Table 13-29) was skipped while its part was open.
    assert!(
        counts
            .get(&("ortho_spl_sur".to_string(), true))
            .copied()
            .unwrap_or(0)
            >= 5,
        "{counts:?}"
    );
}

/// A B-spline surface lies in the convex hull of its control points, so every vertex of a face
/// on an `ortho_spl_sur` lies in their bounding box, widened by the fit tolerance.
#[test]
fn ortho_spline_faces_hold_their_vertices_within_their_control_points() {
    let mut checked = 0;
    for (ctx, brep) in breps() {
        for f in faces(brep) {
            let Surface::Spline {
                definition: Some(d),
                ..
            } = &f.surface
            else {
                continue;
            };
            let (Some(bs), "ortho_spl_sur") = (&d.approximation, d.subtype.as_str()) else {
                continue;
            };
            let slack = d.fit_tolerance.unwrap_or(0.0) + ON_GEOMETRY;
            let edges = f
                .loops
                .iter()
                .flat_map(|l| &l.coedges)
                .filter_map(|c| c.edge);
            for v in edges
                .flat_map(|e| [brep.edges[e].start, brep.edges[e].end])
                .flatten()
            {
                let p = brep.vertices[v].point;
                let inside = (0..3).all(|k| {
                    let (lo, hi) = bs.poles.iter().fold((f64::MAX, f64::MIN), |(lo, hi), q| {
                        (lo.min(q[k]), hi.max(q[k]))
                    });
                    p[k] >= lo - slack && p[k] <= hi + slack
                });
                assert!(
                    inside,
                    "{ctx}: face record {} vertex {p:?} outside its control points",
                    f.record
                );
                checked += 1;
            }
        }
    }
    assert!(checked > 0, "no ortho_spl_sur vertex checked");
}
