//! Reader conformance (R1, R2) over every copied fixture: each layer decodes and re-encodes to
//! the exact bytes Inventor wrote, and the cross-layer invariants of spec clause 5.1 hold.

mod support;

use std::path::Path;

use corliss::database::{RevisionInfo, RseDb, SegInfo};
use corliss::naming::segment_stream_id;
use corliss::propset::PropertySetStream;
use corliss::{Document, DocumentKind};

fn for_each_fixture(check: impl Fn(&Path, &Document)) {
    let fixtures = support::all_fixtures();
    assert!(
        fixtures.len() > 250,
        "fixture corpus is missing: {} files",
        fixtures.len()
    );
    for path in fixtures {
        let doc =
            Document::open(&path).unwrap_or_else(|e| panic!("{}: {e}", support::label(&path)));
        check(&path, &doc);
    }
}

#[test]
fn root_clsid_matches_extension() {
    for_each_fixture(|path, doc| {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or_default();
        assert_eq!(
            doc.kind().map(DocumentKind::extension),
            Some(ext),
            "{}",
            support::label(path)
        );
    });
}

#[test]
fn database_streams_round_trip_exactly() {
    for_each_fixture(|path, doc| {
        let name = support::label(path);
        let rsedb_bytes = doc.stream(&doc.rsedb_path().unwrap()).unwrap();
        let rsedb = RseDb::decode(rsedb_bytes).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(rsedb.encode(), rsedb_bytes, "{name}: RSeDb");
        let seg_bytes = doc.stream("RSeStorage/RSeSegInfo").unwrap();
        let seg_info =
            SegInfo::decode(seg_bytes, rsedb.schema).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(seg_info.encode(), seg_bytes, "{name}: RSeSegInfo");
        let rev_bytes = doc.stream("RSeStorage/RSeDbRevisionInfo").unwrap();
        let rev_info = RevisionInfo::decode(rev_bytes).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(rev_info.encode(), rev_bytes, "{name}: RSeDbRevisionInfo");
    });
}

#[test]
fn property_sets_round_trip_exactly() {
    for_each_fixture(|path, doc| {
        for (stream_path, fmtid) in doc.property_set_paths() {
            let bytes = doc.stream(&stream_path).unwrap();
            let stream = PropertySetStream::decode(bytes)
                .unwrap_or_else(|e| panic!("{}: {fmtid}: {e}", support::label(path)));
            assert_eq!(stream.sets.first().map(|s| s.fmtid), Some(fmtid));
            assert_eq!(stream.encode(), bytes);
        }
    });
}

#[test]
fn every_segment_walks_and_round_trips_exactly() {
    for_each_fixture(|path, doc| {
        for loc in doc.segments().unwrap() {
            let name = format!("{} {}{}", support::label(path), loc.name, loc.stream_id);
            let seg = doc
                .decode_segment(&loc)
                .unwrap_or_else(|e| panic!("{name}: {e}"));
            assert_eq!(seg.blocks().len(), seg.meta.body.present_blocks(), "{name}");
            let (m, b) = seg.encode().unwrap();
            assert_eq!(m, doc.stream(&loc.m_path()).unwrap(), "{name}: M");
            assert_eq!(b, doc.stream(&loc.b_path()).unwrap(), "{name}: B");
        }
    });
}

#[test]
fn decoded_bodies_re_encode_to_the_inflated_bytes() {
    // Guards the structural encoders themselves: the exact-bytes path above would hide a
    // body encoder bug behind the kept original compressed data.
    for_each_fixture(|path, doc| {
        for loc in doc.segments().unwrap() {
            let name = format!("{} {}", support::label(path), loc.name);
            let seg = doc.decode_segment(&loc).unwrap();
            let body = seg.meta.body.encode();
            let reparsed = corliss::segment::MBody::decode(&body, loc.template).unwrap();
            assert_eq!(reparsed, seg.meta.body, "{name}");
            let data = seg.data.data.encode();
            let rewalked =
                corliss::segment::SegmentData::decode(&data, &seg.meta.body.blocks, true).unwrap();
            assert_eq!(rewalked, seg.data.data, "{name}");
        }
    });
}

#[test]
fn segment_directory_agrees_with_m_streams() {
    // [F-02] stream id = GetName(seg_id), [F-04] name and id agree, [F-05] counts agree.
    for_each_fixture(|path, doc| {
        let seg_info = doc.seg_info().unwrap();
        for entry in &seg_info.segments {
            let name = format!("{} {}", support::label(path), entry.name);
            let loc = doc
                .segment_location(&entry.name)
                .unwrap_or_else(|e| panic!("{name}: {e}"));
            assert_eq!(
                loc.stream_id,
                segment_stream_id(&entry.seg_id),
                "{name}: F-02"
            );
            let seg = doc.decode_segment(&loc).unwrap();
            let identity = seg.meta.header.identity.as_ref().unwrap();
            assert_eq!(identity.seg_id, entry.seg_id, "{name}: F-04");
            assert_eq!(
                entry.n_blocks as usize,
                seg.meta.body.present_blocks(),
                "{name}: F-05 n_blocks"
            );
            assert_eq!(
                entry.n_roots as usize,
                seg.meta.body.tail.t9.len() + 1,
                "{name}: F-05 n_roots"
            );
            assert_eq!(
                seg.meta.header.compression, seg.data.compression,
                "{name}: F-06"
            );
        }
    });
}

#[test]
fn template_streams_are_named_by_their_template_guid() {
    // [F-09]: node 1 of a template segment holds the template GUID after `next`,
    // `object_flags` and `object_flags_high` (spec 11.6.3).
    for_each_fixture(|path, doc| {
        for loc in doc.segments().unwrap().into_iter().filter(|l| l.template) {
            let seg = doc.decode_segment(&loc).unwrap();
            let payload = &seg.blocks()[0].payload;
            let guid = corliss::Guid::from_bytes(payload[12..28].try_into().unwrap());
            assert_eq!(
                segment_stream_id(&guid),
                loc.stream_id,
                "{}",
                support::label(path)
            );
        }
    });
}
