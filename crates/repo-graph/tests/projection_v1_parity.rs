//! The retired `mer3ly.portable-projection/v1` artifact against its
//! replacements (site canvas plan, Rulings 132-136).
//!
//! `fixtures/portable_projection_v1.json` is the last `projection-scene.json`
//! the site wrote, at merelyllc.com `9626ffc` (mere `9ccd5f41`), from the
//! authority frozen beside it as `fixtures/site_projection_authority.json`.
//! The new exporter reads the same authority. Its capture, trace and
//! shelfmark must carry the same scene, score and authority, and the trace
//! must replay to the same scene at every position as the old consumer,
//! which applied each step's diff in turn.
//!
//! The fold is the one intended difference (site canvas plan, S5; Rulings
//! 149-155). The retired trace, and `fixtures/projection_trace_visible_diff.json`
//! (the `projection-trace.json` this exporter wrote at merelyllc.com `e6850ca`
//! from the same authority), folded by setting a `["fold", 1]` channel on
//! Mere and `visible: false` on the direct targets of its relations. The new
//! trace adds a `sceno::Fold` fact instead and tombstones it to unfold. At
//! every position both show exactly the same items, Mere carries the same
//! "+N", and every placement, relation and revision is the same.

use chirograph::{ContentHash, ProjectionCaptureV2, Sha256NamedInformation};
use incipit::ShelfmarkV1;
use mer3ly_repo_graph::{SHELFMARK_AUTHORITY_ROLE, portable_projection};
use std::collections::BTreeSet;

use sceno::{FoldDirection, FoldRule, InstanceId, Score, StandIn};
use scenotime::{SceneDiff, SceneOp, SceneSnapshot, SceneTrace};
use serde::Deserialize;
use serde_json::Value;

const AUTHORITY: &str = include_str!("fixtures/site_projection_authority.json");
const RETIRED: &str = include_str!("fixtures/portable_projection_v1.json");
const VISIBLE_DIFF_TRACE: &str = include_str!("fixtures/projection_trace_visible_diff.json");

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

/// What the old encoding showed: each item's own flag.
fn shown_by_flag(snapshot: &SceneSnapshot) -> Vec<bool> {
    snapshot
        .tables
        .items
        .iter()
        .map(|item| item.as_ref().is_some_and(|item| item.visible))
        .collect()
}

/// What the fact shows: the flag, and no fold hiding the item.
fn shown_by_fact(snapshot: &SceneSnapshot) -> Vec<bool> {
    (0..snapshot.tables.items.len())
        .map(|index| snapshot.is_shown(InstanceId(index as u32)))
        .collect()
}

/// The old "+N": an item carrying the `fold` channel counted the distinct
/// targets of relations leaving it in the captured scene, as the retired
/// script's `dependencyIds` did.
fn badges_by_channel(snapshot: &SceneSnapshot, base: &SceneSnapshot) -> Vec<usize> {
    snapshot
        .tables
        .items
        .iter()
        .enumerate()
        .map(|(index, item)| {
            let folded = item.as_ref().is_some_and(|item| {
                item.channels
                    .iter()
                    .any(|(name, value)| name == "fold" && *value > 0.0)
            });
            if !folded {
                return 0;
            }
            base.tables
                .relations
                .iter()
                .flatten()
                .filter(|relation| relation.from.0 == index as u32)
                .map(|relation| relation.to.0)
                .collect::<BTreeSet<_>>()
                .len()
        })
        .collect()
}

/// The fact's "+N": `hidden_count` on each member stand-in.
fn badges_by_fact(snapshot: &SceneSnapshot) -> Vec<usize> {
    let mut badges = vec![0; snapshot.tables.items.len()];
    for (_, fold) in snapshot.active_folds() {
        if let Some(stand_in) = fold.stand_in_member() {
            badges[stand_in.0 as usize] = fold.hidden_count();
        }
    }
    badges
}

/// Everything but how a fold is encoded: placements, relations, revisions.
fn placements(snapshot: &SceneSnapshot) -> Vec<Option<(f32, f32)>> {
    snapshot
        .tables
        .items
        .iter()
        .map(|item| {
            item.as_ref()
                .map(|item| (item.transform.translate.x, item.transform.translate.y))
        })
        .collect()
}

/// Hold the fact trace to an old visible-diff replay, position by position.
fn assert_same_reading(old: &[SceneSnapshot], new: &SceneTrace, base: &SceneSnapshot) {
    assert_eq!(old.len(), new.len() + 1);
    for (position, expected) in old.iter().enumerate() {
        let snapshot = new.snapshot_at(position).expect("in range");
        assert_eq!(
            shown_by_fact(&snapshot),
            shown_by_flag(expected),
            "shown at position {position}"
        );
        assert_eq!(
            badges_by_fact(&snapshot),
            badges_by_channel(expected, base),
            "+N at position {position}"
        );
        assert_eq!(placements(&snapshot), placements(expected), "{position}");
        assert_eq!(snapshot.tables.relations, expected.tables.relations);
        assert_eq!(snapshot.tables.sources, expected.tables.sources);
        assert_eq!(snapshot.revision, expected.revision, "{position}");
        assert_eq!(snapshot.epoch, expected.epoch);
        assert!(
            snapshot
                .tables
                .items
                .iter()
                .flatten()
                .all(|item| item.visible && item.channels.is_empty()),
            "the fact trace never touches an item's own flag or channels"
        );
    }
}

#[test]
fn the_trace_reads_identically_to_the_retired_trace_at_every_position() {
    let retired = retired();
    let artifacts = portable_projection(AUTHORITY).expect("artifacts");
    let trace: SceneTrace = serde_json::from_slice(&artifacts.trace).expect("trace");
    let expected = retired_replay(&retired);

    assert_eq!(trace.len(), retired.default_trace.len());
    assert_same_reading(&expected, &trace, &retired.snapshot);
    for (step, old) in trace.steps().iter().zip(&retired.default_trace) {
        assert_eq!(step.label, old.label);
        assert_eq!(step.annotation, old.selection, "{}", step.label);
        let folds = step.diff.as_ref().is_some_and(|diff| {
            diff.operations.iter().any(|operation| {
                matches!(
                    operation,
                    SceneOp::AddFold { .. } | SceneOp::TombstoneFold { .. }
                )
            })
        });
        if !folds {
            assert_eq!(step.diff, old.diff, "{}", step.label);
        }
    }
}

#[test]
fn the_fact_trace_reads_as_the_visible_diff_trace_did() {
    let old: SceneTrace =
        serde_json::from_str(VISIBLE_DIFF_TRACE).expect("the visible-diff trace replays");
    let artifacts = portable_projection(AUTHORITY).expect("artifacts");
    let new: SceneTrace = serde_json::from_slice(&artifacts.trace).expect("trace");

    assert_eq!(new.base(), old.base(), "the capture is unchanged");
    assert_eq!(new.len(), 7, "8 positions");
    let replay = (0..=old.len())
        .map(|position| old.snapshot_at(position).expect("in range"))
        .collect::<Vec<_>>();
    assert_same_reading(&replay, &new, old.base());

    // The old fold step set a channel; the new one adds the fact, and nothing
    // else. Every other step is byte-for-byte the old one.
    let mut fold_steps = 0;
    for (index, (step, previous)) in new.steps().iter().zip(old.steps()).enumerate() {
        assert_eq!(step.label, previous.label);
        assert_eq!(step.annotation, previous.annotation);
        let operations = &step.diff.as_ref().map(|diff| &diff.operations);
        match operations.map(Vec::as_slice) {
            Some([SceneOp::AddFold { value, .. }]) => {
                fold_steps += 1;
                let before = new.snapshot_at(index).unwrap();
                let mere = InstanceId(0);
                assert_eq!(value.stand_in, StandIn::Member(mere));
                assert_eq!(
                    value.rule,
                    Some(FoldRule::Descendants {
                        root: mere,
                        family: "depends_on".into(),
                        direction: FoldDirection::Outgoing,
                    })
                );
                assert_eq!(value.label.as_deref(), Some("Mere's dependencies"));
                assert_eq!(value.hidden_count(), 6);
                assert_eq!(
                    before.tables.sources[before.active_item(mere).unwrap().source.0 as usize]
                        .as_ref()
                        .unwrap()
                        .id,
                    "mere"
                );
            }
            Some([SceneOp::TombstoneFold { .. }]) => fold_steps += 1,
            _ => assert_eq!(step, previous, "step {}", index + 1),
        }
    }
    assert_eq!(fold_steps, 2, "one fold and its unfold");
    let head = new.head();
    assert!(head.active_folds().is_empty(), "the trace ends unfolded");
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
