//! Relinking every referenced file of every fixture (spec 10): after moving each reference to a
//! new path and saving, the document reads back with the new paths and every other UFRxDoc
//! field unchanged.

mod support;

use corliss::Document;
use corliss::ufrx::UfrxDoc;

#[test]
fn every_reference_of_every_fixture_relinks_alone() {
    let mut relinked = 0;
    let mut failures = Vec::new();
    for path in support::all_fixtures() {
        let mut doc = Document::open(&path).unwrap();
        let original = doc.ufrx_doc().unwrap();
        if original.references().is_empty() {
            continue;
        }
        relinked += 1;
        if let Err(e) = relink_all(&mut doc, &original) {
            failures.push(format!("{}: {e}", support::label(&path)));
        }
    }
    assert!(relinked > 10, "{relinked} documents with references");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// Moves reference k to `D:\Relocated\k\<file name>`, saves, reads back and compares.
fn relink_all(doc: &mut Document, original: &UfrxDoc) -> Result<(), String> {
    let mut edited = original.clone();
    let olds: Vec<String> = original
        .references()
        .iter()
        .map(|r| r.path.clone())
        .collect();
    for (k, old) in olds.iter().enumerate() {
        let name = old.rsplit('\\').next().unwrap_or(old);
        edited.set_reference_path(old, &format!(r"D:\Relocated\{k}\{name}"), None);
    }
    doc.write_ufrx_doc(&edited).map_err(|e| e.to_string())?;
    let reread = Document::from_bytes(&doc.to_bytes().unwrap())
        .unwrap()
        .ufrx_doc()
        .unwrap();
    if reread != edited {
        return Err("the saved record differs from the edited one".into());
    }
    let (before, after) = (original.references(), reread.references());
    if after.iter().any(|r| !r.path.starts_with(r"D:\Relocated\")) {
        return Err("a reference kept its old path".into());
    }
    let unpathed = |refs: Vec<corliss::ufrx::FileReference>| {
        refs.into_iter()
            .map(|mut r| {
                r.path.clear();
                r
            })
            .collect::<Vec<_>>()
    };
    if unpathed(before) != unpathed(after) {
        return Err("a field other than the path changed".into());
    }
    Ok(())
}
