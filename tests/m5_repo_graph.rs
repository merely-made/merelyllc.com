use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use mer3ly_site::pages::repositories;
use mer3ly_site::repositories::PublicSiteData;
use mer3ly_site::site::SITE_CSS;
use serde::Deserialize;

const GRAPH_SANDBOX: &str = include_str!("../assets/graph-sandbox.js");
const GRAPH_SANDBOX_MOUNT: &str = include_str!("../assets/graph-sandbox-mount.js");
const GRAPH_GLUE: &str = include_str!("../assets/mer3ly_repo_graph.js");
const GRAPH_WASM: &[u8] = include_bytes!("../assets/mer3ly_repo_graph_bg.wasm");

/// Gzip bytes at best compression: what a visitor downloads (Ruling 141).
fn gzip(bytes: &[u8]) -> usize {
    use std::io::Write as _;
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::best());
    encoder.write_all(bytes).expect("gzip in memory");
    encoder.finish().expect("gzip in memory").len()
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

#[derive(Deserialize)]
struct GraphAuthority {
    schema: String,
    nodes: Vec<GraphNode>,
    edges: Vec<GraphEdge>,
    feed: Vec<GraphEvent>,
    history: Option<serde_json::Value>,
}

#[derive(Deserialize)]
struct GraphEvent {
    id: String,
    repository: String,
}

#[derive(Deserialize)]
struct HostHistory {
    schema: String,
    revisions: Vec<HostHistoryRevision>,
    compared_fields: Vec<String>,
    compared_relationship_fields: Vec<String>,
}

#[derive(Deserialize)]
struct HostHistoryRevision {
    revision: String,
    dataset: HostHistoryDataset,
    relationships: Vec<serde_json::Value>,
}

#[derive(Deserialize)]
struct HostHistoryDataset {
    occurrences: Vec<HostHistoryOccurrence>,
}

#[derive(Deserialize)]
struct HostHistoryOccurrence {
    occurrence_id: String,
}

#[derive(Deserialize)]
struct GraphHistoryCursor {
    source: String,
    commit: String,
    committed_at: String,
}

#[derive(Deserialize)]
struct GraphNode {
    id: String,
}

#[derive(Deserialize)]
struct GraphEdge {
    id: String,
    source: String,
    target: String,
}

fn graph_authority(document: &str) -> GraphAuthority {
    let marker = "<script id=\"repository-graph-data\" type=\"application/json\">";
    let start = document.find(marker).expect("repository graph bootstrap") + marker.len();
    let end = document[start..]
        .find("</script>")
        .map(|offset| start + offset)
        .expect("repository graph bootstrap end");
    serde_json::from_str(&document[start..end]).expect("valid graph authority JSON")
}

#[test]
fn graph_and_semantic_index_share_exact_public_ids() {
    let root = workspace_root();
    let data = PublicSiteData::load(&root).expect("load validated public site data");
    let document = repositories::document(&root).expect("render repository page");
    let graph = graph_authority(&document);

    assert_eq!(graph.schema, "mer3ly.repo-graph/v1");
    assert_eq!(
        graph
            .nodes
            .iter()
            .map(|node| node.id.as_str())
            .collect::<BTreeSet<_>>(),
        data.authority
            .repositories
            .repository
            .iter()
            .filter(|repository| repository.public)
            .map(|repository| repository.id.as_str())
            .collect::<BTreeSet<_>>()
    );
    assert_eq!(
        graph
            .edges
            .iter()
            .map(|edge| edge.id.as_str())
            .collect::<BTreeSet<_>>(),
        data.authority
            .relations
            .relation
            .iter()
            .map(|relation| relation.id.as_str())
            .collect::<BTreeSet<_>>()
    );

    let node_ids = graph
        .nodes
        .iter()
        .map(|node| node.id.as_str())
        .collect::<BTreeSet<_>>();
    for edge in &graph.edges {
        assert!(node_ids.contains(edge.source.as_str()));
        assert!(node_ids.contains(edge.target.as_str()));
    }
    assert_eq!(graph.feed.len(), data.metadata.event.len());
    assert!(graph.feed.iter().all(|event| {
        !event.id.is_empty()
            && data
                .authority
                .repositories
                .repository
                .iter()
                .any(|repository| repository.github_slug == event.repository)
    }));
    for repository in &data.authority.repositories.repository {
        assert!(document.contains(&format!("id=\"repo-{}\"", repository.id)));
        assert!(document.contains(&format!("data-repository-id=\"{}\"", repository.id)));
    }
    assert!(document.contains("Open the live graph"));

    // The checkpoint history no longer rides inline: it is the v2 history
    // the sandbox fetches on first interaction (Ruling 157).
    assert!(graph.history.is_none());
    let page = repositories::page(&root).expect("render repository page");
    let history: HostHistory =
        serde_json::from_str(&page.frozen.history_json).expect("valid v2 history JSON");
    assert_eq!(history.schema, "scenomise.host-dataset/v2");
    assert_eq!(
        history.compared_fields,
        ["label", "class", "status", "pushed_at"]
    );
    assert_eq!(history.compared_relationship_fields, ["endpoints", "kind"]);
    assert!(
        history.revisions.len() >= 6,
        "history retains public source eras"
    );
    let cursor = |revision: &HostHistoryRevision| -> GraphHistoryCursor {
        serde_json::from_str(&revision.revision).expect("a revision is its checkpoint cursor")
    };
    let has = |revision: &HostHistoryRevision, id: &str| {
        revision
            .dataset
            .occurrences
            .iter()
            .any(|occurrence| occurrence.occurrence_id == id)
    };
    let earliest = &history.revisions[0];
    assert_eq!(
        cursor(earliest).source,
        "merely-made/mere",
        "the earliest historical snapshot identifies its public source"
    );
    assert!(has(earliest, "graphshell"));
    assert!(
        history
            .revisions
            .iter()
            .any(|revision| has(revision, "webrender-wgpu"))
    );
    assert!(history.revisions.iter().any(|revision| {
        cursor(revision).commit == "020170dcc9d526edddbfe5ea3788975498f27281"
            && !has(revision, "graphshell")
    }));
    let latest = history
        .revisions
        .last()
        .expect("live historical checkpoint");
    assert!(!cursor(latest).committed_at.is_empty());
    assert_eq!(
        latest
            .dataset
            .occurrences
            .iter()
            .map(|occurrence| occurrence.occurrence_id.as_str())
            .collect::<BTreeSet<_>>(),
        node_ids,
        "the final history snapshot is fresh current authority"
    );
    assert!(latest.relationships.len() >= graph.edges.len());
}

#[test]
fn graphshell_is_the_only_live_canvas_and_preserves_the_static_index() {
    let root = workspace_root();
    let data = PublicSiteData::load(&root).expect("load validated public site data");
    let document = repositories::document(&root).expect("render repository page");
    let fallback = document
        .find("data-sandbox-fallback")
        .expect("visible sandbox fallback");
    let interface = document
        .find("data-sandbox-interface")
        .expect("hidden sandbox interface");
    let index = document
        .find("class=\"content-section repository-index\"")
        .expect("semantic repository index");

    assert!(interface < fallback);
    assert!(interface < index);
    assert!(document[interface..].contains("hidden=\"hidden\""));
    assert_eq!(
        document.match_indices("data-repository-id=").count(),
        data.authority.repositories.repository.len()
    );
    assert!(!document.contains("data-repository-graph"));
    assert!(!document.contains("/repo-graph.js"));
    assert!(document.contains("semantic repository index remains available below"));
}

#[test]
fn qualified_viewer_is_the_lazy_owner() {
    let page = repositories::document(&workspace_root()).unwrap();
    assert!(page.contains(&mer3ly_site::viewer::href("viewer.js")));
    assert!(page.contains(&mer3ly_site::viewer::href("styles.css")));
    assert!(!page.contains("graph-sandbox-data"));
    assert!(!page.contains("data-sandbox-cycle"));
    let mut mounting = gzip(GRAPH_SANDBOX_MOUNT.as_bytes());
    for (name, bytes) in mer3ly_site::viewer::FILES {
        match *name {
            "pkg/graphshell_web_bg.wasm" => assert!(gzip(bytes) <= 3_407_872),
            "pkg/graphshell_web.js" => assert!(gzip(bytes) <= 20_480),
            "viewer.js" | "mount.js" | "styles.css" => mounting += gzip(bytes),
            _ => (),
        }
    }
    assert!(
        mounting <= 8192,
        "viewer mounting and styles: {mounting} gzip bytes"
    );
}

#[test]
fn graph_assets_and_responsive_styles_are_bounded() {
    assert_eq!(&GRAPH_WASM[..4], b"\0asm");
    // The module carries Seiche and Rapier rather than a positional-layout-only
    // adapter, the portable projection path since 2026-08-16, and since
    // 2026-10-08 the projection proof's session (Rulings 138 and 140):
    // chirograph, scenotime and incipit readers instead of JS copies.
    //
    // The ceilings are deliberate, and another increase needs measurement.
    // Until 2026-10-08 they were raw bytes (1,350 KiB of Wasm, 1,400 KiB of
    // runtime). At Mark's ruling (Ruling 141) they are gzip bytes, what a
    // visitor downloads. At the change the module measured 430,936 B gzip
    // (opt-level "z", fat LTO) and the runtime 452,481 B, against 437,734 B
    // and about 458,400 B before the session moved in. The bounds keep about
    // 30 KiB of headroom each.
    let wasm = gzip(GRAPH_WASM);
    let runtime = gzip(GRAPH_SANDBOX.as_bytes()) + gzip(GRAPH_GLUE.as_bytes()) + wasm;
    assert!(
        wasm < 450 * 1024,
        "graph + physics + portable projection Wasm is {wasm} bytes gzip"
    );
    assert!(
        runtime < 475 * 1024,
        "graph + physics + portable projection runtime is {runtime} bytes gzip"
    );

    for contract in [
        ".graph-sandbox-control-actors",
        ".graph-sandbox-node-detail",
        ".graph-sandbox-matrix",
        "@media (max-width: 760px)",
        "@media (max-width: 440px)",
        "@media (prefers-reduced-motion: reduce)",
    ] {
        assert!(
            SITE_CSS.contains(contract),
            "site CSS is missing {contract}"
        );
    }
}

#[test]
fn repositories_first_load_and_its_lazy_tier_are_bounded() {
    // Ruling 157: /repos/ opens on a frozen first view, and the live sandbox
    // loads on first interaction. Gzip bytes at best compression, as m5's
    // graph bounds are (Ruling 141).
    let page = repositories::page(&workspace_root()).expect("render repository page");

    // Nothing in the first load reaches the graph runtime: the page loads the
    // mount script and cites the sandbox and the history for later.
    assert!(
        !page
            .html
            .contains("<script type=\"module\" src=\"/graph-sandbox.js")
    );
    assert!(!page.html.contains("mer3ly_repo_graph"));
    assert!(page.html.contains(&format!(
        "<script type=\"module\" src=\"{}\"></script>",
        repositories::graph_sandbox_mount_href()
    )));
    assert!(page.html.contains("data-sandbox-runtime=\"/graphshell/"));
    assert!(
        page.html
            .contains("data-sandbox-history-src=\"/repository-host-history.json?v=")
    );

    let html = gzip(page.html.as_bytes());
    let css = gzip(SITE_CSS.as_bytes());
    let mount = gzip(GRAPH_SANDBOX_MOUNT.as_bytes());
    let first_load = html + css + mount;
    let history = gzip(page.frozen.history_json.as_bytes());
    eprintln!(
        "/repos/ first load {first_load} B gzip (HTML {html}, site.css {css}, mount {mount}); v2 history {history} B gzip"
    );
    assert!(
        first_load <= 64 * 1024,
        "the /repos/ first load is {first_load} bytes gzip"
    );
    assert!(
        mount <= 8 * 1024,
        "the sandbox mount script is {mount} bytes gzip"
    );
    assert!(
        history <= mer3ly_site::host_history::HOST_HISTORY_GZIP_LIMIT,
        "the v2 repository history is {history} bytes gzip"
    );
}

#[test]
fn viewer_history_discloses_current_links_without_rewriting_older_checkpoints() {
    use scenomise::projection::ProjectionValue;
    let root = workspace_root();
    let data = PublicSiteData::load(&root).unwrap();
    let page = repositories::page(&root).unwrap();
    let history = &page.frozen.history;
    let authority: serde_json::Value = serde_json::from_str(
        &mer3ly_site::host_dataset::repository_host_dataset_json(&data.authority, &data.metadata)
            .unwrap(),
    )
    .unwrap();
    for occurrence in &history.current().dataset.occurrences {
        let original = authority["dataset"]["occurrences"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["occurrence_id"] == occurrence.occurrence_id)
            .unwrap();
        for field in ["summary", "project_url", "github_url"] {
            assert_eq!(
                serde_json::to_value(&occurrence.values[field]).unwrap(),
                original["values"][field]
            );
        }
    }
    for revision in &history.revisions[..history.revisions.len() - 1] {
        for field in ["summary", "project_url", "github_url"] {
            assert!(!revision.dataset.fields.contains_key(field));
            assert!(
                revision
                    .dataset
                    .occurrences
                    .iter()
                    .all(|item| !item.values.contains_key(field))
            );
        }
    }
    assert_eq!(
        history.compared_fields.as_ref().unwrap(),
        &vec!["label", "class", "status", "pushed_at"]
    );
    let checkpoints = repositories::site_history(&root, &data.authority, &data.metadata).unwrap();
    let mut mismatched = history.current().dataset.clone();
    mismatched.occurrences[0].values.insert(
        "label".into(),
        ProjectionValue::Text("Wrong checkpoint".into()),
    );
    assert!(
        mer3ly_site::host_history::repository_viewer_history_json(&checkpoints, &mismatched)
            .is_err()
    );
}
