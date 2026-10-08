//! The projection proof's browser session (site canvas plan, Rulings 138 and
//! 140).
//!
//! The Mere profile's proof used to carry its own copies of this stack logic in
//! JavaScript: a replay of `SceneTrace`, BLAKE3, a lossless JSON reader and a
//! mirror of `edit_history::History`. They retire into this session, which the
//! page reaches through the site's existing graph Wasm on first interaction.
//!
//! - Opening runs [`read_portable_projection`]'s checks unchanged: chirograph
//!   decodes and validates the V2 capture, the shelfmark is checked against the
//!   capture's bytes by chirograph's content address (Ruling 135, host side),
//!   the scene's epoch must be the authority generation (Ruling 132), and the
//!   trace deserializes through scenotime with its chain checked.
//! - Replay is `SceneTrace::snapshot_at`.
//! - Position, undo, redo and truncate-on-commit are `edit_history::History`
//!   over a `SceneTrace` (Ruling 133). The step bound is host policy (Ruling
//!   15), refused here rather than capped.
//! - The epoch and generation leave as decimal strings: they exceed 2^53.
//!
//! The JavaScript keeps rendering, controls, labels from the host dataset, and
//! the share link's text.

use edit_history::History;
use scenotime::{SceneOp, SceneTrace, TraceStep};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use wasm_bindgen::prelude::*;

use super::*;
use crate::portable::open_portable_projection;

/// One step as the page records it. The page never writes a diff's epoch or
/// revisions: they come from the scene the step is recorded onto, so the epoch
/// never passes through a JavaScript number.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PageStep {
    label: String,
    #[serde(default)]
    operations: Option<Vec<SceneOp>>,
    #[serde(default)]
    annotation: Option<Value>,
}

/// Why a shared trace was not restored. The page words the notice.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "refusal", rename_all = "kebab-case")]
pub enum RestoreRefusal {
    /// The steps are not JSON.
    Unreadable,
    /// The steps are not a list, or are more than the page keeps.
    TooLong,
    /// Step `step` (counted from 1) is not a trace step or does not chain.
    Broken { step: usize },
    /// The position is not a step the trace has.
    Position,
}

/// The proof's trace, its history and the identities the page shows.
#[wasm_bindgen]
pub struct ProjectionSession {
    default_trace: SceneTrace,
    current: SceneTrace,
    history: History<SceneTrace>,
    capture_address: String,
    epoch: String,
    generation: String,
}

impl ProjectionSession {
    /// Open the three artifacts, refusing them as the native consumer does.
    pub fn open(capture: &[u8], trace: &[u8], shelfmark: &[u8]) -> Result<Self, String> {
        let opened = open_portable_projection(&PortableProjection {
            capture: capture.to_vec(),
            trace: trace.to_vec(),
            shelfmark: shelfmark.to_vec(),
        })?;
        let receipt = &opened.reading.receipt;
        let mut session = Self {
            default_trace: opened.trace.clone(),
            current: opened.trace.clone(),
            history: unbounded_history(),
            capture_address: receipt.capture_address.clone(),
            epoch: opened.reading.epoch.clone(),
            generation: receipt.generation.clone(),
        };
        session.open_at(opened.trace, 0)?;
        Ok(session)
    }

    /// The capture's content address (BLAKE3, lowercase hex), as the shelfmark
    /// cites it.
    pub fn capture_address(&self) -> &str {
        &self.capture_address
    }

    /// The scene epoch, as decimal text.
    pub fn epoch(&self) -> &str {
        &self.epoch
    }

    /// The authority generation the shelfmark expects, as decimal text.
    pub fn generation(&self) -> &str {
        &self.generation
    }

    /// Where the page stands: the number of steps behind it.
    pub fn position(&self) -> usize {
        self.current.len()
    }

    /// The trace at the end of redo: what the scrubber spans and a link
    /// shares. It is read by walking a copy of the history forward, so the
    /// history stays the only record of where redo ends.
    pub fn furthest(&self) -> SceneTrace {
        let mut history = self.history.clone();
        let mut current = self.current.clone();
        while let Some(next) = history.redo(current.clone()) {
            current = next;
        }
        current
    }

    /// The number of steps the scrubber spans.
    pub fn length(&self) -> usize {
        self.furthest().len()
    }

    /// The scene at `position` along the furthest trace: `snapshot_at`.
    pub fn snapshot_at(&self, position: usize) -> Result<SceneSnapshot, String> {
        self.furthest()
            .snapshot_at(position)
            .map_err(|error| format_trace_error(&error))
    }

    /// The scene where the page stands.
    pub fn snapshot(&self) -> SceneSnapshot {
        self.current.head()
    }

    /// Move to `position`, clamped to the trace: undo or redo, one step at a
    /// time. Returns where the page now stands.
    pub fn move_to(&mut self, position: usize) -> usize {
        while self.current.len() > position && self.undo() {}
        while self.current.len() < position && self.redo() {}
        self.current.len()
    }

    /// One step back. `false` at the base.
    pub fn undo(&mut self) -> bool {
        match self.history.undo(self.current.clone()) {
            Some(previous) => {
                self.current = previous;
                true
            }
            None => false,
        }
    }

    /// One step forward. `false` at the end of redo.
    pub fn redo(&mut self) -> bool {
        match self.history.redo(self.current.clone()) {
            Some(next) => {
                self.current = next;
                true
            }
            None => false,
        }
    }

    /// Record `step` where the page stands. Committing after a move truncates
    /// there, because `History::record` clears redo. Returns `Ok(false)`,
    /// recording nothing, when the step would pass the page's bound.
    pub fn record(&mut self, step: &str) -> Result<bool, String> {
        if self.current.len() + 1 > PROJECTION_STEP_BOUND {
            return Ok(false);
        }
        let next = self.appended(step)?;
        self.history.record(self.current.clone(), None, 0);
        self.current = next;
        Ok(true)
    }

    /// The scene `step` would make, without recording it: a drag's preview.
    pub fn preview(&self, step: &str) -> Result<SceneSnapshot, String> {
        Ok(self.appended(step)?.head())
    }

    /// Return to the supplied trace, at its base.
    pub fn reset(&mut self) {
        let default = self.default_trace.clone();
        self.open_at(default, 0)
            .expect("the supplied trace opens at its base");
    }

    /// Open a shared trace at a shared position. `steps` is the link's step
    /// list as JSON, or `None` for the supplied trace. On refusal nothing
    /// changes.
    pub fn restore(&mut self, steps: Option<&str>, position: f64) -> Result<(), RestoreRefusal> {
        let trace = match steps {
            None => self.default_trace.clone(),
            Some(steps) => self.shared_trace(steps)?,
        };
        if !position.is_finite()
            || position.fract() != 0.0
            || position < 0.0
            || position > trace.len() as f64
        {
            return Err(RestoreRefusal::Position);
        }
        self.open_at(trace, position as usize)
            .map_err(|_| RestoreRefusal::Position)
    }

    /// The furthest trace's steps as JSON, when they differ from the supplied
    /// trace's; a link carries them only then.
    pub fn shared_steps(&self) -> Option<String> {
        let furthest = self.furthest();
        (furthest.steps() != self.default_trace.steps())
            .then(|| serde_json::to_string(furthest.steps()).expect("trace steps serialize"))
    }

    /// Each step's host annotation along the furthest trace, `null` when a
    /// step has none.
    pub fn annotations(&self) -> Vec<Value> {
        self.furthest()
            .steps()
            .iter()
            .map(|step| step.annotation.clone().unwrap_or(Value::Null))
            .collect()
    }

    fn shared_trace(&self, steps: &str) -> Result<SceneTrace, RestoreRefusal> {
        let value: Value =
            serde_json::from_slice(steps.as_bytes()).map_err(|_| RestoreRefusal::Unreadable)?;
        let Value::Array(values) = value else {
            return Err(RestoreRefusal::TooLong);
        };
        if values.len() > PROJECTION_STEP_BOUND {
            return Err(RestoreRefusal::TooLong);
        }
        let steps = values
            .into_iter()
            .enumerate()
            .map(|(index, value)| {
                // Read through the same JSON reader as every other step, so
                // the Wasm carries one copy of the step deserializer.
                let bytes = serde_json::to_vec(&value).expect("a JSON value serializes");
                serde_json::from_slice::<TraceStep>(&bytes)
                    .map_err(|_| RestoreRefusal::Broken { step: index + 1 })
            })
            .collect::<Result<Vec<_>, _>>()?;
        SceneTrace::from_steps(self.default_trace.base().clone(), steps).map_err(
            |error| match error {
                scenotime::TraceError::Step { index, .. } => {
                    RestoreRefusal::Broken { step: index + 1 }
                }
                _ => RestoreRefusal::Broken { step: 1 },
            },
        )
    }

    /// Rebuild the history as a host opening a shared trace does: commit each
    /// step, then step back to `position`.
    fn open_at(&mut self, trace: SceneTrace, position: usize) -> Result<(), String> {
        if position > trace.len() {
            return Err(format!(
                "position {position} is past the trace's {} steps",
                trace.len()
            ));
        }
        let mut history = unbounded_history();
        let mut current = trace
            .truncated(0)
            .map_err(|error| format_trace_error(&error))?;
        for step in trace.steps() {
            let next = current
                .appended(step.clone())
                .map_err(|error| format_trace_error(&error))?;
            history.record(current, None, 0);
            current = next;
        }
        for _ in position..trace.len() {
            current = history
                .undo(current)
                .expect("each committed step can be undone");
        }
        self.history = history;
        self.current = current;
        Ok(())
    }

    fn appended(&self, step: &str) -> Result<SceneTrace, String> {
        let step: PageStep = serde_json::from_slice(step.as_bytes())
            .map_err(|error| format!("not a page step: {error}"))?;
        let head = self.current.head();
        let mut trace_step = match step.operations {
            Some(operations) => TraceStep::diff(step.label, next_diff(&head, operations)),
            None => TraceStep::mark(step.label),
        };
        if let Some(annotation) = step.annotation {
            trace_step = trace_step.with_annotation(annotation);
        }
        self.current
            .appended(trace_step)
            .map_err(|error| format_trace_error(&error))
    }
}

/// The page keeps every step up to its own bound, so the history has no cap of
/// its own.
fn unbounded_history() -> History<SceneTrace> {
    History::new().with_cap(0)
}

/// scenotime's `TraceError` has no `Display`; the site words it here.
fn format_trace_error(error: &scenotime::TraceError) -> String {
    match error {
        scenotime::TraceError::UnsupportedVersion { found } => {
            format!("scene trace version {found} is not supported")
        }
        scenotime::TraceError::InvalidBase(error) => {
            format!("the trace's base is not a valid scene: {error:?}")
        }
        scenotime::TraceError::Step { index, error } => {
            format!("step {} does not chain: {error:?}", index + 1)
        }
        scenotime::TraceError::OutOfRange { position, steps } => {
            format!("position {position} is past the trace's {steps} steps")
        }
    }
}

fn to_json<T: Serialize>(value: &T) -> Result<String, JsValue> {
    serde_json::to_string(value).map_err(|error| JsValue::from_str(&error.to_string()))
}

#[wasm_bindgen]
impl ProjectionSession {
    #[wasm_bindgen(constructor)]
    pub fn js_new(
        capture: &[u8],
        trace: &[u8],
        shelfmark: &[u8],
    ) -> Result<ProjectionSession, JsValue> {
        Self::open(capture, trace, shelfmark).map_err(|error| JsValue::from_str(&error))
    }

    #[wasm_bindgen(js_name = captureAddress)]
    pub fn js_capture_address(&self) -> String {
        self.capture_address.clone()
    }

    #[wasm_bindgen(js_name = epoch)]
    pub fn js_epoch(&self) -> String {
        self.epoch.clone()
    }

    #[wasm_bindgen(js_name = generation)]
    pub fn js_generation(&self) -> String {
        self.generation.clone()
    }

    #[wasm_bindgen(js_name = stepBound)]
    pub fn js_step_bound(&self) -> usize {
        PROJECTION_STEP_BOUND
    }

    #[wasm_bindgen(js_name = position)]
    pub fn js_position(&self) -> usize {
        self.position()
    }

    #[wasm_bindgen(js_name = length)]
    pub fn js_length(&self) -> usize {
        self.length()
    }

    /// The scene where the page stands, as scenotime's snapshot JSON.
    #[wasm_bindgen(js_name = snapshot)]
    pub fn js_snapshot(&self) -> Result<String, JsValue> {
        to_json(&self.snapshot())
    }

    #[wasm_bindgen(js_name = snapshotAt)]
    pub fn js_snapshot_at(&self, position: usize) -> Result<String, JsValue> {
        to_json(
            &self
                .snapshot_at(position)
                .map_err(|error| JsValue::from_str(&error))?,
        )
    }

    #[wasm_bindgen(js_name = moveTo)]
    pub fn js_move_to(&mut self, position: usize) -> usize {
        self.move_to(position)
    }

    #[wasm_bindgen(js_name = undo)]
    pub fn js_undo(&mut self) -> bool {
        self.undo()
    }

    #[wasm_bindgen(js_name = redo)]
    pub fn js_redo(&mut self) -> bool {
        self.redo()
    }

    #[wasm_bindgen(js_name = record)]
    pub fn js_record(&mut self, step: &str) -> Result<bool, JsValue> {
        self.record(step).map_err(|error| JsValue::from_str(&error))
    }

    #[wasm_bindgen(js_name = preview)]
    pub fn js_preview(&self, step: &str) -> Result<String, JsValue> {
        to_json(
            &self
                .preview(step)
                .map_err(|error| JsValue::from_str(&error))?,
        )
    }

    #[wasm_bindgen(js_name = reset)]
    pub fn js_reset(&mut self) {
        self.reset()
    }

    /// Restore a link; a refusal is thrown as JSON, `{"refusal": ...}`.
    #[wasm_bindgen(js_name = restore)]
    pub fn js_restore(&mut self, steps: Option<String>, position: f64) -> Result<(), JsValue> {
        self.restore(steps.as_deref(), position)
            .map_err(|refusal| JsValue::from_str(&to_json(&refusal).unwrap_or_default()))
    }

    #[wasm_bindgen(js_name = sharedSteps)]
    pub fn js_shared_steps(&self) -> Option<String> {
        self.shared_steps()
    }

    #[wasm_bindgen(js_name = annotations)]
    pub fn js_annotations(&self) -> Result<String, JsValue> {
        to_json(&self.annotations())
    }
}
