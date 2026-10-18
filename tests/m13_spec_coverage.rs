use std::path::{Path, PathBuf};

use mer3ly_site::spec_coverage::{CatalogSnapshot, CatalogSpec, Coverage};
use mer3ly_site::specs::SpecRegistry;

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

const REGISTRY: &str = r#"
schema = "mer3ly.spec-registry/v1"

[[scope]]
repo = "genet"
catalog = "browser-specs"
categories = ["browser"]

[[spec]]
id = "fetch"
name = "Fetch Standard"
body = "WHATWG"
catalog = "browser-specs"
suite = "wpt:fetch"

[[spec]]
id = "url"
name = "URL Standard"
body = "WHATWG"
catalog = "browser-specs"
suite = "wpt:url"

[[spec]]
id = "gone"
name = "A spec the catalog dropped"
body = "W3C"
catalog = "browser-specs"
suite = "none"

[[claim]]
spec = "fetch"
repo = "genet"
relation = "implements"
depth = "partial"
verdict = "ADOPT"
measured = { on = "2026-09-29", scope = "census", wpt_pass = 1, wpt_total = 2 }
source = "test"
verified_on = "2026-10-18"

[[claim]]
spec = "url"
repo = "genet"
relation = "implements"
depth = "full"
verdict = "open"
source = "test"
verified_on = "2026-07-19"

[[claim]]
spec = "gone"
repo = "mere"
relation = "implements"
depth = "partial"
verdict = "open"
source = "test"
verified_on = "2026-07-20"
"#;

fn catalog_spec(shortname: &str, categories: &[&str]) -> CatalogSpec {
    CatalogSpec {
        shortname: shortname.to_owned(),
        title: shortname.to_uppercase(),
        organization: "WHATWG".to_owned(),
        url: format!("https://{shortname}.spec.whatwg.org/"),
        categories: categories.iter().map(|c| (*c).to_owned()).collect(),
        standing: "good".to_owned(),
        test_paths: Vec::new(),
    }
}

fn coverage() -> Coverage {
    let registry = SpecRegistry::parse(REGISTRY).expect("fixture parses");
    let catalog = CatalogSnapshot {
        source: "test".to_owned(),
        version: "0".to_owned(),
        fetched_on: "2026-10-18".to_owned(),
        license: "CC0-1.0".to_owned(),
        spec: vec![
            catalog_spec("fetch", &["browser"]),
            catalog_spec("url", &["browser"]),
            catalog_spec("streams", &["browser"]),
            catalog_spec("server-only", &[]),
        ],
    };
    Coverage::compute(&registry, &catalog, "2026-10-18")
}

#[test]
fn counts_claims_by_repository() {
    let coverage = coverage();
    let genet = coverage
        .repositories
        .iter()
        .find(|r| r.repo == "genet")
        .unwrap();
    assert_eq!(genet.claims, 2);
    assert_eq!(genet.depth.get("full"), Some(&1));
    assert_eq!(genet.verdict.get("ADOPT"), Some(&1));
}

#[test]
fn reports_unclaimed_specs_inside_the_scope_only() {
    let coverage = coverage();
    let scope = &coverage.unclaimed[0];
    assert_eq!(scope.in_scope, 3);
    let names: Vec<_> = scope.specs.iter().map(|s| s.shortname.as_str()).collect();
    assert_eq!(names, ["streams"]);
}

#[test]
fn reports_catalog_drift_staleness_and_suite_gaps() {
    let coverage = coverage();
    assert_eq!(coverage.missing_from_catalog, ["gone"]);
    // 2026-07-19 is 91 days before the report date; 2026-07-20 is exactly 90.
    let stale: Vec<_> = coverage.stale.iter().map(|c| c.spec.as_str()).collect();
    assert_eq!(stale, ["url"]);
    let no_suite: Vec<_> = coverage
        .depth_without_suite
        .iter()
        .map(|c| c.spec.as_str())
        .collect();
    assert_eq!(no_suite, ["gone"]);
    let unmeasured: Vec<_> = coverage
        .suite_unmeasured
        .iter()
        .map(|g| g.spec.as_str())
        .collect();
    assert_eq!(unmeasured, ["url"]);
}

#[test]
fn committed_registry_reports_against_the_committed_snapshot() {
    let registry = SpecRegistry::load(workspace_root()).expect("registry loads");
    let catalog = CatalogSnapshot::load(workspace_root()).expect("snapshot loads");
    let coverage = Coverage::compute(&registry, &catalog, "2026-10-18");
    assert_eq!(coverage.claims, registry.claim.len());
    assert!(catalog.spec.len() > 600);
    assert!(coverage.unclaimed[0].specs.len() < coverage.unclaimed[0].in_scope);
    assert!(
        coverage
            .to_string()
            .starts_with("spec coverage on 2026-10-18")
    );
}
