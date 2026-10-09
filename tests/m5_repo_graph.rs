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
    assert!(document.contains("data-sandbox-clear-facets"));
    assert!(document.contains("data-sandbox-camera=\"pan-right\""));
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
        "projectionSourceAdapter",
        "projectionSourceId",
        "projectionInstance",
        "composeMatrixShelfmark",
        "resolveMatrixShelfmark",
        "buildRepeatedAppearances",
        "applyCoordinatedSelection",
        "clearMatrixFilter",
        "clearFacets",
        "selectFacet",
        "changeCamera",
        "spatialRequest",
        "shelfmarkPlacement",
        "mer3ly.camera",
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
        ".graph-sandbox-camera-control",
        "[data-source-id].is-facet-selected",
    ] {
        assert!(
            SITE_CSS.contains(contract),
            "sandbox CSS is missing {contract}"
        );
    }
    // The loader is served as its own asset rather than inlined, so this is a
    // growth guard rather than a page-payload constraint. It sat at 99.3% of
    // the old 64 KiB ceiling, which left no room for the narrow-screen tools
    // drawer; raised to keep the guard meaningful instead of removing it.
    assert!(
        GRAPH_SANDBOX.len() < 72 * 1024,
        "sandbox loader is too large: {} bytes",
        GRAPH_SANDBOX.len()
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
    assert!(
        page.html
            .contains("data-sandbox-runtime=\"/graph-sandbox.js?v=")
    );
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
