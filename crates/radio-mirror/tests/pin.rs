//! The pin's two promises: the resolve holds no Reticulum-licensed crate
//! (Ruling 127), and the copied fixtures are retinue's at the pinned revision.

use std::path::{Path, PathBuf};
use std::process::Command;

use mer3ly_radio_mirror::{HOST_FIXTURE, LOCAL_FIXTURE, RETINUE_REVISION};

fn manifest_dir() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn the_lockfile_pins_radio_mirror_without_retinue_or_retinue_sim() {
    let lock = std::fs::read_to_string(manifest_dir().join("Cargo.lock")).expect("Cargo.lock");
    let names = lock
        .lines()
        .filter_map(|line| line.strip_prefix("name = \""))
        .map(|name| name.trim_end_matches('"'))
        .collect::<Vec<_>>();
    for forbidden in ["retinue", "retinue-sim"] {
        assert!(
            !names.contains(&forbidden),
            "{forbidden} entered the radio-mirror resolve"
        );
    }
    let pinned = format!("retinue.git?rev={RETINUE_REVISION}#{RETINUE_REVISION}");
    for package in ["radio-mirror", "radio-face", "embedded-graphics"] {
        let entry = lock
            .split("[[package]]")
            .find(|entry| entry.contains(&format!("name = \"{package}\"\n")))
            .unwrap_or_else(|| panic!("{package} is not in the resolve"));
        assert!(
            entry.contains(&pinned),
            "{package} is not taken from retinue at the pinned revision"
        );
    }
    let manifest = std::fs::read_to_string(manifest_dir().join("Cargo.toml")).expect("Cargo.toml");
    assert_eq!(
        manifest
            .matches(&format!("rev = \"{RETINUE_REVISION}\""))
            .count(),
        2,
        "both radio-mirror entries name the pinned revision"
    );
}

/// radio-mirror's source directory in Cargo's git checkout.
fn pinned_radio_mirror_dir() -> PathBuf {
    let output = Command::new(env!("CARGO"))
        .args(["metadata", "--format-version", "1", "--locked", "--offline"])
        .arg("--manifest-path")
        .arg(manifest_dir().join("Cargo.toml"))
        .output()
        .expect("cargo metadata runs");
    assert!(
        output.status.success(),
        "cargo metadata failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let metadata: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("cargo metadata is JSON");
    let package = metadata["packages"]
        .as_array()
        .expect("packages")
        .iter()
        .find(|package| package["name"] == "radio-mirror")
        .expect("radio-mirror in the metadata");
    assert!(
        package["source"]
            .as_str()
            .is_some_and(|source| source.ends_with(&format!("#{RETINUE_REVISION}"))),
        "radio-mirror source is not the pinned revision"
    );
    Path::new(package["manifest_path"].as_str().expect("manifest path"))
        .parent()
        .expect("crate directory")
        .to_path_buf()
}

#[test]
fn copied_fixtures_match_the_pinned_revision_byte_for_byte() {
    let fixtures = pinned_radio_mirror_dir().join("fixtures");
    for (name, copied) in [
        ("receipts-local.json", LOCAL_FIXTURE),
        ("receipts-host.json", HOST_FIXTURE),
    ] {
        let pinned = std::fs::read_to_string(fixtures.join(name)).expect("pinned fixture");
        assert_eq!(copied, pinned, "{name} drifted from the pinned revision");
    }
}
