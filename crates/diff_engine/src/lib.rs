use std::collections::{HashMap, HashSet};

use bitwig_core::{
    AssetIndex, Change, ChangeType, ClipIndex, Conflict, DeviceIndex, ProjectDiff, SemanticIndex, TrackIndex,
};

pub fn diff_indexes(base: Option<&SemanticIndex>, next: &SemanticIndex) -> ProjectDiff {
    let mut changes = Vec::new();

    if let Some(base_idx) = base {
        if base_idx.tempo != next.tempo {
            changes.push(Change {
                change_type: ChangeType::TempoChanged,
                entity_id: "tempo".to_string(),
                entity_name: "Tempo".to_string(),
                description: format!(
                    "Tempo changed from {} to {} BPM",
                    base_idx
                        .tempo
                        .map(|t| t.to_string())
                        .unwrap_or_else(|| "unknown".to_string()),
                    next.tempo
                        .map(|t| t.to_string())
                        .unwrap_or_else(|| "unknown".to_string())
                ),
            });
        }

        diff_tracks(base_idx, next, &mut changes);
        diff_assets(base_idx, next, &mut changes);

        ProjectDiff {
            from_snapshot: Some(base_idx.snapshot_id.clone()),
            to_snapshot: next.snapshot_id.clone(),
            changes,
            has_conflicts: false,
        }
    } else {
        for track in &next.tracks {
            changes.push(Change {
                change_type: ChangeType::TrackAdded,
                entity_id: track.id.clone(),
                entity_name: track.name.clone(),
                description: format!("Track '{}' was added", track.name),
            });
        }
        for asset in &next.assets {
            changes.push(Change {
                change_type: ChangeType::AssetAdded,
                entity_id: asset.path.clone(),
                entity_name: asset.path.clone(),
                description: format!("Asset '{}' was added", asset.path),
            });
        }
        ProjectDiff {
            from_snapshot: None,
            to_snapshot: next.snapshot_id.clone(),
            changes,
            has_conflicts: false,
        }
    }
}

pub fn detect_conflicts(base: &SemanticIndex, local: &SemanticIndex, remote: &SemanticIndex) -> Vec<Conflict> {
    let mut conflicts = Vec::new();

    let base_tracks = to_track_map(&base.tracks);
    let local_tracks = to_track_map(&local.tracks);
    let remote_tracks = to_track_map(&remote.tracks);

    let all_keys: HashSet<String> = base_tracks
        .keys()
        .chain(local_tracks.keys())
        .chain(remote_tracks.keys())
        .cloned()
        .collect();

    for key in all_keys {
        let b = base_tracks.get(&key);
        let l = local_tracks.get(&key);
        let r = remote_tracks.get(&key);

        let local_changed = changed_track(b, l);
        let remote_changed = changed_track(b, r);

        if local_changed && remote_changed {
            let local_deleted = l.is_none();
            let remote_deleted = r.is_none();

            let is_conflict = (local_deleted && !remote_deleted)
                || (!local_deleted && remote_deleted)
                || (l.is_some() && r.is_some() && l.unwrap().fingerprint != r.unwrap().fingerprint);

            if is_conflict {
                conflicts.push(Conflict {
                    entity_id: key.clone(),
                    description: "Track changed differently in local and remote".to_string(),
                    local_value: l
                        .map(|t| format!("{} ({})", t.name, t.fingerprint))
                        .unwrap_or_else(|| "<deleted>".to_string()),
                    remote_value: r
                        .map(|t| format!("{} ({})", t.name, t.fingerprint))
                        .unwrap_or_else(|| "<deleted>".to_string()),
                });
            }
        }
    }

    conflicts
}

fn diff_tracks(base: &SemanticIndex, next: &SemanticIndex, changes: &mut Vec<Change>) {
    let mut used_next = HashSet::new();

    for base_track in &base.tracks {
        if let Some((idx, next_track)) = find_track(base_track, &next.tracks, &used_next) {
            used_next.insert(idx);
            if base_track.fingerprint != next_track.fingerprint {
                changes.push(Change {
                    change_type: ChangeType::TrackModified,
                    entity_id: next_track.id.clone(),
                    entity_name: next_track.name.clone(),
                    description: format!("Track '{}' was modified", next_track.name),
                });

                diff_clips(base_track, next_track, changes);
                diff_devices(base_track, next_track, changes);
            }
        } else {
            changes.push(Change {
                change_type: ChangeType::TrackRemoved,
                entity_id: base_track.id.clone(),
                entity_name: base_track.name.clone(),
                description: format!("Track '{}' was removed", base_track.name),
            });
        }
    }

    for (idx, track) in next.tracks.iter().enumerate() {
        if !used_next.contains(&idx) {
            changes.push(Change {
                change_type: ChangeType::TrackAdded,
                entity_id: track.id.clone(),
                entity_name: track.name.clone(),
                description: format!("Track '{}' was added", track.name),
            });
        }
    }
}

fn diff_clips(base_track: &TrackIndex, next_track: &TrackIndex, changes: &mut Vec<Change>) {
    let mut used_next = HashSet::new();
    for clip in &base_track.clips {
        if let Some((idx, next_clip)) = find_clip(clip, &next_track.clips, &used_next) {
            used_next.insert(idx);
            if clip.fingerprint != next_clip.fingerprint {
                changes.push(Change {
                    change_type: ChangeType::ClipModified,
                    entity_id: next_clip.id.clone(),
                    entity_name: next_clip
                        .name
                        .clone()
                        .unwrap_or_else(|| "Unnamed Clip".to_string()),
                    description: format!("Clip on track '{}' was modified", next_track.name),
                });
            }
        } else {
            changes.push(Change {
                change_type: ChangeType::ClipRemoved,
                entity_id: clip.id.clone(),
                entity_name: clip
                    .name
                    .clone()
                    .unwrap_or_else(|| "Unnamed Clip".to_string()),
                description: format!("Clip at beat {} on track '{}' was removed", clip.start, base_track.name),
            });
        }
    }

    for (idx, clip) in next_track.clips.iter().enumerate() {
        if !used_next.contains(&idx) {
            changes.push(Change {
                change_type: ChangeType::ClipAdded,
                entity_id: clip.id.clone(),
                entity_name: clip
                    .name
                    .clone()
                    .unwrap_or_else(|| "Unnamed Clip".to_string()),
                description: format!("Clip at beat {} on track '{}' was added", clip.start, next_track.name),
            });
        }
    }
}

fn diff_devices(base_track: &TrackIndex, next_track: &TrackIndex, changes: &mut Vec<Change>) {
    let mut used_next = HashSet::new();
    for device in &base_track.devices {
        if let Some((idx, next_device)) = find_device(device, &next_track.devices, &used_next) {
            used_next.insert(idx);
            if device.fingerprint != next_device.fingerprint {
                changes.push(Change {
                    change_type: ChangeType::DeviceStateChanged,
                    entity_id: next_device.id.clone(),
                    entity_name: next_device.name.clone(),
                    description: format!("Device '{}' on track '{}' changed", next_device.name, next_track.name),
                });
            }
        } else {
            changes.push(Change {
                change_type: ChangeType::DeviceRemoved,
                entity_id: device.id.clone(),
                entity_name: device.name.clone(),
                description: format!("Device '{}' on track '{}' was removed", device.name, base_track.name),
            });
        }
    }

    for (idx, device) in next_track.devices.iter().enumerate() {
        if !used_next.contains(&idx) {
            changes.push(Change {
                change_type: ChangeType::DeviceAdded,
                entity_id: device.id.clone(),
                entity_name: device.name.clone(),
                description: format!("Device '{}' on track '{}' was added", device.name, next_track.name),
            });
        }
    }
}

fn diff_assets(base: &SemanticIndex, next: &SemanticIndex, changes: &mut Vec<Change>) {
    let mut used_next = HashSet::new();
    for base_asset in &base.assets {
        if let Some((idx, next_asset)) = find_asset(base_asset, &next.assets, &used_next) {
            used_next.insert(idx);
            if base_asset.hash != next_asset.hash {
                changes.push(Change {
                    change_type: ChangeType::AssetChanged,
                    entity_id: next_asset.path.clone(),
                    entity_name: next_asset.path.clone(),
                    description: format!("Asset '{}' content changed", next_asset.path),
                });
            }
        } else {
            changes.push(Change {
                change_type: ChangeType::AssetRemoved,
                entity_id: base_asset.path.clone(),
                entity_name: base_asset.path.clone(),
                description: format!("Asset '{}' was removed", base_asset.path),
            });
        }
    }
    for (idx, asset) in next.assets.iter().enumerate() {
        if !used_next.contains(&idx) {
            changes.push(Change {
                change_type: ChangeType::AssetAdded,
                entity_id: asset.path.clone(),
                entity_name: asset.path.clone(),
                description: format!("Asset '{}' was added", asset.path),
            });
        }
    }
}

fn find_track<'a>(target: &TrackIndex, candidates: &'a [TrackIndex], used: &HashSet<usize>) -> Option<(usize, &'a TrackIndex)> {
    find_by_priority(candidates, used, |c| c.id == target.id)
        .or_else(|| find_by_priority(candidates, used, |c| c.fingerprint == target.fingerprint))
        .or_else(|| find_by_priority(candidates, used, |c| c.name == target.name))
}

fn find_clip<'a>(target: &ClipIndex, candidates: &'a [ClipIndex], used: &HashSet<usize>) -> Option<(usize, &'a ClipIndex)> {
    find_by_priority(candidates, used, |c| c.id == target.id)
        .or_else(|| find_by_priority(candidates, used, |c| c.fingerprint == target.fingerprint))
        .or_else(|| find_by_priority(candidates, used, |c| c.start == target.start && c.length == target.length))
}

fn find_device<'a>(target: &DeviceIndex, candidates: &'a [DeviceIndex], used: &HashSet<usize>) -> Option<(usize, &'a DeviceIndex)> {
    find_by_priority(candidates, used, |c| c.id == target.id)
        .or_else(|| find_by_priority(candidates, used, |c| c.fingerprint == target.fingerprint))
        .or_else(|| find_by_priority(candidates, used, |c| c.name == target.name))
}

fn find_asset<'a>(target: &AssetIndex, candidates: &'a [AssetIndex], used: &HashSet<usize>) -> Option<(usize, &'a AssetIndex)> {
    find_by_priority(candidates, used, |c| c.path == target.path)
        .or_else(|| find_by_priority(candidates, used, |c| c.hash == target.hash))
}

fn find_by_priority<'a, T, F>(candidates: &'a [T], used: &HashSet<usize>, pred: F) -> Option<(usize, &'a T)>
where
    F: Fn(&T) -> bool,
{
    candidates
        .iter()
        .enumerate()
        .find(|(idx, c)| !used.contains(idx) && pred(c))
}

fn to_track_map(tracks: &[TrackIndex]) -> HashMap<String, &TrackIndex> {
    tracks
        .iter()
        .map(|t| (if t.id.is_empty() { t.name.clone() } else { t.id.clone() }, t))
        .collect()
}

fn changed_track(base: Option<&&TrackIndex>, other: Option<&&TrackIndex>) -> bool {
    match (base, other) {
        (Some(b), Some(o)) => b.fingerprint != o.fingerprint,
        (Some(_), None) => true,
        (None, Some(_)) => true,
        (None, None) => false,
    }
}
