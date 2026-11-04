use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::net::IpAddr;
use std::path::{Path, PathBuf};

use regex::Regex;
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::devices::{DeviceCatalog, DeviceRecord, DeviceStatus};
use crate::discovery::{ROBOTS_TXT, canonical_urls_from_authority_and_devices};
use crate::host_dataset::{HOST_DATASET_FILE, validate_repository_host_dataset};
use crate::host_history::{HOST_HISTORY_FILE, host_history_href};
use crate::message_path;
use crate::pages::{devices, radio as radio_page, repositories as repositories_page};
use crate::projection_proof::{
    CAPTURE_FILE, PortableProjection, ProjectionProof, SHELFMARK_FILE, TRACE_FILE,
};
use crate::repositories::{Authority, PublicMetadataCache, RepositoryRecord, ShowcaseManifest};
use crate::repository_frozen::{self, FrozenFirstView};
use crate::retinue_traces::{TRACE_DIRECTORY, TraceSet};
use crate::site::{
    DEFAULT_SOCIAL_IMAGE_ALT, DEFAULT_SOCIAL_IMAGE_URL, ORGANIZATION_ID, WEBSITE_ID,
};
use crate::spec_roadmap::ROADMAP_DATASET_FILE;

const RECEIPT_SCHEMA: &str = "mer3ly.public-artifact-receipt/v1";
const GRAPH_SCHEMA: &str = "mer3ly.repo-graph/v1";
const APPROVED_CONTACT_EMAIL: &str = "markik@mer3ly.net";
const BASE_FILES: &[&str] = &[
    "CNAME",
    "devices.css",
    "favicon.svg",
    "graph-sandbox-mount.js",
    "devices/index.html",
    "index.html",
    "message-path-lab.js",
    "mer3ly_repo_graph.js",
    "mer3ly_repo_graph_bg.wasm",
    "og.jpg",
    "projection-proof.js",
    CAPTURE_FILE,
    SHELFMARK_FILE,
    TRACE_FILE,
    "radio.html",
    "radio-simulator.js",
    "radio_mirror.js",
    "radio_mirror_bg.wasm",
    HOST_DATASET_FILE,
    HOST_HISTORY_FILE,
    "repos/index.html",
    "roadmap/index.html",
    ROADMAP_DATASET_FILE,
    crate::pages::roadmap::MOUNT_FILE,
    "robots.txt",
    "sitemap.xml",
    "site.css",
];

#[derive(Debug, Serialize)]
pub struct ArtifactReceipt {
    schema: &'static str,
    source_sha: String,
    files: Vec<ArtifactFileReceipt>,
    total_bytes: u64,
    repositories: usize,
    relation_text_projections: usize,
    graph_nodes: usize,
    graph_edges: usize,
    project_profiles: usize,
    project_relation_projections: usize,
    showcase_images: usize,
    sitemap_urls: usize,
    project_social_previews: usize,
    project_structured_records: usize,
    projection_score_items: usize,
    projection_final_revision: u64,
    projection_active_relations: usize,
    projection_trace_steps: usize,
    projection_capture_address: String,
    repository_history_revisions: usize,
    repository_frozen_matrix_cells: usize,
    device_profiles: usize,
    device_structured_records: usize,
    radio_mirror_screens: usize,
    message_path_traces: usize,
    message_path_events: usize,
    message_path_ledger_rows: usize,
    sellable_devices: usize,
    metadata_generated_at_utc: String,
    metadata_sha256: String,
}

#[derive(Debug, Serialize)]
struct ArtifactFileReceipt {
    path: String,
    bytes: u64,
    sha256: String,
}

#[derive(Debug, serde::Deserialize)]
struct GraphPayload {
    schema: String,
    nodes: Vec<GraphNode>,
    edges: Vec<GraphEdge>,
}

#[derive(Debug, serde::Deserialize)]
struct GraphNode {
    id: String,
}

#[derive(Debug, serde::Deserialize)]
struct GraphEdge {
    id: String,
    source: String,
    target: String,
}

struct ExpectedSocialMetadata<'a> {
    canonical: &'a str,
    image_url: &'a str,
    image_type: &'a str,
    image_alt: &'a str,
}

fn collect_files(root: &Path, current: &Path, files: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in fs::read_dir(current)? {
        let entry = entry?;
        let path = entry.path();
        let file_type = entry.file_type()?;
        if file_type.is_symlink() {
            return Err(std::io::Error::other(format!(
                "symbolic links are not allowed in the public artifact: {}",
                artifact_relative_path(root, &path)
            )));
        }
        if file_type.is_dir() {
            collect_files(root, &path, files)?;
        } else if file_type.is_file() {
            files.push(path);
        }
    }
    Ok(())
}

fn artifact_relative_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

/// The V4 bench: its module, its status documents, every rendered screen with
/// its text projection, and the statement of where the screens come from.
fn validate_radio_bench(
    document: &str,
    screens: &[mer3ly_radio_mirror::StaticScreen],
    errors: &mut Vec<String>,
) {
    let module = format!(
        "<script type=\"module\" src=\"{}\"></script>",
        devices::radio_simulator_href()
    );
    if !document.contains(&module) {
        errors.push(
            "V4 device profile is missing its versioned radio-mirror bench module".to_owned(),
        );
    }
    if inline_json(document, devices::RADIO_MIRROR_SCENARIOS_ID)
        != Some(devices::radio_mirror_scenarios_embedded().as_str())
    {
        errors.push(
            "V4 radio bench scenarios differ from the pinned radio-mirror fixtures".to_owned(),
        );
    }
    let statement = escape_html_text(&devices::radio_mirror_source_statement());
    if !document.contains(&statement) {
        errors.push("V4 radio bench does not state where its screens come from".to_owned());
    }
    if document.contains("accurate static example") {
        errors.push("V4 radio bench repeats the retired static-example claim".to_owned());
    }
    for screen in screens {
        let image = format!(
            "src=\"/{}\" alt=\"{}\"",
            screen.path,
            escape_html_attr(&devices::radio_screen_alt(&screen.lines))
        );
        if !document.contains(&image) {
            errors.push(format!(
                "V4 radio bench is missing {} with its text projection",
                screen.path
            ));
        }
    }
}

/// The lab's static reading is the cold trace's: its manifest, its route
/// sentence and ledger, and its screen with radio-face's text projection.
fn validate_message_path(
    document: &str,
    traces: &TraceSet,
    screen: &mer3ly_radio_mirror::StaticScreen,
    errors: &mut Vec<String>,
) {
    match message_path::manifest_embedded(traces) {
        Ok(manifest)
            if inline_json(document, message_path::MANIFEST_ID) == Some(manifest.as_str()) => {}
        Ok(_) => errors.push("message path manifest differs from the committed traces".to_owned()),
        Err(error) => errors.push(format!("message path manifest: {error}")),
    }
    let Ok(view) = message_path::default_view(traces) else {
        errors.push("message path has no default view".to_owned());
        return;
    };
    let Some(scenario) = traces.scenario(&view.scenario) else {
        return;
    };
    let route = escape_html_text(&message_path::route_sentence(scenario));
    if !document.contains(&format!("<p data-path-route=\"\">{route}</p>")) {
        errors.push("message path route sentence differs from the trace".to_owned());
    }
    for row in message_path::milestones(scenario) {
        let copy = format!(
            "data-lab-event=\"{}\" data-kind=\"{}\"",
            row.event, row.kind
        );
        if !document.contains(&copy) || !document.contains(&escape_html_text(&row.text)) {
            errors.push(format!(
                "message path ledger is missing trace event {}",
                row.event
            ));
        }
    }
    if !document.contains(&format!("src=\"/{}\"", screen.path)) {
        errors.push("message path is missing its rendered screen".to_owned());
    }
    for line in &screen.lines {
        if !document.contains(&format!("<li>{}</li>", escape_html_text(line))) {
            errors.push("message path screen text differs from its rendering".to_owned());
        }
    }
    if !document.contains(&escape_html_text(&message_path::source_statement(traces))) {
        errors.push("message path does not state where its traces come from".to_owned());
    }
    for retired in [
        "three relays",
        "RET · ROUTE",
        "RET · DELIVERED",
        "TX QUEUED",
        "RX FRAME",
    ] {
        if document.contains(retired) {
            errors.push(format!("message path repeats the retired {retired:?}"));
        }
    }
}

fn escape_html_text(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn read_text(root: &Path, relative: &str, errors: &mut Vec<String>) -> String {
    match fs::read_to_string(root.join(relative)) {
        Ok(contents) => contents,
        Err(error) => {
            errors.push(format!("could not read {relative}: {error}"));
            String::new()
        }
    }
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[cfg(test)]
mod tests;

mod assets;

use assets::{validate_copied_asset, validate_static_assets};

mod validate;

pub use validate::validate_public_artifact;

mod projection;

use projection::{
    attribute_values, inline_json, parse_graph_payload, validate_graph_payload,
    validate_project_authority, validate_projection_artifacts, validate_projection_page,
    validate_repositories_first_view, validate_static_authority,
};

mod policy;

use policy::{is_scannable, scan_public_bytes};

mod metadata;

use metadata::{
    escape_html_attr, validate_device_metadata, validate_fixed_metadata, validate_project_metadata,
    validate_sitemap,
};

#[cfg(test)]
use policy::scan_public_text;
