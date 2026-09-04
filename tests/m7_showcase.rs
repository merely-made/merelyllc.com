use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use genet_scripted_dom::ScriptedDom;
use layout_dom_api::LayoutDom;
use mer3ly_site::pages::{home, projects};
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
        ["mere", "genet", "turnstone", "woodshed", "isometry"]
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
        assert!(document.contains(&format!("https://mer3ly.net/projects/{repository_id}/")));
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

    let marker = "<script id=\"mere-projection-artifact\" type=\"application/json\">";
    let start = mere.find(marker).expect("Mere projection artifact") + marker.len();
    let end = mere[start..]
        .find("</script>")
        .map(|offset| start + offset)
        .expect("Mere projection artifact terminator");
    let artifact_json = projects::projection_artifact_json(&data);
    assert_eq!(&mere[start..end], artifact_json);
    let native_receipt = mer3ly_repo_graph::consume_portable_projection_json(&artifact_json)
        .expect("native Scenotime consumer accepts the exact page artifact");
    assert_eq!(native_receipt.score_items, 9);
    assert_eq!(native_receipt.initial_revision, 1);
    assert_eq!(native_receipt.final_revision, 5);
    assert_eq!(native_receipt.active_items, 9);
    assert_eq!(native_receipt.active_relations, 10);
    assert_eq!(native_receipt.picked_source, "mere");
    let artifact: serde_json::Value =
        serde_json::from_str(&mere[start..end]).expect("valid portable projection JSON");
    assert_eq!(artifact["schema"], "mer3ly.portable-projection/v1");
    assert_eq!(artifact["adapter"], "mer3ly.repository-graph/v1");
    // The score wire version moves when the contract does (2 added holds,
    // 3 renamed Board to Grid, 4 added the arrangement catalog and its item
    // disclosures), and this assertion moving with it is the consumer noticing
    // rather than silently accepting a shape it never knew.
    assert_eq!(artifact["score"]["version"], 4);
    let projection_proof = std::fs::read_to_string(root.join("assets/projection-proof.js"))
        .expect("projection proof runtime");
    assert!(
        projection_proof.contains("artifact?.score?.version !== 4"),
        "the browser consumer must recognize the same Score wire as the native consumer"
    );
    assert_eq!(
        artifact["score"]["items"]
            .as_array()
            .expect("score items")
            .len(),
        9
    );
    assert_eq!(
        artifact["snapshot"]["tables"]["items"]
            .as_array()
            .expect("scene items")
            .len(),
        9
    );
    assert_eq!(
        artifact["relations"].as_array().expect("relations").len(),
        11
    );
    assert_eq!(
        artifact["default_trace"]
            .as_array()
            .expect("revision trace")
            .len(),
        7
    );
    assert!(
        artifact["relations"]
            .as_array()
            .expect("relations")
            .iter()
            .all(|relation| relation["source"] == "mere" || relation["target"] == "mere")
    );

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
            < 32 * 1024
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
