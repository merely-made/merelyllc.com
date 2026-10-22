use std::fs;
use std::path::{Path, PathBuf};

use mer3ly_site::repositories::Authority;
use mer3ly_site::spec_coverage::CatalogSnapshot;
use mer3ly_site::spec_results;
use mer3ly_site::specs::SpecRegistry;

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

fn authority() -> Authority {
    Authority::load(workspace_root()).expect("authority loads")
}

const REGISTRY: &str = r#"
schema = "mer3ly.spec-registry/v1"

[[spec]]
id = "ecma262"
name = "ECMAScript"
body = "Ecma"
catalog = "none"
suite = "test262"

[[spec]]
id = "lua-5.4"
name = "Lua 5.4"
body = "community"
catalog = "none"
suite = "lua-5.4-tests"

[[claim]]
spec = "ecma262"
repo = "vano"
relation = "implements"
depth = "partial"
verdict = "open"
measured_from = { path = "tests/metrics.json", format = "test262-metrics" }
source = "test"
verified_on = "2026-10-22"

[[claim]]
spec = "lua-5.4"
repo = "piccolo-fork"
relation = "implements"
depth = "partial"
verdict = "open"
measured_from = { path = "spec-results.toml", format = "spec-results" }
source = "test"
verified_on = "2026-10-22"
"#;

/// A scratch site root holding a fetched cache.
fn scratch(name: &str, manifest_sources: &[(&str, &str, &str, &str)]) -> PathBuf {
    let root = std::env::temp_dir().join(format!("spec-results-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    let dir = root.join(spec_results::RESULTS_DIR);
    fs::create_dir_all(dir.join("vano/tests")).unwrap();
    fs::create_dir_all(dir.join("piccolo-fork")).unwrap();
    fs::write(
        dir.join("vano/tests/metrics.json"),
        r#"{ "results": { "pass": 7, "fail": 3 }, "total": 10 }"#,
    )
    .unwrap();
    fs::write(
        dir.join("piccolo-fork/spec-results.toml"),
        "schema = \"merely.spec-results/v1\"\n[[result]]\nspec = \"lua-5.4\"\nmeasured = { on = \"2026-10-21\", scope = \"suite\", files_all_pass = 2, files = 32 }\n",
    )
    .unwrap();
    let sources: Vec<String> = manifest_sources
        .iter()
        .map(|(repo, slug, git_ref, path)| {
            format!(
                r#"{{ "repo": "{repo}", "slug": "{slug}", "ref": "{git_ref}", "path": "{path}", "commit": "0123456789abcdef", "committed_on": "2026-10-20" }}"#
            )
        })
        .collect();
    fs::write(
        root.join(spec_results::SOURCES_FILE),
        format!(
            r#"{{ "fetched_on": "2026-10-22", "source": [{}] }}"#,
            sources.join(",")
        ),
    )
    .unwrap();
    root
}

const BOTH: [(&str, &str, &str, &str); 2] = [
    ("vano", "merely-made/vano", "main", "tests/metrics.json"),
    (
        "piccolo-fork",
        "merely-made/didgeridoo",
        "master",
        "spec-results.toml",
    ),
];

#[test]
fn committed_cache_supplies_every_measured_from_claim() {
    let mut registry = SpecRegistry::load(workspace_root()).expect("registry loads");
    let catalog = CatalogSnapshot::load(workspace_root()).expect("snapshot loads");
    registry
        .validate(&authority(), &catalog)
        .expect("registry validates");
    spec_results::apply(&mut registry, &authority(), workspace_root()).expect("cache applies");
    let from: Vec<_> = registry
        .claim
        .iter()
        .filter(|c| c.measured_from.is_some())
        .collect();
    assert!(!from.is_empty());
    for claim in from {
        let measured = claim.measured.as_ref().expect("filled from the cache");
        assert!(
            measured.scope.contains(" at "),
            "{} lacks its source commit",
            claim.spec
        );
    }
}

#[test]
fn fetched_files_fill_the_claims_with_their_source_commit() {
    let root = scratch("fill", &BOTH);
    let mut registry = SpecRegistry::parse(REGISTRY).unwrap();
    spec_results::apply(&mut registry, &authority(), &root).expect("applies");
    let vano = registry.claim[0].measured.as_ref().unwrap();
    assert_eq!((vano.wpt_pass, vano.wpt_total), (Some(7), Some(10)));
    assert_eq!(vano.on, "2026-10-20");
    assert!(
        vano.scope
            .ends_with("(merely-made/vano tests/metrics.json at 0123456789)")
    );
    let lua = registry.claim[1].measured.as_ref().unwrap();
    assert_eq!((lua.files_all_pass, lua.files), (Some(2), Some(32)));
    assert_eq!(lua.on, "2026-10-21");
    let _ = fs::remove_dir_all(root);
}

#[test]
fn a_source_missing_from_the_fetch_is_an_error() {
    let root = scratch("missing", &BOTH[..1]);
    let mut registry = SpecRegistry::parse(REGISTRY).unwrap();
    let errors = spec_results::apply(&mut registry, &authority(), &root).unwrap_err();
    assert!(errors[0].contains("merely-made/didgeridoo spec-results.toml@master is not in"));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn sources_resolve_through_the_migration_ledger() {
    let registry = SpecRegistry::parse(REGISTRY).unwrap();
    let sources = spec_results::sources(&registry, &authority()).unwrap();
    let listed: Vec<_> = sources
        .iter()
        .map(|s| (s.slug.as_str(), s.git_ref.as_str()))
        .collect();
    assert_eq!(
        listed,
        [
            ("merely-made/didgeridoo", "master"),
            ("merely-made/vano", "main")
        ]
    );
}

#[test]
fn measured_and_measured_from_together_are_rejected() {
    let text = REGISTRY.replacen(
        "measured_from = { path = \"tests/metrics.json\", format = \"test262-metrics\" }",
        "measured_from = { path = \"tests/metrics.json\", format = \"test262-metrics\" }\nmeasured = { on = \"2026-10-22\", scope = \"x\", wpt_pass = 1, wpt_total = 2 }",
        1,
    );
    let registry = SpecRegistry::parse(&text).unwrap();
    let catalog = CatalogSnapshot::load(workspace_root()).unwrap();
    let errors = registry.validate(&authority(), &catalog).unwrap_err();
    assert!(
        errors
            .iter()
            .any(|e| e.contains("has both measured and measured_from"))
    );
}
