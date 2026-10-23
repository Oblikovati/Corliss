# Test fixtures

Inventor 2027 documents copied from the reference implementation's fixtures:

- `spec/`: the controlled single-change saves used to reverse-engineer the format. Each one is
  one UI change from a named predecessor. The lineage table stays with the rest of the format
  documentation.
- `fixture_part.ipt`, `fixture_assy.iam`: the reference part and assembly of the writer
  verification tests.

Tests read these files and never write to this directory.
