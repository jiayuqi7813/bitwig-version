use std::fs;
use std::io::Write;

use dawproject::{build_semantic_index, inspect_dawproject};
use tempfile::tempdir;
use zip::write::FileOptions;

fn create_minimal_dawproject(path: &std::path::Path) {
    let project_xml = r#"<Project name=\"Demo Song\" tempo=\"120\" timeSignature=\"4/4\"><Track id=\"t1\" name=\"Drums\"><Clip id=\"c1\" name=\"Beat\" start=\"0\" length=\"4\"/><Device id=\"d1\" name=\"EQ\"/></Track><AudioFile path=\"samples/kick.wav\"/></Project>"#;
    let metadata_xml = r#"<Meta><Application name=\"Bitwig Studio\" version=\"5.2\"/><Project name=\"Demo Song\"/></Meta>"#;

    let file = std::fs::File::create(path).expect("create dawproject");
    let mut zip = zip::ZipWriter::new(file);

    zip.start_file::<_, ()>("project.xml", FileOptions::default())
        .expect("start project.xml");
    zip.write_all(project_xml.as_bytes())
        .expect("write project.xml");

    zip.start_file::<_, ()>("metadata.xml", FileOptions::default())
        .expect("start metadata.xml");
    zip.write_all(metadata_xml.as_bytes())
        .expect("write metadata.xml");

    zip.finish().expect("finish zip");
}

#[test]
fn inspect_dawproject_reads_project_name() {
    let dir = tempdir().expect("tempdir");
    let daw = dir.path().join("minimal.dawproject");
    create_minimal_dawproject(&daw);

    let info = inspect_dawproject(&daw).expect("inspect");
    assert_eq!(info.project_name, "Demo Song");
}

#[test]
fn build_semantic_index_tracks_not_empty_for_track_xml() {
    let dir = tempdir().expect("tempdir");
    let daw = dir.path().join("minimal.dawproject");
    create_minimal_dawproject(&daw);

    let index = build_semantic_index(&daw, "snap-1").expect("index");

    assert!(!index.tracks.is_empty());
}

#[test]
fn invalid_zip_returns_error_instead_of_panic() {
    let dir = tempdir().expect("tempdir");
    let invalid = dir.path().join("broken.dawproject");
    fs::write(&invalid, "not a zip").expect("write");

    let result = inspect_dawproject(&invalid);
    assert!(result.is_err());
}
