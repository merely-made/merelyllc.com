use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use mer3ly_site::discovery::{ROBOTS_TXT, sitemap};
use mer3ly_site::host_dataset::{HOST_DATASET_FILE, repository_host_dataset_json};
use mer3ly_site::host_history::HOST_HISTORY_FILE;
use mer3ly_site::message_path;
use mer3ly_site::pages::{devices, home, projects, radio, repositories};
use mer3ly_site::projection_proof::ProjectionProof;
use mer3ly_site::repositories::PublicSiteData;
use mer3ly_site::retinue_traces::TraceSet;
use mer3ly_site::site::{DEVICE_CSS, SITE_CSS};

const FAVICON: &[u8] = include_bytes!("../assets/favicon.svg");
const GRAPH_SANDBOX: &[u8] = include_bytes!("../assets/graph-sandbox.js");
const GRAPH_SANDBOX_MOUNT: &[u8] = include_bytes!("../assets/graph-sandbox-mount.js");
const MESSAGE_PATH_LAB: &[u8] = include_bytes!("../assets/message-path-lab.js");
const OG_IMAGE: &[u8] = include_bytes!("../assets/og.jpg");
const PROJECTION_PROOF: &[u8] = include_bytes!("../assets/projection-proof.js");
const RADIO_SIMULATOR: &[u8] = include_bytes!("../assets/radio-simulator.js");
const RADIO_MIRROR_WASM_GLUE: &[u8] = include_bytes!("../assets/radio_mirror.js");
const RADIO_MIRROR_WASM: &[u8] = include_bytes!("../assets/radio_mirror_bg.wasm");
const REPO_GRAPH_WASM_GLUE: &[u8] = include_bytes!("../assets/mer3ly_repo_graph.js");
const REPO_GRAPH_WASM: &[u8] = include_bytes!("../assets/mer3ly_repo_graph_bg.wasm");

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = output_directory()?;
    build_site(&output)?;
    println!("wrote static site to {}", output.display());
    Ok(())
}

fn output_directory() -> Result<PathBuf, String> {
    let mut args = env::args().skip(1);
    let mut output = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("html");
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--output" => {
                let value = args
                    .next()
                    .ok_or_else(|| "--output requires a directory".to_owned())?;
                output = PathBuf::from(value);
            }
            "-h" | "--help" => {
                println!("Usage: cargo run --bin site -- [--output DIRECTORY]");
                std::process::exit(0);
            }
            other => return Err(format!("unknown argument: {other}")),
        }
    }
    Ok(output)
}

fn build_site(output: &Path) -> std::io::Result<()> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let data = PublicSiteData::load(root).map_err(std::io::Error::other)?;
    fs::create_dir_all(output)?;
    let obsolete_repository_canvas = output.join("repo-graph.js");
    if obsolete_repository_canvas.exists() {
        fs::remove_file(obsolete_repository_canvas)?;
    }
    fs::create_dir_all(output.join("repos"))?;
    fs::create_dir_all(output.join("projects"))?;
    fs::create_dir_all(output.join("devices"))?;
    fs::write(output.join("index.html"), home::document_for(&data))?;
    fs::write(output.join("radio.html"), radio::document())?;
    // The repositories page and the v2 history its sandbox fetches on first
    // interaction (Ruling 157).
    let repositories_page = repositories::page(root).map_err(std::io::Error::other)?;
    fs::write(
        output.join("repos").join("index.html"),
        &repositories_page.html,
    )?;
    fs::write(
        output.join(HOST_HISTORY_FILE),
        &repositories_page.frozen.history_json,
    )?;
    fs::write(
        output.join("devices").join("index.html"),
        devices::index_document_for(&data.devices),
    )?;
    for (device_id, document) in devices::documents(&data) {
        let device_directory = output.join("devices").join(device_id);
        fs::create_dir_all(&device_directory)?;
        fs::write(device_directory.join("index.html"), document)?;
    }
    for (repository_id, document) in projects::documents(&data) {
        let project_directory = output.join("projects").join(repository_id);
        fs::create_dir_all(&project_directory)?;
        fs::write(project_directory.join("index.html"), document)?;
    }
    for showcase in &data.showcases.showcase {
        let images = std::iter::once(showcase.image.as_str())
            .chain(showcase.images.iter().map(|extra| extra.image.as_str()));
        for image in images {
            let source = root.join("assets").join(image);
            let destination = output.join(image);
            if let Some(parent) = destination.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(source, destination)?;
        }
    }
    fs::write(output.join("site.css"), SITE_CSS)?;
    fs::write(output.join("graph-sandbox.js"), GRAPH_SANDBOX)?;
    fs::write(output.join("graph-sandbox-mount.js"), GRAPH_SANDBOX_MOUNT)?;
    fs::write(output.join("devices.css"), DEVICE_CSS)?;
    fs::write(output.join("message-path-lab.js"), MESSAGE_PATH_LAB)?;
    fs::write(output.join("projection-proof.js"), PROJECTION_PROOF)?;
    fs::write(
        output.join(HOST_DATASET_FILE),
        repository_host_dataset_json(&data.authority, &data.metadata)
            .map_err(std::io::Error::other)?,
    )?;
    // The projection proof's capture, trace and shelfmark, beside the host
    // dataset that labels them.
    let proof = ProjectionProof::build(&data).map_err(std::io::Error::other)?;
    for (name, bytes) in proof.files() {
        fs::write(output.join(name), bytes)?;
    }
    fs::write(output.join("radio-simulator.js"), RADIO_SIMULATOR)?;
    fs::write(output.join("radio_mirror.js"), RADIO_MIRROR_WASM_GLUE)?;
    fs::write(output.join("radio_mirror_bg.wasm"), RADIO_MIRROR_WASM)?;
    // The message path lab's committed traces, published unchanged, and its
    // no-script screen.
    let traces = TraceSet::embedded().map_err(|errors| std::io::Error::other(errors.join("\n")))?;
    for (path, bytes) in traces.artifact_files() {
        let destination = output.join(path);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(destination, bytes)?;
    }
    let lab_screen = message_path::static_screen(&traces).map_err(std::io::Error::other)?;
    let screens = mer3ly_radio_mirror::static_screens()
        .map_err(std::io::Error::other)?
        .into_iter()
        .chain([lab_screen]);
    for screen in screens {
        let destination = output.join(&screen.path);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(destination, screen.png)?;
    }
    fs::write(output.join("mer3ly_repo_graph.js"), REPO_GRAPH_WASM_GLUE)?;
    fs::write(output.join("mer3ly_repo_graph_bg.wasm"), REPO_GRAPH_WASM)?;
    fs::write(output.join("og.jpg"), OG_IMAGE)?;
    fs::write(output.join("favicon.svg"), FAVICON)?;
    fs::write(output.join("sitemap.xml"), sitemap(&data))?;
    fs::write(output.join("robots.txt"), ROBOTS_TXT)?;
    fs::write(output.join("CNAME"), "merelyllc.com\n")?;
    Ok(())
}
