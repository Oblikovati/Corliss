//! Prints the structure of an Inventor document: type, save version, segments and references.
//!
//! ```text
//! cargo run --example inspect -- path/to/part.ipt
//! ```

use corliss::{Document, Result};

fn main() {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: inspect <document.ipt|iam|idw|ipn|ide>");
        std::process::exit(2);
    };
    if let Err(err) = inspect(&path) {
        eprintln!("{path}: {err}");
        std::process::exit(1);
    }
}

fn inspect(path: &str) -> Result<()> {
    let doc = Document::open(path)?;
    let db = doc.rsedb()?;
    println!(
        "{path}: {:?}, saved with Inventor {:?}",
        doc.kind(),
        db.saved_with.release_year()
    );
    println!("property sets: {}", doc.property_set_paths().len());
    print_segments(&doc)?;
    print_references(&doc)
}

fn print_segments(doc: &Document) -> Result<()> {
    for loc in doc.segments()? {
        let seg = doc.decode_segment(&loc)?;
        let name = if loc.template {
            "(template)"
        } else {
            loc.name.as_str()
        };
        let types = seg.meta.body.types.len();
        println!(
            "  {name:<24} {:>6} blocks {types:>4} classes",
            seg.blocks().len()
        );
    }
    Ok(())
}

fn print_references(doc: &Document) -> Result<()> {
    for file in doc.ufrx_doc()?.references() {
        println!(
            "  references {} ({} occurrences)",
            file.path, file.n_occurrences
        );
    }
    Ok(())
}
