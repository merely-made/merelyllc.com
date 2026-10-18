use std::path::{Path, PathBuf};

use mer3ly_site::repositories::Authority;
use mer3ly_site::specs::SpecRegistry;

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

fn authority() -> Authority {
    Authority::load(workspace_root()).expect("authority loads")
}

const VALID: &str = r#"
schema = "mer3ly.spec-registry/v1"

[[spec]]
id = "fetch"
name = "Fetch Standard"
body = "WHATWG"
catalog = "browser-specs"
url = "https://fetch.spec.whatwg.org/"
suite = "wpt:fetch"

[[spec]]
id = "rfc9846"
name = "TLS 1.3"
body = "IETF"
catalog = "rfc"
suite = "none"

[[claim]]
spec = "fetch"
repo = "genet"
crate = "netfetcher"
relation = "implements"
depth = "partial"
verdict = "ADOPT"
measured = { on = "2026-09-29", scope = "census", engine = "vano", wpt_pass = 2016, wpt_total = 5901 }
source = "test"
verified_on = "2026-10-18"

[[claim]]
spec = "rfc9846"
repo = "family"
relation = "via-dep"
depth = "full"
verdict = "open"
source = "test"
verified_on = "2026-10-18"
"#;

/// Validates `text`, returning the error lines (a parse failure is one line).
fn errors(text: &str) -> Vec<String> {
    match SpecRegistry::parse(text) {
        Err(error) => vec![error.to_string()],
        Ok(registry) => registry.validate(&authority()).err().unwrap_or_default(),
    }
}

fn assert_rejected(text: &str, needle: &str) {
    let errors = errors(text);
    assert!(
        errors.iter().any(|error| error.contains(needle)),
        "expected an error naming {needle:?}, got {errors:?}"
    );
}

#[test]
fn committed_registry_validates() {
    let registry = SpecRegistry::load(workspace_root()).expect("registry parses");
    registry.validate(&authority()).expect("registry validates");
    assert!(registry.spec.len() > 300);
    assert!(registry.repositories().contains("genet"));
}

#[test]
fn fixture_validates() {
    assert_eq!(errors(VALID), Vec::<String>::new());
}

#[test]
fn duplicate_spec_id_is_rejected() {
    let text = VALID.replace(r#"id = "rfc9846""#, r#"id = "fetch""#);
    assert_rejected(&text, "duplicate spec id fetch");
}

#[test]
fn unknown_spec_and_repo_are_rejected() {
    assert_rejected(
        &VALID.replace(r#"spec = "rfc9846""#, r#"spec = "rfc1""#),
        "claim rfc1 / family names unknown spec",
    );
    assert_rejected(
        &VALID.replace(r#"repo = "family""#, r#"repo = "nowhere""#),
        "claim rfc9846 / nowhere names unknown repo",
    );
}

#[test]
fn enum_outside_the_grammar_is_rejected() {
    assert_rejected(
        &VALID.replace(r#"depth = "full""#, r#"depth = "deep""#),
        "deep",
    );
    assert_rejected(
        &VALID.replace(r#"verdict = "ADOPT""#, r#"verdict = "adopt""#),
        "adopt",
    );
}

#[test]
fn measured_block_needs_a_date() {
    let text = VALID.replace(r#"on = "2026-09-29", "#, "");
    assert_rejected(&text, "missing field `on`");
    assert_rejected(
        &VALID.replace("wpt_pass = 2016", "wpt_pass = 9999"),
        "claim fetch / genet / netfetcher wpt_pass 9999 exceeds its total 5901",
    );
}

#[test]
fn duplicate_claim_is_rejected() {
    let first = VALID.find("[[claim]]").unwrap();
    let second = VALID[first + 1..].find("[[claim]]").unwrap() + first + 1;
    let text = format!("{}{}", VALID, &VALID[first..second]);
    assert_rejected(&text, "duplicate claim fetch / genet / netfetcher");
}

#[test]
fn malformed_suite_is_rejected() {
    assert_rejected(
        &VALID.replace(r#"suite = "wpt:fetch""#, r#"suite = "wpt: fetch""#),
        "spec fetch suite is malformed",
    );
    assert_rejected(
        &VALID.replace(r#"suite = "none""#, r#"suite = "WPT""#),
        "spec rfc9846 suite is malformed",
    );
}

#[test]
fn spec_without_a_claim_and_bad_rfc_id_are_rejected() {
    let text = format!(
        "{VALID}\n[[spec]]\nid = \"rfc-tls\"\nname = \"TLS\"\nbody = \"IETF\"\ncatalog = \"rfc\"\nsuite = \"none\"\n"
    );
    assert_rejected(&text, "spec rfc-tls has no claim");
    assert_rejected(
        &text,
        "spec rfc-tls is catalog rfc but its id is not rfcNNNN",
    );
}

#[test]
fn bad_verified_on_is_rejected() {
    let text = VALID.replacen(
        r#"verified_on = "2026-10-18""#,
        r#"verified_on = "2026/10/18""#,
        1,
    );
    assert_rejected(
        &text,
        "claim fetch / genet / netfetcher verified_on is not YYYY-MM-DD",
    );
}
