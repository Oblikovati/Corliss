# Node decoding and semantic models

Status: implemented (unreleased). This covers reader class R3 of the Inventor File Format
Specification (spec 5.2): node payloads are decoded into typed fields, and parts, assemblies and
drawings are read into typed models.

## How node payloads are decoded

The layout of every node type is defined by the grammars of the reference implementation.
These are about 184,000 lines of Python, still being
extended. They are not data: they use helper calls, closures, version switches, loops over counts
read from the payload, and "re-cut" layers that re-read token ranges of an older grammar.

Porting them by hand would not survive that churn, so Corliss runs them instead.

- The reference implementation's export tooling snapshots everything the grammar tables can reach. It
  writes every function as its Python AST, together with its defaults, closure cells and attributes
  (`__wrapped__`, `_docnames` …), module globals, constants, and each reader's TypeID → grammar
  table. The result is `src/grammar/grammars.bin.zst`: 14.7 MB raw, 1.2 MB compressed.
- `src/grammar/interp` is an interpreter for the Python subset the grammars use: functions and
  closures, `global`, `try`/`except`/`finally`, comprehensions and generators, slicing, `%`
  formatting, and builtins.
- `src/grammar/native` reimplements the engine objects natively, matching the reference byte for
  byte: the reference grammar engine's parser with its readers and list walkers, its token type,
  its grammar-table lookup, and the `struct` and `importlib` modules.
- `src/grammar/decode.rs` turns the resulting tokens into `DecodedNode { fields: Vec<Field> }`.
  Each `Field` has a name, a typed `FieldValue`, its source bytes, and its former names.

The interpreter allocates into a heap that belongs to one decode session; it uses no `Rc` or
`RefCell`. Constructs it does not implement fail at run time, naming the construct; the node then
becomes partial. The fixtures never reach such a construct.

### Oracle

The reference implementation's export tooling runs the reference grammar engine over every
fixture. For each segment it writes an FNV-1a fingerprint of the canonical text of every
token: kind, name, value, bytes, ref kind and list framing. These fingerprints are stored in
`tests/fixtures/grammar-digests.tsv`. `corliss::grammar::fingerprint` computes the same text from
Corliss's typed fields, so a match also proves that converting tokens to fields loses nothing.

Result: 2,480 of 2,480 segments (569,132 nodes, templates included) match the reference engine
exactly, and none is partial. Release builds check the full corpus in about 30 s. Debug builds
check every fifth fixture; set `CORLISS_FIXTURE` to check chosen fixtures.

### Updating

When the grammars change, the snapshot and the reference files are regenerated with the
reference implementation's export tooling, which is not part of this repository, and then
`cargo test --release --test grammar` is run.

## Semantic models

`corliss::model` reads documents the way the reference readers for parts, assemblies and
drawings do:

| Model | Methods |
| --- | --- |
| `Part` | `parameters`, `expression`, `work_planes`, `work_axes`, `work_points`, `user_coordinate_systems`, `sketches`, `features` (about 55 feature types), `imates`, `sheet_metal`, `materials`, `ipart_table` |
| `Assembly` | `files`, `occurrences` (with placement matrices), `representations`, `design_views`, `positional_overrides`, `patterns`, `constraints`, `features` |
| `Drawing` | `sheets`, `sheet_formats`, `references`, `views`, `title_block`, `notes`, `dimension_texts`, `dimensions` |

`Model::load` decodes every segment once; the models only read from it.

The reference implementation's export tooling writes the reference readers' output for every fixture to
`tests/fixtures/models.json`, and `tests/model_*.rs` compare each method with it. Floats use a
relative tolerance of 1e-9. Every method matches on every fixture: 196 parts, 33 assemblies and
69 drawings.

### Known differences

- `Drawing::title_block` copies a bug in the reference reader on purpose. The reference drawing reader
  counts the *fields* of `local_origin`; that field is one two-float array, so the count is always
  below 2. As a result the title-block origin is never added to the field positions. Fixing it
  needs a fix in the reference drawing reader, a regenerated `models.json`, and a one-line change in
  `src/model/drawing/texts.rs`.
- Where the reference reader would raise on a malformed document (a null selection, a missing
  segment), Corliss skips the entry or returns a typed error. No fixture reaches these paths.

## Next steps

- Writer class W3 (adding and removing nodes), built on the typed fields and their byte spans.
- ACIS/ShapeManager bodies are read into topology by `corliss::acis`
  (see `docs/features/acis-brep`).
- Samples from older releases (2025 zlib, 2010 layouts). The grammars switch on `ctx.file`, but
  only 2027 files are in the fixture corpus.

## Specification fragments

- **5.2 Readers**: R3 adds the content clauses it claims, listed by clause number
  (clauses 14–18), to R2, which reads the database, document record, segments, blocks and node
  framing (clauses 9–13).
- **5.3 Writers (editors)**: W3 adds or removes nodes.
