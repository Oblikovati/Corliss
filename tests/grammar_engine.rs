//! The grammar interpreter on payloads Inventor did not write (the corpus has no partial node):
//! a payload with bytes the grammar does not read decodes as partial and still re-encodes
//! exactly, and changed fields re-encode at their new length while the rest of the payload
//! keeps its bytes.

mod support;

use corliss::grammar::{self, DecodeStatus, DecodedNode, FieldValue};
use corliss::{Document, TypeId};

const T_PARAMETER: u32 = 0x9087_4D26;

/// The first parameter node of `fixture_part.ipt`, with what decoding it needs.
fn parameter() -> (String, grammar::DecodeContext, Vec<u8>) {
    let doc: Document = support::open_label("fixture_part.ipt");
    let loc = doc.segment_location("PmDCSegment").unwrap();
    let decoded = doc.decode_nodes(&loc).unwrap();
    let node = decoded
        .nodes
        .iter()
        .find(|n| n.type_id.0 == T_PARAMETER)
        .unwrap();
    (
        decoded.segment_type,
        doc.decode_context(false).unwrap(),
        node.payload(),
    )
}

fn decode(segment_type: &str, ctx: grammar::DecodeContext, payload: &[u8]) -> DecodedNode {
    let nodes =
        grammar::decode_segment(segment_type, ctx, &[(TypeId(T_PARAMETER), payload)]).unwrap();
    nodes.into_iter().next().unwrap()
}

#[test]
fn trailing_bytes_make_a_partial_node_that_still_round_trips() {
    let (st, ctx, mut payload) = parameter();
    assert_eq!(decode(&st, ctx, &payload).status, DecodeStatus::Full);
    payload.extend([0xDE, 0xAD, 0xBE]);
    let node = decode(&st, ctx, &payload);
    assert_eq!(node.status, DecodeStatus::Partial);
    assert_eq!(node.payload(), payload);
}

#[test]
fn a_truncated_payload_is_partial_and_round_trips() {
    let (st, ctx, payload) = parameter();
    let cut = &payload[..payload.len() / 2];
    let node = decode(&st, ctx, cut);
    assert_ne!(node.status, DecodeStatus::Full);
    assert_eq!(node.payload(), cut);
}

#[test]
fn a_longer_name_re_encodes_and_leaves_the_other_fields() {
    let (st, ctx, payload) = parameter();
    let mut node = decode(&st, ctx, &payload);
    let old = node.text("name").unwrap().to_string();
    let new = format!("{old}_renamed");
    node.set("name", FieldValue::Text(new.clone())).unwrap();
    let out = node.payload();
    assert_eq!(
        out.len(),
        payload.len() + 2 * (new.chars().count() - old.chars().count())
    );
    let again = decode(&st, ctx, &out);
    assert_eq!(again.text("name"), Some(new.as_str()));
    let others = |n: &DecodedNode| {
        n.fields
            .iter()
            .filter(|f| !f.is("name"))
            .map(|f| f.raw.clone())
            .collect::<Vec<_>>()
    };
    assert_eq!(others(&again), others(&decode(&st, ctx, &payload)));
}

#[test]
fn remapped_references_keep_their_flag_and_kind() {
    let (st, ctx, payload) = parameter();
    let mut node = decode(&st, ctx, &payload);
    let refs = node.refs();
    let (first, kind) = *refs.iter().find(|(r, _)| r.index().is_some()).unwrap();
    let target = first.index().unwrap();
    node.remap_refs(&|i| (i == target).then_some(target + 1000));
    let moved = node
        .refs()
        .into_iter()
        .find(|(r, _)| r.index() == Some(target + 1000))
        .unwrap();
    assert_eq!((moved.0.is_flagged(), moved.1), (first.is_flagged(), kind));
    assert_eq!(node.payload().len(), payload.len());
}
