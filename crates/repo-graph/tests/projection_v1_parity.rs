//! The retired `mer3ly.portable-projection/v1` artifact against its
//! replacements (site canvas plan, Rulings 132-136).
//!
//! `fixtures/portable_projection_v1.json` is the last `projection-scene.json`
//! the site wrote, at merelyllc.com `9626ffc` (mere `9ccd5f41`), from the
//! authority frozen beside it as `fixtures/site_projection_authority.json`.
//! The new exporter reads the same authority. Its capture, trace and
//! shelfmark must carry the same scene, score and authority, and the trace
//! must replay to the same snapshot at every position as the old consumer,
//! which applied each step's diff in turn.

use chirograph::{ContentHash, ProjectionCaptureV2, Sha256NamedInformation};
use incipit::ShelfmarkV1;
use mer3ly_repo_graph::{SHELFMARK_AUTHORITY_ROLE, portable_projection};
use sceno::Score;
use scenotime::{SceneDiff, SceneSnapshot, SceneTrace};
use serde::Deserialize;
use serde_json::Value;

const AUTHORITY: &str = include_str!("fixtures/site_projection_authority.json");
const RETIRED: &str = include_str!("fixtures/portable_projection_v1.json");

/// The retired artifact's own shape, as the retired consumer read it.
#[derive(Deserialize)]
struct Retired {
    schema: String,
    adapter: String,
    authority_schema: String,
    authority_sha256: String,
    score: Score,
    snapshot: SceneSnapshot,
    default_trace: Vec<RetiredStep>,
}

#[derive(Deserialize)]
struct RetiredStep {
    label: String,
    selection: Option<Value>,
    diff: Option<SceneDiff>,
}

fn retired() -> Retired {
    let retired: Retired = serde_json::from_str(RETIRED).expect("the retired artifact parses");
    assert_eq!(retired.schema, "mer3ly.portable-projection/v1");
    retired
}

/// The retired consumer, position by position.
fn retired_replay(retired: &Retired) -> Vec<SceneSnapshot> {
    let mut snapshot = retired.snapshot.clone();
    let mut positions = vec![snapshot.clone()];
    for step in &retired.default_trace {
        if let Some(diff) = &step.diff {
            snapshot
                .apply_diff(diff)
                .expect("the retired consumer applies it");
        }
        positions.push(snapshot.clone());
    }
    positions
}

#[test]
fn the_new_artifacts_carry_the_retired_scene_score_and_authority() {
    let retired = retired();
    let artifacts = portable_projection(AUTHORITY).expect("the exporter accepts the authority");
    let capture = ProjectionCaptureV2::decode(&artifacts.capture).expect("capture");
    let authority = capture.authority.as_ref().expect("authority");

    assert_eq!(capture.scene, retired.snapshot);
    assert_eq!(capture.score.as_ref(), Some(&retired.score));
    assert_eq!(authority.adapter, retired.adapter);
    assert_eq!(authority.schema, retired.authority_schema);
    assert_eq!(
        authority.sha256,
        Sha256NamedInformation::from_hex(&retired.authority_sha256).expect("hex")
    );
    assert_eq!(authority.generation, retired.score.generation);
}

#[test]
fn the_trace_replays_identically_to_the_retired_trace_at_every_position() {
    let retired = retired();
    let artifacts = portable_projection(AUTHORITY).expect("artifacts");
    let trace: SceneTrace = serde_json::from_slice(&artifacts.trace).expect("trace");
    let expected = retired_replay(&retired);

    assert_eq!(trace.len(), retired.default_trace.len());
    assert_eq!(expected.len(), trace.len() + 1);
    for (position, snapshot) in expected.iter().enumerate() {
        assert_eq!(
            &trace.snapshot_at(position).expect("in range"),
            snapshot,
            "position {position}"
        );
    }
    for (step, old) in trace.steps().iter().zip(&retired.default_trace) {
        assert_eq!(step.label, old.label);
        assert_eq!(step.diff, old.diff);
        assert_eq!(step.annotation, old.selection, "{}", step.label);
    }
}

#[test]
fn the_epoch_travels_losslessly_as_text() {
    let retired = retired();
    let artifacts = portable_projection(AUTHORITY).expect("artifacts");
    let epoch = retired.snapshot.epoch.0;
    assert!(
        epoch > (1u64 << 53),
        "the fixture's epoch {epoch} must exceed 2^53, or this test proves nothing"
    );
    let digits = epoch.to_string();
    // A JavaScript number would round this.
    assert_ne!((epoch as f64) as u64, epoch);

    // The shelfmark carries it as a string, and every artifact's bytes carry
    // the exact digits, so a lossless reader recovers it unrounded.
    let shelfmark: ShelfmarkV1 = serde_json::from_slice(&artifacts.shelfmark).expect("shelfmark");
    assert_eq!(
        shelfmark.inputs[SHELFMARK_AUTHORITY_ROLE].expects_generation,
        digits
    );
    for (name, bytes) in [("capture", &artifacts.capture), ("trace", &artifacts.trace)] {
        let text = std::str::from_utf8(bytes).expect("UTF-8");
        assert!(
            text.contains(&format!("\"epoch\":{digits}")),
            "the {name} carries the epoch's exact digits"
        );
    }
    assert_eq!(
        shelfmark.projection,
        ContentHash::of(&artifacts.capture).to_string()
    );
}
