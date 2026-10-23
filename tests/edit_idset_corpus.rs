//! The object-id allocation set (M stream t5, spec 11.2.7) over every segment of the corpus:
//! the section splits and re-encodes exactly, its records re-encode exactly, and allocating the
//! next id changes the set by that id only, leaving the other chunk records' bytes alone.

mod support;

use std::collections::BTreeSet;

use corliss::Document;
use corliss::edit::idset::{self, T5};

/// One segment's t5 section with the id space it records (the last sec2 N).
struct Section {
    label: String,
    bytes: Vec<u8>,
    id_space: u32,
}

fn sections() -> Vec<Section> {
    let mut out = Vec::new();
    for path in support::all_fixtures() {
        let doc = Document::open(&path).unwrap();
        for loc in doc.segments().unwrap() {
            let body = doc.decode_segment(&loc).unwrap().meta.body;
            let Some(bytes) = body.tail.t5 else { continue };
            let id_space = body.sec2.last().map_or(0, |e| e.1);
            out.push(Section {
                label: format!("{} {}", support::label(&path), loc.name),
                bytes,
                id_space,
            });
        }
    }
    out
}

/// Ids at or below `id_space` (bits above it are implicit).
fn allocated(first: u32, bitset: &[u8], id_space: u64) -> BTreeSet<u64> {
    idset::decode(first, bitset)
        .unwrap()
        .into_iter()
        .filter(|i| *i <= id_space)
        .collect()
}

#[test]
fn every_allocation_set_round_trips_and_takes_the_next_id() {
    let all = sections();
    assert!(all.len() > 2000, "{} sections", all.len());
    let failures: Vec<String> = all
        .iter()
        .filter_map(|s| check(s).err().map(|e| format!("{}: {e}", s.label)))
        .collect();
    assert!(
        failures.is_empty(),
        "{} of {} sections:\n{}",
        failures.len(),
        all.len(),
        failures.join("\n")
    );
}

fn check(s: &Section) -> Result<(), String> {
    let t5 = T5::split(&s.bytes, s.id_space).ok_or("t5 does not split at its id space")?;
    if t5.encode() != s.bytes {
        return Err("split and encode change the bytes".into());
    }
    // Adding nothing parses and re-serializes the records.
    let (first, bits) = (t5.first_type, t5.bitset.clone());
    if idset::add(first, &bits, &[], u64::from(s.id_space)).map_err(|e| e.to_string())?
        != (first, bits.clone())
    {
        return Err("records do not re-serialize to their bytes".into());
    }
    let top = u64::from(s.id_space) + 1;
    let (f2, b2) = idset::add(first, &bits, &[top], top).map_err(|e| e.to_string())?;
    let mut want = allocated(first, &bits, u64::from(s.id_space));
    want.insert(top);
    if allocated(f2, &b2, top) != want {
        return Err(format!("adding id {top} changes other ids"));
    }
    untouched_chunks_kept(first, &bits, f2, &b2, top)
}

/// Every chunk record other than the new id's chunk keeps its bytes.
fn untouched_chunks_kept(
    first: u32,
    bits: &[u8],
    f2: u32,
    b2: &[u8],
    top: u64,
) -> Result<(), String> {
    let before = chunk_ids(first, bits);
    let after = chunk_ids(f2, b2);
    let chunk = top / 256;
    let lost: Vec<u64> = before
        .iter()
        .filter(|(c, ids)| **c != chunk && after.get(*c) != Some(*ids))
        .map(|(c, _)| *c)
        .collect();
    if lost.is_empty() {
        Ok(())
    } else {
        Err(format!("chunks {lost:?} changed"))
    }
}

/// Allocated ids grouped by 256-id chunk.
fn chunk_ids(first: u32, bits: &[u8]) -> std::collections::BTreeMap<u64, BTreeSet<u64>> {
    let mut out: std::collections::BTreeMap<u64, BTreeSet<u64>> = Default::default();
    for id in idset::decode(first, bits).unwrap() {
        out.entry(id / 256).or_default().insert(id);
    }
    out
}
