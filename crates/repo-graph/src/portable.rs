//! The projection proof's portable artifacts (site canvas plan, Rulings 18 and
//! 130-136).
//!
//! The site's former `mer3ly.portable-projection/v1` artifact is replaced by
//! sibling artifacts that reference one another (Ruling 132):
//! - a [`ProjectionCaptureV2`], holding the solved scene, the score that
//!   produced it and its authority identity (Ruling 131);
//! - a [`SceneTrace`] whose base is the captured scene. Its steps are the
//!   proof's default trace, and each step's selection rides as the host
//!   annotation (Ruling 130);
//! - an [`incipit::ShelfmarkV1`] that cites the capture by its content
//!   address and the generation it expects (Ruling 135).
//!
//! Names, classes, statuses and relation provenance are not here. They come
//! from the S1 host dataset (Ruling 136), which the site publishes beside these.

use chirograph::{
    CaptureAuthorityV1, ContentHash, PROJECTION_CAPTURE_V2, PresentationManifest,
    ProjectionCaptureV2, Sha256NamedInformation,
};
use incipit::{ShelfmarkAuthorityV1, ShelfmarkInputV1, ShelfmarkV1};
use scenotime::{SceneTrace, TraceStep};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::*;

/// Where the site publishes each artifact, beside the S1 host dataset.
pub const CAPTURE_FILE: &str = "projection-capture.json";
pub const TRACE_FILE: &str = "projection-trace.json";
pub const SHELFMARK_FILE: &str = "projection-shelfmark.json";

/// The shelfmark input role that names the capture's authority.
pub const SHELFMARK_AUTHORITY_ROLE: &str = "authority";

/// The most steps the proof keeps. This is host policy (Ruling 15), not part
/// of the trace: a trace can be longer, and the page refuses to record or
/// restore past this bound with a message.
pub const PROJECTION_STEP_BOUND: usize = 16;

const PROJECTION_RECEIPT_SCHEMA: &str = "mer3ly.projection-receipt/v2";

/// The three projection artifacts, as the bytes the site publishes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PortableProjection {
    /// chirograph's encoding of the V2 capture. Its content address is the
    /// BLAKE3 hash of exactly these bytes.
    pub capture: Vec<u8>,
    /// The `SceneTrace` wire (version 1).
    pub trace: Vec<u8>,
    /// The `ShelfmarkV1` that cites the capture.
    pub shelfmark: Vec<u8>,
}

/// What the native consumer checked, for the artifact receipt and CI.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct ProjectionReceipt {
    pub schema: String,
    /// The capture's content address (BLAKE3, lowercase hex).
    pub capture_address: String,
    /// The authority's RFC 6920 SHA-256 name.
    pub authority_sha256: String,
    /// The authority generation, as a decimal string: it exceeds 2^53, so a
    /// JSON number would not survive a JavaScript reader.
    pub generation: String,
    pub score_items: usize,
    pub initial_revision: u64,
    pub final_revision: u64,
    pub active_items: usize,
    pub active_relations: usize,
    pub picked_source: String,
    pub trace_steps: usize,
    /// How many authored holds the realized scene actually honored.
    ///
    /// Equal to the score's hold count on a sound capture. Consuming checks it
    /// rather than trusting it, because a citation that says "pinned here" and
    /// reconstitutes elsewhere is the exact failure the seam exists to close.
    pub honored_holds: usize,
}

/// One relation of the captured scene, by its endpoints' sources.
///
/// The scene carries no relation id. A reader joins these to the host
/// dataset's disclosed relationships by endpoints and kind (Ruling 136).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CapturedRelation {
    pub index: u32,
    pub from: String,
    pub to: String,
    pub kind: String,
}

/// One trace step, as a reader without the compiler sees it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReadingStep {
    pub label: String,
    /// The revision the trace reaches after this step.
    pub revision: u64,
    pub changes_scene: bool,
    /// The host annotation's selection, as `(kind, id)`.
    pub selection: Option<(String, String)>,
}

/// The decoded artifacts, reduced to what a host page reads at build time.
#[derive(Clone, Debug, PartialEq)]
pub struct ProjectionReading {
    pub receipt: ProjectionReceipt,
    /// Every captured item's source id, in instance order.
    pub items: Vec<String>,
    pub relations: Vec<CapturedRelation>,
    pub steps: Vec<ReadingStep>,
    /// The scene epoch as decimal text, exactly as the capture's bytes carry it.
    pub epoch: String,
}

/// Build the proof's three artifacts from the repository authority.
pub fn portable_projection(input: &str) -> Result<PortableProjection, String> {
    let input = parse_authority(input)?;
    let capture = solve_capture(&input, &PlacementDelta::default())?;
    let trace = default_trace(&input, &capture.scene)?;
    let shelfmark = cite_capture(&capture)?;
    let artifacts = PortableProjection {
        capture: capture
            .encode()
            .map_err(|error| format!("could not encode the capture: {error}"))?,
        trace: serde_json::to_vec(&trace)
            .map_err(|error| format!("could not serialize the scene trace: {error}"))?,
        shelfmark: serde_json::to_vec(&shelfmark)
            .map_err(|error| format!("could not serialize the shelfmark: {error}"))?,
    };
    consume_portable_projection(&artifacts)?;
    Ok(artifacts)
}

/// The seam: a live arrangement's placement reaching a portable score.
///
/// `placement` is the sandbox's own scene state (or just its placement half).
/// The pins it carries become [`Score::holds`], the solver honors them ahead of
/// the arrangement, and the capture comes back only if each one landed where
/// the visitor put it. The result is chirograph's encoding of one V2 capture:
/// a capture is one instant, so no trace travels with it.
pub fn projection_capture_with_placement_json(
    input: &str,
    placement: &str,
) -> Result<String, String> {
    let delta: PlacementDelta = serde_json::from_str(placement)
        .map_err(|error| format!("invalid placement delta: {error}"))?;
    let input = parse_authority(input)?;
    let capture = solve_capture(&input, &delta)?;
    honored_holds_of(&capture)?;
    let bytes = capture
        .encode()
        .map_err(|error| format!("could not encode the capture: {error}"))?;
    String::from_utf8(bytes).map_err(|error| format!("capture is not UTF-8: {error}"))
}

fn parse_authority(input: &str) -> Result<GraphInput, String> {
    let input: GraphInput =
        serde_json::from_str(input).map_err(|error| format!("invalid graph JSON: {error}"))?;
    validate(&input)?;
    Ok(input)
}

/// Solve the authority into a captured scene, carrying its score and its
/// authority identity.
fn solve_capture(
    input: &GraphInput,
    placement: &PlacementDelta,
) -> Result<ProjectionCaptureV2, String> {
    let (authority_sha256, generation) = authority_identity(input)?;

    let mut score = Score::new(SceneArrangement::Spiral(Spiral::default()));
    score.generation = generation;
    // A pin naming a node this authority does not contain is a broken citation,
    // not a placement. Say so rather than solving a scene that quietly omits it.
    for pin in &placement.pins {
        if !input.nodes.iter().any(|node| node.id == pin.id) {
            return Err(format!("placement pins unknown node {}", pin.id));
        }
    }
    score.holds = placement.holds();
    let mut ordered_nodes = input.nodes.iter().enumerate().collect::<Vec<_>>();
    ordered_nodes.sort_by_key(|(index, node)| (node.id != PREFERRED_FOCUS_REPOSITORY, *index));
    for (ordinal, (_, node)) in ordered_nodes.into_iter().enumerate() {
        score.items.push(ScoreItem {
            source: SourceRef::new(PROJECTION_ADAPTER, &node.id),
            ordinal: ordinal as u32,
            footprint: Footprint::Circle { radius: 28.0 },
            representation: Representation::Glyph,
            placement: Placement::Ordinal,
            layer: 0,
            visible: true,
            // A spiral places by ordinal alone, and this projection's ordinal
            // already carries the focus-first ordering it wants.
            axis: None,
            embedding: None,
            weight: None,
        });
    }

    let mut scene = scenomise::solve(&score);
    let instance_by_source = scene
        .items
        .iter()
        .enumerate()
        .map(|(index, item)| {
            let source = &scene.sources[item.source.0 as usize];
            (source.id.as_str(), sceno::InstanceId(index as u32))
        })
        .collect::<HashMap<_, _>>();
    let mut relations = Vec::with_capacity(input.edges.len());
    for edge in &input.edges {
        let from = *instance_by_source
            .get(edge.source.as_str())
            .ok_or_else(|| format!("projection lost relation source {}", edge.source))?;
        let to = *instance_by_source
            .get(edge.target.as_str())
            .ok_or_else(|| format!("projection lost relation target {}", edge.target))?;
        let from_point = scene.items[from.0 as usize].transform.translate;
        let to_point = scene.items[to.0 as usize].transform.translate;
        relations.push(RoutedRelation {
            from,
            to,
            space: sceno::Scene::WORLD,
            points: vec![from_point, to_point],
            kind: Some(edge.kind.clone()),
            weight: Some(1.0),
        });
    }
    scene.relations.extend(relations);

    // The site's convention (Ruling 132): the scene's epoch is the authority
    // generation, so a scene from another authority cannot chain onto it.
    let snapshot = SceneSnapshot::from_dense(SceneEpoch(generation), Revision(1), scene)
        .map_err(|error| format!("Scenograph rejected the solved scene: {error:?}"))?;
    let capture = ProjectionCaptureV2 {
        version: PROJECTION_CAPTURE_V2,
        scene: snapshot,
        presentation: PresentationManifest::default(),
        score: Some(score),
        authority: Some(CaptureAuthorityV1 {
            adapter: PROJECTION_ADAPTER.to_owned(),
            schema: input.schema.clone(),
            sha256: Sha256NamedInformation::from_hex(&authority_sha256)
                .map_err(|error| format!("authority digest is not SHA-256 hex: {error:?}"))?,
            generation,
        }),
    };
    capture
        .validate()
        .map_err(|error| format!("chirograph refused the capture: {error}"))?;
    Ok(capture)
}

/// The proof's default trace: select and move Turnstone, select and remove
/// its host relationship, select Mere, and fold and expand its dependencies.
fn default_trace(input: &GraphInput, base: &SceneSnapshot) -> Result<SceneTrace, String> {
    let mut trace = SceneTrace::new(base.clone()).map_err(trace_error)?;
    let mut push = |step: TraceStep| -> Result<SceneSnapshot, String> {
        trace = trace.appended(step).map_err(trace_error)?;
        Ok(trace.head())
    };
    let mut current = base.clone();

    if let Some(turnstone) = instance_for_source(&current, "turnstone") {
        push(selection_step("Select Turnstone", "node", "turnstone"))?;
        let diff = move_diff(&current, turnstone, Vec2::new(48.0, 24.0))?;
        current = push(TraceStep::diff("Move Turnstone", diff))?;
    }

    let host = input
        .edges
        .iter()
        .position(|edge| edge.id == "turnstone-hosts-mere");
    if let Some(index) = host {
        push(selection_step(
            "Select the Turnstone host relationship",
            "edge",
            &input.edges[index].id,
        ))?;
        let diff = next_diff(
            &current,
            vec![SceneOp::TombstoneRelation {
                index: RelationId(index as u32),
            }],
        );
        current = push(TraceStep::diff(
            "Remove the relationship from the scene",
            diff,
        ))?;
    }

    push(selection_step(
        "Select Mere",
        "node",
        PREFERRED_FOCUS_REPOSITORY,
    ))?;
    let dependencies = input
        .edges
        .iter()
        .filter(|edge| edge.source == PREFERRED_FOCUS_REPOSITORY)
        .filter_map(|edge| instance_for_source(&current, &edge.target))
        .collect::<Vec<_>>();
    if !dependencies.is_empty() {
        let fold = visibility_diff(&current, PREFERRED_FOCUS_REPOSITORY, &dependencies, false)?;
        current = push(TraceStep::diff("Fold Mere dependencies", fold))?;
        let expand = visibility_diff(&current, PREFERRED_FOCUS_REPOSITORY, &dependencies, true)?;
        push(TraceStep::diff("Expand Mere dependencies", expand))?;
    }

    Ok(trace)
}

fn selection_step(label: &str, kind: &str, id: &str) -> TraceStep {
    TraceStep::mark(label).with_annotation(json!({ "kind": kind, "id": id }))
}

fn trace_error(error: scenotime::TraceError) -> String {
    format!("scene trace refused: {error:?}")
}

/// The shelfmark the site writes beside its capture: the projection is the
/// capture's content address, and the authority input names the authority's
/// digest and the generation it expects.
pub fn cite_capture(capture: &ProjectionCaptureV2) -> Result<ShelfmarkV1, String> {
    let authority = capture
        .authority
        .as_ref()
        .ok_or("the capture names no authority")?;
    let address = capture
        .content_address()
        .map_err(|error| format!("the capture has no address: {error}"))?;
    let mut shelfmark = ShelfmarkV1::new(address.to_string());
    shelfmark.inputs.insert(
        SHELFMARK_AUTHORITY_ROLE.to_owned(),
        ShelfmarkInputV1 {
            authority: ShelfmarkAuthorityV1 {
                adapter: authority.adapter.clone(),
                record: authority.sha256.to_string(),
            },
            reading: authority.schema.clone(),
            reading_parameters: None,
            arrangement: None,
            expects_generation: authority.generation.to_string(),
        },
    );
    Ok(shelfmark)
}

/// The host-side shelfmark-to-capture check (Ruling 135): the shelfmark cites
/// these capture bytes by address, names the same authority, and expects the
/// generation the capture carries. No shipped Mere API carries this edge.
pub fn check_shelfmark(shelfmark: &ShelfmarkV1, capture_bytes: &[u8]) -> Result<(), String> {
    shelfmark
        .validate()
        .map_err(|error| format!("invalid shelfmark: {error:?}"))?;
    let address = ContentHash::of(capture_bytes).to_string();
    if shelfmark.projection != address {
        return Err(format!(
            "the shelfmark cites capture {}, but the capture is {address}",
            shelfmark.projection
        ));
    }
    let capture = ProjectionCaptureV2::decode(capture_bytes)
        .map_err(|error| format!("invalid capture: {error}"))?;
    let authority = capture
        .authority
        .as_ref()
        .ok_or("the capture names no authority")?;
    let input = shelfmark
        .inputs
        .get(SHELFMARK_AUTHORITY_ROLE)
        .ok_or("the shelfmark cites no authority")?;
    if input.authority.adapter != authority.adapter
        || input.authority.record != authority.sha256.to_string()
        || input.reading != authority.schema
    {
        return Err("the shelfmark cites a different authority".to_owned());
    }
    if input.expects_generation != authority.generation.to_string() {
        return Err(format!(
            "the shelfmark expects generation {}, but the capture is {}",
            input.expects_generation, authority.generation
        ));
    }
    Ok(())
}

/// Consume the three artifacts as a static reader would: decode the capture
/// without the compiler, check the site's conventions and its holds, check the
/// shelfmark against the capture, and replay the trace to every position.
pub fn consume_portable_projection(
    artifacts: &PortableProjection,
) -> Result<ProjectionReceipt, String> {
    read_portable_projection(artifacts).map(|reading| reading.receipt)
}

/// [`consume_portable_projection`], keeping what a build-time reader renders.
pub fn read_portable_projection(
    artifacts: &PortableProjection,
) -> Result<ProjectionReading, String> {
    open_portable_projection(artifacts).map(|opened| opened.reading)
}

/// The checked artifacts: what a build-time reader renders, plus the decoded
/// capture and trace that the browser session replays.
pub(crate) struct OpenedProjection {
    pub(crate) reading: ProjectionReading,
    pub(crate) trace: SceneTrace,
}

/// Every check [`read_portable_projection`] makes, keeping the decoded trace.
pub(crate) fn open_portable_projection(
    artifacts: &PortableProjection,
) -> Result<OpenedProjection, String> {
    let capture = ProjectionCaptureV2::decode(&artifacts.capture)
        .map_err(|error| format!("invalid capture: {error}"))?;
    if capture.version != PROJECTION_CAPTURE_V2 {
        return Err(format!("unexpected capture version {}", capture.version));
    }
    let authority = capture
        .authority
        .as_ref()
        .ok_or("the capture names no authority")?;
    let score = capture
        .score
        .as_ref()
        .ok_or("the capture carries no score")?;
    if authority.adapter != PROJECTION_ADAPTER {
        return Err(format!(
            "unsupported projection adapter {}",
            authority.adapter
        ));
    }
    if capture.scene.epoch.0 != authority.generation {
        return Err(format!(
            "the scene's epoch {} is not the authority generation {}",
            capture.scene.epoch.0, authority.generation
        ));
    }
    if score.items.len() != capture.scene.active_item_count() {
        return Err("score and scene item counts diverge".to_owned());
    }
    let honored_holds = honored_holds_of(&capture)?;

    // Read as text: the graph Wasm already reads shelfmarks through
    // serde_json's text reader, so this adds no second copy of incipit's
    // deserializer.
    let shelfmark = std::str::from_utf8(&artifacts.shelfmark)
        .map_err(|error| format!("invalid shelfmark JSON: {error}"))?;
    let shelfmark: ShelfmarkV1 = serde_json::from_str(shelfmark)
        .map_err(|error| format!("invalid shelfmark JSON: {error}"))?;
    check_shelfmark(&shelfmark, &artifacts.capture)?;

    let trace: SceneTrace = serde_json::from_slice(&artifacts.trace)
        .map_err(|error| format!("invalid scene trace: {error}"))?;
    if trace.base() != &capture.scene {
        return Err("the trace's base is not the captured scene".to_owned());
    }
    if trace.len() > PROJECTION_STEP_BOUND {
        return Err(format!(
            "the trace has {} steps, more than the proof's bound of {PROJECTION_STEP_BOUND}",
            trace.len()
        ));
    }
    let mut steps = Vec::with_capacity(trace.len());
    for (index, step) in trace.steps().iter().enumerate() {
        let selection = match &step.annotation {
            None => None,
            Some(annotation) => Some(read_selection(annotation, &trace, index)?),
        };
        steps.push(ReadingStep {
            label: step.label.clone(),
            revision: trace.revision_at(index + 1).map_err(trace_error)?.0,
            changes_scene: step.diff.is_some(),
            selection,
        });
    }

    let head = trace.head();
    head.validate()
        .map_err(|error| format!("invalid final scene snapshot: {error:?}"))?;
    let mere = instance_for_source(&head, PREFERRED_FOCUS_REPOSITORY)
        .ok_or_else(|| "portable scene lost Mere".to_owned())?;
    let mere_item = head
        .active_item(mere)
        .ok_or_else(|| "portable scene tombstoned Mere".to_owned())?;
    let picked = head
        .pick(mere_item.transform.translate)
        .ok_or_else(|| "native Scenotime consumer could not pick Mere".to_owned())?;
    let picked_source = source_of(&head, picked)?;
    if picked_source != PREFERRED_FOCUS_REPOSITORY {
        return Err(format!(
            "native Scenotime consumer picked {picked_source}, not Mere"
        ));
    }

    let scene = &capture.scene;
    let items = (0..scene.tables.items.len())
        .map(|index| source_of(scene, sceno::InstanceId(index as u32)))
        .collect::<Result<Vec<_>, _>>()?;
    let relations = scene
        .tables
        .relations
        .iter()
        .enumerate()
        .map(|(index, relation)| {
            let relation = relation
                .as_ref()
                .ok_or_else(|| format!("captured relation {index} is a tombstone"))?;
            Ok(CapturedRelation {
                index: index as u32,
                from: source_of(scene, relation.from)?,
                to: source_of(scene, relation.to)?,
                kind: relation
                    .kind
                    .clone()
                    .ok_or_else(|| format!("captured relation {index} has no kind"))?,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;

    let reading = ProjectionReading {
        receipt: ProjectionReceipt {
            schema: PROJECTION_RECEIPT_SCHEMA.to_owned(),
            capture_address: shelfmark.projection.clone(),
            authority_sha256: authority.sha256.to_string(),
            generation: authority.generation.to_string(),
            score_items: score.items.len(),
            initial_revision: scene.revision.0,
            final_revision: head.revision.0,
            active_items: head.active_item_count(),
            active_relations: head.tables.relations.iter().flatten().count(),
            picked_source,
            trace_steps: trace.len(),
            honored_holds,
        },
        items,
        relations,
        steps,
        epoch: scene.epoch.0.to_string(),
    };
    Ok(OpenedProjection { reading, trace })
}

/// A step's selection annotation: `{"kind": "node" | "edge", "id": ...}`, and
/// nothing else. A node must be one of the scene's sources at that step; an
/// edge id is the host dataset's, which the host checks.
fn read_selection(
    annotation: &Value,
    trace: &SceneTrace,
    index: usize,
) -> Result<(String, String), String> {
    let refuse = || format!("step {index} carries an annotation that is not a selection");
    let object = annotation.as_object().ok_or_else(refuse)?;
    if object.len() != 2 {
        return Err(refuse());
    }
    let kind = object
        .get("kind")
        .and_then(Value::as_str)
        .ok_or_else(refuse)?;
    let id = object
        .get("id")
        .and_then(Value::as_str)
        .ok_or_else(refuse)?;
    match kind {
        "node" => {
            let scene = trace.snapshot_at(index).map_err(trace_error)?;
            instance_for_source(&scene, id)
                .ok_or_else(|| format!("step {index} selects absent node {id}"))?;
        }
        "edge" => {}
        _ => return Err(refuse()),
    }
    Ok((kind.to_owned(), id.to_owned()))
}

/// Holds are checked against the scene as solved, not the scene after a
/// trace: a trace deliberately moves things, and an authored move later is not
/// a broken pin. What must hold is that the solver placed each held source
/// where the citation said.
pub(crate) fn honored_holds_of(capture: &ProjectionCaptureV2) -> Result<usize, String> {
    let Some(score) = &capture.score else {
        return Ok(0);
    };
    let mut honored = 0usize;
    for held in &score.holds {
        let instance = instance_for_source(&capture.scene, &held.source.id)
            .ok_or_else(|| format!("held source {} is absent from the scene", held.source.id))?;
        let item = capture
            .scene
            .active_item(instance)
            .ok_or_else(|| format!("held source {} is tombstoned", held.source.id))?;
        let at = item.transform.translate;
        if at.x != held.at.x || at.y != held.at.y {
            return Err(format!(
                "hold on {} was not honored: asked ({}, {}), realized ({}, {})",
                held.source.id, held.at.x, held.at.y, at.x, at.y
            ));
        }
        honored += 1;
    }
    Ok(honored)
}

fn source_of(snapshot: &SceneSnapshot, instance: sceno::InstanceId) -> Result<String, String> {
    let item = snapshot
        .active_item(instance)
        .ok_or_else(|| format!("instance {} is a tombstone", instance.0))?;
    Ok(snapshot.tables.sources[item.source.0 as usize]
        .as_ref()
        .ok_or_else(|| format!("instance {} has no source", instance.0))?
        .id
        .clone())
}
