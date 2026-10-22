use std::path::{Path, PathBuf};

use mer3ly_site::pages::roadmap;

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

#[test]
fn roadmap_page_is_built_from_the_registry_and_fetched_counts() {
    let page = roadmap::page(&workspace_root()).expect("roadmap builds");
    assert!(page.html.starts_with("<!doctype html>"));
    assert!(
        page.html
            .contains("<link rel=\"canonical\" href=\"https://merelyllc.com/roadmap/\"")
    );
    assert!(!page.html.contains("@@mer3ly-roadmap-scene@@"));
    // A feature from genet's ledger, with its consumer plan.
    assert!(page.html.contains("web clip as a real gesture"));
    // A count read from vano's own CI-gated metrics, named by its source.
    assert!(page.html.contains("merely-made/vano at "));
    assert!(
        page.html
            .contains("data-dataset-src=\"/spec-roadmap-dataset.json\"")
    );
}

#[test]
fn roadmap_dataset_is_a_host_dataset_the_viewer_reads() {
    let page = roadmap::page(&workspace_root()).expect("roadmap builds");
    let dataset: serde_json::Value = serde_json::from_str(&page.dataset_json).unwrap();
    assert_eq!(dataset["schema"], "scenomise.host-dataset/v1");
    let occurrences = dataset["dataset"]["occurrences"].as_array().unwrap();
    let ids: Vec<&str> = occurrences
        .iter()
        .map(|o| o["occurrence_id"].as_str().unwrap())
        .collect();
    assert!(ids.iter().any(|id| id.starts_with("feature:")));
    assert!(ids.contains(&"spec:selection-api"));
    assert!(ids.contains(&"repo:genet"));
    for relationship in dataset["relationships"].as_array().unwrap() {
        let from = relationship["from_occurrence"].as_str().unwrap();
        let to = relationship["to_occurrence"].as_str().unwrap();
        assert!(ids.contains(&from) && ids.contains(&to), "{from} -> {to}");
    }
}

#[test]
fn roadmap_links_only_to_catalogued_merely_made_repositories() {
    let page = roadmap::page(&workspace_root()).expect("roadmap builds");
    for text in [&page.html, &page.dataset_json] {
        for (index, _) in text.match_indices("https://github.com/") {
            let rest = &text[index + "https://github.com/".len()..];
            assert!(
                rest.starts_with("merely-made/") || rest.starts_with("merely-made\""),
                "{}",
                &rest[..rest.len().min(60)]
            );
        }
    }
}
