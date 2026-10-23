//! Project files (spec clause 19) against the reference implementation's project-file reader:
//! the synthetic project files and their variants in `tests/fixtures/ipj` (written with the
//! reference's reading by the reference implementation's export tooling), and Inventor 2027's own
//! Default.ipj, read in place when installed and never copied into the repository.

mod support;

use std::path::{Path, PathBuf};

use corliss::ipj::{Layout, PathKind, Project, ProjectFile, ProjectType, resolve_path};
use serde_json::{Value, json};

const DEFAULT_IPJ: &str = r"C:\Users\Public\Documents\Autodesk\Inventor 2027\Default.ipj";

fn ipj_dir() -> PathBuf {
    support::fixtures_dir().join("ipj")
}

fn oracle() -> serde_json::Map<String, Value> {
    let text = std::fs::read_to_string(ipj_dir().join("ipj.json")).unwrap();
    serde_json::from_str::<Value>(&text)
        .unwrap()
        .as_object()
        .unwrap()
        .clone()
}

fn layout_name(l: Layout) -> &'static str {
    match l {
        Layout::Inventor => "inventor",
        Layout::Default => "default",
    }
}

/// The typed view in the reference reader's shape.
fn typed_json(p: &Project) -> Value {
    let libs = |l: &[corliss::ipj::AssetLibrary]| {
        l.iter()
            .map(|x| json!([x.path, x.library_type, x.active]))
            .collect::<Vec<_>>()
    };
    let folder = |f: &corliss::ipj::FolderOverride| json!({"path": f.path, "vault_path": f.vault_path, "use_styles": f.use_styles, "config_xml": f.config_xml});
    let o = &p.options;
    json!({
        "schemarevid": p.schema_revision,
        "contentcenterguid": p.content_center_guid.map(|g| g.to_string()),
        "addin_configs": p.addin_configs, "project_type": o.project_type_text,
        "included_project": o.included_project, "old_versions_to_keep": o.old_versions_to_keep,
        "using_unique_filenames": o.using_unique_filenames, "owner": o.owner, "release_id": o.release_id,
        "imported_comp_folder": o.imported_components_folder, "imported_asm_folder": o.imported_assemblies_folder,
        "appearance_libraries": libs(&p.appearance_libraries), "physical_libraries": libs(&p.material_libraries),
        "paths": p.paths.iter().map(|x| json!([x.path_type, x.name, x.path, x.vault_path])).collect::<Vec<_>>(),
        "design_data": folder(&p.design_data), "templates": folder(&p.templates), "presets": folder(&p.presets),
        "content_center": folder(&p.content_center), "vault_server": p.vault.server,
        "vault_database": p.vault.database, "vault_virtual_folder": p.vault.virtual_folder,
        "publish_folder": p.vault.publish_folder, "custom_sections": p.custom_sections,
    })
}

#[test]
fn every_fixture_reads_as_the_reference_reads_it() {
    let oracle = oracle();
    assert_eq!(oracle.len(), 8);
    for (name, want) in &oracle {
        let bytes = std::fs::read(ipj_dir().join(name)).unwrap();
        let file = ProjectFile::parse(&bytes).unwrap();
        let project = file.project().unwrap();
        assert_eq!(
            file.to_bytes() == bytes,
            want["lossless"].as_bool().unwrap(),
            "{name} lossless"
        );
        assert_eq!(layout_name(file.layout()), want["layout"], "{name} layout");
        assert_eq!(
            project.to_ipj(file.layout()) == bytes,
            want["regenerates"].as_bool().unwrap(),
            "{name} regenerates"
        );
        assert_eq!(json!(file.check()), want["findings"], "{name} findings");
        assert_eq!(typed_json(&project), want["project"], "{name} typed view");
    }
}

#[test]
fn every_fixture_renders_in_both_layouts_as_the_reference_does() {
    for name in oracle().keys() {
        let project = ProjectFile::open(ipj_dir().join(name))
            .unwrap()
            .project()
            .unwrap();
        for layout in [Layout::Inventor, Layout::Default] {
            let stem = name.trim_end_matches(".ipj");
            let want = std::fs::read(
                ipj_dir()
                    .join("rendered")
                    .join(format!("{stem}.{}.ipj", layout_name(layout))),
            )
            .unwrap();
            assert!(
                project.to_ipj(layout) == want,
                "{name} in the {} layout",
                layout_name(layout)
            );
        }
    }
}

#[test]
fn typed_accessors_follow_the_spec_rules() {
    let p = ProjectFile::open(ipj_dir().join("syn_workspace_libraries.ipj"))
        .unwrap()
        .project()
        .unwrap();
    let kinds: Vec<Option<PathKind>> = p.paths.iter().map(|x| x.kind()).collect();
    let (ws, lib, fav) = (
        Some(PathKind::Workspace),
        Some(PathKind::Library),
        Some(PathKind::FrequentlyUsedFolder),
    );
    assert_eq!(kinds, [ws, lib, lib, fav, fav]);
    assert_eq!(p.paths_of(PathKind::Library).count(), 2);
    let v = ProjectFile::open(ipj_dir().join("var_invalid.ipj"))
        .unwrap()
        .project()
        .unwrap();
    assert_eq!(
        v.options.project_type(),
        ProjectType::Vault,
        "R-13: 'vault' compares case-insensitively"
    );
    assert_eq!(
        v.paths.last().and_then(|x| x.kind()),
        None,
        "pathtype compares case-sensitively"
    );
}

#[test]
fn an_edit_changes_only_the_edited_character_data() {
    let bytes = std::fs::read(ipj_dir().join("syn_vault_included_custom.ipj")).unwrap();
    let mut file = ProjectFile::parse(&bytes).unwrap();
    let options = file.root.child_mut("ProjectOptions").unwrap();
    options
        .child_mut("UsingUniqueFilenames")
        .unwrap()
        .set_text("Yes");
    let utf16 = |s: &str| {
        s.encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect::<Vec<u8>>()
    };
    let want = replace(&bytes, &utf16(">No</Using"), &utf16(">Yes</Using"));
    assert_eq!(file.to_bytes(), want);
    assert!(
        ProjectFile::parse(&want)
            .unwrap()
            .project()
            .unwrap()
            .options
            .using_unique_filenames
    );
}

fn replace(hay: &[u8], from: &[u8], to: &[u8]) -> Vec<u8> {
    let at = hay.windows(from.len()).position(|w| w == from).unwrap();
    [&hay[..at], to, &hay[at + from.len()..]].concat()
}

#[test]
fn a_new_project_in_inventors_layout_conforms() {
    let mut p = Project::default();
    p.paths.push(corliss::ipj::ProjectPath {
        path_type: "Workspace".into(),
        name: "Workspace".into(),
        path: ".".into(),
        vault_path: None,
    });
    let bytes = p.to_ipj(Layout::Inventor);
    assert_eq!(&bytes[..2], &[0xFF, 0xFE], "F-10: UTF-16LE with a BOM");
    let file = ProjectFile::parse(&bytes).unwrap();
    assert_eq!(file.check(), Vec::<String>::new());
    assert_eq!(file.project().unwrap().workspace(), Some("."));
    assert_eq!(
        file.root.attribute("schemarevid").as_deref(),
        Some("14"),
        "W-16"
    );
}

#[test]
fn guid_names_round_trip() {
    let name = "mresgphncrxblhb6eku2x43fnh"; // Default.ipj's contentcenterguid, spec [C-186]
    let g = corliss::naming::guid_from_name(name).unwrap();
    assert_eq!(g.to_string(), "d91248cf-46e1-4ed1-8afc-705685f7bbe9");
    assert_eq!(corliss::naming::segment_stream_id(&g), name);
    assert_eq!(
        corliss::naming::guid_from_name("Aaaaaaaaaaaaaaaaaaaaaaaaaa"),
        None
    );
    assert_eq!(corliss::naming::guid_from_name("aaaa"), None);
}

#[test]
fn relative_paths_resolve_against_the_project_folder() {
    let dir = r"C:\Work\Design";
    assert_eq!(resolve_path(dir, "."), r"C:\Work\Design");
    assert_eq!(resolve_path(dir, r".\Designs\"), r"C:\Work\Design\Designs\");
    assert_eq!(resolve_path(dir, r"..\Company.ipj"), r"C:\Work\Company.ipj");
    assert_eq!(
        resolve_path(dir, r"..\..\..\x"),
        r"C:\x",
        "'..' stops at the drive root"
    );
    assert_eq!(resolve_path(dir, "Lib/Std"), r"C:\Work\Design\Lib\Std");
    assert_eq!(resolve_path(dir, r"\Shared"), r"C:\Shared");
    assert_eq!(resolve_path(r"\\srv\proj\a", r"..\..\b"), r"\\srv\proj\b");
    assert_eq!(resolve_path(dir, r"E:\Lib\.\x\..\y"), r"E:\Lib\y");
}

#[test]
fn malformed_files_are_refused_with_their_offending_value() {
    let be: Vec<u8> = [0xFE, 0xFF]
        .into_iter()
        .chain("<a/>".encode_utf16().flat_map(u16::to_be_bytes))
        .collect();
    assert!(
        ProjectFile::parse(&be)
            .unwrap_err()
            .to_string()
            .contains("UTF-16BE")
    );
    let utf16 = |s: &str| {
        [0xFF, 0xFE]
            .into_iter()
            .chain(s.encode_utf16().flat_map(u16::to_le_bytes))
            .collect::<Vec<u8>>()
    };
    let err = ProjectFile::parse(&utf16("<InventorProject><A></B></InventorProject>")).unwrap_err();
    assert!(err.to_string().contains("</A>"), "{err}");
    assert!(
        ProjectFile::parse(&[0xFF, 0xFE, b'<']).is_err(),
        "odd UTF-16 length"
    );
    let bad_guid =
        ProjectFile::parse(&utf16("<InventorProject contentcenterguid=\"xyz\"/>")).unwrap();
    assert!(bad_guid.project().unwrap_err().to_string().contains("xyz"));
}

#[test]
fn utf8_files_from_other_tools_round_trip() {
    let text = "\u{feff}<?xml version=\"1.0\" encoding=\"utf-8\"?>\r\n<InventorProject schemarevid='14'>\r\n  <ProjectOptions><Owner>A &amp; B</Owner></ProjectOptions>\r\n</InventorProject>";
    let file = ProjectFile::parse(text.as_bytes()).unwrap();
    assert_eq!(file.to_bytes(), text.as_bytes());
    assert_eq!(file.project().unwrap().options.owner, "A & B");
    assert_eq!(file.check()[0], "encoding: not UTF-16LE with BOM");
}

/// Inventor 2027's shipped Default.ipj, when installed (spec F.1 project set).
fn installed_default() -> Option<Vec<u8>> {
    Path::new(DEFAULT_IPJ)
        .exists()
        .then(|| std::fs::read(DEFAULT_IPJ).unwrap())
}

#[test]
fn the_installed_default_ipj_round_trips_and_regenerates() {
    let Some(bytes) = installed_default() else {
        return;
    };
    let file = ProjectFile::parse(&bytes).unwrap();
    assert_eq!(file.to_bytes(), bytes);
    assert_eq!(file.layout(), Layout::Default);
    assert_eq!(file.check(), Vec::<String>::new());
    let p = file.project().unwrap();
    assert!(
        p.to_ipj(Layout::Default) == bytes,
        "regenerated from the typed view"
    );
    assert_eq!(p.schema_revision, Some(14));
    assert_eq!(p.options.project_type(), ProjectType::SingleUser);
    assert!(p.paths.is_empty());
    assert_eq!(p.design_data.use_styles.as_deref(), Some("Read Only"));
    let flags = |l: &[corliss::ipj::AssetLibrary]| {
        l.iter()
            .map(|x| (x.library_type, x.active))
            .collect::<Vec<_>>()
    };
    assert_eq!(
        flags(&p.appearance_libraries),
        [(1, false), (2, false), (3, true)]
    );
    assert_eq!(flags(&p.material_libraries), [(2, false), (3, true)]);
}

#[test]
fn the_installed_default_ipj_survives_other_free_layouts() {
    let Some(bytes) = installed_default() else {
        return;
    };
    let text = String::from_utf16(
        &bytes[2..]
            .chunks(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect::<Vec<_>>(),
    )
    .unwrap();
    let crlf = text.replace('\n', "\r\n").replace("<Owner/>", "<Owner />");
    let b: Vec<u8> = [0xFF, 0xFE]
        .into_iter()
        .chain(crlf.encode_utf16().flat_map(u16::to_le_bytes))
        .collect();
    let file = ProjectFile::parse(&b).unwrap();
    assert_eq!(file.to_bytes(), b);
    assert_eq!(file.project().unwrap().options.owner, "");
}

/// The shipped Default.ipj's layout (spec 19.2, Table 19-1): 9,466 bytes, a BOM, the plain
/// declaration, LF only, and no final line break.
#[test]
fn the_installed_default_ipj_has_the_default_layout() {
    let Some(bytes) = installed_default() else {
        return;
    };
    assert_eq!(bytes.len(), 9466);
    let units: Vec<u16> = bytes[2..]
        .chunks(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .collect();
    let text = String::from_utf16(&units).unwrap();
    assert_eq!(&bytes[..2], &[0xFF, 0xFE]);
    assert!(text.starts_with("<?xml version=\"1.0\" encoding=\"utf-16\"?>\n"));
    assert!(!text.contains('\r'), "LF only");
    assert!(!text.ends_with('\n'), "no final line break");
}
