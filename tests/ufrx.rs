//! UFRxDoc (spec clause 10): exact round trip, references, occurrences and relinking.

mod support;

use std::collections::HashMap;

use corliss::{Document, DocumentKind, Guid};

#[test]
fn every_fixture_record_round_trips_exactly() {
    for path in support::all_fixtures() {
        let doc = Document::open(&path).unwrap();
        let record = doc
            .ufrx_doc()
            .unwrap_or_else(|e| panic!("{}: {e}", support::label(&path)));
        assert_eq!(
            record.encode(),
            doc.stream("UFRxDoc").unwrap(),
            "{}",
            support::label(&path)
        );
    }
}

fn identities() -> HashMap<String, Guid> {
    let mut out = HashMap::new();
    for path in support::all_fixtures() {
        let doc = Document::open(&path).unwrap();
        let name = support::label(&path).to_lowercase();
        out.insert(name, doc.ufrx_doc().unwrap().internal_name().unwrap());
    }
    out
}

#[test]
fn references_name_the_referenced_documents_identity() {
    let known = identities();
    let mut checked = 0;
    for path in support::all_fixtures() {
        let doc = Document::open(&path).unwrap();
        for file in doc.ufrx_doc().unwrap().references() {
            let name = file
                .path
                .rsplit(['\\', '/'])
                .next()
                .unwrap_or_default()
                .to_lowercase();
            if let Some(identity) = known.get(&name) {
                assert_eq!(
                    file.internal_name,
                    *identity,
                    "{} -> {name}",
                    support::label(&path)
                );
                checked += 1;
            }
        }
    }
    assert!(
        checked > 30,
        "only {checked} references resolved to fixtures"
    );
}

#[test]
fn file_occurrence_counts_match_the_occurrences() {
    for path in support::all_fixtures() {
        let doc = Document::open(&path).unwrap();
        if doc.kind() != Some(DocumentKind::Assembly) {
            continue;
        }
        let record = doc.ufrx_doc().unwrap();
        let occurrences = record.occurrences();
        for file in record.references() {
            let users = occurrences
                .iter()
                .filter(|o| o.file_ref_id == file.ref_id)
                .count();
            assert_eq!(
                file.n_occurrences as usize,
                users,
                "{} {}",
                support::label(&path),
                file.path
            );
        }
    }
}

#[test]
fn spent_occurrence_ids_differ_from_ordinals() {
    // Spec 10.7: as_place3 holds (type 0, file 1, id 4, ordinal 3, suffix 3).
    let record = support::open_spec("as_place3.iam").ufrx_doc().unwrap();
    let third = record
        .occurrences()
        .into_iter()
        .find(|o| o.order == 3)
        .unwrap();
    assert_eq!(
        (
            third.instance_type,
            third.file_ref_id,
            third.id,
            third.name_suffix
        ),
        (0, 1, 4, 3)
    );
}

#[test]
fn relinked_reference_is_the_only_change() {
    let mut doc = support::open_spec("as_mate.iam");
    let original = doc.ufrx_doc().unwrap();
    let old = original.references()[0].path.clone();
    let mut record = original.clone();
    assert!(record.set_reference_path(&old.to_uppercase(), r"D:\moved\ex_rect20.ipt", None) > 0);
    doc.write_ufrx_doc(&record).unwrap();

    let reread = Document::from_bytes(&doc.to_bytes().unwrap())
        .unwrap()
        .ufrx_doc()
        .unwrap();
    let (before, after) = (original.references(), reread.references());
    assert_eq!(after[0].path, r"D:\moved\ex_rect20.ipt");
    assert_eq!(
        (after[0].internal_name, after[0].ref_id),
        (before[0].internal_name, before[0].ref_id)
    );
    assert_eq!(reread.occurrences(), original.occurrences());
}
