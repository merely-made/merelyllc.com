use std::path::{Path, PathBuf};

use genet_scripted_dom::ScriptedDom;
use layout_dom_api::LayoutDom;
use mer3ly_site::pages::repositories;
use mer3ly_site::repositories::PublicSiteData;
use mer3ly_site::site::SITE_CSS;

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

#[test]
fn repository_page_projects_the_complete_typed_authority() {
    let root = workspace_root();
    let data = PublicSiteData::load(&root).expect("load validated public site data");
    let document = repositories::document(&root).expect("render repository page");

    assert_eq!(
        document.match_indices("data-repository-id=").count(),
        data.authority.repositories.repository.len()
    );
    for repository in &data.authority.repositories.repository {
        assert!(
            document.contains(&format!("data-repository-id=\"{}\"", repository.id)),
            "page is missing repository {}",
            repository.id
        );
        assert!(
            document.contains(&repository.github_slug),
            "page is missing canonical slug {}",
            repository.github_slug
        );
    }

    for relation in &data.authority.relations.relation {
        assert_eq!(
            document
                .match_indices(&format!("data-relation-id=\"{}\"", relation.id))
                .count(),
            2,
            "relation {} must appear once in each direction",
            relation.id
        );
    }
}

#[test]
fn repository_page_is_static_semantic_and_filterable() {
    let root = workspace_root();
    let document = repositories::document(&root).expect("render repository page");

    assert_eq!(document.matches("<h1").count(), 1);
    assert!(document.contains("<main id=\"main\""));
    assert!(document.contains("https://mer3ly.net/repos/"));
    assert_eq!(
        document.match_indices("name=\"repository-class\"").count(),
        5
    );
    assert!(document.contains("<fieldset class=\"repository-filter-shell\">"));
    assert!(document.contains("aria-label=\"Relationship key\""));
    assert!(document.contains("provenance-derived"));
    assert!(document.contains("provenance-curated"));
    assert!(document.contains("<script type=\"module\" src=\"/repo-graph.js?v="));
    assert!(document.contains("data-graph-scene-caption=\"\""));
    assert!(document.contains("Constellation medallions"));
    assert!(document.contains("href=\"mailto:markik@mer3ly.net\""));
    assert!(document.contains(">Merely organization profile</a></h2>"));
    assert!(document.contains("data-project-href=\"/projects/mere/\""));
    assert!(document.contains("recent organization activity"));
    assert!(document.contains("data-activity-repository=\"mere\""));
    assert!(!document.contains(">Merely Made organization profile</a></h2>"));

    for forbidden in [
        "x-dc",
        "webpack",
        "__next",
        "mark-ik",
        "C:\\Users\\",
        "tel:",
    ] {
        assert!(
            !document.contains(forbidden),
            "repository page contains forbidden marker {forbidden:?}"
        );
    }

    let dom = ScriptedDom::from_serialized_document(&document);
    let serialized = dom.inner_html(dom.document());
    assert!(serialized.contains("<html lang=\"en\">"));
    assert!(serialized.contains("data-repository-id=\"genet\""));
}

#[test]
fn public_metadata_cache_is_reduced_and_bounded() {
    let root = workspace_root();
    let data = PublicSiteData::load(&root).expect("load validated public site data");
    let document = repositories::document(&root).expect("render repository page");
    let cache =
        serde_json::to_string(&data.metadata).expect("serialize validated public metadata cache");
    let bytes = document.len() + SITE_CSS.len() + cache.len();

    // Generated HTML is intentionally readable source. The graph data uses one
    // compact line per JSON structure, while the surrounding document keeps
    // two-space indentation. This bound also covers the scene sandbox controls
    // and their representation-registry styles.
    assert!(
        bytes < 300 * 1024,
        "repository HTML, CSS, and public metadata use {bytes} bytes"
    );
    assert_eq!(
        data.metadata.repository.len(),
        data.authority.repositories.repository.len()
    );
    assert_eq!(data.metadata.schema, "mer3ly.github-organization/v2");
    assert_eq!(data.metadata.organization, "merely-made");
    assert!(!data.metadata.event.is_empty());
    assert!(data.metadata.event.len() <= 40);
    assert!(data.metadata.event.iter().all(|event| {
        data.authority
            .repositories
            .repository
            .iter()
            .any(|repository| repository.github_slug == event.repository)
    }));
    assert!(!cache.contains("merely-made/esp"));
    assert!(!cache.contains("merely-made/tulpa"));
    assert!(!cache.contains("merely-made/mer3ly-net"));
    for forbidden_field in [
        "\"private\"",
        "\"visibility\"",
        "\"viewer_permission\"",
        "\"token\"",
        "\"actor\"",
        "\"payload\"",
        "\"ssh_url\"",
    ] {
        assert!(
            !cache.contains(forbidden_field),
            "public metadata contains forbidden field {forbidden_field}"
        );
    }
}

#[test]
fn stylesheet_contains_keyboard_and_narrow_filter_contracts() {
    for contract in [
        ".repository-filter-input:focus-visible + .repository-filter-label",
        "#repository-filter-product:checked",
        ".repository-card:not(.class-product)",
        ".relationship-grid",
        ".repository-profile-link",
        ".repository-activity-feed",
        "@media (max-width: 760px)",
        "@media (max-width: 440px)",
    ] {
        assert!(
            SITE_CSS.contains(contract),
            "site CSS is missing {contract}"
        );
    }
}
