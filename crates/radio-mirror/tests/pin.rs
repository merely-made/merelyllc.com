//! The source's two promises: the resolve holds no Reticulum-licensed crate
//! (Ruling 127), and the copied fixtures are Murm's as Cargo resolved it. No
//! lock is tracked (engine split D139), so both read `cargo metadata`.

use std::path::{Path, PathBuf};
use std::process::Command;

use mer3ly_radio_mirror::{HOST_FIXTURE, LOCAL_FIXTURE, RETINUE_BRANCH, RETINUE_REPOSITORY};

fn manifest_dir() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

/// The radio-mirror root's resolve, as Cargo reports it.
fn resolve() -> serde_json::Value {
    let output = Command::new(env!("CARGO"))
        // Resolve the declared source outside repository-local path redirects.
        .current_dir(std::env::temp_dir())
        .args(["metadata", "--format-version", "1", "--offline"])
        .arg("--manifest-path")
        .arg(manifest_dir().join("Cargo.toml"))
        .output()
        .expect("cargo metadata runs");
    assert!(
        output.status.success(),
        "cargo metadata failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("cargo metadata is JSON")
}

/// Whether a package source is Murm on the branch the manifest names.
fn from_murm(source: Option<&str>) -> bool {
    let branch = format!("git+{RETINUE_REPOSITORY}.git?branch={RETINUE_BRANCH}#");
    source.is_some_and(|source| source.starts_with(&branch))
}

#[test]
fn the_resolve_takes_radio_mirror_from_murm_without_retinue_or_retinue_sim() {
    let resolve = resolve();
    let packages = resolve["packages"].as_array().expect("Cargo packages");
    let names = packages
        .iter()
        .map(|package| package["name"].as_str().expect("Cargo package name"))
        .collect::<Vec<_>>();
    for forbidden in ["retinue", "retinue-sim"] {
        assert!(
            !names.contains(&forbidden),
            "{forbidden} entered the radio-mirror resolve"
        );
    }
    for package in ["radio-mirror", "radio-face", "embedded-graphics"] {
        let entry = packages
            .iter()
            .find(|entry| entry["name"].as_str() == Some(package))
            .unwrap_or_else(|| panic!("{package} is not in the resolve"));
        assert!(
            from_murm(entry["source"].as_str()),
            "{package} is not taken from Murm's {RETINUE_BRANCH}"
        );
    }
    let manifest = std::fs::read_to_string(manifest_dir().join("Cargo.toml")).expect("Cargo.toml");
    let manifest: toml::Value = toml::from_str(&manifest).expect("valid manifest");
    for dependency in [
        &manifest["dependencies"]["radio-mirror"],
        &manifest["target"]["cfg(not(target_arch = \"wasm32\"))"]["dependencies"]["radio-mirror"],
    ] {
        assert_eq!(
            dependency["git"].as_str(),
            Some(format!("{RETINUE_REPOSITORY}.git").as_str())
        );
        assert_eq!(dependency["branch"].as_str(), Some(RETINUE_BRANCH));
    }
}

/// radio-mirror's source directory in Cargo's git checkout.
fn resolved_radio_mirror_dir() -> PathBuf {
    let resolve = resolve();
    let package = resolve["packages"]
        .as_array()
        .expect("packages")
        .iter()
        .find(|package| package["name"] == "radio-mirror")
        .expect("radio-mirror in the metadata");
    assert!(
        from_murm(package["source"].as_str()),
        "radio-mirror source is not Murm's {RETINUE_BRANCH}"
    );
    Path::new(package["manifest_path"].as_str().expect("manifest path"))
        .parent()
        .expect("crate directory")
        .to_path_buf()
}

#[test]
fn copied_fixtures_match_the_resolved_source_byte_for_byte() {
    let fixtures = resolved_radio_mirror_dir().join("fixtures");
    for (name, copied) in [
        ("receipts-local.json", LOCAL_FIXTURE),
        ("receipts-host.json", HOST_FIXTURE),
    ] {
        let source = std::fs::read_to_string(fixtures.join(name)).expect("Murm fixture");
        assert_eq!(copied, source, "{name} drifted from Murm's fixture");
    }
}
