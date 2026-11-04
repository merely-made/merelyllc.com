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
            link_key: None,
            marks: Default::default(),
            strength: None,
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
    // The fold is the S5 fact (Rulings 149-155): one AddFold, then its
    // TombstoneFold. No item's own visibility changes.
    let mere = instance_for_source(&current, PREFERRED_FOCUS_REPOSITORY)
        .ok_or_else(|| "the projection lost Mere".to_owned())?;
    let name = input
        .nodes
        .iter()
        .find(|node| node.id == PREFERRED_FOCUS_REPOSITORY)
        .map_or(PREFERRED_FOCUS_REPOSITORY, |node| node.name.as_str());
    if let Some((label, operations)) = fold_operations(&current, mere, name) {
        current = push(TraceStep::diff(label, next_diff(&current, operations)))?;
        let (label, operations) = fold_operations(&current, mere, name)
            .ok_or_else(|| "the folded scene cannot unfold".to_owned())?;
        push(TraceStep::diff(label, next_diff(&current, operations)))?;
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

mod open;

pub(crate) use open::{honored_holds_of, open_portable_projection};
