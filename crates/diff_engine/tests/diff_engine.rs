use bitwig_core::{AssetIndex, ClipIndex, DeviceIndex, SemanticIndex, TrackIndex};
use diff_engine::{detect_conflicts, diff_indexes};

fn mk_track(id: &str, name: &str) -> TrackIndex {
    TrackIndex {
        id: id.to_string(),
        name: name.to_string(),
        track_type: "track".to_string(),
        clips: vec![ClipIndex {
            id: format!("{id}-clip"),
            name: Some("clip".to_string()),
            start: 0.0,
            length: 4.0,
            fingerprint: format!("fp-{id}-c"),
        }],
        devices: vec![DeviceIndex {
            id: format!("{id}-dev"),
            name: "EQ".to_string(),
            device_type: "device".to_string(),
            fingerprint: format!("fp-{id}-d"),
        }],
        fingerprint: format!("fp-{id}"),
    }
}

fn mk_index(snapshot: &str, tracks: Vec<TrackIndex>) -> SemanticIndex {
    SemanticIndex {
        snapshot_id: snapshot.to_string(),
        project_name: "Demo".to_string(),
        tempo: Some(120.0),
        time_signature: Some("4/4".to_string()),
        tracks,
        assets: vec![AssetIndex {
            path: "samples/kick.wav".to_string(),
            hash: "hash1".to_string(),
            asset_type: "audiofile".to_string(),
        }],
        fingerprint: format!("fp-{snapshot}"),
    }
}

#[test]
fn base_none_marks_all_tracks_added() {
    let next = mk_index("s2", vec![mk_track("a", "A"), mk_track("b", "B"), mk_track("c", "C")]);
    let diff = diff_indexes(None, &next);

    let added_tracks = diff
        .changes
        .iter()
        .filter(|c| matches!(c.change_type, bitwig_core::ChangeType::TrackAdded))
        .count();
    assert_eq!(added_tracks, 3);
}

#[test]
fn removed_track_detected() {
    let base = mk_index("s1", vec![mk_track("a", "A")]);
    let next = mk_index("s2", vec![]);
    let diff = diff_indexes(Some(&base), &next);

    assert!(diff
        .changes
        .iter()
        .any(|c| matches!(c.change_type, bitwig_core::ChangeType::TrackRemoved) && c.entity_id == "a"));
}

#[test]
fn local_rename_remote_delete_is_conflict() {
    let base = mk_index("base", vec![mk_track("a", "A")]);

    let mut local_track = mk_track("a", "A-Renamed");
    local_track.fingerprint = "fp-local-rename".to_string();
    let local = mk_index("local", vec![local_track]);

    let remote = mk_index("remote", vec![]);

    let conflicts = detect_conflicts(&base, &local, &remote);
    assert_eq!(conflicts.len(), 1);
}
