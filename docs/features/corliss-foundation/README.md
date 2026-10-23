# Corliss foundation: structural reader and lossless writer

Status: implemented in 0.1.0 (unreleased). Covers reader classes R1 and R2 and writer classes W1
and W2 of the Inventor File Format Specification (spec 5.2, 5.3).

## Goal

CAD kernels need to read Inventor documents and to write changes back that Inventor accepts.
This milestone builds the layers that every later content decoder depends on. Each layer
decodes to typed Rust values and re-encodes to the bytes Inventor wrote.

## Layers

| Module | Spec | Notes |
| --- | --- | --- |
| `container` | 7 | Whole compound file in memory; written back as CFB version 3 with every CLSID, state bits and storage times kept. |
| `propset` | 8 | [MS-OLEPS] streams; typed values; the fixed Microsoft names are mapped before the FMTID name decoding. |
| `database` | 9 | RSeDb, RSeSegInfo (references and root records), RSeDbRevisionInfo. |
| `ufrx` | 10 | UFRxDoc as an ordered record of typed fields, parsed with the stream's own section-version switches. |
| `segment` | 11–12 | M stream header, block table, sec2/sec3, type table, chained t4–t10 tail; B stream blocks, trailers and end table; template segments. |
| `node` | 13, 4.5.1 | Common header, NodeRef, MaskedMatrix4. |
| `document` | 6.3 | Finds RSeDb, segments (by M stream name, [R-01]) and template pairs; applies edits. |

## Design decisions

- **Exact bytes come from the original data, not from the encoder.** Inventor's zstd and zlib
  output cannot be reproduced. Each M and B stream keeps its compressed bytes and the inflated
  bytes they produced, and the encoder reuses them when the re-encoded content and the method
  are unchanged. Everything else is re-encoded structurally. The `zlib` tests caught an early
  version of this shortcut that ignored a changed method.
- **Property sets keep their original bytes too.** In 269 of 2,245 fixture property sets the
  values are not stored in id-table order, and some sections leave 8 or 16 bytes between the
  table and the first value. A canonical encoder therefore cannot reproduce them. An edited set
  is re-encoded canonically: table order, contiguous values, 4-byte alignment.
- **Unseen branches are errors, not guesses.** RSeSegInfo reference variant 1, UFRxPersist
  auxiliary data, state references, mirror links and unknown BOM setting kinds return
  `FailureClass::Unsupported` with the offset. The spec does not specify their lengths
  (spec 9.2.3, 10.1).
- **RSeSegInfo follows the spec's re-cut.** An older version of the reference reader shifted every segment
  reference by one u32. Corliss reads `ref_state` at the start of each reference and
  `n_root_records` after the last one (spec 9.2.2–9.2.3).
- **Block tables follow the payloads.** `Segment::encode` rewrites the present entries of the
  block table from the payload lengths. Absent entries are kept [W-09], so a payload that
  changes length still gives a consistent segment.
- **No unsafe code, pure Rust codecs.** `flate2` (miniz_oxide backend, the same pin as
  Martensite), `ruzstd` and `cfb`. The zstd decoder handles concatenated frames without a
  declared content size [R-02].

## Evidence

`cargo test` over 298 Inventor 2027 documents (`tests/fixtures`):

- RSeDb, RSeSegInfo, RSeDbRevisionInfo, every property-set stream, UFRxDoc and every M and B
  stream re-encode to the stored bytes.
- Every decoded M body and B data block list re-encodes to bytes that decode back to the same
  values, which checks the structural encoders without the original-bytes shortcut.
- [F-02], [F-04], [F-05], [F-06] and [F-09] hold for every segment and template pair.
- Referenced-file identities match the referenced fixtures' own `internal_name`. Per-file
  occurrence counts match the occurrence list in every assembly.
- Writer tests: a full container rewrite keeps every stream and CLSID; a part-number edit,
  an in-place payload edit, a longer payload, a zlib re-encode and a reference relink all read
  back, and nothing else changes.

Not verified: opening Corliss-written files in Inventor. The writer only reproduces edit classes
that the reference implementation has already verified in Inventor (spec Annex D), but the files Corliss
itself writes have not been opened in Inventor yet.

## Next milestones

1. **Node payload decoding (R3)**, done; see `docs/features/node-decoding`. The grammars of every node type live in
   the reference implementation: about 184,000 lines of Python, with control flow, helper
   calls and version switches. Porting them by hand is not practical. The plan is a generator in
   the reference implementation that translates the grammar subset (`p.u32`, `p.list2`, `h.read_*`,
   `ctx.file` conditions, loops over read counts) into a data table, plus a small interpreter in
   Corliss that walks that table into a token tree. The gate is the spec's own: every node of the
   grammar corpus parses with no bytes left over, and serializing the tokens gives the payload back.
2. **Typed content for kernels.** On top of the token tree: parameters and units (spec 14.1),
   work geometry and sketches (15.1–15.2), features (15.3), occurrence placements (16.1), and the
   ACIS SAB body framing (13.6) that Martensite needs to rebuild B-rep.
3. **W3 edits.** Adding and removing nodes, with the derived fields of spec clause 20: type-table
   counters, id allocation set, t6/t7/t9 and their RSeSegInfo mirror, and the revision move.
4. **Older releases.** Samples from Inventor 2025 (zlib) and 2010 (no block trailers, 4-byte
   spacers) for the corpus, then the pre-2015 UFRxDoc and RSeDb layouts.

## Specification fragments

- **5.2 Readers**: R1 reads the container and the property sets (clauses 7–8). R2 adds the
  database, document record, segments, blocks and node framing (clauses 9–13).
- **5.3 Writers (editors)**: W1 edits property sets only. W2 changes payload values of existing
  nodes in place, without changing any length.
- **Clauses 7–13** (scope of the clauses in the layer table):
  - 7.1: every document is one Microsoft Compound File ([MS-CFB]) version 3 with 512-byte
    sectors; the root storage's CLSID identifies the document type.
  - 8.1: iProperties are ordinary OLE property sets ([MS-OLEPS]), one stream per set at the root
    of the container. Some files also hold the standard `SummaryInformation` and
    `DocumentSummaryInformation` streams; those use Microsoft's fixed names and standard
    FMTIDs rather than the FMTID name encoding of 8.2.
  - 9: three small uncompressed streams describe the database: RSeDb (database header),
    RSeSegInfo (segment directory) and RSeDbRevisionInfo (revision history).
  - 10.1: `UFRxDoc` is the document record of every .ipt, .iam, .idw and .ipn; its first header
    holds the schema and the section versions.
  - 11, 12: a segment is a pair of streams, `M<id>` (outer header, block table, sec2 and sec3,
    type table, t4–t10, chained back from the end by their trailing sizes) and `B<id>`
    (constant GUID, method word, compressed blocks). A decompressed B stream is a run of
    blocks in block-table order, then a segment end table. Template segments are stream pairs
    under `RSeStorage/Templates` (11.6).
  - 13: each block payload is one node; nearly every node payload starts with a common header
    (13.1); a NodeRef is a u32 whose bit 31 is a flag and whose bits 0–30 index a present block
    of the same segment, 0 = null (13.3).
- **4.5.1 MaskedMatrix4**: a row-major 4 × 4 matrix of f64 stored with two u16 masks; used by
  the sketch transform, the occurrence placement, the SM view placement, the scene transform
  and the graphics placements.
- **6.3 Reading procedure** (steps 3–4): Find `RSeStorage/V<n>/RSeDb` and read RSeSegInfo. For
  each `M<id>` stream, read the outer header and identify the segment by its `name`;
  decompress the B stream with the method of 11.3.
- **[R-01]** (11.2.1): Readers MUST identify segments by the M stream `name`, never by the
  stream name.
- **9.2.3 Segment reference**: field 5 `variant` is 0 = by segment id, or 1 = by roots
  (RSeRefdSegInfoByRoots, not seen). The variant-1 fields (a name and root ids) are not seen and
  have status ? (OI-014).
- **10.1 Stream layout** (conformance limits): Four branches that Inventor's writer has are not
  specified because no sample has them (OI-093): UFRxPersist auxiliary data
  (`persist_flag` = 1), BOM settings of a kind other than 0 or 1, state references
  (`n_refs` > 0) and mirror link records (`n_mirror_links` > 0). A reader that meets one of them
  MUST reject the stream rather than guess its length.
- **9.2.2–9.2.3** (field order): a segment directory entry ends with field 17 `refs`
  (SegmentReference[`n_refs`]), then field 18 `n_root_records` (u32: 0, or `n_roots` when the
  segment has root records), then the root records. Each segment reference starts with field
  1 `ref_state` (u32; 0 in all 3,458 records).
- **[W-09]** (5.3): W3 writers MUST regenerate the block table (absent entries kept), the
  type-table counters, `slot_count`, the id space and id allocation set, the RSeSegInfo
  counts, t6/t7/t9 with their RSeSegInfo mirror, and the revision (clause 20).
- **[R-02]** (11.3): Readers MUST accept zlib and zstd B streams, and MUST decode zstd frames
  without a declared content size by streaming decompression.
- **[F-02]** (7.2): Every segment MUST be an `M<id>`/`B<id>` stream pair whose common
  26-character id is `GetName(seg_id)`.
- **[F-04]** (11.2.1, 9.2.2): The segment name and segment id in the M stream MUST equal the
  segment's RSeSegInfo entry.
- **[F-05]** (9.2.2): The RSeSegInfo `n_blocks` MUST equal the number of present block-table
  entries, and `n_roots` MUST equal len(M `t9`) + 1.
- **[F-06]** (11.3): The high byte of the B stream's method word MUST equal the M stream's
  compression byte.
- **[F-09]** (11.6.1): The stream id of a template segment MUST be `GetName(template_guid)` of
  its template object.
- **Annex D Edit procedures**: Annex D gives verified edit procedures: adding a user parameter,
  adding an occurrence, deleting occurrences and constraints, adding a constraint, adding or
  deleting a drawing note. D.6 lists the assembly and drawing edits whose outputs were opened
  in Inventor 2027, the assembly edits without a repair prompt. Clause 5.3 notes that W1 and
  W2 edits read back exactly.
- **Content clauses** (titles): 14.1 Parameters, units and equations; 15.1 Origin and work
  geometry, UCS; 15.2 2D sketches and sketch helpers; 15.3 The feature node 90874D91;
  16.1 Occurrences and placements; 13.6 Embedded ACIS body (node F645595C, whose `sab` token
  holds one ACIS / ShapeManager binary save file).
- **20.2 Derived fields (W3)** (excerpt of the fields a W3 writer recomputes [W-09]):

  | Field | Clause | Formula or rule |
  | --- | --- | --- |
  | type-table instance counters | 11.2.5 | cumulative; a class counter MUST NOT be lower than in the file the nodes came from |
  | `id_space`, t5 | 11.2.1, 11.2.7, 11.4 | highest id handed out; allocation set covers every live id and no deleted one |
  | t6, t7, t9 and the RSeSegInfo mirror | 11.5 | new t7 entry per save, t9 keys = t7 indexes, every segment moved to the new revision |
  | RSeSegInfo `revision` | 9.2.2, 11.5.3 | the save's new revision in every segment |
