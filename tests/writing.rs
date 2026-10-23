//! Writer behaviour: container round trip, W1 property edits and W2 payload edits.

mod support;

use corliss::compression::Compression;
use corliss::propset::{PropertyValue, fmtid};
use corliss::segment::Segment;
use corliss::{Document, FailureClass};

/// Design Tracking Properties pid 5: Part Number (spec Table 8-2).
const PART_NUMBER: u32 = 5;

fn reopen(doc: &Document) -> Document {
    Document::from_bytes(&doc.to_bytes().expect("document encodes"))
        .expect("written document decodes")
}

#[test]
fn rewritten_container_keeps_every_stream_and_clsid() {
    for path in support::all_fixtures().iter().step_by(7) {
        let doc = Document::open(path).unwrap();
        let again = reopen(&doc);
        assert_eq!(
            again.container().root_clsid(),
            doc.container().root_clsid(),
            "{}",
            support::label(path)
        );
        assert_eq!(
            again.container().entries(),
            doc.container().entries(),
            "{}",
            support::label(path)
        );
    }
}

#[test]
fn saved_file_reopens_from_disk() {
    let doc = support::open_spec("ex_rect20.ipt");
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("copy.ipt");
    doc.save(&path).unwrap();
    assert_eq!(Document::open(&path).unwrap(), doc);
}

#[test]
fn part_number_edit_is_read_back() {
    let mut doc = support::open_spec("ex_rect20.ipt");
    let mut stream = doc
        .property_set(&fmtid::DESIGN_TRACKING)
        .unwrap()
        .expect("design tracking set");
    let set = stream.set_mut(&fmtid::DESIGN_TRACKING).unwrap();
    set.set(PART_NUMBER, PropertyValue::Text("CORLISS-0001".into()));
    doc.write_property_set(&stream).unwrap();

    let again = reopen(&doc)
        .property_set(&fmtid::DESIGN_TRACKING)
        .unwrap()
        .unwrap();
    let value = again
        .set(&fmtid::DESIGN_TRACKING)
        .unwrap()
        .get(PART_NUMBER)
        .cloned();
    assert_eq!(value, Some(PropertyValue::Text("CORLISS-0001".into())));
}

#[test]
fn edited_property_set_keeps_its_other_properties() {
    let doc = support::open_spec("ex_rect20.ipt");
    let original = doc.property_set(&fmtid::DESIGN_TRACKING).unwrap().unwrap();
    let mut edited = original.clone();
    edited
        .set_mut(&fmtid::DESIGN_TRACKING)
        .unwrap()
        .set(PART_NUMBER, PropertyValue::Text("X".into()));
    let reparsed = corliss::propset::PropertySetStream::decode(&edited.encode()).unwrap();
    let before = &original.sets[0].properties;
    let after = &reparsed.sets[0].properties;
    assert_eq!(before.len(), after.len());
    for (b, a) in before
        .iter()
        .zip(after)
        .filter(|(b, _)| b.id != PART_NUMBER)
    {
        assert_eq!(a, b);
    }
}

fn last_byte_flipped(seg: &mut Segment) -> (usize, Vec<u8>) {
    let block = seg.blocks_mut().last_mut().unwrap();
    let byte = block.payload.last_mut().unwrap();
    *byte ^= 0xFF;
    (block.slot, block.payload.clone())
}

#[test]
fn in_place_payload_edit_is_read_back_and_isolated() {
    let mut doc = support::open_spec("ex_rect20.ipt");
    let loc = doc.segment_location("PmDCSegment").unwrap();
    let mut seg = doc.decode_segment(&loc).unwrap();
    let (slot, payload) = last_byte_flipped(&mut seg);
    let untouched = doc.clone();
    doc.write_segment(&loc, &seg).unwrap();

    let again = reopen(&doc);
    let reread = again.decode_segment(&loc).unwrap();
    assert_eq!(
        reread.blocks().last().map(|b| (b.slot, b.payload.clone())),
        Some((slot, payload))
    );
    assert_eq!(
        reread.meta.body, seg.meta.body,
        "a same-length edit leaves the M body unchanged"
    );
    for (path, entry) in untouched.container().entries() {
        if path != &loc.b_path() {
            assert_eq!(again.container().entries().get(path), Some(entry), "{path}");
        }
    }
}

#[test]
fn longer_payload_updates_the_block_table() {
    let doc = support::open_spec("ex_rect20.ipt");
    let loc = doc.segment_location("PmDCSegment").unwrap();
    let mut seg = doc.decode_segment(&loc).unwrap();
    let block = seg.blocks_mut().last_mut().unwrap();
    block.payload.extend_from_slice(&[0; 8]);
    let (slot, len) = (block.slot, block.payload.len());
    let (m, b) = seg.encode().unwrap();
    let reread = Segment::decode(&m, &b, false, true).unwrap();
    assert_eq!(reread.meta.body.blocks[slot].length(), len);
    assert_eq!(reread.blocks(), seg.blocks());
}

#[test]
fn segment_re_encodes_with_zlib() {
    let doc = support::open_spec("ex_rect20.ipt");
    let loc = doc.segment_location("PmDCSegment").unwrap();
    let mut seg = doc.decode_segment(&loc).unwrap();
    seg.meta.header.compression = Compression::Zlib;
    seg.data.compression = Compression::Zlib;
    let (m, b) = seg.encode().unwrap();
    let reread = Segment::decode(&m, &b, false, true).unwrap();
    assert_eq!(reread.meta.header.compression, Compression::Zlib);
    assert_eq!(reread.meta.body, seg.meta.body);
    assert_eq!(reread.blocks(), seg.blocks());
}

#[test]
fn mismatched_compression_is_rejected() {
    // [F-06]: the B method word's high byte must equal the M compression byte.
    let doc = support::open_spec("ex_rect20.ipt");
    let loc = doc.segment_location("PmDCSegment").unwrap();
    let mut seg = doc.decode_segment(&loc).unwrap();
    seg.data.compression = Compression::Zlib;
    let (m, b) = seg.encode().unwrap();
    let err = Segment::decode(&m, &b, false, true).unwrap_err();
    assert_eq!(err.field, "method");
    assert!(
        matches!(err.class, FailureClass::InvalidValue { .. }),
        "{err}"
    );
}
