//! The object-id allocation set (M stream t5, spec 11.2.7) on hand-built sets.

use std::collections::BTreeSet;

use corliss::edit::idset;

/// One chunk record (type 1): chunk 0 with block 0 fully used.
fn block0_used() -> Vec<u8> {
    let mut b = vec![0, 0, 0];
    b.extend(1u32.to_le_bytes());
    b.extend(0u32.to_le_bytes());
    b.extend([0, 0, 0]);
    b
}

#[test]
fn a_fully_used_block_allocates_its_eight_ids() {
    let ids = idset::decode(1, &block0_used()).unwrap();
    assert_eq!(ids, (0..8).collect::<BTreeSet<u64>>());
}

#[test]
fn adding_an_allocated_id_keeps_the_bytes() {
    assert_eq!(
        idset::add(1, &block0_used(), &[3], 7).unwrap(),
        (1, block0_used())
    );
}

#[test]
fn a_new_block_frees_only_its_other_ids_up_to_the_id_space() {
    // Expected from the reference writer (first type 1, add 9, id space 9): 8 is free,
    // 10..15 lie past the id space and stay allocated.
    let (first, bytes) = idset::add(1, &block0_used(), &[9], 9).unwrap();
    let ids = idset::decode(first, &bytes).unwrap();
    assert_eq!(ids, (0..8).chain(9..16).collect::<BTreeSet<u64>>());
}

#[test]
fn an_id_inside_a_run_splits_the_run_around_its_chunk() {
    // One run record (type 2): chunks 0..3, fill 0.
    let run: Vec<u8> = [0u32, 0, 3]
        .iter()
        .flat_map(|v| v.to_le_bytes()[..3].to_vec())
        .chain([0, 0, 0])
        .collect();
    let (first, bytes) = idset::add(2, &run, &[256], 256).unwrap();
    assert_eq!(first, 2, "the set still starts with the run's first part");
    // run 0..1, chunk 1 (type word, record), run 2..3 (type word, record), terminator
    assert_eq!(bytes.len(), 9 + 4 + 11 + 4 + 9 + 3);
    assert!(idset::decode(first, &bytes).unwrap().contains(&256));
}

#[test]
fn an_unknown_record_type_is_reported() {
    let err = idset::decode(5, &block0_used()).unwrap_err();
    assert!(err.to_string().contains('5'), "{err}");
}
