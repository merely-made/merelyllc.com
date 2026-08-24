use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use mer3ly_site::pages::repositories;
use mer3ly_site::repositories::PublicSiteData;
use mer3ly_site::site::SITE_CSS;
use serde::Deserialize;

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
    assert!(document.contains("share scene"));

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

    assert!(fallback < interface);
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
fn graphshell_sandbox_keeps_truth_face_arrangement_and_motion_distinct() {
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
    assert_eq!(sandbox["sandbox"]["schema"], "mer3ly.graphshell-sandbox/v5");
    assert_eq!(sandbox["sandbox"]["scene_state_schema"], "mere.shelfmark/1");
    assert_eq!(
        sandbox["sandbox"]["reading_registry_schema"],
        "mere.graph-reading-registry/v1"
    );
    assert_eq!(
        sandbox["sandbox"]["representation_registry_schema"],
        "mere.graph-representation-registry/v2"
    );
    assert!(document.contains("data-graph-sandbox"));
    assert!(document.contains("The graph is also its own control surface."));
    assert!(document.contains("spreadsheet chart can be another projection"));
    assert!(document.contains("data-sandbox-cycle=\"reading\""));
    assert!(document.contains("data-sandbox-cycle=\"arrangement\""));
    assert!(document.contains("data-sandbox-cycle=\"mobility\""));
    assert!(document.contains("data-sandbox-matrix"));
    assert!(document.contains("data-sandbox-scatter"));
    assert!(document.contains("data-sandbox-deck"));
    assert!(document.contains("data-sandbox-clear-matrix"));
    assert!(!document.contains("data-sandbox-control="));

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
        "projectMatrix",
        "composeMatrixShelfmark",
        "resolveMatrixShelfmark",
        "buildRepeatedAppearances",
        "applyCoordinatedSelection",
        "clearMatrixFilter",
        "dataset.sandboxScene",
        "dataset.sandboxFace",
        "READING_FACES",
        "controlActors",
        "cycleControl",
        "updateSelectionFaces",
        "readingRegistry",
        "projectReading",
        "representationRegistry",
        "encodeSceneState",
        "pinsByDataset",
        "physics.tick",
        "ResizeObserver",
    ] {
        assert!(
            GRAPH_SANDBOX.contains(contract),
            "sandbox runtime is missing {contract}"
        );
    }
    for contract in [
        ".graph-sandbox-control-actor",
        ".graph-sandbox-node[data-face=\"delta\"]",
        ".graph-sandbox-node[data-face=\"signal\"]",
        ".graph-sandbox-node[data-face=\"orbit\"]",
        ".graph-sandbox-node.primitive-diamond",
        ".graph-sandbox-node.primitive-square",
        ".graph-sandbox-history",
        ".graph-sandbox-share",
        "[data-sandbox-scene=\"neighbors\"]",
        ".graph-sandbox-matrix-cell.has-relation",
        ".graph-sandbox-receipts",
        ".graph-sandbox-scatter-point",
        ".graph-sandbox-deck-card",
        "[data-source-id].is-filtered-out",
    ] {
        assert!(
            SITE_CSS.contains(contract),
            "sandbox CSS is missing {contract}"
        );
    }
    assert!(
        GRAPH_SANDBOX.len() < 64 * 1024,
        "sandbox loader is too large"
    );
}

#[test]
fn graph_assets_and_responsive_styles_are_bounded() {
    assert_eq!(&GRAPH_WASM[..4], b"\0asm");
    // The module carries Seiche and Rapier rather than a positional-layout-only
    // adapter, and since 2026-08-16 the portable projection path as well:
    // exporting portable_projection_with_placement makes score and scene serde
    // plus scenomise::solve reachable from the browser, which the live path
    // alone never needed.
    //
    // That export was measured before it was accepted, because the ceiling is
    // deliberate and not a formality. Against a 858,444-byte baseline without
    // it: 932,204 bytes stripped of both the demo trace and the self-consume
    // check, and 958,016 with them kept. No variant fit under the old 900 KiB
    // bound, so the choice was to pay for the useful version or drop the
    // feature. The ceilings below are the paid price, with roughly 65 KiB of
    // headroom. Wave 1's Matrix capture, coordinated selection, and composed
    // Shelfmark resolver measured 1,268,979 bytes together. The revised
    // ceilings keep roughly 110 KiB of Wasm headroom and 80 KiB over the whole
    // runtime. They remain a bound, and another increase needs measurement.
    assert!(
        GRAPH_WASM.len() < 1_350 * 1024,
        "graph + physics + portable projection Wasm is {} bytes",
        GRAPH_WASM.len()
    );
    assert!(
        GRAPH_SANDBOX.len() + GRAPH_GLUE.len() + GRAPH_WASM.len() < 1_400 * 1024,
        "graph + physics + portable projection runtime is {} bytes",
        GRAPH_SANDBOX.len() + GRAPH_GLUE.len() + GRAPH_WASM.len()
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
