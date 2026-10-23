# Structural editing (writer class W3)

Status: implemented (unreleased). Covers spec clause 20 for structural edits: adding, removing
and reordering nodes, changing fields, and rebuilding every derived field of the segments that
were touched.

## How an edit runs

`corliss::edit::Editor` opens a copy of a document. `Editor::segment(name)` decodes a segment
into an `EditSegment`: its nodes as typed fields (`EditNode`), in block order, with their block
headers and trailers. Callers change fields (`DecodedNode::set`, `list_mut`), append nodes
(`append`, `clone_node`, `clone_subgraph`), delete them (`delete`, which renumbers every NodeRef
and refuses while a reference to a deleted node remains) or reorder them.

`Editor::finish` rebuilds each opened segment the way the reference writer commits a
segment:

- B stream: one block per node, the type index in the header's low byte; a new node gets the
  header `0x100 | index` and, in 2015+ files, the trailer byte 0.
- M stream:
  - the block table keeps absent slots and takes the new lengths;
  - the instance counter of every class that gained nodes goes up;
  - `slot_count` moves by the number of nodes added;
  - new object ids go into the allocation set (t5), the id space and the last sec2 entry;
  - `end_count`, `n_roots` and the header id space are derived again.
- RSeSegInfo entry: block and root counts, heap object count, and references taken from t6/t7.

Streams of segments that were not opened are kept byte for byte [W-01]. Field values are
re-encoded the way the reference grammar engine re-encodes a changed token: text16 is the character count followed by
UTF-16LE, with no terminator.

New object ids follow the reference writer's id allocation. An id is never one that the allocation set already
marks used: Inventor saves whole 8-id blocks, and giving a new node an id from such a block
crashes Inventor on load. Ids handed out to nodes that were deleted again in the same session are
left out of the allocation set.

GUIDs that an edit needs (revisions, state ids) come from `GuidSource::Random`, or from
`GuidSource::Fixed` for reproducible output.

## Operations

- `edit::add_user_parameter` adds a user parameter to a part. It clones the first user
  parameter's exclusively owned subgraph (parameter, unit, formula value), renames it, sets its
  values and object index, and links it into the component node's `objects` list and, in key
  order, its `parameters` name map.
- `edit::assembly`:
  - `set_constraint_value` changes a constraint's offset or angle.
  - `set_grounded` grounds or frees an occurrence.
  - `set_occurrence_transform` places an occurrence by a 4×4 matrix (`Placement`).
  - `add_occurrence` copies an occurrence under a new name.
  - `remove_occurrence` and `remove_constraint` delete one together with what it owns.
  - `add_constraint_from` copies a constraint from a template assembly onto a pair of
    occurrences.
  - Each operation keeps the data Inventor checks on load in step: AmRx, AmGraphics and
    browser records, the cross-segment t6, t7 and t9 tables and their RSeSegInfo copies, the
    UFRxDoc occurrence records, and the document revision.
- `edit::drawing`:
  - `set_note_text` replaces a sheet note's text and keeps its leading format codes.
  - `rename_sheet` renames a sheet.
  - `move_view` and `set_view_scale` change a drawing view's position and scale, including
    the scale label.
  - `add_note` copies a note's node groups from a template drawing into a sheet. It links them
    into the sheet, the document DC and the sheet's SM segment, and gives them fresh ids past
    the allocation set's top.
  - `delete_note` removes a note and lowers the usage counts the note held.
  - A note is named by its plain text.

Each edit draws one revision GUID per session (`Editor::session_revision`). It is stored in
the layout the reference uses, which is `uuid.bytes` rather than the Windows layout.

## Evidence

The reference implementation's export tooling runs its verified edit operations on the
fixtures with GUID generation (`uuid.uuid4`) made deterministic, and writes each output to `tests/fixtures/edits`
together with the GUIDs each job drew (`jobs.json`). `tests/edit.rs` runs the same edit in
Corliss with those GUIDs. It requires every stream of the output to equal the reference after
decompression: M header and body, B blocks and end table, RSeSegInfo, and every other stream.
Compressed bytes are allowed to differ, because the compressor is not the same.

- `P0_touch`: opening PmDCSegment and committing it unchanged.
- `P1_userparam`: `add_user_parameter(…, "my_len2", 3.0)` on `fixture_part.ipt`.

- `W1`–`W14` (`tests/edit_assembly.rs`): constraint value, grounding, placement, adding and
  removing an occurrence, removing a constraint, and copying mate, flush and angle constraints,
  on `as_mate`, `as_place2`, `as_place3` and `as_delmate`.
- `R1`–`R7` (`tests/edit_drawing.rs`): note text, sheet rename, view move and scale on
  `dw_note.idw` and `dw_proj.idw`, plus adding and deleting a note.

A further test checks that the comparison reports a document that was not edited.
`tests/edit_idset.rs` checks the allocation-set codec on hand-built sets. Its expected values
come from the reference writer's allocation-set update.

Four reference bugs were fixed in the reference implementation and in the ports:

- `add_constraint_from` inserts classes into RSeSegInfo's `uid_list1` and `uid_list2`, which
  shifts later indexes. Root records are now kept on their classes. In the segments the edit
  opened, `root_a` is set to the class of the record's target, as spec 9.2.4 [W-09] requires.
  W10 and W14 changed with this fix. `tests/support/roots.rs` checks the rule on every
  constraint copy, and fails without the fix.
- When a note is added, a template root record's `root_a` or `root_b` of −1 ("target not
  loaded") stays −1. It no longer becomes the last class of the list. An index past the
  template's list is refused.
- Copied note nodes have their references renumbered through typed fields: NodeRefs, flagged
  u32/i32 values, and flagged words in raw tails, without overlapping matches. Floats, text
  and GUIDs that look like a NodeRef are no longer rewritten. The note record's style is found
  through its typed `style` ref.
- A new note keeps only the leading run of the template text's format codes.

The last three leave every reference output unchanged. In-crate unit tests in
`edit::drawing` (`xref`, `note_parts`, `fields`) pin them.

## Limits

- Only the operations above exist. Their options that no reference job exercises are left out:
  - `add_occurrence`: `parts` and `oid`;
  - `add_constraint_from`: `name`, `faces` and `copy_solver_state`;
  - `set_iproperty`.
- Adding a class that the segment's type table lacks is refused, as in the reference.
- The ports reproduce the reference byte for byte. These reference behaviours still look
  wrong and are kept:
  - Clearing a copied constraint's solver state reads fields that the current placement grammar
    does not name, so it never runs. The Inventor-crash fix it describes (W14m) is not in effect.
  - The reference reader's placement transform reads the placement's `transformation.marker` word as a matrix
    value. Identity placements are unaffected.
  - Revision GUIDs and the CB072B3B `uid1`–`uid4` fields use `uuid.bytes` (RFC order), not the
    Windows layout that Inventor uses elsewhere.
  - Copying a template note's parts renumbers any flagged u32 field that names a remapped node.
  - Lowering a usage count whose key is missing (a step of -1) adds a count of 0xFFFFFFFF.
  - `delete_note` lowers usage counts before it knows the document DC record was found.
  - `Fraction.limit_denominator(1000)` for view scale labels is ported as a closest-fraction
    search. An exact tie between two fractions could resolve differently.
- Operations read the session's edited segments, not the file on disk as the reference
  assembly reader does. This is the same within one operation, and safer when several run in one session.

## Specification fragments

- **5.3 Writers (editors)**: W3 adds or removes nodes. A conforming writer edits an existing
  document; generating documents from nothing is out of scope.
- **20.2 Derived fields (W3)** (excerpt of the fields a W3 writer recomputes [W-09]):

  | Field | Clause | Formula or rule |
  | --- | --- | --- |
  | block-table entries | 11.2.3 | payload length with bit 31 set for present blocks; absent entries kept as they are |
  | block header | 12.1 | `0x100 \| type index` |
  | `end_count` | 11.2.2 | = end-table count |
  | `n_roots` | 11.2.2, 9.2.2 | = len(t9) + 1 = RSeSegInfo `n_roots` |
  | `slot_count` | 11.2.2 | ≥ block count |
  | type-table instance counters | 11.2.5 | cumulative; a class counter MUST NOT be lower than in the file the nodes came from |
  | `id_space`, t5 | 11.2.1, 11.2.7, 11.4 | highest id handed out; allocation set covers every live id and no deleted one |
  | RSeSegInfo `n_blocks` | 9.2.2 | = present block count |
  | RSeSegInfo `ref_count` | 9.2.3 | = sum of t7 counts for the reference |
  | `uid_list1`, `uid_list2` | 9.2.1 | contain every class a root record names |
  | t6, t7, t9 and the RSeSegInfo mirror | 11.5 | new t7 entry per save, t9 keys = t7 indexes, every segment moved to the new revision |
  | RSeSegInfo `revision` | 9.2.2, 11.5.3 | the save's new revision in every segment |
  | UFRxDoc counters | 10.1 | `max_inv_file_ref_id`, `max_occurrence_id`, per-file occurrence counts, duplicated element counts |

- **[W-01]** (5.3): Writers MUST preserve every byte that the edit does not need to change:
  unknown streams, nodes and fields.
- **9.2.4 Root record**: `root_a` (i16) is the index in `uid_list1` of the target object's
  class, −1 when the target was not loaded at save time; `root_b` (i16) is the index in
  `uid_list2` of the class of the object that holds the reference. A segment that a save does
  not rewrite keeps its root records and their indices. Writers that rewrite a segment MUST
  write indices into the current lists [W-09]. 9.2.1 adds: a writer that adds a root whose
  target class is not listed MUST append the class GUID [W-09].
- **[W-09]** (5.3): W3 writers MUST regenerate the block table (absent entries kept), the
  type-table counters, `slot_count`, the id space and id allocation set, the RSeSegInfo
  counts, t6/t7/t9 with their RSeSegInfo mirror, and the revision (clause 20).
