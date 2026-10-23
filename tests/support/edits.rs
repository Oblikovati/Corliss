//! The reference edit outputs (`tests/fixtures/edits`, written by the reference
//! implementation's export tooling) and a stream-by-stream comparison that ignores only
//! compression.

use corliss::container::ContainerEntry;
use corliss::edit::{Editor, GuidSource};
use corliss::segment::Segment;
use corliss::{Document, Guid};

/// The GUIDs job `name` drew from uuid4, in order.
pub fn guids(name: &str) -> Vec<Guid> {
    let path = super::fixtures_dir().join("edits").join("jobs.json");
    let text = std::fs::read_to_string(path).expect("jobs.json");
    let jobs: serde_json::Value = serde_json::from_str(&text).expect("jobs.json parses");
    let job = jobs
        .as_array()
        .unwrap()
        .iter()
        .find(|j| j["job"] == name)
        .expect("job listed");
    job["guids"]
        .as_array()
        .unwrap()
        .iter()
        .map(|h| {
            let hex = h.as_str().unwrap();
            let bytes: Vec<u8> = (0..16)
                .map(|k| u8::from_str_radix(&hex[2 * k..2 * k + 2], 16).unwrap())
                .collect();
            Guid::from_bytes(bytes.try_into().unwrap())
        })
        .collect()
}

/// An editor on the job's source fixture, drawing the job's GUIDs.
pub fn editor(name: &str, source: &str) -> Editor {
    let doc = super::open_label(source);
    Editor::new(&doc)
        .expect("editor opens")
        .with_guids(GuidSource::Fixed(guids(name)))
}

/// The reference output of job `name`.
pub fn expected(name: &str) -> Document {
    Document::open(super::fixtures_dir().join("edits").join(name)).expect("reference output opens")
}

/// The first difference between two documents' streams, comparing segments decoded.
pub fn diff(got: &Document, want: &Document) -> Option<String> {
    let (g, w) = (got.container().entries(), want.container().entries());
    if let Some(p) = w.keys().find(|k| !g.contains_key(*k)) {
        return Some(format!("{p}: missing"));
    }
    for (path, entry) in w {
        let (
            ContainerEntry::Stream {
                data: want_bytes, ..
            },
            Some(ContainerEntry::Stream {
                data: got_bytes, ..
            }),
        ) = (entry, g.get(path))
        else {
            continue;
        };
        if got_bytes == want_bytes {
            continue;
        }
        if let Some(d) = segment_diff(got, want, path) {
            return Some(d);
        }
        if !is_segment_stream(path) {
            return Some(format!(
                "{path}: {} bytes != {} bytes (or content)",
                got_bytes.len(),
                want_bytes.len()
            ));
        }
    }
    None
}

fn is_segment_stream(path: &str) -> bool {
    path.split('/')
        .next_back()
        .is_some_and(|n| (n.starts_with('M') || n.starts_with('B')) && n.len() == 27)
}

/// Compares the decoded M body / header and B data of the segment owning stream `path`.
fn segment_diff(got: &Document, want: &Document, path: &str) -> Option<String> {
    if !is_segment_stream(path) {
        return None;
    }
    let loc = want
        .segments()
        .ok()?
        .into_iter()
        .find(|l| l.m_path() == path || l.b_path() == path)?;
    let decode = |d: &Document| {
        Segment::decode(
            d.stream(&loc.m_path()).ok()?,
            d.stream(&loc.b_path()).ok()?,
            loc.template,
            true,
        )
        .ok()
    };
    let (g, w) = (decode(got)?, decode(want)?);
    if g.meta.header != w.meta.header {
        return Some(format!(
            "{} M header: {:?} != {:?}",
            loc.name, g.meta.header, w.meta.header
        ));
    }
    if g.meta.body != w.meta.body {
        return Some(format!(
            "{} M body differs: {}",
            loc.name,
            first_body_diff(&g, &w)
        ));
    }
    let (gb, wb) = (g.blocks(), w.blocks());
    if let Some(k) = (0..gb.len().max(wb.len())).find(|k| gb.get(*k) != wb.get(*k)) {
        return Some(format!(
            "{} block {}: {:?} != {:?}",
            loc.name,
            k + 1,
            gb.get(k).map(|b| (b.header, b.payload.len())),
            wb.get(k).map(|b| (b.header, b.payload.len()))
        ));
    }
    (g.data.data.end_table != w.data.data.end_table)
        .then(|| format!("{} end table differs", loc.name))
}

fn first_body_diff(g: &Segment, w: &Segment) -> String {
    let (a, b) = (&g.meta.body, &w.meta.body);
    let checks = [
        ("slot_count", a.slot_count == b.slot_count),
        ("end_count", a.end_count == b.end_count),
        ("n_roots", a.n_roots == b.n_roots),
        ("blocks", a.blocks == b.blocks),
        ("sec2", a.sec2 == b.sec2),
        ("types", a.types == b.types),
        ("t4", a.tail.t4 == b.tail.t4),
        ("t5", a.tail.t5 == b.tail.t5),
        ("t6", a.tail.t6 == b.tail.t6),
        ("t7", a.tail.t7 == b.tail.t7),
        ("t8", a.tail.t8 == b.tail.t8),
        ("t9", a.tail.t9 == b.tail.t9),
    ];
    checks
        .iter()
        .filter(|(_, ok)| !ok)
        .map(|(n, _)| *n)
        .collect::<Vec<_>>()
        .join(", ")
}
