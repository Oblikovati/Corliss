//! Invariants between a segment's M stream, B stream and directory entry over the corpus: the
//! fields a writer recomputes (spec 20.2, [W-09]) must hold in every file Inventor wrote.

mod support;

use std::collections::HashMap;

use corliss::Document;
use corliss::database::SegmentEntry;
use corliss::edit::idset::T5;
use corliss::segment::Segment;

#[test]
fn every_segment_keeps_its_derived_fields_consistent() {
    let mut failures = Vec::new();
    let mut checked = 0;
    for path in support::all_fixtures() {
        let doc = Document::open(&path).unwrap();
        let entries = doc.seg_info().unwrap().segments;
        for loc in doc.segments().unwrap() {
            let seg = doc.decode_segment(&loc).unwrap();
            let entry = entries.iter().find(|e| {
                seg.meta
                    .header
                    .identity
                    .as_ref()
                    .is_some_and(|i| i.seg_id == e.seg_id)
            });
            checked += 1;
            for broken in violations(&seg, entry) {
                failures.push(format!("{} {}: {broken}", support::label(&path), loc.name));
            }
        }
    }
    assert!(checked > 2000, "{checked} segments");
    assert!(
        failures.is_empty(),
        "{} violations:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

/// The names of the invariants `seg` (and its directory entry) break.
fn violations(seg: &Segment, entry: Option<&SegmentEntry>) -> Vec<&'static str> {
    let mut out = m_and_b(seg);
    if let Some(e) = entry {
        out.extend(directory(seg, e));
    }
    out
}

fn m_and_b(seg: &Segment) -> Vec<&'static str> {
    let body = &seg.meta.body;
    let blocks = seg.blocks();
    let present: Vec<usize> = body
        .blocks
        .iter()
        .filter(|b| b.is_present())
        .map(|b| b.length())
        .collect();
    let mut per_type: HashMap<usize, u32> = HashMap::new();
    blocks
        .iter()
        .for_each(|b| *per_type.entry(b.type_index()).or_default() += 1);
    let sparse = body.present_blocks() != body.blocks.len();
    let end = &seg.data.data.end_table;
    let id_space = seg.meta.header.identity.as_ref().map(|i| i.id_space);
    let sec2_n = body.sec2.last().map(|e| u64::from(e.1));
    let t5_space = body
        .tail
        .t5
        .as_ref()
        .zip(sec2_n)
        .and_then(|(t, n)| T5::split(t, n as u32))
        .map(|t| u64::from(t.id_space));
    let checks = [
        (
            "block table lengths == B payload lengths",
            present == blocks.iter().map(|b| b.payload.len()).collect::<Vec<_>>(),
        ),
        (
            "block header == 0x100 | type index",
            blocks
                .iter()
                .all(|b| b.header >> 8 == 1 && b.type_index() < body.types.len()),
        ),
        (
            "instance counter >= blocks of the class",
            body.types
                .iter()
                .enumerate()
                .all(|(k, t)| t.instance_counter >= per_type.get(&k).copied().unwrap_or(0)),
        ),
        (
            "sparse marker == 0xFFFF exactly when the block table has absent slots",
            body.types
                .iter()
                .all(|t| t.sparse_marker == if sparse { 0xFFFF } else { 0 }),
        ),
        (
            "slot_count >= present blocks",
            body.slot_count as usize >= body.present_blocks(),
        ),
        ("end_count == end table a", body.end_count == end.a),
        (
            "n_roots == len(t9) + 1",
            body.n_roots as usize == body.tail.t9.len() + 1,
        ),
        (
            "header id space == last sec2 N",
            sec2_n.is_none() || id_space == sec2_n,
        ),
        (
            "t5 id space == header id space",
            body.tail.t5.is_none() || t5_space == id_space,
        ),
        (
            "end table index list ends at the record count",
            end.indexes.last().map_or(0, |n| *n as usize) == end.records.len(),
        ),
    ];
    checks
        .iter()
        .filter(|(_, ok)| !ok)
        .map(|(n, _)| *n)
        .collect()
}

fn directory(seg: &Segment, e: &SegmentEntry) -> Vec<&'static str> {
    let tail = &seg.meta.body.tail;
    let mut sums: HashMap<u16, u32> = HashMap::new();
    tail.t7
        .iter()
        .for_each(|t| *sums.entry(t.segment).or_default() += u32::from(t.count));
    let links_match = e.refs.len() == tail.t6.len()
        && e.refs
            .iter()
            .zip(&tail.t6)
            .all(|(r, l)| r.ref_seg_id == l.seg_ref && r.ref_revision == l.revision);
    let counts_match = e
        .refs
        .iter()
        .enumerate()
        .all(|(k, r)| r.ref_count == sums.get(&(k as u16)).copied().unwrap_or(0));
    let roots_mirror = e.roots.is_empty()
        || (e.roots.len() == tail.t9.len()
            && e.roots.iter().zip(&tail.t9).all(|(r, t)| mirrors(r, t)));
    let n_root_records = if e.roots.is_empty() {
        e.n_root_records == 0 || e.n_root_records == e.n_roots
    } else {
        e.n_root_records == e.n_roots
    };
    let checks = [
        (
            "entry n_blocks == present blocks",
            e.n_blocks as usize == seg.meta.body.present_blocks(),
        ),
        (
            "entry n_roots == len(t9) + 1",
            e.n_roots as usize == tail.t9.len() + 1,
        ),
        ("entry references == t6 (segment, revision)", links_match),
        ("entry reference counts == t7 sums", counts_match),
        ("root records mirror t9", roots_mirror),
        (
            "n_root_records == n_roots with root records, else 0 or n_roots",
            n_root_records,
        ),
    ];
    checks
        .iter()
        .filter(|(_, ok)| !ok)
        .map(|(n, _)| *n)
        .collect()
}

/// A root record repeats its t9 entry: key, then the object id's three u16 words.
fn mirrors(r: &corliss::database::RootRecord, t: &[u16; 4]) -> bool {
    let words = [r.obj_id_hi as u16, (r.obj_id_hi >> 16) as u16, r.obj_id_lo];
    r.root_key == t[0] && words == [t[1], t[2], t[3]]
}
