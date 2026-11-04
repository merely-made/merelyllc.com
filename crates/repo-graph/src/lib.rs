use std::collections::{BTreeMap, HashMap, HashSet};

mod arrangement;
mod portable;
mod proof_session;

use arrangement::{degree_weights, radial_rings, stack_layers};
use cartography::{
    ActorScope, PrimitiveBody, default_graph_reading_registry,
    default_graph_representation_registry,
};
use chirograph::{
    CoordinatedSelection, PROJECTION_CAPTURE_VERSION, ProjectionCaptureV1, SelectionResolution,
};
use euclid::default::Point2D;
use incipit::{ShelfmarkAuthorityV1, ShelfmarkInputV1, ShelfmarkV1};
use sceno::{
    Arrangement as SceneArrangement, AxisValue, Fold, FoldDirection, FoldRule, Footprint,
    HeldPlacement, InstanceId, Placement, ProjectedItem, Rect, Representation, RoutedRelation,
    Score, ScoreItem, Size2, SourceRef, Spiral, StandIn, Transform2, Vec2,
};
use scenotime::{FoldId, RelationId, Revision, SceneDiff, SceneEpoch, SceneOp, SceneSnapshot};
use seiche::{
    AnchorSpring, Boundary, EdgeSpring, NodeCollider, NodeExclusion, NodeKey, SceneBodySpec,
    SceneField, SceneSpec, Simulation,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use wasm_bindgen::prelude::*;

pub use portable::{
    CAPTURE_FILE, CapturedRelation, PROJECTION_STEP_BOUND, PortableProjection, ProjectionReading,
    ProjectionReceipt, ReadingStep, SHELFMARK_AUTHORITY_ROLE, SHELFMARK_FILE, TRACE_FILE,
    check_shelfmark, consume_portable_projection, portable_projection,
    projection_capture_with_placement_json, read_portable_projection,
};
pub use proof_session::{FoldView, ProjectionSession, RestoreRefusal, SceneView};

const PREFERRED_FOCUS_REPOSITORY: &str = "mere";
const DEFAULT_ARRANGEMENT: &str = "graph_layout:radial";
const TIMELINE_AXIS_LENGTH: f32 = 620.0;
const TIMELINE_LANE_COUNT: usize = 9;
const TIMELINE_LANE_GAP: f32 = 110.0;
const TIMELINE_MIN_X_GAP: f32 = 110.0;
const ARRANGEMENT_ORDER: &[&str] = &[
    "graph_layout:radial",
    "graph_layout:stack",
    "graph_layout:grid",
    "graph_layout:phyllotaxis",
    "graph_layout:timeline",
    "graph_layout:kanban",
    "graph_layout:penrose",
    "graph_layout:lsystem",
];
const UNAVAILABLE_ARRANGEMENTS: &[(&str, &str)] = &[(
    "graph_layout:semantic_embedding",
    "This site does not yet publish semantic coordinates.",
)];
const PROJECTION_ADAPTER: &str = "mer3ly.repository-graph/v1";
const MATRIX_PROJECTION_SCHEMA: &str = "mer3ly.two-reading-matrix/v1";
const MATRIX_RELATION_ADAPTER: &str = "mer3ly.repository-relation/v1";
const MATRIX_DERIVATION_ADAPTER: &str = "mer3ly.matrix-derivation/v1";

#[derive(Clone, Debug, Deserialize, Serialize)]
struct GraphInput {
    schema: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    focus: Option<String>,
    nodes: Vec<GraphNodeInput>,
    edges: Vec<GraphEdge>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct GraphNodeInput {
    id: String,
    name: String,
    class: String,
    status: String,
    pushed_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    change: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    summary: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct GraphEdge {
    id: String,
    source: String,
    target: String,
    kind: String,
    provenance: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    change: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
struct GraphLayout {
    schema: &'static str,
    authority_schema: String,
    engine: String,
    focus: String,
    default_arrangement: &'static str,
    nodes: Vec<GraphNodeLayout>,
    edges: Vec<GraphEdge>,
    arrangements: Vec<GraphArrangement>,
    unavailable_arrangements: Vec<UnavailableArrangement>,
}

#[derive(Clone, Debug, Serialize)]
struct GraphNodeLayout {
    id: String,
    name: String,
    class: String,
    status: String,
    pushed_at: String,
    x: f32,
    y: f32,
}

#[derive(Clone, Debug, Serialize)]
struct GraphArrangement {
    id: String,
    name: String,
    description: String,
    engine: String,
    nodes: Vec<GraphNodePosition>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct GraphNodePosition {
    id: String,
    x: f32,
    y: f32,
}

#[derive(Clone, Debug, Serialize)]
struct UnavailableArrangement {
    id: String,
    name: String,
    reason: String,
}

#[derive(Clone, Debug, Serialize)]
struct PhysicsFrame {
    schema: &'static str,
    nodes: Vec<PhysicsNode>,
    props: Vec<PhysicsProp>,
    at_rest: bool,
}

#[derive(Clone, Debug, Serialize)]
struct PhysicsNode {
    id: String,
    x: f32,
    y: f32,
    pinned: bool,
}

#[derive(Clone, Debug, Serialize)]
struct PhysicsProp {
    x: f32,
    y: f32,
    rotation: f32,
    shape: &'static str,
    radius: Option<f32>,
    half: Option<f32>,
    points: Vec<(f32, f32)>,
}

/// Stateful browser adapter over Mere's real Seiche simulation.
///
/// Arrangements supply slots. This adapter uses those slots as anchor-spring
/// targets or as initial conditions for free physics. Frozen output belongs to
/// a non-interactive renderer, not this interactive simulation.
#[wasm_bindgen]
pub struct GraphPhysics {
    simulation: Simulation,
    key_by_id: HashMap<String, NodeKey>,
    id_by_key: HashMap<NodeKey, String>,
    manually_pinned: HashSet<String>,
    mobility: String,
}

#[wasm_bindgen]
impl GraphPhysics {
    #[wasm_bindgen(constructor)]
    pub fn new(input: &str) -> Result<GraphPhysics, JsValue> {
        graph_physics(input).map_err(|error| JsValue::from_str(&error))
    }

    #[wasm_bindgen(js_name = setArrangement)]
    pub fn set_arrangement(&mut self, positions: &str, mobility: &str) -> Result<(), JsValue> {
        self.apply_arrangement(positions, mobility)
            .map_err(|error| JsValue::from_str(&error))
    }

    #[wasm_bindgen(js_name = setBackdrop)]
    pub fn set_backdrop(&mut self, backdrop: &str, tangible: bool) -> Result<(), JsValue> {
        self.apply_backdrop(backdrop, tangible)
            .map_err(|error| JsValue::from_str(&error))
    }

    #[wasm_bindgen(js_name = pinNode)]
    pub fn pin_node(&mut self, id: &str, x: f32, y: f32) -> Result<(), JsValue> {
        let key = self
            .key_by_id
            .get(id)
            .copied()
            .ok_or_else(|| JsValue::from_str(&format!("unknown graph node {id}")))?;
        self.manually_pinned.insert(id.to_owned());
        self.simulation.pin(key, Point2D::new(x, y));
        Ok(())
    }

    #[wasm_bindgen(js_name = unpinNode)]
    pub fn unpin_node(&mut self, id: &str) -> Result<(), JsValue> {
        let key = self
            .key_by_id
            .get(id)
            .copied()
            .ok_or_else(|| JsValue::from_str(&format!("unknown graph node {id}")))?;
        self.manually_pinned.remove(id);
        self.simulation.unpin(key);
        Ok(())
    }

    #[wasm_bindgen(js_name = isPinned)]
    pub fn is_pinned(&self, id: &str) -> bool {
        self.manually_pinned.contains(id)
    }

    pub fn tick(&mut self, dt: f32) -> Result<String, JsValue> {
        self.simulation.tick(dt.clamp(1.0 / 240.0, 1.0 / 20.0));
        self.frame_json().map_err(|error| JsValue::from_str(&error))
    }

    pub fn frame(&self) -> Result<String, JsValue> {
        self.frame_json().map_err(|error| JsValue::from_str(&error))
    }
}

/// The same portable primitive and host-behavior registry consumed by Mere.
#[wasm_bindgen]
pub fn representation_registry() -> Result<String, JsValue> {
    serde_json::to_string(&default_graph_representation_registry()).map_err(|error| {
        JsValue::from_str(&format!(
            "could not encode representation registry: {error}"
        ))
    })
}

/// Mere's portable catalog of graph readings.
#[wasm_bindgen]
pub fn reading_registry() -> Result<String, JsValue> {
    serde_json::to_string(&default_graph_reading_registry())
        .map_err(|error| JsValue::from_str(&format!("could not encode reading registry: {error}")))
}

/// Project one graph revision through a reading selected from Mere's registry.
#[wasm_bindgen]
pub fn project_reading(input: &str) -> Result<String, JsValue> {
    project_reading_json(input).map_err(|error| JsValue::from_str(&error))
}

/// Build a Matrix over two independently produced readings and carry its
/// scene through the Graphshell projection-capture envelope.
#[wasm_bindgen]
pub fn project_matrix(input: &str) -> Result<String, JsValue> {
    matrix_projection_json(input).map_err(|error| JsValue::from_str(&error))
}

/// Cite a composed Matrix with each authority input separately checkable.
#[wasm_bindgen]
pub fn compose_matrix_shelfmark(input: &str) -> Result<String, JsValue> {
    composed_matrix_shelfmark_json(input).map_err(|error| JsValue::from_str(&error))
}

/// Reconstitute and verify a composed Matrix citation against supplied
/// authorities.
#[wasm_bindgen]
pub fn resolve_matrix_shelfmark(input: &str) -> Result<String, JsValue> {
    resolve_matrix_shelfmark_json(input).map_err(|error| JsValue::from_str(&error))
}

/// The generation a citation should expect for this authority.
///
/// A shared scene link carries `expects.generation`; opening the link
/// recomputes this over the loaded authority and compares. Returned as a
/// decimal string because a u64 does not survive a JS number.
#[wasm_bindgen]
pub fn authority_generation(graph: &str) -> Result<String, JsValue> {
    let input: GraphInput = serde_json::from_str(graph)
        .map_err(|error| JsValue::from_str(&format!("invalid graph JSON: {error}")))?;
    let (_, generation) = authority_identity(&input).map_err(|error| JsValue::from_str(&error))?;
    Ok(generation.to_string())
}

/// Turn a shared scene state into a projection capture that keeps its pins.
///
/// The sandbox hands over the graph authority and its own scene state; what
/// comes back is chirograph's encoding of a V2 capture whose score holds the
/// visitor's placement. Distinct from sharing a scene: a share is a citation,
/// small enough for a URL fragment, and this is the realized thing it cites.
///
/// This export is why the module's size ceiling is what it is. Reaching it
/// pulls the whole portable path into the browser, score and scene serde plus
/// `scenomise::solve`, which the live path alone never needed.
#[wasm_bindgen]
pub fn projection_capture_with_placement(graph: &str, placement: &str) -> Result<String, JsValue> {
    projection_capture_with_placement_json(graph, placement)
        .map_err(|error| JsValue::from_str(&error))
}

/// One visitor-placed node, in the shape the sandbox already shares.
///
/// This is the wire's `pins` entry verbatim, so the seam reads the record the
/// live path already produces rather than inventing a second one.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct PlacementPin {
    pub id: String,
    pub x: f32,
    pub y: f32,
}

/// The placement half of a shared scene state: what a visitor did to the
/// arrangement that the authority does not know.
///
/// Deserialized straight from the sandbox's scene state, extra fields ignored,
/// so a caller may hand over the whole record.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct PlacementDelta {
    /// `anchored` or `free`, the live path's two motion classes. Absent means
    /// free. It does not decide a pin's class: pins record as pinned in either.
    #[serde(default)]
    pub motion: Option<String>,
    #[serde(default)]
    pub pins: Vec<PlacementPin>,
}

impl PlacementDelta {
    /// Translate the live path's placement into score holds.
    ///
    /// A manual pin is hard in the sandbox in either motion: it survives until
    /// it is removed, so it records as [`sceno::Hold::Pinned`] whatever the motion
    /// (Ruling 103). The motion is the item's default role; a pin overrides it.
    ///
    /// The class is *recorded* here rather than left to be re-inferred from a
    /// spring stiffness, which is the whole point of the seam.
    fn holds(&self) -> Vec<HeldPlacement> {
        self.pins
            .iter()
            .map(|pin| {
                HeldPlacement::pinned(
                    SourceRef::new(PROJECTION_ADAPTER, &pin.id),
                    Vec2::new(pin.x, pin.y),
                )
            })
            .collect()
    }
}

/// The relation family the proof folds along (site canvas plan, Ruling 154).
const FOLD_FAMILY: &str = "depends_on";

#[wasm_bindgen]
pub fn layout_graph(input: &str) -> Result<String, JsValue> {
    layout_graph_json(input).map_err(|error| JsValue::from_str(&error))
}

// `arrangement_description` lived here to override the layout registry's own
// descriptions for a repository audience, falling back to whatever the registry
// carried. The scenograph absorption retired that registry, and the overrides
// moved into `arrangement::CATALOG` where the names live beside them, so there
// is no second source to reconcile against. Removed rather than left dead: its
// remaining callers were the ones the catalog replaced.

#[cfg(test)]
mod tests;

mod matrix;

use matrix::{consume_matrix_projection, matrix_projection, matrix_projection_json};

mod shelfmarks;

use shelfmarks::{composed_matrix_shelfmark_json, resolve_matrix_shelfmark_json};

mod readings;

use readings::{
    authority_identity, move_diff, next_diff, project_reading_json, project_reading_request,
};

mod layout;

use layout::{focal_node, layout_graph_json, validate};

mod physics_state;

mod physics;

use physics::{graph_physics, physics_prop, sandbox_props_scene};

mod folds;

pub(crate) use folds::fold_operations;

use folds::instance_for_source;

mod matrix_types;

use matrix_types::{
    BackdropDelta, CameraDelta, ComposedShelfmarkRequest, MatrixAuthorityRecord, MatrixAxis,
    MatrixAxisRequest, MatrixAxisSource, MatrixCell, MatrixCellKind, MatrixContributor,
    MatrixInstanceDelta, MatrixProjectionArtifact, MatrixProjectionRequest,
    MatrixReadingParameters, MatrixReceipt, MatrixShelfmarkReceipt,
    MatrixShelfmarkResolutionRequest, ProjectedInstanceAddress, ReadingRequest,
    ResolvedMatrixDataset,
};

#[cfg(test)]
use layout::{place_timeline_strips, timestamp_coordinate};
#[cfg(test)]
use matrix_types::SpatialShelfmarkRequest;
#[cfg(test)]
use shelfmarks::{composed_matrix_shelfmark, read_placement, resolve_matrix_shelfmark_value};
