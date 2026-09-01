use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::net::IpAddr;
use std::path::{Path, PathBuf};

use mer3ly_repo_graph::consume_portable_projection_json;
use regex::Regex;
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::devices::{DeviceCatalog, DeviceRecord, DeviceStatus};
use crate::discovery::{ROBOTS_TXT, canonical_urls_from_authority_and_devices};
use crate::repositories::{Authority, PublicMetadataCache, RepositoryRecord, ShowcaseManifest};
use crate::site::{
    DEFAULT_SOCIAL_IMAGE_ALT, DEFAULT_SOCIAL_IMAGE_URL, ORGANIZATION_ID, WEBSITE_ID,
};

const RECEIPT_SCHEMA: &str = "mer3ly.public-artifact-receipt/v1";
const GRAPH_SCHEMA: &str = "mer3ly.repo-graph/v1";
const APPROVED_CONTACT_EMAIL: &str = "markik@mer3ly.net";
const BASE_FILES: &[&str] = &[
    "CNAME",
    "devices.css",
    "favicon.svg",
    "graph-sandbox.js",
    "devices/index.html",
    "index.html",
    "message-path-lab.js",
    "mer3ly_repo_graph.js",
    "mer3ly_repo_graph_bg.wasm",
    "og.jpg",
    "projection-proof.js",
    "projection-scene.json",
    "radio.html",
    "radio-simulator.js",
    "repos/index.html",
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
    device_profiles: usize,
    device_structured_records: usize,
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

pub fn validate_public_artifact(
    artifact_root: &Path,
    source_root: &Path,
    authority: &Authority,
    metadata: &PublicMetadataCache,
    showcases: &ShowcaseManifest,
    devices: &DeviceCatalog,
    metadata_path: &Path,
) -> Result<ArtifactReceipt, Vec<String>> {
    let mut errors = Vec::new();
    let mut files = Vec::new();
    if let Err(error) = collect_files(artifact_root, artifact_root, &mut files) {
        return Err(vec![format!("could not read public artifact: {error}")]);
    }
    files.sort();

    let actual_paths = files
        .iter()
        .map(|path| artifact_relative_path(artifact_root, path))
        .collect::<BTreeSet<_>>();
    let mut expected_paths = BASE_FILES
        .iter()
        .map(|path| (*path).to_owned())
        .collect::<BTreeSet<_>>();
    for repository in authority
        .repositories
        .repository
        .iter()
        .filter(|repository| repository.public)
    {
        expected_paths.insert(format!("projects/{}/index.html", repository.id));
    }
    for device in devices.ordered() {
        expected_paths.insert(format!("devices/{}/index.html", device.id));
    }
    for showcase in &showcases.showcase {
        expected_paths.insert(showcase.image.clone());
        for extra in &showcase.images {
            expected_paths.insert(extra.image.clone());
        }
    }
    if actual_paths != expected_paths {
        errors.push("public artifact file set differs from the approved shape".to_owned());
    }

    let allowed_github_slugs = authority
        .repositories
        .repository
        .iter()
        .filter(|repository| repository.public)
        .map(|repository| repository.github_slug.to_ascii_lowercase())
        .collect::<BTreeSet<_>>();

    let mut file_receipts = Vec::new();
    let mut total_bytes = 0_u64;
    for path in &files {
        let relative = artifact_relative_path(artifact_root, path);
        let bytes = match fs::read(path) {
            Ok(bytes) => bytes,
            Err(error) => {
                errors.push(format!(
                    "could not read public artifact file {relative}: {error}"
                ));
                continue;
            }
        };
        total_bytes = total_bytes.saturating_add(bytes.len() as u64);
        if is_scannable(&relative) {
            scan_public_text(
                &relative,
                &String::from_utf8_lossy(&bytes),
                &allowed_github_slugs,
                &mut errors,
            );
        }
        file_receipts.push(ArtifactFileReceipt {
            path: relative,
            bytes: bytes.len() as u64,
            sha256: sha256(&bytes),
        });
    }

    validate_cname(artifact_root, "CNAME", &mut errors);
    validate_copied_asset(
        artifact_root,
        source_root,
        "favicon.svg",
        "assets/favicon.svg",
        "favicon",
        &mut errors,
    );
    validate_copied_asset(
        artifact_root,
        source_root,
        "devices.css",
        "assets/devices.css",
        "device stylesheet",
        &mut errors,
    );
    validate_copied_asset(
        artifact_root,
        source_root,
        "message-path-lab.js",
        "assets/message-path-lab.js",
        "message path lab",
        &mut errors,
    );
    validate_copied_asset(
        artifact_root,
        source_root,
        "projection-proof.js",
        "assets/projection-proof.js",
        "portable projection proof",
        &mut errors,
    );
    validate_copied_asset(
        artifact_root,
        source_root,
        "radio-simulator.js",
        "assets/radio-simulator.js",
        "radio simulator",
        &mut errors,
    );

    let home = read_text(artifact_root, "index.html", &mut errors);
    let radio = read_text(artifact_root, "radio.html", &mut errors);
    let repositories = read_text(artifact_root, "repos/index.html", &mut errors);
    let projection_scene = read_text(artifact_root, "projection-scene.json", &mut errors);
    let device_index = read_text(artifact_root, "devices/index.html", &mut errors);
    let robots = read_text(artifact_root, "robots.txt", &mut errors);
    let sitemap = read_text(artifact_root, "sitemap.xml", &mut errors);
    if !home.starts_with("<!doctype html>") || !radio.starts_with("<!doctype html>") {
        errors.push("home or community-radio output is not a complete HTML document".to_owned());
    }
    validate_fixed_metadata(
        &home,
        "https://mer3ly.net/",
        DEFAULT_SOCIAL_IMAGE_URL,
        "image/jpeg",
        DEFAULT_SOCIAL_IMAGE_ALT,
        &mut errors,
    );
    validate_fixed_metadata(
        &radio,
        "https://mer3ly.net/radio.html",
        DEFAULT_SOCIAL_IMAGE_URL,
        "image/jpeg",
        DEFAULT_SOCIAL_IMAGE_ALT,
        &mut errors,
    );
    if !radio.contains("<script type=\"module\" src=\"/message-path-lab.js?v=") {
        errors.push("community-radio output is missing its message path lab module".to_owned());
    }
    if !radio.contains("data-message-path-lab") {
        errors.push("community-radio output is missing its message path lab landmark".to_owned());
    }
    validate_fixed_metadata(
        &repositories,
        "https://mer3ly.net/repos/",
        DEFAULT_SOCIAL_IMAGE_URL,
        "image/jpeg",
        DEFAULT_SOCIAL_IMAGE_ALT,
        &mut errors,
    );
    validate_fixed_metadata(
        &device_index,
        "https://mer3ly.net/devices/",
        DEFAULT_SOCIAL_IMAGE_URL,
        "image/jpeg",
        DEFAULT_SOCIAL_IMAGE_ALT,
        &mut errors,
    );
    if !device_index.contains("href=\"/devices.css?v=") {
        errors.push("device index is missing its content-addressed stylesheet".to_owned());
    }
    if !home.contains("href=\"/devices/\"") {
        errors.push("home page is missing the hardware catalog link".to_owned());
    }
    if robots != ROBOTS_TXT {
        errors.push("robots policy differs from the approved public policy".to_owned());
    }
    let expected_sitemap_urls = canonical_urls_from_authority_and_devices(authority, devices);
    let sitemap_urls = validate_sitemap(&sitemap, &expected_sitemap_urls, &mut errors);

    let repository_ids = attribute_values(&repositories, "data-repository-id");
    let relation_ids = attribute_values(&repositories, "data-relation-id");
    validate_static_authority(&repository_ids, &relation_ids, authority, &mut errors);
    let repository_count = repository_ids.len();
    let relation_text_projections = relation_ids.len();
    if !repositories.contains("<script type=\"module\" src=\"/graph-sandbox.js?v=") {
        errors.push("repository page is missing the Graphshell sandbox module".to_owned());
    }
    if !repositories.contains("data-graph-sandbox")
        || !repositories.contains("id=\"graph-sandbox-data\"")
    {
        errors
            .push("repository page is missing the Graphshell sandbox landmark or data".to_owned());
    }

    let mut project_ids = Vec::new();
    let mut project_relation_ids = Vec::new();
    let mut project_social_previews = 0;
    let mut project_structured_records = 0;
    let projection_receipt = match consume_portable_projection_json(&projection_scene) {
        Ok(receipt) => Some(receipt),
        Err(error) => {
            errors.push(format!("portable projection artifact is invalid: {error}"));
            None
        }
    };
    for repository in authority
        .repositories
        .repository
        .iter()
        .filter(|repository| repository.public)
    {
        let relative = format!("projects/{}/index.html", repository.id);
        let project = read_text(artifact_root, &relative, &mut errors);
        if !project.starts_with("<!doctype html>") {
            errors.push(format!(
                "project profile {} is not a complete HTML document",
                repository.id
            ));
        }
        project_ids.extend(attribute_values(&project, "data-project-id"));
        project_relation_ids.extend(attribute_values(&project, "data-relation-id"));
        let canonical = format!("https://mer3ly.net/projects/{}/", repository.id);
        let showcase = showcases.for_repository(&repository.id);
        let social_image = showcase.map_or(DEFAULT_SOCIAL_IMAGE_URL.to_owned(), |showcase| {
            format!("https://mer3ly.net/{}", showcase.image)
        });
        let social_type = if showcase.is_some() {
            "image/png"
        } else {
            "image/jpeg"
        };
        let social_alt =
            showcase.map_or(DEFAULT_SOCIAL_IMAGE_ALT, |showcase| showcase.alt.as_str());
        let expected_social = ExpectedSocialMetadata {
            canonical: &canonical,
            image_url: &social_image,
            image_type: social_type,
            image_alt: social_alt,
        };
        let (social_valid, structured_valid) = validate_project_metadata(
            &project,
            repository,
            metadata,
            &expected_social,
            &mut errors,
        );
        if social_valid {
            project_social_previews += 1;
        }
        if structured_valid {
            project_structured_records += 1;
        }
        let profile_href = format!("/projects/{}/", repository.id);
        if !home.contains(&profile_href) && showcases.for_repository(&repository.id).is_some() {
            errors.push(format!(
                "home page is missing showcased project profile {}",
                repository.id
            ));
        }
        if !repositories.contains(&format!("data-project-href=\"{profile_href}\"")) {
            errors.push(format!(
                "repository page is missing project profile link {}",
                repository.id
            ));
        }
        if let Some(showcase) = showcases.for_repository(&repository.id) {
            let mut missing = !project.contains(&format!("src=\"/{}\"", showcase.image))
                || !project.contains(&showcase.source_url)
                || !project.contains(&showcase.alt);
            for extra in &showcase.images {
                missing = missing
                    || !project.contains(&format!("src=\"/{}\"", extra.image))
                    || !project.contains(&extra.source_url)
                    || !project.contains(&extra.alt);
            }
            if missing {
                errors.push(format!(
                    "project profile {} is missing approved showcase evidence",
                    repository.id
                ));
            }
        }
        if repository.id == "mere" {
            if !project.contains("data-projection-proof")
                || !project.contains("<script type=\"module\" src=\"/projection-proof.js?v=")
            {
                errors.push(
                    "Mere project profile is missing its portable projection proof".to_owned(),
                );
            }
            match inline_json(&project, "mere-projection-artifact") {
                Some(embedded) if embedded == projection_scene => {}
                Some(_) => errors
                    .push("Mere project profile and public projection artifact differ".to_owned()),
                None => errors.push(
                    "Mere project profile is missing its serialized projection artifact".to_owned(),
                ),
            }
        } else if project.contains("data-projection-proof")
            || project.contains("/projection-proof.js?v=")
        {
            errors.push(format!(
                "project profile {} unexpectedly includes the Mere projection proof",
                repository.id
            ));
        }
    }
    validate_project_authority(&project_ids, &project_relation_ids, authority, &mut errors);

    let mut device_ids = Vec::new();
    let mut device_structured_records = 0;
    let mut sellable_devices = 0;
    for device in devices.ordered() {
        let relative = format!("devices/{}/index.html", device.id);
        let document = read_text(artifact_root, &relative, &mut errors);
        if !document.starts_with("<!doctype html>") {
            errors.push(format!(
                "device profile {} is not a complete HTML document",
                device.id
            ));
        }
        if !document.contains("href=\"/devices.css?v=") {
            errors.push(format!(
                "device profile {} is missing its content-addressed stylesheet",
                device.id
            ));
        }
        if device.id == "v4-desktop-radio"
            && !document.contains("<script type=\"module\" src=\"/radio-simulator.js?v=")
        {
            errors.push("V4 device profile is missing its radio simulator module".to_owned());
        }
        if device.id != "v4-desktop-radio" && document.contains("data-radio-simulator") {
            errors.push(format!(
                "device profile {} unexpectedly includes the V4 radio simulator",
                device.id
            ));
        }
        let ids = attribute_values(&document, "data-device-id");
        if ids != [device.id.clone()] {
            errors.push(format!(
                "device profile {} does not project its exact authority id",
                device.id
            ));
        }
        device_ids.extend(ids);
        let canonical = format!("https://mer3ly.net/devices/{}/", device.id);
        let expected_social = ExpectedSocialMetadata {
            canonical: &canonical,
            image_url: DEFAULT_SOCIAL_IMAGE_URL,
            image_type: "image/jpeg",
            image_alt: DEFAULT_SOCIAL_IMAGE_ALT,
        };
        if validate_device_metadata(&document, device, &expected_social, &mut errors) {
            device_structured_records += 1;
        }
        if !device_index.contains(&format!("data-device-id=\"{}\"", device.id)) {
            errors.push(format!(
                "device index is missing catalog record {}",
                device.id
            ));
        }
        let source_url = format!(
            "{}/blob/main/{}",
            device.source_repository, device.source_document
        );
        if !document.contains(&source_url) {
            errors.push(format!(
                "device profile {} is missing its public evidence link",
                device.id
            ));
        }
        match (&device.status, &device.sale.purchase_url) {
            (DeviceStatus::Sellable, Some(url)) => {
                sellable_devices += 1;
                if !document.contains(&format!("href=\"{url}\""))
                    || !document.contains("class=\"button button-primary purchase-link\"")
                {
                    errors.push(format!(
                        "sellable device profile {} is missing its final purchase link",
                        device.id
                    ));
                }
            }
            _ => {
                if !document.contains("data-purchase-status=\"unavailable\"")
                    || document.contains("class=\"button button-primary purchase-link\"")
                {
                    errors.push(format!(
                        "non-sellable device profile {} has an invalid purchase control",
                        device.id
                    ));
                }
            }
        }
    }
    let expected_device_ids = devices
        .ordered()
        .into_iter()
        .map(|device| device.id.clone())
        .collect::<Vec<_>>();
    if device_ids != expected_device_ids {
        errors.push("device profile ids differ from catalog authority".to_owned());
    }

    for showcase in &showcases.showcase {
        let images = std::iter::once(showcase.image.as_str())
            .chain(showcase.images.iter().map(|extra| extra.image.as_str()));
        for image in images {
            let artifact_image = artifact_root.join(image);
            let source_image = source_root.join("assets").join(image);
            match (fs::read(&artifact_image), fs::read(&source_image)) {
                (Ok(artifact_bytes), Ok(source_bytes)) if artifact_bytes == source_bytes => {}
                (Ok(_), Ok(_)) => errors.push(format!(
                    "showcase {} artifact image differs from its approved source",
                    showcase.repository
                )),
                _ => errors.push(format!(
                    "showcase {} artifact or approved source image is missing",
                    showcase.repository
                )),
            }
        }
    }
    let timestamp = format!(
        "{} {} UTC",
        &metadata.generated_at_utc[..10],
        &metadata.generated_at_utc[11..16]
    );
    let refresh_statement = format!("Refreshed {timestamp}.");
    if !repositories.contains(&refresh_statement) {
        errors.push("repository page does not display the validated metadata timestamp".to_owned());
    }

    let graph = parse_graph_payload(&repositories, &mut errors);
    let graph_nodes = graph.as_ref().map_or(0, |payload| payload.nodes.len());
    let graph_edges = graph.as_ref().map_or(0, |payload| payload.edges.len());
    if let Some(payload) = &graph {
        validate_graph_payload(payload, authority, &mut errors);
    }

    let wasm_path = artifact_root.join("mer3ly_repo_graph_bg.wasm");
    match fs::read(&wasm_path) {
        Ok(bytes) if bytes.starts_with(b"\0asm") => {}
        Ok(_) => errors.push("repository graph Wasm has an invalid magic header".to_owned()),
        Err(error) => errors.push(format!("could not read repository graph Wasm: {error}")),
    }

    let metadata_bytes = match fs::read(metadata_path) {
        Ok(bytes) => bytes,
        Err(error) => {
            errors.push(format!("could not hash public metadata input: {error}"));
            Vec::new()
        }
    };

    if errors.is_empty() {
        Ok(ArtifactReceipt {
            schema: RECEIPT_SCHEMA,
            source_sha: env::var("GITHUB_SHA").unwrap_or_else(|_| "local".to_owned()),
            files: file_receipts,
            total_bytes,
            repositories: repository_count,
            relation_text_projections,
            graph_nodes,
            graph_edges,
            project_profiles: project_ids.len(),
            project_relation_projections: project_relation_ids.len(),
            showcase_images: showcases.showcase.len(),
            sitemap_urls,
            project_social_previews,
            project_structured_records,
            projection_score_items: projection_receipt
                .as_ref()
                .map_or(0, |receipt| receipt.score_items),
            projection_final_revision: projection_receipt
                .as_ref()
                .map_or(0, |receipt| receipt.final_revision),
            projection_active_relations: projection_receipt
                .as_ref()
                .map_or(0, |receipt| receipt.active_relations),
            device_profiles: device_ids.len(),
            device_structured_records,
            sellable_devices,
            metadata_generated_at_utc: metadata.generated_at_utc.clone(),
            metadata_sha256: sha256(&metadata_bytes),
        })
    } else {
        Err(errors)
    }
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

fn is_scannable(path: &str) -> bool {
    path.ends_with(".html")
        || path.ends_with(".css")
        || path.ends_with(".js")
        || path.ends_with(".json")
        || path.ends_with(".svg")
        || path.ends_with(".txt")
        || path.ends_with(".xml")
        || path.ends_with(".wasm")
        || path.ends_with("CNAME")
}

fn scan_public_text(
    relative_path: &str,
    text: &str,
    allowed_github_slugs: &BTreeSet<String>,
    errors: &mut Vec<String>,
) {
    let lower = text.to_ascii_lowercase();
    // The bare username marker must sit at a word start: product vocabulary
    // like SHELFMARK_SCHEMA lowercases to shelfmark_schema, which contains
    // "mark_" mid-identifier without leaking anything.
    let username_leak = lower
        .match_indices("mark_")
        .any(|(index, _)| index == 0 || !lower.as_bytes()[index - 1].is_ascii_alphanumeric());
    if username_leak {
        errors.push(format!(
            "{relative_path} contains a forbidden public-data marker"
        ));
    }
    for marker in [
        "c:\\users\\",
        "\\users\\",
        "/users/",
        "/home/",
        "file://",
        "viewerpermission",
        "\"viewer_permission\"",
        "\"ssh_url\"",
        "\"sshurl\"",
        "authorization: bearer",
        "github_pat_",
        "begin rsa private key",
        "begin ec private key",
        "begin openssh private key",
    ] {
        if lower.contains(marker) {
            errors.push(format!(
                "{relative_path} contains a forbidden public-data marker"
            ));
            break;
        }
    }

    let drive_path = Regex::new(
        r#"(?i)(?:^|[^a-z0-9])(?:[a-z]:[\\/](?:users|home|documents and settings|workspaces?|code)[\\/]|\\\\[a-z0-9._-]+[\\/])"#,
    )
    .expect("valid path regex");
    if drive_path.is_match(text) {
        errors.push(format!(
            "{relative_path} contains an absolute or network filesystem path"
        ));
    }

    let email =
        Regex::new(r"(?i)\b[a-z0-9._%+-]+@[a-z0-9.-]+\.[a-z]{2,}\b").expect("valid email regex");
    if email.find_iter(text).any(|candidate| {
        !candidate
            .as_str()
            .eq_ignore_ascii_case(APPROVED_CONTACT_EMAIL)
    }) {
        errors.push(format!(
            "{relative_path} contains an unapproved contact address"
        ));
    }

    let private_host = Regex::new(r"(?i)(?:https?|wss?)://[a-z0-9.-]+\.(?:internal|local)\b")
        .expect("valid host regex");
    if private_host.is_match(text) {
        errors.push(format!("{relative_path} contains a private hostname"));
    }

    let ipv4 = Regex::new(r"\b(?:[0-9]{1,3}\.){3}[0-9]{1,3}\b").expect("valid IPv4 regex");
    if ipv4.find_iter(text).any(|candidate| {
        candidate
            .as_str()
            .parse::<IpAddr>()
            .is_ok_and(|address| match address {
                IpAddr::V4(address) => {
                    address.is_private()
                        || address.is_loopback()
                        || address.is_link_local()
                        || address.is_unspecified()
                }
                IpAddr::V6(_) => false,
            })
    }) {
        errors.push(format!(
            "{relative_path} contains a private or local network address"
        ));
    }

    let github = Regex::new(r"(?i)https://github\.com/([a-z0-9_.-]+)(?:/([a-z0-9_.-]+))?")
        .expect("valid GitHub URL regex");
    for captures in github.captures_iter(text) {
        let owner = captures
            .get(1)
            .map(|value| value.as_str().to_ascii_lowercase())
            .unwrap_or_default();
        let repository = captures
            .get(2)
            .map(|value| value.as_str().trim_end_matches(".git").to_ascii_lowercase());
        let approved = owner == "merely-made"
            && repository.as_ref().is_none_or(|repository| {
                allowed_github_slugs.contains(&format!("{owner}/{repository}"))
            });
        if !approved {
            errors.push(format!(
                "{relative_path} contains an unapproved GitHub repository link"
            ));
            break;
        }
    }
}

fn validate_cname(root: &Path, relative: &str, errors: &mut Vec<String>) {
    let contents = read_text(root, relative, errors);
    if contents.trim() != "mer3ly.net" {
        errors.push(format!(
            "{relative} does not name the approved public domain"
        ));
    }
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

fn validate_copied_asset(
    artifact_root: &Path,
    source_root: &Path,
    artifact_relative: &str,
    source_relative: &str,
    label: &str,
    errors: &mut Vec<String>,
) {
    match (
        fs::read(artifact_root.join(artifact_relative)),
        fs::read(source_root.join(source_relative)),
    ) {
        (Ok(artifact), Ok(source)) if artifact == source => {}
        (Ok(_), Ok(_)) => errors.push(format!("{label} artifact differs from its approved source")),
        _ => errors.push(format!("{label} artifact or approved source is missing")),
    }
}

fn validate_fixed_metadata(
    document: &str,
    canonical: &str,
    image_url: &str,
    image_type: &str,
    image_alt: &str,
    errors: &mut Vec<String>,
) {
    let expected = ExpectedSocialMetadata {
        canonical,
        image_url,
        image_type,
        image_alt,
    };
    validate_social_head(document, "fixed page", &expected, errors);
    let Some(value) = parse_json_ld(document, "fixed page", errors) else {
        return;
    };
    let Some(graph) = schema_graph(&value, "fixed page", errors) else {
        return;
    };
    validate_base_schema(graph, "fixed page", errors);
}

fn validate_project_metadata(
    document: &str,
    repository: &RepositoryRecord,
    metadata: &PublicMetadataCache,
    expected: &ExpectedSocialMetadata<'_>,
    errors: &mut Vec<String>,
) -> (bool, bool) {
    let social_start = errors.len();
    validate_social_head(
        document,
        &format!("project profile {}", repository.id),
        expected,
        errors,
    );
    let social_valid = errors.len() == social_start;

    let structured_start = errors.len();
    let Some(value) = parse_json_ld(
        document,
        &format!("project profile {}", repository.id),
        errors,
    ) else {
        return (social_valid, false);
    };
    let Some(graph) = schema_graph(
        &value,
        &format!("project profile {}", repository.id),
        errors,
    ) else {
        return (social_valid, false);
    };
    validate_base_schema(graph, &format!("project profile {}", repository.id), errors);

    let entity_id = format!("{}#repository", expected.canonical);
    let page = graph
        .iter()
        .find(|node| node.get("@id").and_then(Value::as_str) == Some(expected.canonical));
    if page.is_none_or(|page| {
        page.get("@type").and_then(Value::as_str) != Some("WebPage")
            || page.pointer("/about/@id").and_then(Value::as_str) != Some(entity_id.as_str())
            || page.pointer("/isPartOf/@id").and_then(Value::as_str) != Some(WEBSITE_ID)
    }) {
        errors.push(format!(
            "project profile {} has invalid WebPage structured data",
            repository.id
        ));
    }

    let entity = graph
        .iter()
        .find(|node| node.get("@id").and_then(Value::as_str) == Some(entity_id.as_str()));
    let expected_type = if repository.id == "org-profile" {
        "CreativeWork"
    } else {
        "SoftwareSourceCode"
    };
    let repository_url = format!("https://github.com/{}", repository.github_slug);
    if entity.is_none_or(|entity| {
        entity.get("@type").and_then(Value::as_str) != Some(expected_type)
            || entity.get("url").and_then(Value::as_str) != Some(expected.canonical)
            || entity.pointer("/publisher/@id").and_then(Value::as_str) != Some(ORGANIZATION_ID)
            || entity
                .get("sameAs")
                .and_then(Value::as_array)
                .is_none_or(|same_as| {
                    same_as.len() != 1 || same_as[0].as_str() != Some(repository_url.as_str())
                })
    }) {
        errors.push(format!(
            "project profile {} has invalid repository structured data",
            repository.id
        ));
    }

    if expected_type == "SoftwareSourceCode" {
        if entity
            .and_then(|entity| entity.get("codeRepository"))
            .and_then(Value::as_str)
            != Some(repository_url.as_str())
        {
            errors.push(format!(
                "project profile {} has invalid source repository structured data",
                repository.id
            ));
        }
        let public_metadata = metadata
            .repository
            .iter()
            .find(|record| record.id == repository.id);
        let expected_language =
            public_metadata.and_then(|record| record.primary_language.as_deref());
        let actual_language = entity
            .and_then(|entity| entity.get("programmingLanguage"))
            .and_then(Value::as_str);
        if actual_language != expected_language {
            errors.push(format!(
                "project profile {} has invalid language structured data",
                repository.id
            ));
        }
        let expected_topics = public_metadata
            .map(|record| record.topics.as_slice())
            .unwrap_or_default();
        let actual_topics = entity
            .and_then(|entity| entity.get("keywords"))
            .and_then(Value::as_array);
        if expected_topics.is_empty() {
            if actual_topics.is_some() {
                errors.push(format!(
                    "project profile {} has unexpected topic structured data",
                    repository.id
                ));
            }
        } else if actual_topics.is_none_or(|topics| {
            topics.len() != expected_topics.len()
                || topics
                    .iter()
                    .zip(expected_topics)
                    .any(|(actual, expected)| actual.as_str() != Some(expected.as_str()))
        }) {
            errors.push(format!(
                "project profile {} has invalid topic structured data",
                repository.id
            ));
        }
    }

    (social_valid, errors.len() == structured_start)
}

fn validate_device_metadata(
    document: &str,
    device: &DeviceRecord,
    expected: &ExpectedSocialMetadata<'_>,
    errors: &mut Vec<String>,
) -> bool {
    let start = errors.len();
    let label = format!("device profile {}", device.id);
    validate_social_head(document, &label, expected, errors);
    let Some(value) = parse_json_ld(document, &label, errors) else {
        return false;
    };
    let Some(graph) = schema_graph(&value, &label, errors) else {
        return false;
    };
    validate_base_schema(graph, &label, errors);

    let article_id = format!("{}#recipe", expected.canonical);
    let page = graph
        .iter()
        .find(|node| node.get("@id").and_then(Value::as_str) == Some(expected.canonical));
    if page.is_none_or(|page| {
        page.get("@type").and_then(Value::as_str) != Some("WebPage")
            || page.pointer("/about/@id").and_then(Value::as_str) != Some(article_id.as_str())
            || page.pointer("/isPartOf/@id").and_then(Value::as_str) != Some(WEBSITE_ID)
    }) {
        errors.push(format!("{label} has invalid WebPage structured data"));
    }

    let evidence_url = format!(
        "{}/blob/main/{}",
        device.source_repository, device.source_document
    );
    let article = graph
        .iter()
        .find(|node| node.get("@id").and_then(Value::as_str) == Some(article_id.as_str()));
    if article.is_none_or(|article| {
        article.get("@type").and_then(Value::as_str) != Some("TechArticle")
            || article.get("url").and_then(Value::as_str) != Some(expected.canonical)
            || article.pointer("/publisher/@id").and_then(Value::as_str) != Some(ORGANIZATION_ID)
            || article.get("isBasedOn").and_then(Value::as_str) != Some(evidence_url.as_str())
    }) {
        errors.push(format!("{label} has invalid TechArticle structured data"));
    }
    if device.status != DeviceStatus::Sellable
        && graph.iter().any(|node| node.get("offers").is_some())
    {
        errors.push(format!(
            "{label} publishes an Offer for a non-sellable device"
        ));
    }

    errors.len() == start
}

fn validate_social_head(
    document: &str,
    label: &str,
    expected: &ExpectedSocialMetadata<'_>,
    errors: &mut Vec<String>,
) {
    for (name, needle) in [
        (
            "canonical URL",
            format!(
                "<link rel=\"canonical\" href=\"{}\">",
                escape_html_attr(expected.canonical)
            ),
        ),
        (
            "Open Graph type",
            "<meta property=\"og:type\" content=\"website\">".to_owned(),
        ),
        (
            "Open Graph site name",
            "<meta property=\"og:site_name\" content=\"Merely\">".to_owned(),
        ),
        (
            "Open Graph URL",
            format!(
                "<meta property=\"og:url\" content=\"{}\">",
                escape_html_attr(expected.canonical)
            ),
        ),
        (
            "Open Graph image",
            format!(
                "<meta property=\"og:image\" content=\"{}\">",
                escape_html_attr(expected.image_url)
            ),
        ),
        (
            "Open Graph image type",
            format!(
                "<meta property=\"og:image:type\" content=\"{}\">",
                escape_html_attr(expected.image_type)
            ),
        ),
        (
            "Open Graph image alt",
            format!(
                "<meta property=\"og:image:alt\" content=\"{}\">",
                escape_html_attr(expected.image_alt)
            ),
        ),
        (
            "Twitter card",
            "<meta name=\"twitter:card\" content=\"summary_large_image\">".to_owned(),
        ),
        (
            "Twitter image",
            format!(
                "<meta name=\"twitter:image\" content=\"{}\">",
                escape_html_attr(expected.image_url)
            ),
        ),
        (
            "Twitter image alt",
            format!(
                "<meta name=\"twitter:image:alt\" content=\"{}\">",
                escape_html_attr(expected.image_alt)
            ),
        ),
        (
            "favicon",
            "<link rel=\"icon\" href=\"/favicon.svg\" type=\"image/svg+xml\">".to_owned(),
        ),
        (
            "sitemap link",
            "<link rel=\"sitemap\" href=\"/sitemap.xml\" type=\"application/xml\" title=\"Sitemap\">"
                .to_owned(),
        ),
    ] {
        if document.matches(&needle).count() != 1 {
            errors.push(format!("{label} has invalid {name} metadata"));
        }
    }

    for (name, first, second) in [
        (
            "title",
            "<meta property=\"og:title\" content=\"",
            "<meta name=\"twitter:title\" content=\"",
        ),
        (
            "description",
            "<meta property=\"og:description\" content=\"",
            "<meta name=\"twitter:description\" content=\"",
        ),
    ] {
        let first_value = quoted_attribute_value(document, first);
        let second_value = quoted_attribute_value(document, second);
        if first_value.is_none()
            || first_value.is_some_and(str::is_empty)
            || first_value != second_value
            || document.matches(first).count() != 1
            || document.matches(second).count() != 1
        {
            errors.push(format!("{label} has invalid social {name} metadata"));
        }
    }
}

fn quoted_attribute_value<'a>(document: &'a str, marker: &str) -> Option<&'a str> {
    let start = document.find(marker)? + marker.len();
    let end = document[start..].find('"')? + start;
    Some(&document[start..end])
}

fn escape_html_attr(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn parse_json_ld(document: &str, label: &str, errors: &mut Vec<String>) -> Option<Value> {
    let marker = "<script type=\"application/ld+json\">";
    if document.matches(marker).count() != 1 {
        errors.push(format!("{label} does not have exactly one JSON-LD record"));
        return None;
    }
    let start = document.find(marker)? + marker.len();
    let Some(end) = document[start..]
        .find("</script>")
        .map(|offset| start + offset)
    else {
        errors.push(format!("{label} JSON-LD record is not terminated"));
        return None;
    };
    match serde_json::from_str(&document[start..end]) {
        Ok(value) => Some(value),
        Err(_) => {
            errors.push(format!("{label} JSON-LD record is invalid"));
            None
        }
    }
}

fn schema_graph<'a>(
    value: &'a Value,
    label: &str,
    errors: &mut Vec<String>,
) -> Option<&'a [Value]> {
    if value.get("@context").and_then(Value::as_str) != Some("https://schema.org") {
        errors.push(format!("{label} has invalid JSON-LD context"));
    }
    let Some(graph) = value.get("@graph").and_then(Value::as_array) else {
        errors.push(format!("{label} has no JSON-LD graph"));
        return None;
    };
    Some(graph)
}

fn validate_base_schema(graph: &[Value], label: &str, errors: &mut Vec<String>) {
    let organization = graph
        .iter()
        .find(|node| node.get("@id").and_then(Value::as_str) == Some(ORGANIZATION_ID));
    if organization.is_none_or(|node| {
        node.get("@type").and_then(Value::as_str) != Some("Organization")
            || node.get("name").and_then(Value::as_str) != Some("Merely LLC")
    }) {
        errors.push(format!("{label} has invalid organization structured data"));
    }
    let website = graph
        .iter()
        .find(|node| node.get("@id").and_then(Value::as_str) == Some(WEBSITE_ID));
    if website.is_none_or(|node| {
        node.get("@type").and_then(Value::as_str) != Some("WebSite")
            || node.pointer("/publisher/@id").and_then(Value::as_str) != Some(ORGANIZATION_ID)
    }) {
        errors.push(format!("{label} has invalid website structured data"));
    }
}

fn validate_sitemap(sitemap: &str, expected: &[String], errors: &mut Vec<String>) -> usize {
    if !sitemap.starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n")
        || !sitemap.contains("<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">")
        || !sitemap.ends_with("</urlset>\n")
    {
        errors.push("sitemap does not use the approved XML envelope".to_owned());
    }
    for unsupported in ["<lastmod>", "<changefreq>", "<priority>"] {
        if sitemap.contains(unsupported) {
            errors.push("sitemap contains unsupported freshness metadata".to_owned());
            break;
        }
    }

    let loc = Regex::new(r"<loc>([^<]+)</loc>").expect("valid sitemap location regex");
    let actual = loc
        .captures_iter(sitemap)
        .filter_map(|capture| capture.get(1).map(|value| value.as_str().to_owned()))
        .collect::<Vec<_>>();
    let unique = actual.iter().collect::<BTreeSet<_>>();
    if actual != expected
        || unique.len() != actual.len()
        || actual
            .iter()
            .any(|url| !url.starts_with("https://mer3ly.net/"))
    {
        errors.push("sitemap URLs differ from canonical public authority".to_owned());
    }
    actual.len()
}

fn parse_graph_payload(document: &str, errors: &mut Vec<String>) -> Option<GraphPayload> {
    let marker = "<script id=\"repository-graph-data\" type=\"application/json\">";
    let Some(start) = document.find(marker).map(|offset| offset + marker.len()) else {
        errors.push("repository page is missing graph authority data".to_owned());
        return None;
    };
    let Some(end) = document[start..]
        .find("</script>")
        .map(|offset| start + offset)
    else {
        errors.push("repository graph authority data is not terminated".to_owned());
        return None;
    };
    match serde_json::from_str(&document[start..end]) {
        Ok(payload) => Some(payload),
        Err(error) => {
            errors.push(format!(
                "repository graph authority data is invalid: {error}"
            ));
            None
        }
    }
}

fn inline_json<'a>(document: &'a str, id: &str) -> Option<&'a str> {
    let marker = format!("<script id=\"{id}\" type=\"application/json\">");
    let start = document.find(&marker)? + marker.len();
    let end = document[start..].find("</script>")? + start;
    Some(&document[start..end])
}

fn attribute_values(document: &str, attribute: &str) -> Vec<String> {
    let pattern = format!(r#"{attribute}="([^"]+)""#);
    Regex::new(&pattern)
        .expect("valid generated attribute regex")
        .captures_iter(document)
        .filter_map(|captures| captures.get(1).map(|value| value.as_str().to_owned()))
        .collect()
}

fn validate_static_authority(
    repository_ids: &[String],
    relation_ids: &[String],
    authority: &Authority,
    errors: &mut Vec<String>,
) {
    let expected_repositories = authority
        .repositories
        .repository
        .iter()
        .filter(|repository| repository.public)
        .map(|repository| repository.id.as_str())
        .collect::<BTreeSet<_>>();
    let actual_repositories = repository_ids
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    if repository_ids.len() != expected_repositories.len()
        || actual_repositories != expected_repositories
    {
        errors.push("public artifact repository ids differ from authority".to_owned());
    }

    let expected_relations = authority
        .relations
        .relation
        .iter()
        .map(|relation| (relation.id.as_str(), 2_usize))
        .collect::<BTreeMap<_, _>>();
    let mut actual_relations = BTreeMap::new();
    for relation_id in relation_ids {
        *actual_relations.entry(relation_id.as_str()).or_insert(0) += 1;
    }
    if actual_relations != expected_relations {
        errors.push("public artifact relation text ids differ from authority".to_owned());
    }
}

fn validate_graph_payload(payload: &GraphPayload, authority: &Authority, errors: &mut Vec<String>) {
    if payload.schema != GRAPH_SCHEMA {
        errors.push("repository graph authority schema is not approved".to_owned());
    }
    let expected_nodes = authority
        .repositories
        .repository
        .iter()
        .filter(|repository| repository.public)
        .map(|repository| repository.id.as_str())
        .collect::<BTreeSet<_>>();
    let actual_nodes = payload
        .nodes
        .iter()
        .map(|node| node.id.as_str())
        .collect::<BTreeSet<_>>();
    if actual_nodes != expected_nodes {
        errors.push("repository graph node ids differ from authority".to_owned());
    }

    let expected_edges = authority
        .relations
        .relation
        .iter()
        .map(|edge| edge.id.as_str())
        .collect::<BTreeSet<_>>();
    let actual_edges = payload
        .edges
        .iter()
        .map(|edge| edge.id.as_str())
        .collect::<BTreeSet<_>>();
    if actual_edges != expected_edges {
        errors.push("repository graph edge ids differ from authority".to_owned());
    }
    if payload.edges.iter().any(|edge| {
        !actual_nodes.contains(edge.source.as_str()) || !actual_nodes.contains(edge.target.as_str())
    }) {
        errors.push("repository graph contains an edge with an unknown endpoint".to_owned());
    }
}

fn validate_project_authority(
    project_ids: &[String],
    relation_ids: &[String],
    authority: &Authority,
    errors: &mut Vec<String>,
) {
    let expected_projects = authority
        .repositories
        .repository
        .iter()
        .filter(|repository| repository.public)
        .map(|repository| (repository.id.as_str(), 1_usize))
        .collect::<BTreeMap<_, _>>();
    let mut actual_projects = BTreeMap::new();
    for project_id in project_ids {
        *actual_projects.entry(project_id.as_str()).or_insert(0) += 1;
    }
    if actual_projects != expected_projects {
        errors.push("project profile ids differ from repository authority".to_owned());
    }

    let expected_relations = authority
        .relations
        .relation
        .iter()
        .map(|relation| (relation.id.as_str(), 2_usize))
        .collect::<BTreeMap<_, _>>();
    let mut actual_relations = BTreeMap::new();
    for relation_id in relation_ids {
        *actual_relations.entry(relation_id.as_str()).or_insert(0) += 1;
    }
    if actual_relations != expected_relations {
        errors.push("project profile relation ids differ from relation authority".to_owned());
    }
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_text_scan_accepts_approved_github_links() {
        let allowed = BTreeSet::from(["merely-made/mere".to_owned()]);
        let mut errors = Vec::new();
        scan_public_text(
            "index.html",
            "mailto:markik@mer3ly.net https://github.com/merely-made https://github.com/merely-made/mere",
            &allowed,
            &mut errors,
        );
        assert!(errors.is_empty(), "{errors:?}");
    }

    #[test]
    fn public_text_scan_rejects_private_boundaries_without_echoing_values() {
        let allowed = BTreeSet::from(["merely-made/mere".to_owned()]);
        let mut errors = Vec::new();
        scan_public_text(
            "index.html",
            "C:\\Users\\person\\secret 192.168.1.4 person@example.com https://github.com/private-owner/private-repo",
            &allowed,
            &mut errors,
        );
        assert!(errors.len() >= 4);
        let joined = errors.join("\n");
        assert!(!joined.contains("person"));
        assert!(!joined.contains("192.168.1.4"));
        assert!(!joined.contains("private-owner"));
    }
}
