# Project files (.ipj)

Status: implemented (unreleased). Covers spec clause 19 (Edition 1.13): reading, checking,
editing and writing Inventor project files. Project files are the UTF-16 XML files that name
the folders where Inventor looks for a design's documents, plus the project's options. They
are not compound files, and clauses 7–18 do not apply to them.

## What Corliss does

`corliss::ipj` works in two layers, as the reference implementation does:

- **Lossless tree.** `ProjectFile::parse` keeps every byte: the byte order mark, the
  declaration, whitespace, attribute order and quoting, the `<X/>` or `<X></X>` form, and
  entities as written. An unchanged file writes back byte for byte in any layout (19.1,
  Table 19-1). `Element::set_text` and `set_attribute` change only the value they touch.
  UTF-16LE with or without a BOM, and UTF-8, are read. UTF-16BE is refused.
- **Typed view.** `ProjectFile::project` returns a `Project` with:
  - the schema revision and the Content Center GUID, decoded from its 26-character name
    with `naming::guid_from_name`;
  - the project options (`ProjectOptions::project_type` compares case-insensitively
    [R-13]);
  - the appearance and material libraries;
  - the search paths in file order (`ProjectPath::kind` compares case-sensitively);
  - the folder overrides, including the legacy `StylesFolder`;
  - the Vault settings;
  - the XML that Inventor keeps verbatim: add-in configurations, `ContentCenterConfig` and
    custom sections [W-17].

  Unknown elements and attributes are ignored [R-14].
- **Writing.** `Project::to_ipj(Layout)` writes a complete file in Inventor's layout (19.2,
  [W-19]) or in the layout of the shipped Default.ipj, with schema revision 14 and the elements
  in the writer's order [W-16].
- **Checking.** `ProjectFile::check` lists departures from clause 19: encoding, the
  declaration, the root, the schema revision, unknown elements and attributes, enumeration
  values, integers, Name26 GUIDs, sections older revisions ignore, legacy elements, and entries
  Inventor skips (empty `PathName` or `Path` [F-12], empty library type or flag). The messages
  are worded as the reference implementation words them.
- **Paths.** `resolve_path(project_dir, path)` joins a relative path to the project folder
  with a backslash and folds `.` and `..`, as `GetFullPathForProjectPath` does [R-15].
  Absolute and UNC paths are kept; a rooted `\X` takes the project folder's drive. Results
  stay Windows paths on every host.

## Evidence

The reference implementation's export tooling copies its four synthetic project files into
`tests/fixtures/ipj`. It also derives four variants from them, the ones the reference tests
build in memory:

- CRLF line breaks with a spaced empty element;
- an invalid file;
- revision 11 with a legacy `StylesFolder` and a custom section;
- a pre-revision file that has only `version`.

For each file it records what the reference reader reads (`ipj.json`) and the file rendered in both
layouts (`rendered/`).

`tests/ipj.rs` checks every file against that oracle:
- a lossless round trip;
- the detected layout;
- whether the typed view writes the file back byte for byte;
- the exact findings;
- the typed view;
- the rendering in both layouts, byte for byte.

It also checks:
- that an edit changes only the edited value;
- that a new project conforms;
- the Name26 codec (Default.ipj's GUID decodes as spec [C-186] gives it);
- path resolution;
- malformed input;
- UTF-8 files.

Inventor 2027's own `Default.ipj` is read in place when it is installed
(`C:\Users\Public\Documents\Autodesk\Inventor 2027\`). It is never copied into the repository.
Corliss round-trips it, finds nothing to report, writes it again byte for byte from the typed
view, and reads the library lists that spec F.1 gives. Spec F.1 lists a second real sample, a
user project saved by Inventor 2025, which is not available here. The Inventor layout is
covered by the synthetic files instead.

## Limits

- The INI-style project files that came before the XML form are outside the specification
  (19.5) and are not read.
- `IncludedProject` is not followed: how Inventor resolves it is open (OI-313). What a
  `FrequentlyUsedFolder` path is relative to is open too (OI-315); `resolve_path` treats it
  like the other paths.
- Comments, processing instructions and CDATA sections inside elements are kept as written but
  not counted as character data. Inventor never writes them (19.1).
- Like the reference, the writer leaves out a `ProjectPath` whose `pathtype` Inventor does not
  know.
- The reference's byte-coverage report is spec evidence tooling and is not
  part of Corliss.

## Specification fragments

- **19 Project files (.ipj)**: A project file is an XML document with the extension `.ipj`. It
  names the folders in which Inventor looks for the documents of a design (workspace,
  workgroups, libraries), the folders for templates, design data, presets and Content Center
  files, and the project's options. A project file is not a compound file, and clauses 7 to 18
  do not apply to it. The clause describes schema revision 14, which Inventor 2027 writes.
  Clause 19 was added in Edition 1.13; the writer requirements moved from clause 19 to 20.
- **19.1 Encoding and serialization**: A project file MUST be UTF-16LE and start with the byte
  order mark FF FE, MUST have an XML declaration with `version="1.0"` and `encoding="utf-16"`
  [F-10], and its document element MUST be `InventorProject` [F-11]. The file is well-formed XML
  1.0 without a DTD, namespaces, processing instructions other than the declaration, or CDATA
  sections. An editor that changes one value can keep every other byte.
- **Table 19-1** (excerpt, layout features):

  | Feature | Inventor writer | Default.ipj | Rule |
  | --- | --- | --- | --- |
  | Declaration | `standalone="no"` | no `standalone` | Free. |
  | Line break | LF | LF | Free; readers MUST accept LF and CRLF [R-12]. |
  | Empty text element | `<Owner></Owner>` | `<Owner/>` | Free; both mean the empty string. |
  | Attribute order | ordinal by name | writer-code order | Free; readers MUST NOT depend on it [R-12]. |
  | Final line break | one LF | none | Free. |
  | Order of repeated `ProjectPath`, `ProteinLibrary` | fixed | the same | Significant: it is the search and list order. |
  | Character data of text elements | exact | exact | Significant [W-20]. |
  | Enumeration case | as in clause 19 | the same | `ProjectType` and `UsingUniqueFilenames` compare case-insensitively, `pathtype` case-sensitively [W-16]. |

- **[R-13]** (19.3.3): Readers of project files SHOULD compare `ProjectType` and
  `UsingUniqueFilenames` case-insensitively.
- **[W-17]** (19.3.2, 19.3.6, 19.3.8): Writers MUST preserve the RawXML elements (add-in
  configurations, `ContentCenterConfig`, custom sections). RawXML is an element that Inventor
  keeps as an XML string and writes back as it was read, with attributes sorted.
- **[R-14]** (19.3.1): Readers of project files MUST ignore unknown elements and attributes.
- **19.2 Writers in the field**: Inventor rebuilds the whole document on every save from its
  in-memory project; its writer keeps attributes in a sorted map. The shipped `Default.ipj` is
  laid out differently: no `standalone`, attributes in code order, self-closed empty elements
  and no final LF.
- **[W-19]** (19.2): Writers SHOULD use Inventor's layout: declaration with `standalone="no"`,
  sorted attributes, `<X></X>` for empty text elements, blank lines between top-level
  elements, a final LF.
- **[W-16]** (19.3.1, 19.6): Writers MUST write `schemarevid` 14, the elements in the order of
  Figure 19-1 and the enumeration spellings of clause 19.
- **[F-12]** (19.3.5): `ProjectPath` `PathName` and `Path` MUST NOT be empty: Inventor ignores an
  entry with an empty name or path.
- **[R-15]** (19.4): A relative `Path` is resolved against the folder that contains the .ipj.
  Inventor (`GetFullPathForProjectPath`) joins it to that folder with a backslash and
  canonicalizes the result, so `.` is the project folder and `..\X` is a sibling folder. Readers
  MUST resolve relative paths this way.
- **[C-186] GUID attributes**: Inventor writes a GUID attribute as the 26-character string of
  `UTxGUID::GetName` and reads it with `UTxGUID::SetName`. Decoded this way, Default.ipj's
  `mresgphncrxblhb6eku2x43fnh` is d91248cf-46e1-4ed1-8afc-705685f7bbe9.
- **F.1 Sample populations** (project set): 2 project files, `Default.ipj` shipped with Inventor
  2027 (9,466 bytes, read from the installation) and a user project saved by Inventor 2025
  (10,308 bytes); neither is in the repository. Both have `schemarevid` 14.
- **19.5 Schema revisions**: Project files older than the XML form were INI-style text, which
  Inventor 2027 still converts. That format is outside the specification.
- **OI-313** (19.3.3, 19.4, open): Resolution base of a relative `IncludedProject`, and the full
  `CanInclude` rule.
- **OI-315** (19.3.5, open): What a `FrequentlyUsedFolder` path is relative to (project folder or
  workspace).
