//! The browser session against the native consumer (site canvas plan,
//! Rulings 138 and 140).
//!
//! `ProjectionSession` is what the Mere profile's script calls through the
//! graph Wasm. These tests drive its Rust API, the same functions the
//! wasm-bindgen wrappers forward to, and hold it to `portable.rs`'s native
//! consumer: the same identities, the same refusals, and the same scene at
//! every position of the trace.

use mer3ly_repo_graph::{
    PROJECTION_STEP_BOUND, PortableProjection, ProjectionSession, RestoreRefusal,
    portable_projection, read_portable_projection,
};
use scenotime::{SceneSnapshot, SceneTrace};
use serde_json::{Value, json};

const AUTHORITY: &str = include_str!("fixtures/site_projection_authority.json");

fn artifacts() -> PortableProjection {
    portable_projection(AUTHORITY).expect("the exporter accepts the authority")
}

fn open(artifacts: &PortableProjection) -> ProjectionSession {
    ProjectionSession::open(&artifacts.capture, &artifacts.trace, &artifacts.shelfmark)
        .expect("the session opens what the native consumer accepts")
}

/// The JSON the page reads back, parsed as the native type.
fn as_page_reads(snapshot: &SceneSnapshot) -> SceneSnapshot {
    serde_json::from_str(&serde_json::to_string(snapshot).expect("serialize"))
        .expect("the snapshot JSON reads back")
}

#[test]
fn the_session_agrees_with_the_native_consumer_at_every_position() {
    let artifacts = artifacts();
    let reading = read_portable_projection(&artifacts).expect("native reading");
    let native: SceneTrace = serde_json::from_slice(&artifacts.trace).expect("trace");
    let mut session = open(&artifacts);

    assert_eq!(session.capture_address(), reading.receipt.capture_address);
    assert_eq!(session.epoch(), reading.epoch);
    assert_eq!(session.generation(), reading.receipt.generation);
    assert_eq!(session.epoch(), session.generation(), "Ruling 132");
    let epoch: u64 = session.epoch().parse().expect("decimal");
    assert!(
        epoch > 1 << 53,
        "the epoch must exceed 2^53 to prove anything"
    );

    assert_eq!(session.position(), 0);
    assert_eq!(session.length(), reading.receipt.trace_steps);
    assert_eq!(
        native.len() + 1,
        8,
        "the proof's default trace has 8 positions"
    );
    for position in 0..=native.len() {
        let expected = native.snapshot_at(position).expect("in range");
        assert_eq!(
            session.snapshot_at(position).expect("in range"),
            expected,
            "snapshot_at({position})"
        );
        assert_eq!(session.move_to(position), position);
        assert_eq!(session.snapshot(), expected, "moved to {position}");
        assert_eq!(as_page_reads(&session.snapshot()), expected);
        let revision = match position {
            0 => reading.receipt.initial_revision,
            _ => reading.steps[position - 1].revision,
        };
        assert_eq!(
            session.snapshot().revision.0,
            revision,
            "position {position}"
        );
        assert_eq!(session.length(), native.len(), "moving keeps redo");
    }
    assert_eq!(
        session.snapshot().revision.0,
        reading.receipt.final_revision
    );

    // Moving back is undo, and forward is redo, one step at a time.
    assert_eq!(session.move_to(0), 0);
    assert!(!session.undo(), "position 0 is the base");
    assert!(session.redo());
    assert_eq!(session.snapshot(), native.snapshot_at(1).expect("1"));
    assert_eq!(
        session.move_to(usize::MAX),
        native.len(),
        "clamped to the end"
    );

    let annotations = session.annotations();
    assert_eq!(annotations.len(), native.len());
    for (annotation, step) in annotations.iter().zip(native.steps()) {
        assert_eq!(
            Some(annotation),
            step.annotation.as_ref().or(Some(&Value::Null))
        );
    }
    assert_eq!(
        session.shared_steps(),
        None,
        "the supplied trace needs no steps"
    );
}

#[test]
fn committing_after_a_move_truncates_and_the_bound_is_refused() {
    let artifacts = artifacts();
    let native: SceneTrace = serde_json::from_slice(&artifacts.trace).expect("trace");
    let mut session = open(&artifacts);

    session.move_to(2);
    let mark = json!({"label": "Select Mere", "annotation": {"kind": "node", "id": "mere"}});
    assert_eq!(session.record(&mark.to_string()), Ok(true));
    assert_eq!(session.position(), 3);
    assert_eq!(session.length(), 3, "record cleared redo");
    assert!(!session.redo());
    assert_eq!(session.snapshot(), native.snapshot_at(2).expect("2"));

    // A scene change: the session supplies the epoch and revisions.
    let head = session.snapshot();
    let item = serde_json::to_value(head.tables.items[0].as_ref().expect("item")).expect("item");
    let mut moved = item.clone();
    moved["transform"]["translate"] = json!({"x": 12.5, "y": -4.0});
    let step = json!({
        "label": "Move",
        "operations": [{"UpdateItem": {"index": 0, "value": moved}}],
    });
    let preview = session.preview(&step.to_string()).expect("preview");
    assert_eq!(session.position(), 3, "a preview records nothing");
    assert_eq!(preview.revision.0, head.revision.0 + 1);
    assert_eq!(preview.epoch, head.epoch);
    assert_eq!(session.record(&step.to_string()), Ok(true));
    assert_eq!(session.snapshot(), preview);

    let steps = session.shared_steps().expect("a changed trace travels");
    assert!(steps.contains(&format!("\"epoch\":{}", session.epoch())));

    // The page's bound is a refusal, not a cap.
    while session.position() < PROJECTION_STEP_BOUND {
        assert_eq!(session.record(&mark.to_string()), Ok(true));
    }
    assert_eq!(session.record(&mark.to_string()), Ok(false));
    assert_eq!(session.length(), PROJECTION_STEP_BOUND);

    session.reset();
    assert_eq!((session.position(), session.length()), (0, native.len()));
    assert_eq!(session.shared_steps(), None);
}

#[test]
fn a_shared_trace_restores_or_is_refused_as_the_page_words_it() {
    let artifacts = artifacts();
    let native: SceneTrace = serde_json::from_slice(&artifacts.trace).expect("trace");
    let mut session = open(&artifacts);
    let steps = serde_json::to_value(native.steps()).expect("steps");

    // The supplied trace at a shared position.
    assert_eq!(session.restore(None, 3.0), Ok(()));
    assert_eq!(session.position(), 3);
    assert_eq!(session.snapshot(), native.snapshot_at(3).expect("3"));

    // A shorter shared trace, in full.
    let shorter = json!(steps.as_array().unwrap()[..2]).to_string();
    assert_eq!(session.restore(Some(&shorter), 1.0), Ok(()));
    assert_eq!((session.position(), session.length()), (1, 2));
    let shared: Vec<scenotime::TraceStep> =
        serde_json::from_str(&session.shared_steps().expect("shorter steps travel"))
            .expect("steps");
    assert_eq!(shared, native.steps()[..2]);

    let refused = |session: &mut ProjectionSession, steps: Option<&str>, position: f64| {
        let before = (session.position(), session.length());
        let refusal = session.restore(steps, position).unwrap_err();
        assert_eq!(
            (session.position(), session.length()),
            before,
            "refusal changes nothing"
        );
        refusal
    };
    assert_eq!(
        refused(&mut session, Some("{"), 0.0),
        RestoreRefusal::Unreadable
    );
    assert_eq!(
        refused(&mut session, Some("{}"), 0.0),
        RestoreRefusal::TooLong
    );
    let long = json!(vec![json!({"label": "Mark"}); PROJECTION_STEP_BOUND + 1]).to_string();
    assert_eq!(
        refused(&mut session, Some(&long), 0.0),
        RestoreRefusal::TooLong
    );

    // Ruling 134: a repeated diff breaks the chain.
    let mut repeated = steps.as_array().unwrap()[..2].to_vec();
    repeated.push(repeated[1].clone());
    assert_eq!(
        refused(&mut session, Some(&json!(repeated).to_string()), 0.0),
        RestoreRefusal::Broken { step: 3 }
    );
    let unknown = json!([{"label": "Mark", "extra": true}]).to_string();
    assert_eq!(
        refused(&mut session, Some(&unknown), 0.0),
        RestoreRefusal::Broken { step: 1 }
    );
    for position in [-1.0, 0.5, 8.0, f64::NAN] {
        assert_eq!(
            refused(&mut session, None, position),
            RestoreRefusal::Position
        );
    }
    assert_eq!(
        serde_json::to_string(&RestoreRefusal::Broken { step: 3 }).unwrap(),
        r#"{"refusal":"broken","step":3}"#
    );
}

#[test]
fn the_session_refuses_what_the_native_consumer_refuses() {
    let artifacts = artifacts();
    let mut other = artifacts.clone();
    // A shelfmark citing another capture.
    let mut shelfmark: Value = serde_json::from_slice(&other.shelfmark).expect("shelfmark");
    shelfmark["projection"] = json!("0".repeat(64));
    other.shelfmark = serde_json::to_vec(&shelfmark).expect("shelfmark");
    let native = read_portable_projection(&other).unwrap_err();
    let session = ProjectionSession::open(&other.capture, &other.trace, &other.shelfmark)
        .err()
        .expect("refused");
    assert_eq!(session, native);
    assert!(session.contains("the shelfmark cites capture"), "{session}");

    // A trace whose second diff repeats the first.
    let mut trace: Value = serde_json::from_slice(&artifacts.trace).expect("trace");
    let steps = trace["steps"].as_array_mut().expect("steps");
    let first = steps[1].clone();
    steps.insert(2, first);
    let mut broken = artifacts.clone();
    broken.trace = serde_json::to_vec(&trace).expect("trace");
    assert!(read_portable_projection(&broken).is_err());
    assert!(ProjectionSession::open(&broken.capture, &broken.trace, &broken.shelfmark).is_err());
}
