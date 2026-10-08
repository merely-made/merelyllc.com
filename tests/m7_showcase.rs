use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use genet_scripted_dom::ScriptedDom;
use layout_dom_api::LayoutDom;
use mer3ly_site::pages::{home, projects};
use mer3ly_site::projection_proof::{PROJECTION_STEP_BOUND, ProjectionProof};
use mer3ly_site::repositories::PublicSiteData;
use mer3ly_site::site::SITE_CSS;

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

#[test]
fn showcase_authority_is_bounded_and_ordered() {
    let root = workspace_root();
    let data = PublicSiteData::load(&root).expect("load validated public site data");
    let showcases = data.showcases.ordered();

    assert_eq!(showcases.len(), 5);
    assert_eq!(
        showcases
            .iter()
            .map(|showcase| showcase.repository.as_str())
            .collect::<Vec<_>>(),
        ["mere", "genet", "turnstone", "woodshed", "isocosm"]
    );
    for showcase in showcases {
        assert_eq!(
            showcase.image,
            format!("showcase/{}.png", showcase.repository)
        );
        assert!(
            showcase
                .source_url
                .starts_with("https://github.com/merely-made/")
        );
        assert!(root.join("assets").join(&showcase.image).is_file());
        for (index, extra) in showcase.images.iter().enumerate() {
            assert_eq!(
                extra.image,
                format!("showcase/{}-{}.png", showcase.repository, index + 2)
            );
            assert!(
                extra
                    .source_url
                    .starts_with("https://github.com/merely-made/")
            );
            assert!(!extra.alt.is_empty());
            assert!(root.join("assets").join(&extra.image).is_file());
        }
    }
}

#[test]
fn home_projects_every_showcase_into_a_local_profile() {
    let root = workspace_root();
    let data = PublicSiteData::load(&root).expect("load validated public site data");
    let document = home::document_for(&data);

    for showcase in data.showcases.ordered() {
        assert!(document.contains(&format!("src=\"/{}\"", showcase.image)));
        assert!(document.contains(&format!("href=\"/projects/{}/\"", showcase.repository)));
        assert!(document.contains(&showcase.headline));
    }
    assert!(document.contains("class=\"home-showcase-list\""));
    assert_eq!(document.matches("<h1").count(), 1);

    let dom = ScriptedDom::from_serialized_document(&document);
    let serialized = dom.inner_html(dom.document());
    assert!(serialized.contains("class=\"home-showcase-card\""));
}

#[test]
fn every_public_repository_has_one_semantic_project_profile() {
    let root = workspace_root();
    let data = PublicSiteData::load(&root).expect("load validated public site data");
    let documents = projects::documents(&data);
    let expected_repositories = data
        .authority
        .repositories
        .repository
        .iter()
        .filter(|repository| repository.public)
        .count();
    assert_eq!(documents.len(), expected_repositories);

    let mut relation_counts = BTreeMap::new();
    for (repository_id, document) in &documents {
        assert!(document.starts_with("<!doctype html>"));
        assert_eq!(document.matches("<h1").count(), 1);
        assert!(document.contains(&format!("data-project-id=\"{repository_id}\"")));
        assert!(document.contains(&format!("https://merelyllc.com/projects/{repository_id}/")));
        assert!(document.contains("href=\"mailto:markik@mer3ly.net\""));
        for relation in &data.authority.relations.relation {
            *relation_counts
                .entry(relation.id.as_str())
                .or_insert(0_usize) += document
                .matches(&format!("data-relation-id=\"{}\"", relation.id))
                .count();
        }
    }

    for relation in &data.authority.relations.relation {
        assert_eq!(
            relation_counts.get(relation.id.as_str()),
            Some(&2),
            "relation {} appears once on each endpoint profile",
            relation.id
        );
    }
}

#[test]
fn visual_and_text_only_profiles_state_their_evidence_boundary() {
    let root = workspace_root();
    let mere = projects::document(&root, "mere").expect("render Mere profile");
    let retinue = projects::document(&root, "retinue").expect("render Retinue profile");

    assert!(mere.contains("src=\"/showcase/mere.png\""));
    assert!(mere.contains("Source image:"));
    assert!(mere.contains("License: MIT OR Apache-2.0."));
    assert!(retinue.contains("This profile is intentionally text-first."));
    assert!(!retinue.contains("project-showcase-figure"));
}

#[test]
fn mere_profile_projects_one_authority_into_canvas_and_swatch_views() {
    let root = workspace_root();
    let data = PublicSiteData::load(&root).expect("load validated public site data");
    let mere = projects::document(&root, "mere").expect("render Mere profile");

    assert!(mere.contains("data-projection-proof"));
    assert_eq!(mere.matches("data-projection-view=").count(), 2);
    assert!(mere.contains("data-projection-view=\"canvas\""));
    assert!(mere.contains("data-projection-view=\"swatch\""));
    assert!(mere.contains("<script type=\"module\" src=\"/projection-proof.js?v="));
    assert!(mere.contains("Scenograph supplies the score"));
    assert!(mere.contains("portable scene"));
    assert!(mere.contains("project facts"));

    // Four sibling artifacts replace the retired `mer3ly.*` one (Rulings
    // 132 and 136): the page cites each at its content version.
    let proof = ProjectionProof::build(&data).expect("the proof's artifacts consume and join");
    for ((name, bytes), attribute) in
        proof
            .files()
            .into_iter()
            .zip(["data-capture-src", "data-trace-src", "data-shelfmark-src"])
    {
        let href = ProjectionProof::href(name, bytes);
        assert!(
            mere.contains(&format!("{attribute}=\"{href}\"")),
            "{attribute}"
        );
    }
    assert!(mere.contains("data-dataset-src=\"/repository-host-dataset.json\""));
    assert!(!mere.contains("mere-projection-artifact"));
    assert!(!mere.contains("mer3ly.portable-projection"));

    let receipt = &proof.reading.receipt;
    assert_eq!(receipt.score_items, proof.nodes.len());
    assert_eq!(receipt.active_items, proof.nodes.len());
    assert_eq!(receipt.initial_revision, 1);
    assert_eq!(receipt.picked_source, "mere");
    assert_eq!(receipt.trace_steps, proof.reading.steps.len());
    assert!(receipt.trace_steps <= PROJECTION_STEP_BOUND);
    assert_eq!(
        proof.reading.steps.last().map(|step| step.revision),
        Some(receipt.final_revision)
    );
    assert!(
        proof
            .relations
            .iter()
            .all(|relation| relation.label.contains("Mere")),
        "every captured relation has Mere at one end, labelled by the host dataset"
    );

    // The capture is chirograph's V2. The browser reads it through the graph
    // Wasm's ProjectionSession, the native consumer's own code (Ruling 138),
    // loaded on first interaction under the repositories page's version
    // query (Ruling 140).
    let capture: serde_json::Value =
        serde_json::from_slice(&proof.artifacts.capture).expect("capture JSON");
    assert_eq!(capture["version"], 2);
    assert_eq!(capture["score"]["version"], sceno::SCORE_VERSION);
    let projection_proof = std::fs::read_to_string(root.join("assets/projection-proof.js"))
        .expect("projection proof runtime");
    assert!(projection_proof.contains(&format!("const STEP_BOUND = {PROJECTION_STEP_BOUND};")));
    assert!(projection_proof.contains("new runtime.ProjectionSession(capture, trace, shelfmark)"));
    assert!(projection_proof.contains("import(proofRoot.dataset.graphRuntime)"));
    assert!(!projection_proof.contains("mer3ly.portable-projection"));
    // The JavaScript copies of stack logic are retired.
    for retired in [
        "BLAKE3",
        "blake3",
        "parseLossless",
        "BigInt",
        "snapshotAt(",
        "TraceHistory",
    ] {
        assert!(
            !projection_proof.contains(retired),
            "{retired} is back in the script"
        );
    }
    let (graph_runtime, graph_wasm) = mer3ly_site::pages::repositories::graph_runtime_hrefs();
    assert!(mere.contains(&format!("data-graph-runtime=\"{graph_runtime}\"")));
    assert!(mere.contains(&format!("data-graph-wasm=\"{graph_wasm}\"")));
    assert!(mere.contains("data-replay=\"static\""));
    // The Mere page's first load does not fetch the runtime; only the
    // proof's first interaction does.
    assert!(!mere.contains(&format!("src=\"{graph_runtime}\"")));
    assert!(!mere.contains("modulepreload"));

    // The epoch exceeds 2^53. It travels as text in the shelfmark and as exact
    // digits in the capture and trace, and the browser reads those losslessly.
    let epoch: u64 = proof.reading.epoch.parse().expect("decimal epoch");
    assert!(epoch > 1 << 53, "epoch {epoch} would prove nothing");
    let shelfmark: serde_json::Value =
        serde_json::from_slice(&proof.artifacts.shelfmark).expect("shelfmark JSON");
    assert_eq!(
        shelfmark["inputs"]["authority"]["expects_generation"],
        proof.reading.epoch
    );
    for bytes in [&proof.artifacts.capture, &proof.artifacts.trace] {
        assert!(
            std::str::from_utf8(bytes)
                .expect("UTF-8")
                .contains(&format!("\"epoch\":{epoch}"))
        );
    }
    assert!(mere.contains(&format!("data-scene-epoch=\"{epoch}\"")));
    assert!(projection_proof.contains("dataset.sceneEpoch = this.session.epoch()"));

    // The no-script reading is the trace, read at build time.
    assert_eq!(
        mere.matches("data-projection-reading-step=").count(),
        receipt.trace_steps
    );
    for step in &proof.reading.steps {
        assert!(mere.contains(&step.label), "{}", step.label);
    }

    for repository in data
        .authority
        .repositories
        .repository
        .iter()
        .filter(|repository| repository.public && repository.id != "mere")
    {
        let document = projects::document_for(&data, repository);
        assert!(!document.contains("data-projection-proof"));
        assert!(!document.contains("/projection-proof.js?v="));
    }
    assert!(
        root.join("assets/projection-proof.js")
            .metadata()
            .expect("projection proof asset")
            .len()
            < 40 * 1024
    );
}

#[test]
fn showcase_styles_cover_responsive_images_and_profile_relations() {
    for contract in [
        ".home-showcase-card",
        "object-fit: contain",
        ".project-showcase-layout",
        ".project-relation-columns",
        ".project-facts-layout",
        ".project-profile-hero",
        ".projection-proof-views",
        ".projection-proof-node",
        ".projection-proof-edge-control",
        "@media (max-width: 760px)",
        "@media (max-width: 440px)",
    ] {
        assert!(
            SITE_CSS.contains(contract),
            "site CSS is missing {contract}"
        );
    }
}

#[test]
fn multi_image_showcases_render_every_capture_in_a_rotation() {
    let root = workspace_root();
    let data = PublicSiteData::load(&root).expect("load validated public site data");

    for showcase in data.showcases.ordered() {
        let document = projects::document(&root, &showcase.repository)
            .expect("render showcased project profile");
        if showcase.images.is_empty() {
            assert!(!document.contains("project-showcase-rotation"));
            continue;
        }
        assert!(document.contains("class=\"project-showcase-rotation\""));
        assert!(document.contains("class=\"project-showcase-dots\""));
        assert!(document.contains(&format!("src=\"/{}\"", showcase.image)));
        for extra in &showcase.images {
            assert!(document.contains(&format!("src=\"/{}\"", extra.image)));
            assert!(document.contains(&extra.alt));
            assert!(document.contains(&extra.source_url));
        }
        let dots = document.matches("class=\"project-showcase-dot\"").count();
        assert_eq!(dots, showcase.images.len() + 1);
    }
}
