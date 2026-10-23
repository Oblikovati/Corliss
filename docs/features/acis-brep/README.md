# ACIS / ShapeManager B-rep reading

Status: implemented (unreleased). Covers spec 13.6 for reading: every SAB stream in an F645595C
node becomes a typed B-rep that a kernel can rebuild solids from.

## What is read

`corliss::acis` reads in three steps:

- `SabStream::parse` reads the header and the tokens, then splits them into records. It stops
  at the `End-of-ASM-data` marker [R-10] and flags which records belong to the live model.
- `Brep::read` walks the live topology from every `body` record [R-07]: body → lump → shell →
  face and wire → loop → coedge → edge → vertex. Edges and vertices are shared through indexes.
  Faces carry their entity ids, which Inventor's naming uses.
- Geometry is typed:
  - analytic surfaces: plane, cone (cylinders included), sphere, torus;
  - analytic curves: straight, ellipse;
  - `transform` records.
  - Procedural curves and surfaces (`intcurve`, `spline`) carry their definition. Shared
    definitions (`ref n`) are resolved by counting definitions in stream order [R-11].
  - From the definition: the approximating `bs3_curve` / `bs3_surface`, the fit tolerance,
    support surfaces, parameter curves, and helix data.
  - Coedge `pcurve`s are read as their own `exp_par_cur` or as a reference to an intcurve.

`corliss::acis::eval` evaluates points on lines, ellipses and NURBS (de Boor, rational), and
distances to analytic surfaces. `Model::breps("PmBRepSegment")` returns a part's bodies.

## Evidence

`tests/acis.rs` runs over the 461 SAB streams of the fixtures:

- **Record census.** Every stream's version, record count and per-class counts of live records
  equal those of the reference reader (`tests/fixtures/acis.json`, written by the reference implementation's export tooling).
- **Topology.** Every live face is reached from a body. In bodies where every edge is used,
  each edge is used by exactly two coedges.
- **Geometry**, with invariants that need no reference:
  - Every vertex of a straight or elliptic edge is on the curve at the stored parameter, within
    1e-5 cm (more than 5,000 checks). For reversed edges the parameter is negated.
  - Every vertex of a face is on the face's analytic surface (more than 10,000 checks).
  - The B-spline approximation of every intcurve passes through its vertices within the fit
    tolerance.
  - Every spline face has its approximation (or, for `helix_spl_line`, its helix), except the
    subtype the specification leaves open (`VBL_SURF`).
  - Every vertex of an `ortho_spl_sur` face lies in the bounding box of its approximation's
    control points, widened by the fit tolerance. A B-spline surface lies in the convex hull
    of its control points, so a misread surface would fail this.

Definitions decoded in the fixtures:
- All intcurve subtypes that are reached have their approximation or helix data decoded:
  `exact`, `int_int`, `par_int`, `defm`, `spring`, `off_surf` and `helix_int_cur`.
- Spline surfaces of `exact_spl_sur`, `loft_spl_sur`, `sweep_sur`, `defm_spl_sur`,
  `srf_srf_v_bl_spl_sur` and `ortho_spl_sur` have their approximation; `helix_spl_line` has its
  helix. `ortho_spl_sur` is read field by field as Table 13-29 gives it (defining surface and
  curve, 2D curve, scale factor, common part). For the others, whose leading part is still
  open, the common part is located by its `approx_level` and B-spline block.

## Limits

- `VBL_SURF` (1 face) has no approximation: the specification does not yet give its
  vertex-blend data (OI-196).
- Attributes (ATTRIB_CUSTOM naming tags) and the history section are not modelled, only split
  into records. Writing SAB streams is not implemented; the spec has not established whether
  Inventor accepts a rewritten stream (OI-199).
- Elliptic cones (`radius_ratio` ≠ 1) are not in the fixtures; `eval::surface_distance`
  assumes circular bases.

## Specification fragments

- **13.6 Embedded ACIS body**: Node F645595C occurs in PmBRepSegment, PmDCSegment (loft sections
  and sweep paths), AmBREPSegment, AmDcSegment and MbBrepSegment. Its `sab` token holds one ACIS /
  Autodesk ShapeManager (ASM) binary save file, abbreviated SAB. The 14-byte header-only form in
  drawings has no `sab` token.
- **[R-10]** (13.6): The SAB stream is self-delimiting: it starts with a magic string and ends
  with the `End-of-ASM-data` marker. Readers MUST locate its end from that marker, not from the
  node length.
- **[R-07]** (13.6.11): Readers of ACIS bodies MUST walk the live topology from each `body`
  record and ignore the history section and the old entity records.
- **[R-11]** (13.6.3): A subtype ident is a subtype name (a definition; definitions are numbered
  from 0 in stream order, nested ones included), `ref` followed by a `long` n (the n-th
  definition of the stream, shared), or `null`. Readers MUST resolve `ref` by counting
  definitions in stream order.
- **OI-196** (13.6.8, open): Subtype-specific data of `loft_spl_sur`, `sweep_sur`,
  `rb_blend_spl_sur`, `srf_srf_v_bl_spl_sur`, `defm_spl_sur`, `defm_int_cur`, `ortho_spl_sur`
  and `VBL_SURF`. Edition 1.9 resolved `rb_blend_spl_sur` and `ortho_spl_sur`. Still open:
  `loft_spl_sur` sections, `sweep_sur`, `VBL_SURF`, `defm_spl_sur`, the `defm_int_cur`
  deformation, and the variable-radius objects of `srf_srf_v_bl_spl_sur`. Table 13-29 gives
  `VBL_SURF` as `object_version` and a vertex-blend boundary list (OI-196), with no spl_sur
  common part (the part that holds the approximating `bs3` surface and the fit tolerance).
- **Table 13-29** (excerpt, spl_sur subtypes): `ortho_spl_sur`, version 9: `object_version`;
  `defining_surface` surface; `defining_curve` curve; `bs2_curve` bs2_curve; `scale_factor`
  double; common part; `sense` logical. `VBL_SURF`, version 1: `object_version`; vertex-blend
  boundary list (OI-196).
- **OI-199** (13.6.11, open): Does Inventor accept a body written without history (header
  `flags` bit 0 clear) or with renumbered records? Writing ACIS bodies is unproven.
