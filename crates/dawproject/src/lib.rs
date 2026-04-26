use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};

use bitwig_core::{AssetIndex, ClipIndex, DeviceIndex, SemanticIndex, TrackIndex};
use blake3::Hasher;
use roxmltree::Document;
use thiserror::Error;
use zip::ZipArchive;

pub type Result<T> = std::result::Result<T, DawprojectError>;

#[derive(Debug, Error)]
pub enum DawprojectError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("zip error: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("xml parse error: {0}")]
    Xml(#[from] roxmltree::Error),
    #[error("missing required file {0}")]
    MissingFile(String),
}

#[derive(Debug, Clone)]
pub struct DawprojectInfo {
    pub project_name: String,
    pub tempo: Option<f64>,
    pub time_signature: Option<String>,
    pub application_name: Option<String>,
    pub application_version: Option<String>,
}

#[derive(Debug, Clone)]
pub struct XmlTree {
    pub raw_xml: String,
}

pub fn inspect_dawproject(path: &Path) -> Result<DawprojectInfo> {
    let temp_dir = tempfile::tempdir()?;
    let extracted = extract_to_temp(path, temp_dir.path())?;
    let mut info = parse_metadata_xml(&extracted).unwrap_or(DawprojectInfo {
        project_name: path
            .file_stem()
            .and_then(|n| n.to_str())
            .unwrap_or("Unknown Project")
            .to_string(),
        tempo: None,
        time_signature: None,
        application_name: None,
        application_version: None,
    });

    if let Ok(project) = parse_project_xml(&extracted) {
        let doc = Document::parse(&project.raw_xml)?;
        let tempo = extract_tempo(&doc);
        let project_name = extract_project_name(&doc).unwrap_or_else(|| info.project_name.clone());
        info.project_name = project_name;
        info.tempo = info.tempo.or(tempo);
        info.time_signature = info.time_signature.or_else(|| extract_time_signature(&doc));
    }

    Ok(info)
}

pub fn extract_to_temp(path: &Path, temp_dir: &Path) -> Result<PathBuf> {
    let file = File::open(path)?;
    let mut archive = ZipArchive::new(file)?;
    let extracted_dir = temp_dir.join("extracted");
    fs::create_dir_all(&extracted_dir)?;

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let out_path = extracted_dir.join(entry.name());
        if entry.name().ends_with('/') {
            fs::create_dir_all(&out_path)?;
            continue;
        }
        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut out_file = File::create(&out_path)?;
        std::io::copy(&mut entry, &mut out_file)?;
    }

    Ok(extracted_dir)
}

pub fn parse_project_xml(extracted_dir: &Path) -> Result<XmlTree> {
    let project_xml = extracted_dir.join("project.xml");
    if !project_xml.exists() {
        return Err(DawprojectError::MissingFile("project.xml".to_string()));
    }

    let raw_xml = fs::read_to_string(project_xml)?;
    Ok(XmlTree { raw_xml })
}

pub fn parse_metadata_xml(extracted_dir: &Path) -> Result<DawprojectInfo> {
    let metadata_xml = extracted_dir.join("metadata.xml");
    if !metadata_xml.exists() {
        return Err(DawprojectError::MissingFile("metadata.xml".to_string()));
    }

    let raw = fs::read_to_string(metadata_xml)?;
    let doc = Document::parse(&raw)?;

    let project_name = extract_project_name(&doc).unwrap_or_else(|| "Unknown Project".to_string());
    let tempo = extract_tempo(&doc);
    let time_signature = extract_time_signature(&doc);
    let application_name = find_attr_ci(&doc, &["application", "app", "software"], &["name", "application"]);
    let application_version = find_attr_ci(&doc, &["application", "app", "software"], &["version", "appversion"]);

    Ok(DawprojectInfo {
        project_name,
        tempo,
        time_signature,
        application_name,
        application_version,
    })
}

pub fn build_semantic_index(path: &Path, snapshot_id: &str) -> Result<SemanticIndex> {
    let temp_dir = tempfile::tempdir()?;
    let extracted = extract_to_temp(path, temp_dir.path())?;
    let xml_tree = parse_project_xml(&extracted)?;
    let doc = Document::parse(&xml_tree.raw_xml)?;

    let project_name = extract_project_name(&doc).unwrap_or_else(|| {
        path.file_stem()
            .and_then(|n| n.to_str())
            .unwrap_or("Unknown Project")
            .to_string()
    });

    let tempo = extract_tempo(&doc);
    let time_signature = extract_time_signature(&doc);

    let mut tracks = Vec::new();
    let mut assets = Vec::new();

    for (idx, node) in doc.descendants().filter(|n| n.is_element()).enumerate() {
        let tag = node.tag_name().name().to_ascii_lowercase();

        if is_track_tag(&tag) {
            let id = node_id(&node, idx);
            let name = node
                .attribute("name")
                .or_else(|| node.attribute("title"))
                .unwrap_or("Unnamed Track")
                .to_string();

            let mut clips = Vec::new();
            let mut devices = Vec::new();

            for (child_idx, child) in node.descendants().filter(|n| n.is_element()).enumerate() {
                let child_tag = child.tag_name().name().to_ascii_lowercase();

                if is_clip_tag(&child_tag) {
                    let start = attr_f64(&child, &["start", "position", "beat"]).unwrap_or(0.0);
                    let length = attr_f64(&child, &["length", "duration", "len"]).unwrap_or(0.0);
                    clips.push(ClipIndex {
                        id: node_id(&child, child_idx),
                        name: child.attribute("name").map(ToString::to_string),
                        start,
                        length,
                        fingerprint: fingerprint_node(&child),
                    });
                }

                if is_device_tag(&child_tag) {
                    devices.push(DeviceIndex {
                        id: node_id(&child, child_idx),
                        name: child
                            .attribute("name")
                            .or_else(|| child.attribute("plugin"))
                            .unwrap_or("Unnamed Device")
                            .to_string(),
                        device_type: child_tag.clone(),
                        fingerprint: fingerprint_node(&child),
                    });
                }
            }

            tracks.push(TrackIndex {
                id,
                name,
                track_type: tag,
                clips,
                devices,
                fingerprint: fingerprint_node(&node),
            });
        }

        if is_asset_tag(&tag) {
            let path_attr = node.attribute("file").or_else(|| node.attribute("path"));
            if let Some(path_str) = path_attr {
                assets.push(AssetIndex {
                    path: path_str.to_string(),
                    hash: blake3_hex(path_str.as_bytes()),
                    asset_type: tag,
                });
            }
        }
    }

    let mut hasher = Hasher::new();
    hasher.update(snapshot_id.as_bytes());
    hasher.update(project_name.as_bytes());
    for t in &tracks {
        hasher.update(t.fingerprint.as_bytes());
    }
    for a in &assets {
        hasher.update(a.hash.as_bytes());
    }

    Ok(SemanticIndex {
        snapshot_id: snapshot_id.to_string(),
        project_name,
        tempo,
        time_signature,
        tracks,
        assets,
        fingerprint: hasher.finalize().to_hex().to_string(),
    })
}

fn is_track_tag(tag: &str) -> bool {
    ["track", "channel", "lane"].iter().any(|k| tag.contains(k))
}

fn is_clip_tag(tag: &str) -> bool {
    ["clip", "region", "event", "note"].iter().any(|k| tag.contains(k))
}

fn is_device_tag(tag: &str) -> bool {
    ["device", "plugin", "instrument", "effect"]
        .iter()
        .any(|k| tag.contains(k))
}

fn is_asset_tag(tag: &str) -> bool {
    ["audio", "sample", "file"].iter().any(|k| tag.contains(k))
}

fn extract_project_name(doc: &Document<'_>) -> Option<String> {
    for node in doc.descendants().filter(|n| n.is_element()) {
        for attr in node.attributes() {
            if attr.name().eq_ignore_ascii_case("name") {
                let v = attr.value().trim();
                if !v.is_empty() {
                    return Some(v.to_string());
                }
            }
        }
        let tag = node.tag_name().name();
        if tag.eq_ignore_ascii_case("name") {
            let text = node.text().unwrap_or("").trim();
            if !text.is_empty() {
                return Some(text.to_string());
            }
        }
    }
    None
}

fn extract_tempo(doc: &Document<'_>) -> Option<f64> {
    for node in doc.descendants().filter(|n| n.is_element()) {
        if node.tag_name().name().eq_ignore_ascii_case("tempo") {
            if let Some(v) = node.text().and_then(|t| t.trim().parse::<f64>().ok()) {
                return Some(v);
            }
        }
        for attr in node.attributes() {
            if attr.name().eq_ignore_ascii_case("tempo") || attr.name().eq_ignore_ascii_case("bpm") {
                if let Ok(v) = attr.value().trim().parse::<f64>() {
                    return Some(v);
                }
            }
        }
    }
    None
}

fn extract_time_signature(doc: &Document<'_>) -> Option<String> {
    for node in doc.descendants().filter(|n| n.is_element()) {
        for attr in node.attributes() {
            if attr.name().eq_ignore_ascii_case("timesignature") || attr.name().eq_ignore_ascii_case("time_signature") {
                let v = attr.value().trim();
                if !v.is_empty() {
                    return Some(v.to_string());
                }
            }
        }
    }
    None
}

fn find_attr_ci(doc: &Document<'_>, tags: &[&str], attrs: &[&str]) -> Option<String> {
    for node in doc.descendants().filter(|n| n.is_element()) {
        let tag = node.tag_name().name().to_ascii_lowercase();
        if tags.iter().any(|t| tag.contains(t)) {
            for attr in node.attributes() {
                let name = attr.name().to_ascii_lowercase();
                if attrs.iter().any(|a| name == *a) {
                    return Some(attr.value().to_string());
                }
            }
        }
    }
    None
}

fn attr_f64(node: &roxmltree::Node<'_, '_>, names: &[&str]) -> Option<f64> {
    for attr in node.attributes() {
        if names.iter().any(|n| attr.name().eq_ignore_ascii_case(n)) {
            if let Ok(v) = attr.value().trim().parse::<f64>() {
                return Some(v);
            }
        }
    }
    None
}

fn node_id(node: &roxmltree::Node<'_, '_>, index: usize) -> String {
    if let Some(id) = node.attribute("id") {
        return id.to_string();
    }
    let tag = node.tag_name().name();
    let name = node.attribute("name").unwrap_or("");
    let basis = format!("{tag}{name}{index}");
    blake3_hex(basis.as_bytes())[0..12].to_string()
}

fn fingerprint_node(node: &roxmltree::Node<'_, '_>) -> String {
    let mut attrs: Vec<(String, String)> = node
        .attributes()
        .map(|a| (a.name().to_ascii_lowercase(), a.value().trim().to_string()))
        .collect();
    attrs.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));

    let mut children: Vec<String> = node
        .children()
        .filter(|c| c.is_element())
        .map(|c| c.tag_name().name().to_ascii_lowercase())
        .collect();
    children.sort();

    let text = node.text().unwrap_or("").split_whitespace().collect::<String>();
    let payload = format!(
        "{}|{:?}|{:?}|{}",
        node.tag_name().name().to_ascii_lowercase(),
        attrs,
        children,
        text
    );
    blake3_hex(payload.as_bytes())
}

fn blake3_hex(data: &[u8]) -> String {
    blake3::hash(data).to_hex().to_string()
}

pub fn read_file_bytes(path: &Path) -> Result<Vec<u8>> {
    let mut file = File::open(path)?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)?;
    Ok(buf)
}
