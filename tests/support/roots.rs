//! The root-record class rule of spec 9.2.4: in a segment the last save rewrote, `root_a`
//! indexes `uid_list1` at the class of the node the record points at.

use std::collections::HashMap;

use corliss::Document;
use corliss::Guid;

/// Records of segments `names` whose `root_a` does not name their target's class, as
/// "segment #record: found, expected"; unresolvable targets are skipped.
pub fn wrong_root_classes(doc: &Document, names: &[String]) -> Vec<String> {
    let si = doc.seg_info().unwrap();
    let mut wrong = Vec::new();
    for entry in si.segments.iter().filter(|e| names.contains(&e.name)) {
        let t6 = doc
            .decode_segment(&doc.segment_location(&entry.name).unwrap())
            .unwrap()
            .meta
            .body
            .tail
            .t6;
        for (k, r) in entry
            .roots
            .iter()
            .enumerate()
            .filter(|(_, r)| r.root_a >= 0)
        {
            let Some(link) = t6.get(usize::from(r.root_class)) else {
                continue;
            };
            let Some(class) = target_class(doc, link.seg_ref, r.object_id()) else {
                continue;
            };
            let found = si.uid_list1.get(r.root_a as usize).copied();
            if found != Some(class) {
                wrong.push(format!("{} #{k}: {found:?}, expected {class}", entry.name));
            }
        }
    }
    wrong
}

/// The class of object `id` (`(u32 at 0 << 16) | u16 at 4`) in segment `seg_id`.
fn target_class(doc: &Document, seg_id: Guid, id: u64) -> Option<Guid> {
    let loc = doc.segments().unwrap().into_iter().find(|l| {
        doc.decode_segment(l).is_ok_and(|s| {
            s.meta
                .header
                .identity
                .as_ref()
                .is_some_and(|i| i.seg_id == seg_id)
        })
    })?;
    let seg = doc.decode_segment(&loc).unwrap();
    let classes: HashMap<u64, Guid> = seg
        .blocks()
        .iter()
        .filter(|b| b.payload.len() >= 6)
        .map(|b| {
            let p = &b.payload;
            let oid = u64::from(u32::from_le_bytes(p[..4].try_into().unwrap())) << 16
                | u64::from(u16::from_le_bytes(p[4..6].try_into().unwrap()));
            (oid, seg.meta.body.types[b.type_index()].class)
        })
        .collect();
    classes.get(&id).copied()
}
