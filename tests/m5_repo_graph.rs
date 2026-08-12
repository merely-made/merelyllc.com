use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use mer3ly_site::pages::repositories;
use mer3ly_site::repositories::PublicSiteData;
use mer3ly_site::site::SITE_CSS;
use serde::Deserialize;

const GRAPH_LOADER: &str = include_str!("../assets/repo-graph.js");
const GRAPH_SANDBOX: &str = include_str!("../assets/graph-sandbox.js");
const GRAPH_GLUE: &str = include_str!("../assets/mer3ly_repo_graph.js");
const GRAPH_WASM: &[u8] = include_bytes!("../assets/mer3ly_repo_graph_bg.wasm");

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

#[derive(Deserialize)]
struct GraphAuthority {
    schema: String,
    nodes: Vec<GraphNode>,
    edges: Vec<GraphEdge>,
    feed: Vec<GraphEvent>,
    history: Option<GraphHistory>,
}

#[derive(Deserialize)]
struct GraphEvent {
    id: String,
    repository: String,
}

#[derive(Deserialize)]
struct GraphHistory {
    schema: String,
    checkpoints: Vec<GraphHistoryCheckpoint>,
}

#[derive(Deserialize)]
struct GraphHistoryCheckpoint {
    availability: String,
    cursor: GraphHistoryCursor,
    graph: Option<GraphHistoryGraph>,
}

#[derive(Deserialize)]
struct GraphHistoryCursor {
    source: String,
    commit: String,
    committed_at: String,
}

#[derive(Deserialize)]
struct GraphHistoryGraph {
    nodes: Vec<GraphNode>,
    edges: Vec<GraphEdge>,
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

fn sandbox_authority(document: &str) -> serde_json::Value {
    let marker = "<script id=\"graph-sandbox-data\" type=\"application/json\">";
    let start = document.find(marker).expect("graph sandbox bootstrap") + marker.len();
    let end = document[start..]
        .find("</script>")
        .map(|offset| start + offset)
        .expect("graph sandbox bootstrap end");
    serde_json::from_str(&document[start..end]).expect("valid sandbox authority JSON")
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
    assert!(document.contains("Copy shareable repository scene link"));

    let history = graph
        .history
        .as_ref()
        .expect("repository page includes a Git authority history projection");
    assert_eq!(history.schema, "mer3ly.repository-git-history/v1");
    assert!(!history.checkpoints.is_empty());
    assert!(
        history
            .checkpoints
            .iter()
            .any(|checkpoint| checkpoint.availability == "available"),
        "history retains at least one usable committed authority checkpoint"
    );
    assert!(
        history.checkpoints.iter().all(|checkpoint| {
            checkpoint.availability == "available" || checkpoint.availability == "unavailable"
        }),
        "history only exposes explicit checkpoint availability"
    );
    let available_history = history
        .checkpoints
        .iter()
        .filter(|checkpoint| checkpoint.availability == "available")
        .collect::<Vec<_>>();
    assert!(
        available_history.len() >= 6,
        "history retains public source eras"
    );
    assert_eq!(
        available_history[0].cursor.source, "merely-made/mere",
        "the earliest historical snapshot identifies its public source"
    );
    assert!(
        available_history[0]
            .graph
            .as_ref()
            .expect("available checkpoint includes a graph")
            .nodes
            .iter()
            .any(|node| node.id == "graphshell")
    );
    assert!(available_history.iter().any(|checkpoint| {
        checkpoint
            .graph
            .as_ref()
            .expect("available checkpoint includes a graph")
            .nodes
            .iter()
            .any(|node| node.id == "webrender-wgpu")
    }));
    assert!(available_history.iter().any(|checkpoint| {
        checkpoint.cursor.commit == "020170dcc9d526edddbfe5ea3788975498f27281"
            && checkpoint
                .graph
                .as_ref()
                .expect("available checkpoint includes a graph")
                .nodes
                .iter()
                .all(|node| node.id != "graphshell")
    }));
    let latest = available_history
        .last()
        .expect("live historical checkpoint");
    assert!(!latest.cursor.committed_at.is_empty());
    let latest_graph = latest.graph.as_ref().expect("live graph");
    assert_eq!(
        latest_graph
            .nodes
            .iter()
            .map(|node| node.id.as_str())
            .collect::<BTreeSet<_>>(),
        node_ids,
        "the final history snapshot is fresh current authority"
    );
    assert!(latest_graph.edges.len() >= graph.edges.len());
}

#[test]
fn graph_enhancement_preserves_the_visible_static_fallback() {
    let root = workspace_root();
    let data = PublicSiteData::load(&root).expect("load validated public site data");
    let document = repositories::document(&root).expect("render repository page");
    let fallback = document
        .find("data-graph-fallback")
        .expect("visible graph fallback");
    let interface = document
        .find("data-graph-interface")
        .expect("hidden graph interface");
    let index = document
        .find("class=\"content-section repository-index\"")
        .expect("semantic repository index");

    assert!(fallback < interface);
    assert!(interface < index);
    assert!(document[interface..].contains("hidden=\"hidden\""));
    assert_eq!(
        document.match_indices("data-repository-id=").count(),
        data.authority.repositories.repository.len()
    );
    assert!(document.contains("The complete repository index remains available below."));
}

#[test]
fn graph_runtime_covers_interaction_and_failure_contracts() {
    for contract in [
        "navigator.gpu",
        "layoutGraph(JSON.stringify(authority))",
        "runtimeVersion",
        "mer3ly_repo_graph_bg.wasm${runtimeVersion}",
        "dataset.graphState",
        "visibilitychange",
        "requestAnimationFrame",
        "MORPH_DURATION_MS",
        "TIMELINE_ARRANGEMENT",
        "SCENE_PROFILES",
        "dataset.graphArrangement",
        "dataset.graphMorphing",
        "dataset.graphNodeForm",
        "dataset.graphScaffold",
        "validateHistory",
        "replaceLayout",
        "[data-graph-history]",
        "return-live",
        "repository-scene",
        "requested repository source cursor is unavailable",
        "Live authority",
        "updateSceneScaffold",
        "repository-graph-kanban-lane",
        "repository-graph-timeline-rail",
        "repository-graph-timeline-stem",
        "repository-graph-timeline-anchor",
        "repository-graph-facet-cell",
        "repository-graph-branch",
        "[data-graph-arrangement]",
        "Morphing into the",
        "prefers-reduced-motion: reduce",
        "aria-pressed",
        "ArrowLeft",
        "ArrowRight",
        "End",
        "Home",
        "Enter",
        "pointerdown",
        "\"wheel\"",
        "window.location.assign",
        "dataset.projectHref",
        "no-webgpu",
        "no-wasm",
        "init-failure",
        "\"motion\") === \"reduce\"",
    ] {
        assert!(
            GRAPH_LOADER.contains(contract),
            "graph loader is missing {contract}"
        );
    }

    for forbidden in [
        "Personae",
        "browser history",
        "resident host",
        "C:\\Users\\",
        "mark_",
    ] {
        assert!(
            !GRAPH_LOADER.contains(forbidden)
                && !GRAPH_GLUE.contains(forbidden)
                && !String::from_utf8_lossy(GRAPH_WASM).contains(forbidden),
            "graph runtime contains forbidden marker {forbidden:?}"
        );
    }
}

#[test]
fn graphshell_sandbox_keeps_scene_arrangement_motion_and_backdrop_distinct() {
    let root = workspace_root();
    let document = repositories::document(&root).expect("render repository page");
    let sandbox = sandbox_authority(&document);
    let classes = sandbox["nodes"]
        .as_array()
        .expect("sandbox nodes")
        .iter()
        .filter_map(|node| node["class"].as_str())
        .collect::<BTreeSet<_>>();

    assert!(
        classes.len() >= 8,
        "the specimen graph is meaningfully heterogeneous"
    );
    assert_eq!(sandbox["sandbox"]["schema"], "mer3ly.graphshell-sandbox/v1");
    assert!(document.contains("data-graph-sandbox"));
    assert!(document.contains("One graph, several readings, real Mere physics."));
    assert!(document.contains("Graphshell projection sandbox, not the whole browser shell"));

    for contract in [
        "new GraphPhysics",
        "setArrangement",
        "setBackdrop",
        "pinNode",
        "unpinNode",
        "graph_layout:stack",
        "graph_layout:radial",
        "recomputeNeighborhood",
        "buildMatrix",
        "dataset.sandboxScene",
        "physics.tick",
        "ResizeObserver",
    ] {
        assert!(
            GRAPH_SANDBOX.contains(contract),
            "sandbox runtime is missing {contract}"
        );
    }
    for contract in [
        ".graph-sandbox-contract",
        ".graph-sandbox-node.class-event",
        ".graph-sandbox-node.class-document",
        "[data-sandbox-scene=\"changes\"]",
        ".graph-sandbox-matrix-cell.has-relation",
    ] {
        assert!(
            SITE_CSS.contains(contract),
            "sandbox CSS is missing {contract}"
        );
    }
    assert!(
        GRAPH_SANDBOX.len() < 32 * 1024,
        "sandbox loader is too large"
    );
}

#[test]
fn graph_assets_and_responsive_styles_are_bounded() {
    assert_eq!(&GRAPH_WASM[..4], b"\0asm");
    // The module now includes Seiche and Rapier rather than a positional-layout-only
    // adapter. Keep a deliberate raw ceiling while accepting the real physics world.
    assert!(
        GRAPH_WASM.len() < 900 * 1024,
        "graph + physics Wasm is {} bytes",
        GRAPH_WASM.len()
    );
    assert!(
        GRAPH_LOADER.len() + GRAPH_GLUE.len() + GRAPH_WASM.len() < 1_000 * 1024,
        "graph + physics runtime is {} bytes",
        GRAPH_LOADER.len() + GRAPH_GLUE.len() + GRAPH_WASM.len()
    );

    for contract in [
        ".repository-graph-interface[hidden]",
        ".repository-graph-node:focus-visible",
        ".repository-graph-node.is-selected",
        "touch-action: manipulation",
        ".repository-graph-arrangement-picker",
        ".repository-graph-history-picker",
        ".repository-graph-scene-caption",
        "[data-graph-node-form=\"card\"]",
        "[data-graph-node-form=\"flag\"]",
        "min-height: 44px",
        "[data-graph-node-form=\"facet\"]",
        "[data-graph-node-form=\"leaf\"]",
        "@media (max-width: 760px)",
        "@media (max-width: 440px)",
        "@media (prefers-reduced-motion: reduce)",
        ".repository-graph-section",
    ] {
        assert!(
            SITE_CSS.contains(contract),
            "site CSS is missing {contract}"
        );
    }
}
