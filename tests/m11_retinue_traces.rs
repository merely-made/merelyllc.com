//! The committed retinue-sim traces (Ruling 127): their provenance and the
//! pairing of each face track with its route trace. `m10_devices` checks
//! that the site's resolve holds no Reticulum-licensed crate.

use std::fs;
use std::path::{Path, PathBuf};

use mer3ly_site::retinue_traces::{
    PROVENANCE_FILE, TRACE_DIRECTORY, TraceEvent, TraceSet, sha256_hex,
};
use serde_json::Value;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

fn trace_file(name: &str) -> Vec<u8> {
    fs::read(root().join(TRACE_DIRECTORY).join(name)).expect("committed trace file")
}

#[test]
fn each_face_track_names_its_committed_route_trace() {
    for id in ["cold", "warm"] {
        let route = trace_file(&format!("{id}.route-trace.json"));
        let face: Value =
            serde_json::from_slice(&trace_file(&format!("{id}.face-track.json"))).unwrap();
        // retinue-sim hashes the canonical JSON; the committed file carries the
        // generator's trailing newline, which the digest does not cover.
        let canonical = route.strip_suffix(b"\n").expect("trailing newline");
        assert_eq!(
            face["trace_sha256"].as_str(),
            Some(sha256_hex(canonical).as_str()),
            "{id} face track pairs with another route trace"
        );
        assert_eq!(face["schema"], "retinue-sim.face-track/v1");
        assert_eq!(face["route_trace"], "retinue-sim.route-trace/v1");
    }
}

#[test]
fn provenance_records_each_files_hash_revision_and_command() {
    let provenance: toml::Value = toml::from_str(
        &fs::read_to_string(root().join(TRACE_DIRECTORY).join(PROVENANCE_FILE)).unwrap(),
    )
    .unwrap();
    assert_eq!(
        provenance["revision"].as_str(),
        Some("0731bd8c1c488f37839fa9c8bad6973798e59d95")
    );
    for trace in provenance["trace"].as_array().unwrap() {
        for kind in ["route", "face"] {
            let name = trace[kind].as_str().unwrap();
            assert_eq!(
                trace[format!("{kind}_sha256").as_str()].as_str(),
                Some(sha256_hex(&trace_file(name)).as_str()),
                "{name}"
            );
            assert!(
                trace[format!("{kind}_command").as_str()]
                    .as_str()
                    .unwrap()
                    .starts_with("cargo run -q -p retinue-sim --example lab"),
                "{name}"
            );
        }
    }
    let loaded = TraceSet::load(&root()).expect("committed traces validate");
    let embedded = TraceSet::embedded().expect("embedded traces validate");
    for (disk, built) in loaded.scenarios.iter().zip(&embedded.scenarios) {
        assert_eq!(disk.route_bytes, built.route_bytes);
        assert_eq!(disk.face_bytes, built.face_bytes);
    }
}

#[test]
fn a_tampered_pair_is_refused() {
    let provenance =
        fs::read_to_string(root().join(TRACE_DIRECTORY).join(PROVENANCE_FILE)).unwrap();
    // Swap the face tracks: every hash in the provenance and both pairings fail.
    let errors = TraceSet::parse(&provenance, |name| {
        let swapped = match name {
            "cold.face-track.json" => "warm.face-track.json",
            "warm.face-track.json" => "cold.face-track.json",
            other => other,
        };
        Some(trace_file(swapped))
    })
    .expect_err("swapped face tracks are refused");
    assert!(
        errors
            .iter()
            .any(|error| error.contains("trace_sha256 is not its route trace's")),
        "{errors:?}"
    );
}

#[test]
fn the_scenarios_carry_the_facts_the_lab_states() {
    let traces = TraceSet::embedded().unwrap();
    let cold = &traces.scenario("cold").unwrap().trace;
    assert_eq!(cold.scenario, "cold-cut");
    assert_eq!(cold.cuts[0].at, 0);
    let delivered = cold.messages[0].delivered.as_ref().expect("cold delivers");
    assert_eq!(delivered.path, ["fire", "church", "water", "garage"]);

    let warm = &traces.scenario("warm").unwrap().trace;
    assert_eq!(warm.scenario, "warm-cut");
    assert_eq!(warm.cuts[0].at, 60_000);
    let expired = warm
        .events
        .iter()
        .filter(|event| matches!(event, TraceEvent::LinkRequestExpired { .. }))
        .count();
    assert_eq!(expired, 3);
    let paths = warm
        .messages
        .iter()
        .map(|message| message.delivered.as_ref().map(|d| d.path.join(">")))
        .collect::<Vec<_>>();
    assert_eq!(
        paths,
        [
            Some("fire>water>garage".to_owned()),
            None,
            None,
            None,
            Some("fire>church>water>garage".to_owned()),
        ]
    );
}
