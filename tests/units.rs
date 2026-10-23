//! Focused tests of the primitive decoders, using the examples the specification gives.

mod support;

use corliss::bytes::{ByteReader, ByteWriter};
use corliss::compression::Compression;
use corliss::database::{RevisionInfo, RseDb, VersionWords};
use corliss::naming::{property_set_fmtid, property_set_stream_name, segment_stream_id};
use corliss::node::{CommonHeader, MaskedMatrix4, NodeRef};
use corliss::segment::MTail;
use corliss::{FailureClass, Guid};

#[test]
fn guid_text_round_trips_and_rejects_other_shapes() {
    let text = "a42bc29e-11d4-01e8-6000-2db3ee29fbb0";
    let guid = Guid::parse(text).unwrap();
    assert_eq!(guid.to_string(), text);
    assert_eq!(guid.as_bytes()[0], 0x9e, "stored in Windows layout");
    assert_eq!(Guid::parse("a42bc29e11d401e860002db3ee29fbb0"), None);
    assert_eq!(Guid::parse("zz2bc29e-11d4-01e8-6000-2db3ee29fbb0"), None);
}

#[test]
fn template_stream_ids_match_the_spec_examples() {
    // Spec 11.6.1: ViewTemplate and AsmCHTemplate stream ids in every drawing.
    let view = Guid::parse("81b732f4-11d1-a3ce-6000-0c8a806bceb0").unwrap();
    assert_eq!(segment_stream_id(&view), "a2omxur33eorfayadaym313akk");
    let hatch = Guid::parse("e60e8138-11d2-3ecd-6000-1d9feb49cdb0").unwrap();
    assert_eq!(segment_stream_id(&hatch), "ta4ajy6m1eospr1adaym1splmk");
}

#[test]
fn every_fixture_property_set_name_decodes_and_re_encodes() {
    for path in support::all_fixtures().iter().step_by(5) {
        let doc = corliss::Document::open(path).unwrap();
        for (name, fmtid) in doc.property_set_paths() {
            if name.len() == 27 && !name.ends_with("SummaryInformation") {
                assert_eq!(
                    property_set_stream_name(&fmtid),
                    name.to_lowercase(),
                    "{}",
                    support::label(path)
                );
            }
            assert_eq!(
                property_set_fmtid(&property_set_stream_name(&fmtid)),
                Some(fmtid)
            );
        }
    }
}

#[test]
fn standard_property_set_names_map_to_microsoft_fmtids() {
    let doc_summary = property_set_fmtid("\u{5}DocumentSummaryInformation").unwrap();
    assert_eq!(
        doc_summary.to_string(),
        "d5cdd502-2e9c-101b-9397-08002b2cf9ae"
    );
    assert_eq!(property_set_fmtid("Protein"), None);
}

#[test]
fn masked_matrix_decodes_the_spec_sketch_transforms() {
    // Work Plane1 offset 10 mm: translation z = 1 cm is itself a mask bit, no doubles.
    let offset = [0x21, 0x8C, 0xDE, 0x73];
    let (m, used) = MaskedMatrix4::decode(&offset).unwrap();
    assert_eq!((used, m.translation()), (4, [0.0, 0.0, 1.0]));
    // Work Plane1 at 30° about X: four explicit doubles.
    let mut tilted = vec![0x01, 0x80, 0x9E, 0x79];
    for v in [0.866, -0.5, 0.5, 0.866f64] {
        tilted.extend_from_slice(&v.to_le_bytes());
    }
    let (m, used) = MaskedMatrix4::decode(&tilted).unwrap();
    assert_eq!(used, 36);
    assert_eq!(MaskedMatrix4::decode(&m.encode()).unwrap().0, m);
}

#[test]
fn masked_matrix_rounding_snaps_near_unit_cells() {
    let mut cells = MaskedMatrix4::IDENTITY.0;
    cells[0] = 1.000_000_000_000_000_2;
    cells[1] = -2.2e-16;
    assert_eq!(MaskedMatrix4(cells).rounded(), MaskedMatrix4::IDENTITY);
}

#[test]
fn masked_matrix_keeps_negative_zero_explicit() {
    let mut cells = MaskedMatrix4::IDENTITY.0;
    cells[3] = -0.0;
    let decoded = MaskedMatrix4::decode(&MaskedMatrix4(cells).encode())
        .unwrap()
        .0;
    assert!(decoded.0[3].is_sign_negative());
}

#[test]
fn node_refs_and_common_header() {
    assert_eq!(NodeRef(0x8000_0000).index(), None);
    assert_eq!(NodeRef(7).index(), Some(7));
    assert!(!NodeRef(7).is_flagged());
    let header = CommonHeader::read(&[0x02, 0, 0, 0, 0x34, 0x12]).unwrap();
    assert_eq!(header.object_id(), 0x2_1234);
}

#[test]
fn version_words_decode_the_2027_build() {
    // Spec 9.3.3: (0, 16415, 0, 15500) = R31 build 155.
    let v = VersionWords([0, 16415, 0, 15500]);
    assert_eq!((v.major_release(), v.build(), v.intent()), (31, 155, 0));
    let r6 = VersionWords([0, 32774, 6900, 1000]);
    assert_eq!((r6.major_release(), r6.build(), r6.intent()), (6, 10, 6));
}

#[test]
fn truncated_read_reports_offset_and_sizes() {
    let mut r = ByteReader::new(&[1, 2, 3], "test structure");
    r.u16("first").unwrap();
    let err = r.u32("second").unwrap_err();
    assert_eq!(
        (err.structure, err.field, err.offset),
        ("test structure", "second", Some(2))
    );
    assert_eq!(
        err.class,
        FailureClass::Truncated {
            needed: 4,
            available: 1
        }
    );
}

#[test]
fn oversized_count_is_rejected_before_allocation() {
    let err = ByteReader::new(&[0xFF, 0xFF, 0xFF, 0x7F], "list")
        .count("n", 16)
        .unwrap_err();
    assert!(
        matches!(err.class, FailureClass::InvalidValue { .. }),
        "{err}"
    );
}

#[test]
fn text16_round_trips_through_the_writer() {
    let mut w = ByteWriter::default();
    w.text16("Sheet:1 ∅").text8(b"abc");
    let bytes = w.into_bytes();
    let mut r = ByteReader::new(&bytes, "texts");
    assert_eq!(r.text16("a").unwrap(), "Sheet:1 ∅");
    assert_eq!(r.text8("b").unwrap(), b"abc");
    r.finish().unwrap();
}

#[test]
fn old_rsedb_schema_is_reported_unsupported() {
    let mut w = ByteWriter::default();
    w.guid(&Guid::NIL).u32(0x15);
    let err = RseDb::decode(&w.into_bytes()).unwrap_err();
    assert!(
        matches!(err.class, FailureClass::Unsupported { .. }),
        "{err}"
    );
}

#[test]
fn unknown_revision_layout_is_rejected() {
    let err = RevisionInfo::decode(&[9, 0, 0, 0, 0, 0, 0, 0]).unwrap_err();
    assert_eq!(err.field, "version");
}

#[test]
fn m_tail_without_closing_guid_is_rejected() {
    let err = MTail::decode(&[0u8; 40], false).unwrap_err();
    assert_eq!(err.field, "closing_guid");
}

#[test]
fn compression_is_sniffed_from_the_data() {
    assert_eq!(
        Compression::sniff(&[0x28, 0xB5, 0x2F, 0xFD, 0]),
        Some(Compression::Zstd)
    );
    assert_eq!(Compression::sniff(&[0x78, 0x9C]), Some(Compression::Zlib));
    assert_eq!(Compression::sniff(&[0]), None);
    assert_eq!(
        Compression::from_code(Compression::Zstd.code()),
        Some(Compression::Zstd)
    );
}

#[test]
fn compression_round_trips_both_methods() {
    let data: Vec<u8> = (0..10_000u32).map(|i| (i % 251) as u8).collect();
    for method in [Compression::Zlib, Compression::Zstd] {
        let packed = corliss::compression::compress(method, &data, "test").unwrap();
        assert_eq!(Compression::sniff(&packed), Some(method));
        assert_eq!(
            corliss::compression::decompress(method, &packed, "test").unwrap(),
            data
        );
    }
}

#[test]
fn documents_are_send_and_sync() {
    fn assert_thread_safe<T: Send + Sync>() {}
    assert_thread_safe::<corliss::Document>();
    assert_thread_safe::<corliss::segment::Segment>();
    assert_thread_safe::<corliss::propset::PropertySetStream>();
    assert_thread_safe::<corliss::CorlissError>();
}
