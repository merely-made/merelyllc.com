use std::collections::{BTreeMap, HashMap, HashSet};

mod arrangement;

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
    Arrangement as SceneArrangement, AxisValue, Footprint, HeldPlacement, Hold, InstanceId,
    Placement, ProjectedItem, Rect, Representation, RoutedRelation, Score, ScoreItem, Size2,
    SourceRef, Spiral, Transform2, Vec2,
};
use scenotime::{RelationId, Revision, SceneDiff, SceneEpoch, SceneOp, SceneSnapshot};
use seiche::{
    AnchorSpring, Boundary, EdgeSpring, NodeCollider, NodeExclusion, NodeKey, SceneBodySpec,
    SceneField, SceneSpec, Simulation,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use wasm_bindgen::prelude::*;

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
const PORTABLE_PROJECTION_SCHEMA: &str = "mer3ly.portable-projection/v1";
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

#[derive(Clone, Debug, Deserialize)]
struct ReadingRequest {
    reading: String,
    current: GraphInput,
    #[serde(default)]
    previous: Option<GraphInput>,
    #[serde(default)]
    focus: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct MatrixAxisRequest {
    dataset: String,
    record: String,
    reading: String,
    current: GraphInput,
    #[serde(default)]
    previous: Option<GraphInput>,
    #[serde(default)]
    focus: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct MatrixProjectionRequest {
    rows: MatrixAxisRequest,
    columns: MatrixAxisRequest,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct MatrixAxisSource {
    source: SourceRef,
    name: String,
    class: String,
    status: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct MatrixAxis {
    dataset: String,
    record: String,
    reading: String,
    focus: Option<String>,
    authority_sha256: String,
    generation: String,
    sources: Vec<MatrixAxisSource>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum MatrixCellKind {
    Relation,
    IdentityMatch,
    Absence,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct MatrixContributor {
    authority: String,
    source: SourceRef,
    provenance: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct MatrixCell {
    instance: InstanceId,
    row: SourceRef,
    column: SourceRef,
    source: SourceRef,
    kind: MatrixCellKind,
    value: String,
    description: String,
    contributors: Vec<MatrixContributor>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct MatrixProjectionArtifact {
    schema: String,
    rows: MatrixAxis,
    columns: MatrixAxis,
    cells: Vec<MatrixCell>,
    capture: ProjectionCaptureV1,
    accessible_html: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
struct MatrixReceipt {
    schema: String,
    row_sources: usize,
    column_sources: usize,
    cells: usize,
    relation_cells: usize,
    scene_instances: usize,
    capture_bytes: usize,
    accessible_table: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
struct ProjectedInstanceAddress {
    view: String,
    source: SourceRef,
    facet: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
struct MatrixInstanceDelta {
    instance: ProjectedInstanceAddress,
    visible: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct ComposedShelfmarkRequest {
    matrix: MatrixProjectionRequest,
    spatial: SpatialShelfmarkRequest,
    selection: CoordinatedSelection,
    #[serde(default)]
    instances: Vec<MatrixInstanceDelta>,
    #[serde(default)]
    placement: Vec<HeldPlacement>,
    #[serde(default)]
    motion: Option<String>,
    #[serde(default)]
    backdrop: Option<BackdropDelta>,
    #[serde(default)]
    facets: Vec<ProjectedInstanceAddress>,
    #[serde(default)]
    camera: Option<CameraDelta>,
    #[serde(default)]
    carried_delta: BTreeMap<String, String>,
}

/// The adjacent spatial projection belongs beside a Matrix citation, rather
/// than being inferred from the Matrix axes.  It cites the same authority
/// shape but keeps its reading and arrangement as an independently checkable
/// Shelfmark input.
#[derive(Clone, Debug, Deserialize, Serialize)]
struct SpatialShelfmarkRequest {
    dataset: String,
    record: String,
    reading: String,
    current: GraphInput,
    #[serde(default)]
    previous: Option<GraphInput>,
    #[serde(default)]
    focus: Option<String>,
    arrangement: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct BackdropDelta {
    kind: String,
    collidable: bool,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
struct CameraDelta {
    x: f32,
    y: f32,
    zoom: f32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct ResolvedMatrixDataset {
    current: GraphInput,
    #[serde(default)]
    previous: Option<GraphInput>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct MatrixShelfmarkResolutionRequest {
    shelfmark: ShelfmarkV1,
    datasets: BTreeMap<String, ResolvedMatrixDataset>,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
struct MatrixShelfmarkReceipt {
    matrix: MatrixReceipt,
    input_generations: BTreeMap<String, String>,
    selection_resolution: SelectionResolution,
    honored_instance_deltas: usize,
    honored_placements: usize,
    honored_facets: usize,
    camera: CameraDelta,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct MatrixAuthorityRecord {
    dataset: String,
    record: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct MatrixReadingParameters {
    #[serde(default)]
    focus: Option<String>,
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

fn graph_physics(input: &str) -> Result<GraphPhysics, String> {
    let input: GraphInput =
        serde_json::from_str(input).map_err(|error| format!("invalid graph JSON: {error}"))?;
    validate(&input)?;

    let key_by_id = input
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| (node.id.clone(), NodeKey::new(index)))
        .collect::<HashMap<_, _>>();
    let id_by_key = key_by_id
        .iter()
        .map(|(id, key)| (*key, id.clone()))
        .collect::<HashMap<_, _>>();
    let mut simulation = Simulation::new();
    simulation.set_linear_damping(3.6);
    simulation.add_force(NodeExclusion {
        strength: 125_000.0,
        cutoff: 480.0,
        min_distance: 12.0,
    });
    simulation.add_force(EdgeSpring {
        stiffness: 3.4,
        rest_length: 125.0,
    });
    simulation.add_force(Boundary { strength: 0.045 });
    simulation.sync_nodes(input.nodes.iter().enumerate().map(|(index, _)| {
        let angle = index as f32 * 2.399_963_1;
        let radius = 24.0 * (index as f32).sqrt();
        (
            NodeKey::new(index),
            Point2D::new(radius * angle.cos(), radius * angle.sin()),
        )
    }));
    simulation.sync_edges(
        input.edges.iter().filter_map(|edge| {
            Some((*key_by_id.get(&edge.source)?, *key_by_id.get(&edge.target)?))
        }),
    );
    simulation.set_node_colliders(
        input
            .nodes
            .iter()
            .filter_map(|node| Some((*key_by_id.get(&node.id)?, collider_for_class(&node.class)))),
    );

    Ok(GraphPhysics {
        simulation,
        key_by_id,
        id_by_key,
        manually_pinned: HashSet::new(),
        mobility: "anchored".to_owned(),
    })
}

fn collider_for_class(class: &str) -> NodeCollider {
    let registry = default_graph_representation_registry();
    match registry.resolve(class).primitive.body {
        PrimitiveBody::Square => NodeCollider::Square { half: 22.0 },
        PrimitiveBody::RoundedSquare => NodeCollider::RoundedSquare {
            half: 24.0,
            border: 7.0,
        },
        PrimitiveBody::Diamond => NodeCollider::Hull {
            points: vec![(0.0, -27.0), (27.0, 0.0), (0.0, 27.0), (-27.0, 0.0)],
            fallback: 24.0,
        },
        PrimitiveBody::Hexagon => NodeCollider::Hull {
            points: vec![
                (-22.0, -13.0),
                (0.0, -26.0),
                (22.0, -13.0),
                (22.0, 13.0),
                (0.0, 26.0),
                (-22.0, 13.0),
            ],
            fallback: 24.0,
        },
        PrimitiveBody::Circle => NodeCollider::Ball { radius: 24.0 },
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

/// Turn a shared scene state into a portable projection that keeps its pins.
///
/// The sandbox hands over the graph authority and its own scene state; what
/// comes back is a Scenograph artifact whose score holds the visitor's
/// placement. Distinct from sharing a scene: a share is a citation, small
/// enough for a URL fragment, and this is the realized thing it cites.
///
/// This export is why the module's size ceiling is what it is. Reaching it
/// pulls the whole portable path into the browser, score and scene serde plus
/// `scenomise::solve`, which the live path alone never needed.
#[wasm_bindgen]
pub fn portable_projection_with_placement(graph: &str, placement: &str) -> Result<String, JsValue> {
    portable_projection_with_placement_json(graph, placement)
        .map_err(|error| JsValue::from_str(&error))
}

fn project_reading_json(input: &str) -> Result<String, String> {
    let request: ReadingRequest =
        serde_json::from_str(input).map_err(|error| format!("invalid reading request: {error}"))?;
    let projection = project_reading_request(request)?;
    serde_json::to_string(&projection)
        .map_err(|error| format!("could not encode graph reading: {error}"))
}

fn project_reading_request(request: ReadingRequest) -> Result<GraphInput, String> {
    validate(&request.current)?;
    if let Some(previous) = &request.previous {
        validate(previous)?;
    }
    let registry = default_graph_reading_registry();
    let profile = registry
        .resolve(&request.reading)
        .ok_or_else(|| format!("unknown graph reading {}", request.reading))?;
    let authored_changes = request
        .current
        .nodes
        .iter()
        .any(|node| node.change.is_some());
    let current = decorate_current(request.current, request.previous.as_ref());
    let projection = match profile.actor_scope {
        ActorScope::All => current,
        ActorScope::AdjacentRevision if request.previous.is_none() && authored_changes => current,
        ActorScope::AdjacentRevision => diff_graphs(request.previous.as_ref(), &current),
        ActorScope::FocusAndNeighbors => {
            let focus = request
                .focus
                .as_deref()
                .filter(|id| current.nodes.iter().any(|node| node.id == *id))
                .or(current.focus.as_deref())
                .unwrap_or_else(|| focal_node(&current));
            focus_and_neighbors(&current, focus)
        }
    };
    Ok(projection)
}

fn matrix_projection_json(input: &str) -> Result<String, String> {
    let request: MatrixProjectionRequest = serde_json::from_str(input)
        .map_err(|error| format!("invalid Matrix projection request: {error}"))?;
    let artifact = matrix_projection(&request)?;
    serde_json::to_string(&artifact)
        .map_err(|error| format!("could not encode Matrix projection: {error}"))
}

fn matrix_axis(request: &MatrixAxisRequest) -> Result<(MatrixAxis, GraphInput), String> {
    if request.dataset.trim().is_empty() || request.record.trim().is_empty() {
        return Err("a Matrix axis needs a dataset and authority record".to_owned());
    }
    let (authority_sha256, generation) = authority_identity(&request.current)?;
    let projection = project_reading_request(ReadingRequest {
        reading: request.reading.clone(),
        current: request.current.clone(),
        previous: request.previous.clone(),
        focus: request.focus.clone(),
    })?;
    let sources = projection
        .nodes
        .iter()
        .map(|node| MatrixAxisSource {
            source: SourceRef::new(PROJECTION_ADAPTER, &node.id),
            name: node.name.clone(),
            class: node.class.clone(),
            status: node.status.clone(),
        })
        .collect();
    Ok((
        MatrixAxis {
            dataset: request.dataset.clone(),
            record: request.record.clone(),
            reading: request.reading.clone(),
            focus: request.focus.clone(),
            authority_sha256,
            generation: generation.to_string(),
            sources,
        },
        projection,
    ))
}

fn matrix_projection(
    request: &MatrixProjectionRequest,
) -> Result<MatrixProjectionArtifact, String> {
    let (rows, row_projection) = matrix_axis(&request.rows)?;
    let (columns, column_projection) = matrix_axis(&request.columns)?;
    if rows.reading == columns.reading
        && rows.dataset == columns.dataset
        && rows.focus == columns.focus
    {
        return Err("Matrix axes must be independently produced readings".to_owned());
    }
    if rows.sources.is_empty() || columns.sources.is_empty() {
        return Err("Matrix axes must each produce at least one source".to_owned());
    }

    let generation_material = serde_json::to_vec(&(
        &rows.dataset,
        &rows.authority_sha256,
        &rows.reading,
        &rows.focus,
        &columns.dataset,
        &columns.authority_sha256,
        &columns.reading,
        &columns.focus,
    ))
    .map_err(|error| format!("could not identify Matrix projection: {error}"))?;
    let digest = Sha256::digest(generation_material);
    let generation = u64::from_be_bytes(
        digest[..8]
            .try_into()
            .expect("SHA-256 prefix is eight bytes"),
    );

    let mut scene = sceno::Scene::new();
    scene.generation = generation;
    let cell_size = 64.0;
    let heading_footprint = Footprint::Rect {
        size: Size2::new(cell_size - 8.0, cell_size - 8.0),
    };
    for (index, source) in rows.sources.iter().enumerate() {
        let source_ix = scene.intern_source(source.source.clone());
        scene.items.push(ProjectedItem {
            source: source_ix,
            space: sceno::Scene::WORLD,
            transform: Transform2::translation(0.0, (index as f32 + 1.0) * cell_size),
            footprint: heading_footprint.clone(),
            representation: Representation::Open {
                kind: "matrix.row-heading".into(),
            },
            layer: 1,
            visible: true,
            hit: None,
            channels: Vec::new(),
        });
    }
    for (index, source) in columns.sources.iter().enumerate() {
        let source_ix = scene.intern_source(source.source.clone());
        scene.items.push(ProjectedItem {
            source: source_ix,
            space: sceno::Scene::WORLD,
            transform: Transform2::translation((index as f32 + 1.0) * cell_size, 0.0),
            footprint: heading_footprint.clone(),
            representation: Representation::Open {
                kind: "matrix.column-heading".into(),
            },
            layer: 1,
            visible: true,
            hit: None,
            channels: Vec::new(),
        });
    }

    let mut cells = Vec::with_capacity(rows.sources.len() * columns.sources.len());
    for (row_index, row) in rows.sources.iter().enumerate() {
        for (column_index, column) in columns.sources.iter().enumerate() {
            let contributors = matrix_cell_contributors(request, &row.source, &column.source);
            let (kind, value, description, cell_source) = if contributors
                .iter()
                .any(|contributor| contributor.source.adapter == MATRIX_RELATION_ADAPTER)
            {
                let relation_names = contributors
                    .iter()
                    .filter(|contributor| contributor.source.adapter == MATRIX_RELATION_ADAPTER)
                    .map(|contributor| contributor.source.id.as_str())
                    .collect::<Vec<_>>();
                (
                    MatrixCellKind::Relation,
                    "relation".to_owned(),
                    format!(
                        "{} relation{} from {} to {}",
                        relation_names.len(),
                        if relation_names.len() == 1 { "" } else { "s" },
                        row.name,
                        column.name
                    ),
                    SourceRef::new(MATRIX_RELATION_ADAPTER, relation_names.join("+")),
                )
            } else if row.source == column.source {
                (
                    MatrixCellKind::IdentityMatch,
                    "same source".to_owned(),
                    format!("{} is present in both readings", row.name),
                    SourceRef::new(
                        MATRIX_DERIVATION_ADAPTER,
                        format!("identity:{}", row.source.id),
                    ),
                )
            } else {
                (
                    MatrixCellKind::Absence,
                    "no relation".to_owned(),
                    format!("No direct relation from {} to {}", row.name, column.name),
                    SourceRef::new(
                        MATRIX_DERIVATION_ADAPTER,
                        format!("absence:{}:{}", row.source.id, column.source.id),
                    ),
                )
            };
            let source_ix = scene.intern_source(cell_source.clone());
            let instance = InstanceId(scene.items.len() as u32);
            scene.items.push(ProjectedItem {
                source: source_ix,
                space: sceno::Scene::WORLD,
                transform: Transform2::translation(
                    (column_index as f32 + 1.0) * cell_size,
                    (row_index as f32 + 1.0) * cell_size,
                ),
                footprint: Footprint::Rect {
                    size: Size2::new(cell_size - 8.0, cell_size - 8.0),
                },
                representation: Representation::Open {
                    kind: match kind {
                        MatrixCellKind::Relation => "matrix.relation-cell",
                        MatrixCellKind::IdentityMatch => "matrix.identity-cell",
                        MatrixCellKind::Absence => "matrix.absence-cell",
                    }
                    .into(),
                },
                layer: 0,
                visible: true,
                hit: None,
                channels: vec![(
                    "matrix.value".into(),
                    if kind == MatrixCellKind::Absence {
                        0.0
                    } else {
                        1.0
                    },
                )],
            });
            cells.push(MatrixCell {
                instance,
                row: row.source.clone(),
                column: column.source.clone(),
                source: cell_source,
                kind,
                value,
                description,
                contributors,
            });
        }
    }
    scene.bounds = Rect::new(
        Vec2::ZERO,
        Size2::new(
            (columns.sources.len() as f32 + 1.0) * cell_size,
            (rows.sources.len() as f32 + 1.0) * cell_size,
        ),
    );
    let snapshot = SceneSnapshot::from_dense(SceneEpoch(generation), Revision(1), scene)
        .map_err(|error| format!("Scenograph rejected the Matrix scene: {error:?}"))?;
    let capture = ProjectionCaptureV1 {
        version: PROJECTION_CAPTURE_VERSION,
        scene: snapshot,
        presentation: chirograph::PresentationManifest::default(),
    };
    capture
        .validate()
        .map_err(|error| format!("Graphshell refused the Matrix capture: {error}"))?;
    let accessible_html = matrix_accessible_html(&rows, &columns, &cells);
    let artifact = MatrixProjectionArtifact {
        schema: MATRIX_PROJECTION_SCHEMA.to_owned(),
        rows,
        columns,
        cells,
        capture,
        accessible_html,
    };
    consume_matrix_projection(&artifact)?;
    let _ = (row_projection, column_projection);
    Ok(artifact)
}

fn matrix_cell_contributors(
    request: &MatrixProjectionRequest,
    row: &SourceRef,
    column: &SourceRef,
) -> Vec<MatrixContributor> {
    let mut contributors = BTreeMap::new();
    for axis in [&request.rows, &request.columns] {
        for edge in &axis.current.edges {
            if edge.source == row.id && edge.target == column.id {
                let key = format!("{}:{}", axis.dataset, edge.id);
                contributors
                    .entry(key)
                    .or_insert_with(|| MatrixContributor {
                        authority: axis.dataset.clone(),
                        source: SourceRef::new(MATRIX_RELATION_ADAPTER, &edge.id),
                        provenance: edge.provenance.clone(),
                    });
            }
        }
    }
    if contributors.is_empty() {
        for (authority, source) in [
            (&request.rows.dataset, row),
            (&request.columns.dataset, column),
        ] {
            let key = format!("{authority}:{}", source.id);
            contributors
                .entry(key)
                .or_insert_with(|| MatrixContributor {
                    authority: authority.clone(),
                    source: source.clone(),
                    provenance: "axis input".into(),
                });
        }
    }
    contributors.into_values().collect()
}

fn matrix_accessible_html(rows: &MatrixAxis, columns: &MatrixAxis, cells: &[MatrixCell]) -> String {
    let mut html =
        String::from("<table data-projection-family=\"matrix\"><caption>Two-reading Matrix: ");
    html.push_str(&escape_html(&rows.reading));
    html.push_str(" by ");
    html.push_str(&escape_html(&columns.reading));
    html.push_str("</caption><thead><tr><th scope=\"col\">Rows / columns</th>");
    for (column_index, column) in columns.sources.iter().enumerate() {
        html.push_str("<th scope=\"col\" data-projection-instance=\"");
        html.push_str(&(rows.sources.len() + column_index).to_string());
        html.push_str("\" data-source-adapter=\"");
        html.push_str(&escape_html(&column.source.adapter));
        html.push_str("\" data-source-id=\"");
        html.push_str(&escape_html(&column.source.id));
        html.push_str("\">");
        html.push_str(&escape_html(&column.name));
        html.push_str("</th>");
    }
    html.push_str("</tr></thead><tbody>");
    for (row_index, row) in rows.sources.iter().enumerate() {
        html.push_str("<tr><th scope=\"row\" data-projection-instance=\"");
        html.push_str(&row_index.to_string());
        html.push_str("\" data-source-adapter=\"");
        html.push_str(&escape_html(&row.source.adapter));
        html.push_str("\" data-source-id=\"");
        html.push_str(&escape_html(&row.source.id));
        html.push_str("\">");
        html.push_str(&escape_html(&row.name));
        html.push_str("</th>");
        for column_index in 0..columns.sources.len() {
            let cell = &cells[row_index * columns.sources.len() + column_index];
            html.push_str("<td data-projection-instance=\"");
            html.push_str(&cell.instance.0.to_string());
            html.push_str("\" data-source-adapter=\"");
            html.push_str(&escape_html(&cell.source.adapter));
            html.push_str("\" data-source-id=\"");
            html.push_str(&escape_html(&cell.source.id));
            html.push_str("\" data-matrix-row-adapter=\"");
            html.push_str(&escape_html(&cell.row.adapter));
            html.push_str("\" data-matrix-row-id=\"");
            html.push_str(&escape_html(&cell.row.id));
            html.push_str("\" data-matrix-column-adapter=\"");
            html.push_str(&escape_html(&cell.column.adapter));
            html.push_str("\" data-matrix-column-id=\"");
            html.push_str(&escape_html(&cell.column.id));
            html.push_str("\" aria-label=\"");
            html.push_str(&escape_html(&cell.description));
            html.push_str("\">");
            html.push_str(&escape_html(&cell.value));
            html.push_str("</td>");
        }
        html.push_str("</tr>");
    }
    html.push_str("</tbody></table>");
    html
}

fn escape_html(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn consume_matrix_projection(artifact: &MatrixProjectionArtifact) -> Result<MatrixReceipt, String> {
    if artifact.schema != MATRIX_PROJECTION_SCHEMA {
        return Err(format!("unsupported Matrix schema {}", artifact.schema));
    }
    let capture_bytes = artifact
        .capture
        .encode()
        .map_err(|error| format!("could not carry Matrix through Graphshell: {error}"))?;
    let far_side = ProjectionCaptureV1::decode(&capture_bytes)
        .map_err(|error| format!("could not restore Matrix from Graphshell: {error}"))?;
    if far_side != artifact.capture {
        return Err("Matrix capture changed during Graphshell carriage".to_owned());
    }
    let expected_cells = artifact.rows.sources.len() * artifact.columns.sources.len();
    if artifact.cells.len() != expected_cells {
        return Err("Matrix cell count does not match its axes".to_owned());
    }
    let expected_instances =
        artifact.rows.sources.len() + artifact.columns.sources.len() + artifact.cells.len();
    if far_side.scene.active_item_count() != expected_instances {
        return Err("Matrix headings and cells did not all survive carriage".to_owned());
    }
    for cell in &artifact.cells {
        if cell.contributors.is_empty() || far_side.scene.active_item(cell.instance).is_none() {
            return Err(format!(
                "Matrix cell {} lacks an instance or contributor provenance",
                cell.instance.0
            ));
        }
    }
    let accessible_table = artifact.accessible_html.starts_with("<table")
        && artifact.accessible_html.contains("<caption>")
        && artifact.accessible_html.contains("scope=\"row\"")
        && artifact.accessible_html.contains("scope=\"col\"");
    if !accessible_table {
        return Err("Matrix lacks its accessible table realization".to_owned());
    }
    Ok(MatrixReceipt {
        schema: MATRIX_PROJECTION_SCHEMA.to_owned(),
        row_sources: artifact.rows.sources.len(),
        column_sources: artifact.columns.sources.len(),
        cells: artifact.cells.len(),
        relation_cells: artifact
            .cells
            .iter()
            .filter(|cell| cell.kind == MatrixCellKind::Relation)
            .count(),
        scene_instances: expected_instances,
        capture_bytes: capture_bytes.len(),
        accessible_table,
    })
}

fn composed_matrix_shelfmark_json(input: &str) -> Result<String, String> {
    let request: ComposedShelfmarkRequest = serde_json::from_str(input)
        .map_err(|error| format!("invalid composed shelfmark request: {error}"))?;
    let artifact = matrix_projection(&request.matrix)?;
    let shelfmark = composed_matrix_shelfmark(&request, &artifact)?;
    serde_json::to_string(&shelfmark)
        .map_err(|error| format!("could not encode composed shelfmark: {error}"))
}

fn composed_matrix_shelfmark(
    request: &ComposedShelfmarkRequest,
    artifact: &MatrixProjectionArtifact,
) -> Result<ShelfmarkV1, String> {
    let mut shelfmark = ShelfmarkV1::new("matrix");
    for (role, axis) in [("rows", &artifact.rows), ("columns", &artifact.columns)] {
        let authority = MatrixAuthorityRecord {
            dataset: axis.dataset.clone(),
            record: axis.record.clone(),
        };
        let parameters = MatrixReadingParameters {
            focus: axis.focus.clone(),
        };
        shelfmark.inputs.insert(
            role.to_owned(),
            ShelfmarkInputV1 {
                authority: ShelfmarkAuthorityV1 {
                    adapter: "mer3ly.dataset/v1".into(),
                    record: serde_json::to_string(&authority)
                        .map_err(|error| format!("could not cite Matrix authority: {error}"))?,
                },
                reading: axis.reading.clone(),
                reading_parameters: axis.focus.as_ref().map(|_| {
                    serde_json::to_string(&parameters)
                        .expect("Matrix reading parameters are serializable")
                }),
                arrangement: None,
                expects_generation: axis.generation.clone(),
            },
        );
    }
    validate(&request.spatial.current)?;
    if let Some(previous) = &request.spatial.previous {
        validate(previous)?;
    }
    let spatial_projection = project_reading_request(ReadingRequest {
        reading: request.spatial.reading.clone(),
        current: request.spatial.current.clone(),
        previous: request.spatial.previous.clone(),
        focus: request.spatial.focus.clone(),
    })?;
    let (_, spatial_generation) = authority_identity(&request.spatial.current)?;
    let spatial_authority = MatrixAuthorityRecord {
        dataset: request.spatial.dataset.clone(),
        record: request.spatial.record.clone(),
    };
    let spatial_parameters = MatrixReadingParameters {
        focus: request.spatial.focus.clone(),
    };
    shelfmark.inputs.insert(
        "spatial".into(),
        ShelfmarkInputV1 {
            authority: ShelfmarkAuthorityV1 {
                adapter: "mer3ly.dataset/v1".into(),
                record: serde_json::to_string(&spatial_authority)
                    .map_err(|error| format!("could not cite spatial authority: {error}"))?,
            },
            reading: request.spatial.reading.clone(),
            reading_parameters: request.spatial.focus.as_ref().map(|_| {
                serde_json::to_string(&spatial_parameters)
                    .expect("spatial reading parameters are serializable")
            }),
            arrangement: Some(request.spatial.arrangement.clone()),
            expects_generation: spatial_generation.to_string(),
        },
    );
    let facet_sources = spatial_projection
        .nodes
        .iter()
        .map(|node| node.id.clone())
        .chain(
            artifact
                .rows
                .sources
                .iter()
                .chain(&artifact.columns.sources)
                .map(|source| source.source.id.clone()),
        )
        .collect::<HashSet<_>>();
    validate_view_delta(
        &spatial_projection,
        &facet_sources,
        &request.placement,
        request.motion.as_deref(),
        request.backdrop.as_ref(),
        &request.facets,
        request.camera.as_ref(),
    )?;
    shelfmark.delta.insert(
        "selection".into(),
        serde_json::to_string(&request.selection)
            .map_err(|error| format!("could not cite coordinated selection: {error}"))?,
    );
    shelfmark.delta.insert(
        "mer3ly.instances".into(),
        serde_json::to_string(&request.instances)
            .map_err(|error| format!("could not cite instance state: {error}"))?,
    );
    shelfmark.delta.insert(
        "placement".into(),
        serde_json::to_string(&request.placement)
            .map_err(|error| format!("could not cite placement state: {error}"))?,
    );
    shelfmark.delta.insert(
        "mer3ly.motion".into(),
        serde_json::to_string(request.motion.as_deref().unwrap_or("anchored"))
            .map_err(|error| format!("could not cite motion state: {error}"))?,
    );
    shelfmark.delta.insert(
        "mer3ly.backdrop".into(),
        serde_json::to_string(&request.backdrop.clone().unwrap_or(BackdropDelta {
            kind: "ambient".into(),
            collidable: false,
        }))
        .map_err(|error| format!("could not cite backdrop state: {error}"))?,
    );
    shelfmark.delta.insert(
        "mer3ly.facets".into(),
        serde_json::to_string(&request.facets)
            .map_err(|error| format!("could not cite facet state: {error}"))?,
    );
    shelfmark.delta.insert(
        "mer3ly.camera".into(),
        serde_json::to_string(&request.camera.clone().unwrap_or(CameraDelta {
            x: 0.0,
            y: 0.0,
            zoom: 1.0,
        }))
        .map_err(|error| format!("could not cite camera state: {error}"))?,
    );
    for (section, value) in &request.carried_delta {
        if shelfmark.delta.contains_key(section) {
            return Err(format!(
                "carried section {section} shadows a target-owned delta"
            ));
        }
        shelfmark.delta.insert(section.clone(), value.clone());
    }
    shelfmark
        .validate()
        .map_err(|error| format!("invalid composed shelfmark: {error:?}"))?;
    Ok(shelfmark)
}

fn resolve_matrix_shelfmark_json(input: &str) -> Result<String, String> {
    let request: MatrixShelfmarkResolutionRequest = serde_json::from_str(input)
        .map_err(|error| format!("invalid Matrix shelfmark resolution request: {error}"))?;
    let receipt = resolve_matrix_shelfmark_value(&request)?;
    serde_json::to_string(&receipt)
        .map_err(|error| format!("could not encode Matrix shelfmark receipt: {error}"))
}

fn resolve_matrix_shelfmark_value(
    request: &MatrixShelfmarkResolutionRequest,
) -> Result<MatrixShelfmarkReceipt, String> {
    request
        .shelfmark
        .validate()
        .map_err(|error| format!("invalid Matrix shelfmark: {error:?}"))?;
    if request.shelfmark.projection != "matrix" {
        return Err(format!(
            "shelfmark projection {} is not Matrix",
            request.shelfmark.projection
        ));
    }
    let rows = resolve_matrix_axis("rows", &request.shelfmark, &request.datasets)?;
    let columns = resolve_matrix_axis("columns", &request.shelfmark, &request.datasets)?;
    let spatial = resolve_matrix_axis("spatial", &request.shelfmark, &request.datasets)?;
    let spatial_input = request
        .shelfmark
        .inputs
        .get("spatial")
        .expect("spatial input was resolved above");
    if spatial_input
        .arrangement
        .as_deref()
        .filter(|arrangement| arrangement::spec(arrangement).is_some())
        .is_none()
    {
        return Err("spatial Shelfmark input has no offered arrangement".to_owned());
    }
    let spatial_projection = project_reading_request(ReadingRequest {
        reading: spatial.reading.clone(),
        current: spatial.current.clone(),
        previous: spatial.previous.clone(),
        focus: spatial.focus.clone(),
    })?;
    let (_, resolved_spatial_generation) = authority_identity(&spatial.current)?;
    let matrix_request = MatrixProjectionRequest { rows, columns };
    let artifact = matrix_projection(&matrix_request)?;
    let matrix = consume_matrix_projection(&artifact)?;
    let selection: CoordinatedSelection = serde_json::from_str(
        request
            .shelfmark
            .delta
            .get("selection")
            .ok_or_else(|| "Matrix shelfmark lacks coordinated selection".to_owned())?,
    )
    .map_err(|error| format!("invalid coordinated selection section: {error}"))?;
    let instances: Vec<MatrixInstanceDelta> = serde_json::from_str(
        request
            .shelfmark
            .delta
            .get("mer3ly.instances")
            .ok_or_else(|| "Matrix shelfmark lacks instance state".to_owned())?,
    )
    .map_err(|error| format!("invalid Matrix instance section: {error}"))?;
    let placement: Vec<HeldPlacement> = serde_json::from_str(
        request
            .shelfmark
            .delta
            .get("placement")
            .ok_or_else(|| "Matrix shelfmark lacks placement state".to_owned())?,
    )
    .map_err(|error| format!("invalid placement section: {error}"))?;
    let motion: String = serde_json::from_str(
        request
            .shelfmark
            .delta
            .get("mer3ly.motion")
            .ok_or_else(|| "Matrix shelfmark lacks motion state".to_owned())?,
    )
    .map_err(|error| format!("invalid motion section: {error}"))?;
    let backdrop: BackdropDelta = serde_json::from_str(
        request
            .shelfmark
            .delta
            .get("mer3ly.backdrop")
            .ok_or_else(|| "Matrix shelfmark lacks backdrop state".to_owned())?,
    )
    .map_err(|error| format!("invalid backdrop section: {error}"))?;
    let facets: Vec<ProjectedInstanceAddress> = serde_json::from_str(
        request
            .shelfmark
            .delta
            .get("mer3ly.facets")
            .ok_or_else(|| "Matrix shelfmark lacks facet state".to_owned())?,
    )
    .map_err(|error| format!("invalid facet section: {error}"))?;
    let camera: CameraDelta = serde_json::from_str(
        request
            .shelfmark
            .delta
            .get("mer3ly.camera")
            .ok_or_else(|| "Matrix shelfmark lacks camera state".to_owned())?,
    )
    .map_err(|error| format!("invalid camera section: {error}"))?;
    let source_ids = artifact
        .rows
        .sources
        .iter()
        .chain(&artifact.columns.sources)
        .map(|source| source.source.id.clone())
        .collect::<HashSet<_>>();
    for delta in &instances {
        if delta.instance.view.trim().is_empty()
            || delta.instance.facet.trim().is_empty()
            || !source_ids.contains(&delta.instance.source.id)
        {
            return Err("instance-scoped authored state does not resolve".to_owned());
        }
    }
    let mut facet_sources = source_ids.clone();
    facet_sources.extend(spatial_projection.nodes.iter().map(|node| node.id.clone()));
    validate_view_delta(
        &spatial_projection,
        &facet_sources,
        &placement,
        Some(&motion),
        Some(&backdrop),
        &facets,
        Some(&camera),
    )?;
    Ok(MatrixShelfmarkReceipt {
        matrix,
        input_generations: BTreeMap::from([
            ("columns".into(), artifact.columns.generation),
            ("rows".into(), artifact.rows.generation),
            ("spatial".into(), resolved_spatial_generation.to_string()),
        ]),
        selection_resolution: selection.resolution,
        honored_instance_deltas: instances.len(),
        honored_placements: placement.len(),
        honored_facets: facets.len(),
        camera,
    })
}

fn validate_view_delta(
    spatial: &GraphInput,
    facet_sources: &HashSet<String>,
    placement: &[HeldPlacement],
    motion: Option<&str>,
    backdrop: Option<&BackdropDelta>,
    facets: &[ProjectedInstanceAddress],
    camera: Option<&CameraDelta>,
) -> Result<(), String> {
    let sources = spatial
        .nodes
        .iter()
        .map(|node| node.id.as_str())
        .collect::<HashSet<_>>();
    if placement.len() > 64
        || placement.iter().any(|held| {
            held.source.adapter != PROJECTION_ADAPTER
                || !sources.contains(held.source.id.as_str())
                || !held.at.x.is_finite()
                || !held.at.y.is_finite()
                || !(-2000.0..=2000.0).contains(&held.at.x)
                || !(-2000.0..=2000.0).contains(&held.at.y)
        })
    {
        return Err("placement delta does not address the spatial projection".to_owned());
    }
    if !matches!(motion, None | Some("anchored" | "free")) {
        return Err("motion delta must be anchored or free".to_owned());
    }
    if let Some(backdrop) = backdrop
        && !matches!(
            backdrop.kind.as_str(),
            "clear" | "ambient" | "props" | "field"
        )
    {
        return Err("backdrop delta names an unsupported field".to_owned());
    }
    if facets.iter().any(|facet| {
        facet.view.trim().is_empty()
            || facet.facet.trim().is_empty()
            || facet.source.adapter != PROJECTION_ADAPTER
            || !facet_sources.contains(&facet.source.id)
    }) {
        return Err("facet delta does not address a cited projection".to_owned());
    }
    if let Some(camera) = camera
        && (!camera.x.is_finite()
            || !camera.y.is_finite()
            || !camera.zoom.is_finite()
            || !(-4000.0..=4000.0).contains(&camera.x)
            || !(-4000.0..=4000.0).contains(&camera.y)
            || !(0.25..=4.0).contains(&camera.zoom))
    {
        return Err("camera delta lies outside the portable view bounds".to_owned());
    }
    Ok(())
}

fn resolve_matrix_axis(
    role: &str,
    shelfmark: &ShelfmarkV1,
    datasets: &BTreeMap<String, ResolvedMatrixDataset>,
) -> Result<MatrixAxisRequest, String> {
    let input = shelfmark
        .inputs
        .get(role)
        .ok_or_else(|| format!("Matrix shelfmark lacks {role} input"))?;
    if input.authority.adapter != "mer3ly.dataset/v1" {
        return Err(format!(
            "Matrix {role} authority uses unsupported adapter {}",
            input.authority.adapter
        ));
    }
    let authority: MatrixAuthorityRecord = serde_json::from_str(&input.authority.record)
        .map_err(|error| format!("invalid Matrix {role} authority record: {error}"))?;
    let dataset = datasets
        .get(&authority.dataset)
        .ok_or_else(|| format!("Matrix {role} dataset {} is unavailable", authority.dataset))?;
    let (_, generation) = authority_identity(&dataset.current)?;
    if generation.to_string() != input.expects_generation {
        return Err(format!(
            "Matrix {role} authority moved: expected {}, found {}",
            input.expects_generation, generation
        ));
    }
    let parameters = input
        .reading_parameters
        .as_deref()
        .map(serde_json::from_str::<MatrixReadingParameters>)
        .transpose()
        .map_err(|error| format!("invalid Matrix {role} reading parameters: {error}"))?
        .unwrap_or_default();
    Ok(MatrixAxisRequest {
        dataset: authority.dataset,
        record: authority.record,
        reading: input.reading.clone(),
        current: dataset.current.clone(),
        previous: dataset.previous.clone(),
        focus: parameters.focus,
    })
}

fn decorate_current(mut current: GraphInput, previous: Option<&GraphInput>) -> GraphInput {
    let changes = diff_graphs(previous, &current)
        .nodes
        .into_iter()
        .map(|node| (node.id, node.change.unwrap_or_else(|| "stable".to_owned())))
        .collect::<HashMap<_, _>>();
    for node in &mut current.nodes {
        node.change = Some(node.change.clone().unwrap_or_else(|| {
            changes
                .get(&node.id)
                .cloned()
                .unwrap_or_else(|| "stable".to_owned())
        }));
        if node.summary.is_none() {
            node.summary = Some(format!(
                "{} is a {} in the selected public checkpoint.",
                node.name,
                humanize_identifier(&node.class)
            ));
        }
    }
    current
}

fn diff_graphs(previous: Option<&GraphInput>, current: &GraphInput) -> GraphInput {
    let before = previous
        .map(|graph| {
            graph
                .nodes
                .iter()
                .map(|node| (node.id.as_str(), node))
                .collect::<HashMap<_, _>>()
        })
        .unwrap_or_default();
    let after_ids = current
        .nodes
        .iter()
        .map(|node| node.id.as_str())
        .collect::<HashSet<_>>();
    let before_edges = incident_edge_signatures(previous.map_or(&[], |graph| &graph.edges));
    let after_edges = incident_edge_signatures(&current.edges);
    let mut nodes = current
        .nodes
        .iter()
        .cloned()
        .map(|mut node| {
            let change = match before.get(node.id.as_str()) {
                None => "added",
                Some(prior)
                    if node_signature(prior) != node_signature(&node)
                        || before_edges.get(&node.id) != after_edges.get(&node.id) =>
                {
                    "updated"
                }
                Some(_) => "stable",
            };
            node.change = Some(change.to_owned());
            if node.summary.is_none() {
                node.summary = Some(format!(
                    "{} is a {} in the selected public checkpoint.",
                    node.name,
                    humanize_identifier(&node.class)
                ));
            }
            node
        })
        .collect::<Vec<_>>();
    if let Some(previous) = previous {
        nodes.extend(
            previous
                .nodes
                .iter()
                .filter(|node| !after_ids.contains(node.id.as_str()))
                .cloned()
                .map(|mut node| {
                    node.change = Some("removed".to_owned());
                    if node.summary.is_none() {
                        node.summary = Some(format!(
                            "{} is absent from the selected public checkpoint.",
                            node.name
                        ));
                    }
                    node
                }),
        );
    }

    let node_ids = nodes
        .iter()
        .map(|node| node.id.as_str())
        .collect::<HashSet<_>>();
    let mut edges = current.edges.clone();
    if let Some(previous) = previous {
        let current_edge_ids = current
            .edges
            .iter()
            .map(|edge| edge.id.as_str())
            .collect::<HashSet<_>>();
        edges.extend(
            previous
                .edges
                .iter()
                .filter(|edge| {
                    !current_edge_ids.contains(edge.id.as_str())
                        && node_ids.contains(edge.source.as_str())
                        && node_ids.contains(edge.target.as_str())
                })
                .cloned()
                .map(|mut edge| {
                    edge.change = Some("removed".to_owned());
                    edge
                }),
        );
    }
    let focus = current
        .focus
        .as_deref()
        .filter(|focus| node_ids.contains(*focus))
        .map(str::to_owned)
        .or_else(|| nodes.first().map(|node| node.id.clone()));
    GraphInput {
        schema: current.schema.clone(),
        focus,
        nodes,
        edges,
    }
}

fn focus_and_neighbors(current: &GraphInput, focus: &str) -> GraphInput {
    let mut ids = HashSet::from([focus]);
    for edge in &current.edges {
        if edge.source == focus {
            ids.insert(edge.target.as_str());
        }
        if edge.target == focus {
            ids.insert(edge.source.as_str());
        }
    }
    GraphInput {
        schema: current.schema.clone(),
        focus: Some(focus.to_owned()),
        nodes: current
            .nodes
            .iter()
            .filter(|node| ids.contains(node.id.as_str()))
            .cloned()
            .collect(),
        edges: current
            .edges
            .iter()
            .filter(|edge| ids.contains(edge.source.as_str()) && ids.contains(edge.target.as_str()))
            .cloned()
            .collect(),
    }
}

fn node_signature(node: &GraphNodeInput) -> (&str, &str, &str, &str) {
    (&node.name, &node.class, &node.status, &node.pushed_at)
}

fn incident_edge_signatures(edges: &[GraphEdge]) -> HashMap<String, String> {
    let mut by_node = HashMap::<String, Vec<String>>::new();
    for edge in edges {
        let signature = format!(
            "{}:{}:{}:{}:{}",
            edge.id, edge.source, edge.target, edge.kind, edge.provenance
        );
        for id in [&edge.source, &edge.target] {
            by_node
                .entry(id.clone())
                .or_default()
                .push(signature.clone());
        }
    }
    by_node
        .into_iter()
        .map(|(id, mut signatures)| {
            signatures.sort();
            (id, signatures.join("|"))
        })
        .collect()
}

fn humanize_identifier(value: &str) -> String {
    value.replace(['_', '-'], " ")
}

impl GraphPhysics {
    fn apply_arrangement(&mut self, positions: &str, mobility: &str) -> Result<(), String> {
        if !matches!(mobility, "anchored" | "free") {
            return Err(format!("unsupported mobility {mobility}"));
        }
        let positions: Vec<GraphNodePosition> = serde_json::from_str(positions)
            .map_err(|error| format!("invalid arrangement positions: {error}"))?;
        if positions.len() != self.key_by_id.len() {
            return Err("arrangement did not position every graph node".to_owned());
        }

        let current = self.simulation.positions().collect::<HashMap<_, _>>();
        for key in self.id_by_key.keys().copied().collect::<Vec<_>>() {
            self.simulation.unpin(key);
        }
        let mut targets = Vec::with_capacity(positions.len());
        for position in positions {
            let key = self
                .key_by_id
                .get(&position.id)
                .copied()
                .ok_or_else(|| format!("arrangement contains unknown node {}", position.id))?;
            let target = if self.manually_pinned.contains(&position.id) {
                current
                    .get(&key)
                    .copied()
                    .unwrap_or_else(|| Point2D::new(position.x, position.y))
            } else {
                Point2D::new(position.x, position.y)
            };
            targets.push((key, target));
        }
        self.simulation.seed_positions(targets.iter().copied());
        self.simulation.set_anchor_force(match mobility {
            "anchored" => Some(
                AnchorSpring::new(
                    targets
                        .iter()
                        .map(|(key, point)| (*key, (point.x, point.y))),
                )
                .with_stiffness(13.0),
            ),
            _ => None,
        });
        for id in &self.manually_pinned {
            let Some(key) = self.key_by_id.get(id).copied() else {
                continue;
            };
            let point = current
                .get(&key)
                .copied()
                .or_else(|| {
                    targets
                        .iter()
                        .find(|(candidate, _)| *candidate == key)
                        .map(|(_, p)| *p)
                })
                .unwrap_or_else(Point2D::origin);
            self.simulation.pin(key, point);
        }
        self.mobility = mobility.to_owned();
        Ok(())
    }

    fn apply_backdrop(&mut self, backdrop: &str, tangible: bool) -> Result<(), String> {
        match backdrop {
            "clear" | "ambient" => {
                self.simulation.clear_scene();
                self.simulation.set_scene_field(None);
                self.simulation.set_gravity((0.0, 0.0));
                self.simulation.set_nodes_tangible(false);
            }
            "props" => {
                self.simulation.load_scene(&sandbox_props_scene());
                self.simulation.set_nodes_tangible(tangible);
            }
            "field" => {
                self.simulation.load_scene(&seiche::whirlpool_scene());
                self.simulation.set_scene_field(Some(SceneField::Vortex {
                    center: (0.0, 0.0),
                    strength: 54.0,
                    inward: 18.0,
                }));
                self.simulation.set_nodes_tangible(tangible);
            }
            _ => return Err(format!("unsupported backdrop {backdrop}")),
        }
        Ok(())
    }

    fn frame_json(&self) -> Result<String, String> {
        let mut nodes = self
            .simulation
            .positions()
            .filter_map(|(key, position)| {
                let id = self.id_by_key.get(&key)?.clone();
                Some(PhysicsNode {
                    pinned: self.is_pinned(&id),
                    id,
                    x: position.x,
                    y: position.y,
                })
            })
            .collect::<Vec<_>>();
        nodes.sort_by(|left, right| left.id.cmp(&right.id));
        let props = self
            .simulation
            .scene_bodies()
            .map(|body| physics_prop(body.position, body.rotation, body.collider))
            .collect();
        serde_json::to_string(&PhysicsFrame {
            schema: "mer3ly.graph-physics-frame/v1",
            nodes,
            props,
            at_rest: self.simulation.is_at_rest(0.35),
        })
        .map_err(|error| format!("could not serialize physics frame: {error}"))
    }
}

fn sandbox_props_scene() -> SceneSpec {
    let horizontal = NodeCollider::Hull {
        points: vec![
            (-370.0, -10.0),
            (370.0, -10.0),
            (370.0, 10.0),
            (-370.0, 10.0),
        ],
        fallback: 10.0,
    };
    let vertical = NodeCollider::Hull {
        points: vec![
            (-10.0, -280.0),
            (10.0, -280.0),
            (10.0, 280.0),
            (-10.0, 280.0),
        ],
        fallback: 10.0,
    };
    SceneSpec {
        bodies: vec![
            SceneBodySpec::fixed(horizontal.clone(), (0.0, -290.0)),
            SceneBodySpec::fixed(horizontal, (0.0, 290.0)),
            SceneBodySpec::fixed(vertical.clone(), (-380.0, 0.0)),
            SceneBodySpec::fixed(vertical, (380.0, 0.0)),
            SceneBodySpec::dynamic(NodeCollider::Square { half: 19.0 }, (-90.0, -120.0))
                .velocity((42.0, 28.0))
                .restitution(0.85)
                .gravity_scale(0.0),
            SceneBodySpec::dynamic(NodeCollider::Ball { radius: 17.0 }, (120.0, 90.0))
                .velocity((-34.0, -46.0))
                .restitution(0.9)
                .gravity_scale(0.0),
        ],
        gravity: (0.0, 0.0),
        default_tangible: true,
        perpetual: true,
        joints: Vec::new(),
    }
}

fn physics_prop(position: Point2D<f32>, rotation: f32, collider: NodeCollider) -> PhysicsProp {
    match collider {
        NodeCollider::Ball { radius } => PhysicsProp {
            x: position.x,
            y: position.y,
            rotation,
            shape: "ball",
            radius: Some(radius),
            half: None,
            points: Vec::new(),
        },
        NodeCollider::Square { half } | NodeCollider::RoundedSquare { half, .. } => PhysicsProp {
            x: position.x,
            y: position.y,
            rotation,
            shape: "square",
            radius: None,
            half: Some(half),
            points: Vec::new(),
        },
        NodeCollider::Hull { points, fallback } => PhysicsProp {
            x: position.x,
            y: position.y,
            rotation,
            shape: "hull",
            radius: Some(fallback),
            half: None,
            points,
        },
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PortableProjectionArtifact {
    pub schema: String,
    pub adapter: String,
    pub authority_schema: String,
    pub authority_sha256: String,
    pub score: Score,
    pub snapshot: SceneSnapshot,
    pub nodes: Vec<ProjectionNodeMetadata>,
    pub relations: Vec<ProjectionRelationMetadata>,
    pub default_trace: Vec<ProjectionStep>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ProjectionNodeMetadata {
    pub id: String,
    pub name: String,
    pub class: String,
    pub status: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ProjectionRelationMetadata {
    pub index: RelationId,
    pub id: String,
    pub source: String,
    pub target: String,
    pub kind: String,
    pub provenance: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ProjectionSelection {
    pub kind: String,
    pub id: String,
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
    /// free, so an unlabelled share still records its pins as ensure-class.
    #[serde(default)]
    pub motion: Option<String>,
    #[serde(default)]
    pub pins: Vec<PlacementPin>,
}

impl PlacementDelta {
    /// Translate the live path's placement into score holds.
    ///
    /// A manual pin is hard in the sandbox: it survives until it is removed,
    /// so it becomes [`Hold::Pinned`]. Under `anchored` the arrangement is a
    /// suggestion for everything, so a pin placed in that mode is recorded as
    /// [`Hold::Anchored`]: best effort by the visitor's own choice.
    ///
    /// The live path carries two motion classes, `anchored` and `free`; the
    /// former `frozen` class became a non-interactive renderer's concern
    /// rather than this simulation's. Score holds keep both classes anyway,
    /// because a frozen realization still needs to say which placements were
    /// ensure-class when it records one.
    ///
    /// The class is *recorded* here rather than left to be re-inferred from a
    /// spring stiffness, which is the whole point of the seam.
    fn holds(&self) -> Vec<HeldPlacement> {
        let hold = match self.motion.as_deref() {
            Some("anchored") => Hold::Anchored,
            _ => Hold::Pinned,
        };
        self.pins
            .iter()
            .map(|pin| HeldPlacement {
                source: SourceRef::new(PROJECTION_ADAPTER, &pin.id),
                at: Vec2::new(pin.x, pin.y),
                hold,
            })
            .collect()
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ProjectionStep {
    pub label: String,
    pub selection: Option<ProjectionSelection>,
    pub diff: Option<SceneDiff>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ProjectionReceipt {
    pub schema: String,
    pub authority_sha256: String,
    pub score_items: usize,
    pub initial_revision: u64,
    pub final_revision: u64,
    pub active_items: usize,
    pub active_relations: usize,
    pub picked_source: String,
    pub trace_steps: usize,
    /// How many authored holds the realized scene actually honored.
    ///
    /// Equal to the score's hold count on a sound artifact. Consuming checks
    /// it rather than trusting it, because a citation that says "pinned here"
    /// and reconstitutes elsewhere is the exact failure the seam exists to
    /// close.
    pub honored_holds: usize,
}

pub fn portable_projection_json(input: &str) -> Result<String, String> {
    let artifact = portable_projection(input)?;
    serde_json::to_string(&artifact)
        .map_err(|error| format!("could not serialize portable projection: {error}"))
}

/// The seam: a live arrangement's placement reaching a portable score.
///
/// `placement` is the sandbox's own scene state (or just its placement half).
/// The pins it carries become [`Score::holds`], the solver honors them ahead of
/// the arrangement, and [`consume_portable_projection`] proves afterwards that
/// each one landed where the visitor put it. Before this, a pinned arrangement
/// could only travel as site-local JSON that no score could express.
pub fn portable_projection_with_placement_json(
    input: &str,
    placement: &str,
) -> Result<String, String> {
    let delta: PlacementDelta = serde_json::from_str(placement)
        .map_err(|error| format!("invalid placement delta: {error}"))?;
    let artifact = portable_projection_holding(input, &delta)?;
    serde_json::to_string(&artifact)
        .map_err(|error| format!("could not serialize portable projection: {error}"))
}

pub fn portable_projection(input: &str) -> Result<PortableProjectionArtifact, String> {
    portable_projection_holding(input, &PlacementDelta::default())
}

/// The authority's content identity: its SHA-256 and the score generation
/// derived from that digest's first eight bytes.
///
/// One recipe, shared by the portable path and the citation check, because a
/// citation is only checkable if the generation it carries was computed the
/// same way the resolver recomputes it.
fn authority_identity(input: &GraphInput) -> Result<(String, u64), String> {
    let digest = Sha256::digest(
        serde_json::to_vec(input)
            .map_err(|error| format!("could not canonicalize graph authority: {error}"))?,
    );
    let generation = u64::from_be_bytes(
        digest[..8]
            .try_into()
            .expect("SHA-256 prefix is eight bytes"),
    );
    Ok((format!("{digest:x}"), generation))
}

pub fn portable_projection_holding(
    input: &str,
    placement: &PlacementDelta,
) -> Result<PortableProjectionArtifact, String> {
    let input: GraphInput =
        serde_json::from_str(input).map_err(|error| format!("invalid graph JSON: {error}"))?;
    validate(&input)?;

    let (authority_sha256, generation) = authority_identity(&input)?;

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
        let index = RelationId(scene.relations.len() as u32);
        scene.relations.push(RoutedRelation {
            from,
            to,
            space: sceno::Scene::WORLD,
            points: vec![from_point, to_point],
            kind: Some(edge.kind.clone()),
            weight: Some(1.0),
        });
        relations.push(ProjectionRelationMetadata {
            index,
            id: edge.id.clone(),
            source: edge.source.clone(),
            target: edge.target.clone(),
            kind: edge.kind.clone(),
            provenance: edge.provenance.clone(),
        });
    }

    let snapshot = SceneSnapshot::from_dense(SceneEpoch(generation), Revision(1), scene)
        .map_err(|error| format!("Scenograph rejected the solved scene: {error:?}"))?;
    let default_trace = default_projection_trace(&input, &relations, &snapshot)?;
    let artifact = PortableProjectionArtifact {
        schema: PORTABLE_PROJECTION_SCHEMA.to_owned(),
        adapter: PROJECTION_ADAPTER.to_owned(),
        authority_schema: input.schema.clone(),
        authority_sha256,
        score,
        snapshot,
        nodes: input
            .nodes
            .iter()
            .map(|node| ProjectionNodeMetadata {
                id: node.id.clone(),
                name: node.name.clone(),
                class: node.class.clone(),
                status: node.status.clone(),
            })
            .collect(),
        relations,
        default_trace,
    };
    consume_portable_projection(&artifact)?;
    Ok(artifact)
}

pub fn consume_portable_projection_json(input: &str) -> Result<ProjectionReceipt, String> {
    let artifact: PortableProjectionArtifact = serde_json::from_str(input)
        .map_err(|error| format!("invalid portable projection JSON: {error}"))?;
    consume_portable_projection(&artifact)
}

pub fn consume_portable_projection(
    artifact: &PortableProjectionArtifact,
) -> Result<ProjectionReceipt, String> {
    if artifact.schema != PORTABLE_PROJECTION_SCHEMA {
        return Err(format!(
            "unsupported portable projection schema {}",
            artifact.schema
        ));
    }
    if artifact.adapter != PROJECTION_ADAPTER {
        return Err(format!(
            "unsupported projection adapter {}",
            artifact.adapter
        ));
    }
    artifact
        .snapshot
        .validate()
        .map_err(|error| format!("invalid initial scene snapshot: {error:?}"))?;
    if artifact.score.items.len() != artifact.nodes.len()
        || artifact.snapshot.active_item_count() != artifact.nodes.len()
    {
        return Err("score, scene, and node metadata counts diverge".to_owned());
    }
    if artifact.snapshot.tables.relations.len() != artifact.relations.len() {
        return Err("scene and relation metadata counts diverge".to_owned());
    }

    // Holds are checked against the scene as solved, not the scene after the
    // trace: the trace deliberately moves things, and an authored move later is
    // not a broken pin. What must hold is that the solver placed each held
    // source where the citation said.
    let mut honored_holds = 0usize;
    for held in &artifact.score.holds {
        let instance = instance_for_source(&artifact.snapshot, &held.source.id)
            .ok_or_else(|| format!("held source {} is absent from the scene", held.source.id))?;
        let item = artifact
            .snapshot
            .active_item(instance)
            .ok_or_else(|| format!("held source {} is tombstoned", held.source.id))?;
        let at = item.transform.translate;
        if at.x != held.at.x || at.y != held.at.y {
            return Err(format!(
                "hold on {} was not honored: asked ({}, {}), realized ({}, {})",
                held.source.id, held.at.x, held.at.y, at.x, at.y
            ));
        }
        honored_holds += 1;
    }

    let initial_revision = artifact.snapshot.revision.0;
    let mut snapshot = artifact.snapshot.clone();
    for step in &artifact.default_trace {
        if let Some(diff) = &step.diff {
            snapshot
                .apply_diff(diff)
                .map_err(|error| format!("portable trace step {} failed: {error:?}", step.label))?;
        }
    }
    snapshot
        .validate()
        .map_err(|error| format!("invalid final scene snapshot: {error:?}"))?;

    let mere = instance_for_source(&snapshot, PREFERRED_FOCUS_REPOSITORY)
        .ok_or_else(|| "portable scene lost Mere".to_owned())?;
    let mere_item = snapshot
        .active_item(mere)
        .ok_or_else(|| "portable scene tombstoned Mere".to_owned())?;
    let picked = snapshot
        .pick(mere_item.transform.translate)
        .ok_or_else(|| "native Scenotime consumer could not pick Mere".to_owned())?;
    let picked_item = snapshot
        .active_item(picked)
        .ok_or_else(|| "native Scenotime consumer picked a tombstone".to_owned())?;
    let picked_source = snapshot.tables.sources[picked_item.source.0 as usize]
        .as_ref()
        .ok_or_else(|| "picked item has no source".to_owned())?
        .id
        .clone();
    if picked_source != PREFERRED_FOCUS_REPOSITORY {
        return Err(format!(
            "native Scenotime consumer picked {picked_source}, not Mere"
        ));
    }

    Ok(ProjectionReceipt {
        schema: "mer3ly.portable-projection-receipt/v1".to_owned(),
        authority_sha256: artifact.authority_sha256.clone(),
        score_items: artifact.score.items.len(),
        initial_revision,
        final_revision: snapshot.revision.0,
        active_items: snapshot.active_item_count(),
        active_relations: snapshot.tables.relations.iter().flatten().count(),
        picked_source,
        trace_steps: artifact.default_trace.len(),
        honored_holds,
    })
}

fn default_projection_trace(
    input: &GraphInput,
    relations: &[ProjectionRelationMetadata],
    snapshot: &SceneSnapshot,
) -> Result<Vec<ProjectionStep>, String> {
    let mut trace = Vec::new();
    let mut current = snapshot.clone();

    if let Some(turnstone) = instance_for_source(&current, "turnstone") {
        trace.push(selection_step("Select Turnstone", "node", "turnstone"));
        let diff = move_diff(&current, turnstone, Vec2::new(48.0, 24.0))?;
        current
            .apply_diff(&diff)
            .map_err(|error| format!("default move diff failed: {error:?}"))?;
        trace.push(diff_step("Move Turnstone", diff));
    }

    if let Some(relation) = relations
        .iter()
        .find(|relation| relation.id == "turnstone-hosts-mere")
    {
        trace.push(selection_step(
            "Select the Turnstone host relationship",
            "edge",
            &relation.id,
        ));
        let diff = next_diff(
            &current,
            vec![SceneOp::TombstoneRelation {
                index: relation.index,
            }],
        );
        current
            .apply_diff(&diff)
            .map_err(|error| format!("default relation diff failed: {error:?}"))?;
        trace.push(diff_step("Remove the relationship from the scene", diff));
    }

    trace.push(selection_step(
        "Select Mere",
        "node",
        PREFERRED_FOCUS_REPOSITORY,
    ));
    let dependencies = input
        .edges
        .iter()
        .filter(|edge| edge.source == PREFERRED_FOCUS_REPOSITORY)
        .filter_map(|edge| instance_for_source(&current, &edge.target))
        .collect::<Vec<_>>();
    if !dependencies.is_empty() {
        let fold = visibility_diff(&current, PREFERRED_FOCUS_REPOSITORY, &dependencies, false)?;
        current
            .apply_diff(&fold)
            .map_err(|error| format!("default fold diff failed: {error:?}"))?;
        trace.push(diff_step("Fold Mere dependencies", fold));
        let expand = visibility_diff(&current, PREFERRED_FOCUS_REPOSITORY, &dependencies, true)?;
        current
            .apply_diff(&expand)
            .map_err(|error| format!("default expand diff failed: {error:?}"))?;
        trace.push(diff_step("Expand Mere dependencies", expand));
    }

    Ok(trace)
}

fn selection_step(label: &str, kind: &str, id: &str) -> ProjectionStep {
    ProjectionStep {
        label: label.to_owned(),
        selection: Some(ProjectionSelection {
            kind: kind.to_owned(),
            id: id.to_owned(),
        }),
        diff: None,
    }
}

fn diff_step(label: &str, diff: SceneDiff) -> ProjectionStep {
    ProjectionStep {
        label: label.to_owned(),
        selection: None,
        diff: Some(diff),
    }
}

fn next_diff(snapshot: &SceneSnapshot, operations: Vec<SceneOp>) -> SceneDiff {
    SceneDiff {
        epoch: snapshot.epoch,
        base: snapshot.revision,
        revision: Revision(snapshot.revision.0 + 1),
        operations,
    }
}

fn move_diff(
    snapshot: &SceneSnapshot,
    instance: sceno::InstanceId,
    delta: Vec2,
) -> Result<SceneDiff, String> {
    let mut moved = snapshot
        .active_item(instance)
        .ok_or_else(|| format!("cannot move absent instance {}", instance.0))?
        .clone();
    moved.transform.translate.x += delta.x;
    moved.transform.translate.y += delta.y;
    let mut operations = vec![SceneOp::UpdateItem {
        index: instance,
        value: moved.clone(),
    }];
    for (index, relation) in snapshot.tables.relations.iter().enumerate() {
        let Some(relation) = relation else { continue };
        if relation.from != instance && relation.to != instance {
            continue;
        }
        let mut updated = relation.clone();
        let endpoint = |id: sceno::InstanceId| {
            if id == instance {
                moved.transform.translate
            } else {
                snapshot
                    .active_item(id)
                    .expect("validated relation endpoint is active")
                    .transform
                    .translate
            }
        };
        updated.points = vec![endpoint(updated.from), endpoint(updated.to)];
        operations.push(SceneOp::UpdateRelation {
            index: RelationId(index as u32),
            value: updated,
        });
    }
    Ok(next_diff(snapshot, operations))
}

fn visibility_diff(
    snapshot: &SceneSnapshot,
    root: &str,
    dependencies: &[sceno::InstanceId],
    visible: bool,
) -> Result<SceneDiff, String> {
    let root_id = instance_for_source(snapshot, root)
        .ok_or_else(|| format!("cannot fold absent source {root}"))?;
    let mut root_item = snapshot
        .active_item(root_id)
        .ok_or_else(|| format!("cannot fold absent instance {}", root_id.0))?
        .clone();
    root_item.channels.retain(|(name, _)| name != "fold");
    if !visible {
        root_item.channels.push(("fold".to_owned(), 1.0));
    }
    let mut operations = vec![SceneOp::UpdateItem {
        index: root_id,
        value: root_item,
    }];
    for dependency in dependencies {
        let mut item = snapshot
            .active_item(*dependency)
            .ok_or_else(|| format!("cannot update absent dependency {}", dependency.0))?
            .clone();
        item.visible = visible;
        operations.push(SceneOp::UpdateItem {
            index: *dependency,
            value: item,
        });
    }
    Ok(next_diff(snapshot, operations))
}

fn instance_for_source(snapshot: &SceneSnapshot, source_id: &str) -> Option<sceno::InstanceId> {
    snapshot
        .tables
        .items
        .iter()
        .enumerate()
        .find_map(|(index, item)| {
            let item = item.as_ref()?;
            let source = snapshot
                .tables
                .sources
                .get(item.source.0 as usize)?
                .as_ref()?;
            (source.id == source_id).then_some(sceno::InstanceId(index as u32))
        })
}

#[wasm_bindgen]
pub fn layout_graph(input: &str) -> Result<String, JsValue> {
    layout_graph_json(input).map_err(|error| JsValue::from_str(&error))
}

fn layout_graph_json(input: &str) -> Result<String, String> {
    let input: GraphInput =
        serde_json::from_str(input).map_err(|error| format!("invalid graph JSON: {error}"))?;
    validate(&input)?;
    let focus = focal_node(&input);

    let mut arrangements = Vec::with_capacity(ARRANGEMENT_ORDER.len());
    for arrangement_id in ARRANGEMENT_ORDER {
        let spec = arrangement::spec(arrangement_id)
            .ok_or_else(|| format!("arrangement catalog is missing {arrangement_id}"))?;
        let positions = arrangement_positions(&input, arrangement_id, focus)?;
        arrangements.push(GraphArrangement {
            id: spec.id.to_owned(),
            name: spec.display_name.to_owned(),
            description: spec.description.to_owned(),
            engine: spec.id.to_owned(),
            nodes: positions,
        });
    }
    let unavailable_arrangements = UNAVAILABLE_ARRANGEMENTS
        .iter()
        .map(|(arrangement_id, reason)| {
            let spec = arrangement::spec(arrangement_id)
                .ok_or_else(|| format!("arrangement catalog is missing {arrangement_id}"))?;
            Ok(UnavailableArrangement {
                id: spec.id.to_owned(),
                name: spec.display_name.to_owned(),
                reason: (*reason).to_owned(),
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let default_arrangement = arrangements
        .iter()
        .find(|arrangement| arrangement.id == DEFAULT_ARRANGEMENT)
        .ok_or_else(|| "default repository arrangement is unavailable".to_owned())?;
    let default_positions = default_arrangement
        .nodes
        .iter()
        .map(|position| (position.id.as_str(), position))
        .collect::<HashMap<_, _>>();
    let nodes = input
        .nodes
        .iter()
        .map(|node| {
            let position = default_positions
                .get(node.id.as_str())
                .ok_or_else(|| format!("default arrangement lost node {}", node.id))?;
            Ok(GraphNodeLayout {
                id: node.id.clone(),
                name: node.name.clone(),
                class: node.class.clone(),
                status: node.status.clone(),
                pushed_at: node.pushed_at.clone(),
                x: position.x,
                y: position.y,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    serde_json::to_string(&GraphLayout {
        schema: "mer3ly.repo-graph-layout/v2",
        authority_schema: input.schema.clone(),
        engine: default_arrangement.engine.clone(),
        focus: focus.to_owned(),
        default_arrangement: DEFAULT_ARRANGEMENT,
        nodes,
        edges: input.edges.clone(),
        arrangements,
        unavailable_arrangements,
    })
    .map_err(|error| format!("could not serialize graph layout: {error}"))
}

/// Place every node under `arrangement_id`, normalized into the site's frame.
///
/// Each arrangement is a `sceno` score: the arrangement and its config, plus
/// whatever this site had to walk the graph to disclose. `scenomise` places it.
/// Nothing here computes a position.
fn arrangement_positions(
    input: &GraphInput,
    arrangement_id: &str,
    focus: &str,
) -> Result<Vec<GraphNodePosition>, String> {
    let node_ids: HashSet<&str> = input.nodes.iter().map(|node| node.id.as_str()).collect();
    let edges: Vec<(String, String)> = input
        .edges
        .iter()
        .map(|edge| (edge.source.clone(), edge.target.clone()))
        .collect();

    // What the arrangement reads, and what this site had to walk the graph to
    // know. An id absent from a map disclosed nothing, which every arrangement
    // treats differently from disclosing a zero.
    let mut axis: HashMap<String, AxisValue> = HashMap::new();
    let mut weight: HashMap<String, f32> = HashMap::new();
    let mut unreachable: Vec<&str> = Vec::new();

    let arrangement = match arrangement_id {
        "graph_layout:radial" => {
            let rings = radial_rings(&node_ids, &edges, focus);
            // Absent from the ring map means the walk never got there. The old
            // path inferred this from a missing delta; reading it from the
            // rings says the same thing without depending on what the solver
            // chose to emit.
            unreachable = input
                .nodes
                .iter()
                .filter(|node| node.id != focus && !rings.contains_key(&node.id))
                .map(|node| node.id.as_str())
                .collect();
            for (id, ring) in rings {
                axis.insert(id, AxisValue::Numeric(ring as f64));
            }
            weight = degree_weights(&node_ids, &edges);
            SceneArrangement::Radial(sceno::Radial {
                center: Vec2::ZERO,
                ring_spacing: 190.0,
                angular_policy: sceno::RadialAngularPolicy::Weighted,
                rotation_offset: 0.0,
                unreachable_policy: sceno::RadialUnreachablePolicy::LeaveInPlace,
            })
        }
        "graph_layout:stack" => {
            for (id, layer) in stack_layers(&node_ids, &edges) {
                axis.insert(id, AxisValue::Numeric(layer as f64));
            }
            SceneArrangement::Stack(sceno::Stack::default())
        }
        "graph_layout:timeline" => {
            for node in &input.nodes {
                axis.insert(
                    node.id.clone(),
                    AxisValue::Numeric(timestamp_coordinate(&node.pushed_at)?),
                );
            }
            SceneArrangement::Timeline(sceno::Timeline {
                row_gap: 120.0,
                ..sceno::Timeline::default()
            })
        }
        "graph_layout:kanban" => {
            for node in &input.nodes {
                axis.insert(node.id.clone(), AxisValue::Categorical(node.status.clone()));
            }
            SceneArrangement::Kanban(sceno::Kanban::default())
        }
        "graph_layout:grid" => SceneArrangement::Grid(sceno::Grid {
            cell: Vec2::ZERO,
            columns: (input.nodes.len() as f32).sqrt().ceil().max(1.0) as u32,
            gap: 120.0,
            ..sceno::Grid::default()
        }),
        "graph_layout:phyllotaxis" => SceneArrangement::Spiral(Spiral::default()),
        "graph_layout:penrose" => SceneArrangement::Penrose(sceno::Penrose::default()),
        "graph_layout:lsystem" => SceneArrangement::LSystem(sceno::LSystem::default()),
        other => return Err(format!("no arrangement is wired for {other}")),
    };

    let mut score = Score::new(arrangement);
    for (ordinal, node) in input.nodes.iter().enumerate() {
        score.items.push(ScoreItem {
            source: SourceRef::new(PROJECTION_ADAPTER, node.id.clone()),
            ordinal: ordinal as u32,
            footprint: Footprint::Circle { radius: 24.0 },
            representation: Representation::Glyph,
            placement: Placement::Ordinal,
            layer: 0,
            visible: true,
            axis: axis.get(&node.id).cloned(),
            embedding: None,
            weight: weight.get(&node.id).copied(),
        });
    }

    let scene = scenomise::solve(&score);
    if scene.items.len() != input.nodes.len() {
        return Err(format!(
            "arrangement {arrangement_id} placed {} of {} repositories",
            scene.items.len(),
            input.nodes.len()
        ));
    }
    let mut placed: HashMap<&str, Point2D<f32>> = input
        .nodes
        .iter()
        .zip(&scene.items)
        .map(|(node, item)| {
            (
                node.id.as_str(),
                Point2D::new(item.transform.translate.x, item.transform.translate.y),
            )
        })
        .collect();

    // The site's own lane for repositories outside the focus neighborhood: a
    // column down the left, rather than wherever "leave in place" resolved to.
    let lane_center = unreachable.len().saturating_sub(1) as f32 * 0.5;
    for (index, id) in unreachable.into_iter().enumerate() {
        placed.insert(
            id,
            Point2D::new(-470.0, (index as f32 - lane_center) * 190.0),
        );
    }

    let raw_positions: Vec<(String, Point2D<f32>)> = input
        .nodes
        .iter()
        .map(|node| {
            (
                node.id.clone(),
                placed
                    .get(node.id.as_str())
                    .copied()
                    .unwrap_or_else(Point2D::origin),
            )
        })
        .collect();
    let raw_positions = if arrangement_id == "graph_layout:timeline" {
        place_timeline_strips(raw_positions)
    } else {
        raw_positions
    };
    normalize_positions(arrangement_id, raw_positions)
}

fn focal_node(input: &GraphInput) -> &str {
    if let Some(focus) = input.focus.as_deref() {
        return focus;
    }
    input
        .nodes
        .iter()
        .find(|node| node.id == PREFERRED_FOCUS_REPOSITORY)
        .map(|node| node.id.as_str())
        .unwrap_or_else(|| input.nodes[0].id.as_str())
}

fn place_timeline_strips(
    mut positions: Vec<(String, Point2D<f32>)>,
) -> Vec<(String, Point2D<f32>)> {
    positions.sort_by(|(left_id, left), (right_id, right)| {
        left.x
            .total_cmp(&right.x)
            .then_with(|| left.y.total_cmp(&right.y))
            .then_with(|| left_id.cmp(right_id))
    });
    let min_x = positions.first().map_or(0.0, |(_, position)| position.x);
    let max_x = positions.last().map_or(min_x, |(_, position)| position.x);
    let span = (max_x - min_x).max(f32::EPSILON);
    let mut last_x_by_lane = [f32::NEG_INFINITY; TIMELINE_LANE_COUNT];

    positions
        .into_iter()
        .map(|(id, position)| {
            let x = ((position.x - min_x) / span - 0.5) * TIMELINE_AXIS_LENGTH;
            let lane = last_x_by_lane
                .iter()
                .position(|last_x| x - *last_x >= TIMELINE_MIN_X_GAP)
                .unwrap_or_else(|| {
                    last_x_by_lane
                        .iter()
                        .enumerate()
                        .min_by(|(_, left), (_, right)| left.total_cmp(right))
                        .map_or(0, |(index, _)| index)
                });
            last_x_by_lane[lane] = x;
            (id, Point2D::new(x, timeline_lane_offset(lane)))
        })
        .collect()
}

fn timeline_lane_offset(lane: usize) -> f32 {
    if lane == 0 {
        return 0.0;
    }
    let distance = lane.div_ceil(2) as f32 * TIMELINE_LANE_GAP;
    if lane % 2 == 1 { -distance } else { distance }
}

fn normalize_positions(
    arrangement_id: &str,
    positions: Vec<(String, Point2D<f32>)>,
) -> Result<Vec<GraphNodePosition>, String> {
    if positions.len() == 1 {
        let (id, _) = positions
            .into_iter()
            .next()
            .expect("one-node graph has one position");
        return Ok(vec![GraphNodePosition { id, x: 0.0, y: 0.0 }]);
    }
    let mut min_x = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_y = f32::NEG_INFINITY;
    for (_, position) in &positions {
        if !position.x.is_finite() || !position.y.is_finite() {
            return Err(format!(
                "arrangement {arrangement_id} emitted a non-finite position"
            ));
        }
        min_x = min_x.min(position.x);
        max_x = max_x.max(position.x);
        min_y = min_y.min(position.y);
        max_y = max_y.max(position.y);
    }
    let width = max_x - min_x;
    let height = max_y - min_y;
    if width <= f32::EPSILON && height <= f32::EPSILON {
        return Err(format!(
            "arrangement {arrangement_id} collapsed every repository"
        ));
    }
    let height_limit = if arrangement_id == "graph_layout:timeline" {
        720.0
    } else {
        520.0
    };
    let scale = (620.0 / width.max(1.0)).min(height_limit / height.max(1.0));
    let center_x = (min_x + max_x) * 0.5;
    let center_y = (min_y + max_y) * 0.5;
    Ok(positions
        .into_iter()
        .map(|(id, position)| GraphNodePosition {
            id,
            x: (position.x - center_x) * scale,
            y: (position.y - center_y) * scale,
        })
        .collect())
}

fn timestamp_coordinate(value: &str) -> Result<f64, String> {
    let digits = value
        .chars()
        .filter(char::is_ascii_digit)
        .collect::<String>();
    if digits.len() < 14 {
        return Err(format!(
            "repository push timestamp is not sortable: {value}"
        ));
    }
    let component = |range: std::ops::Range<usize>| -> Result<i64, String> {
        digits[range]
            .parse::<i64>()
            .map_err(|error| format!("repository push timestamp is not sortable: {error}"))
    };
    let year = component(0..4)?;
    let month = component(4..6)?;
    let day = component(6..8)?;
    let hour = component(8..10)?;
    let minute = component(10..12)?;
    let second = component(12..14)?;
    if !(1..=12).contains(&month)
        || !(1..=31).contains(&day)
        || !(0..=23).contains(&hour)
        || !(0..=59).contains(&minute)
        || !(0..=60).contains(&second)
    {
        return Err(format!(
            "repository push timestamp is not sortable: {value}"
        ));
    }

    let adjusted_year = year - i64::from(month <= 2);
    let era = if adjusted_year >= 0 {
        adjusted_year
    } else {
        adjusted_year - 399
    } / 400;
    let year_of_era = adjusted_year - era * 400;
    let adjusted_month = month + if month > 2 { -3 } else { 9 };
    let day_of_year = (153 * adjusted_month + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    let days_since_epoch = era * 146_097 + day_of_era - 719_468;
    Ok((days_since_epoch * 86_400 + hour * 3_600 + minute * 60 + second) as f64)
}

// `arrangement_description` lived here to override the layout registry's own
// descriptions for a repository audience, falling back to whatever the registry
// carried. The scenograph absorption retired that registry, and the overrides
// moved into `arrangement::CATALOG` where the names live beside them, so there
// is no second source to reconcile against. Removed rather than left dead: its
// remaining callers were the ones the catalog replaced.

fn validate(input: &GraphInput) -> Result<(), String> {
    if input.schema != "mer3ly.repo-graph/v1" {
        return Err(format!("unsupported graph schema {}", input.schema));
    }
    if input.nodes.is_empty() {
        return Err("repository graph has no nodes".to_owned());
    }

    let mut node_ids = HashSet::with_capacity(input.nodes.len());
    for node in &input.nodes {
        if node.id.is_empty()
            || node.name.is_empty()
            || node.class.is_empty()
            || node.status.is_empty()
            || node.pushed_at.is_empty()
        {
            return Err("repository graph contains an incomplete node".to_owned());
        }
        if !node_ids.insert(node.id.as_str()) {
            return Err(format!("duplicate repository graph node {}", node.id));
        }
    }
    if input
        .focus
        .as_deref()
        .is_some_and(|focus| !node_ids.contains(focus))
    {
        return Err(format!(
            "repository graph focus {} is not a graph node",
            input.focus.as_deref().unwrap_or_default()
        ));
    }
    let mut edge_ids = HashSet::with_capacity(input.edges.len());
    for edge in &input.edges {
        if !edge_ids.insert(edge.id.as_str()) {
            return Err(format!("duplicate repository graph edge {}", edge.id));
        }
        if !node_ids.contains(edge.source.as_str()) || !node_ids.contains(edge.target.as_str()) {
            return Err(format!(
                "repository graph edge {} has an unknown endpoint",
                edge.id
            ));
        }
    }

    let mut positions = HashMap::with_capacity(input.nodes.len());
    for (index, node) in input.nodes.iter().enumerate() {
        positions.insert(node.id.as_str(), index);
    }
    if positions.len() != input.nodes.len() {
        return Err("repository graph node ordering is not stable".to_owned());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{
      "schema": "mer3ly.repo-graph/v1",
      "nodes": [
        {"id":"mere","name":"Mere","class":"platform","status":"active","pushed_at":"2026-07-30T05:44:23Z"},
        {"id":"genet","name":"Genet","class":"platform","status":"active","pushed_at":"2026-07-30T05:44:24Z"},
        {"id":"turnstone","name":"Turnstone","class":"product","status":"prototype","pushed_at":"2026-07-31T05:07:42Z"}
      ],
      "edges": [
        {"id":"mere-depends-on-genet","source":"mere","target":"genet","kind":"depends_on","provenance":"derived"},
        {"id":"turnstone-hosts-mere","source":"turnstone","target":"mere","kind":"host_for","provenance":"curated"}
      ]
    }"#;

    #[test]
    fn arrangement_catalog_preserves_graph_identity() {
        let encoded = layout_graph_json(SAMPLE).expect("layout graph");
        let value: serde_json::Value = serde_json::from_str(&encoded).expect("parse layout");
        assert_eq!(value["schema"], "mer3ly.repo-graph-layout/v2");
        assert_eq!(value["engine"], "graph_layout:radial");
        assert_eq!(value["focus"], "mere");
        assert_eq!(value["default_arrangement"], "graph_layout:radial");
        assert_eq!(value["nodes"].as_array().expect("nodes").len(), 3);
        assert_eq!(value["edges"].as_array().expect("edges").len(), 2);
        assert_eq!(
            value["arrangements"]
                .as_array()
                .expect("arrangements")
                .len(),
            8
        );
        assert_eq!(
            value["unavailable_arrangements"]
                .as_array()
                .expect("unavailable arrangements")
                .len(),
            1
        );
        assert_eq!(value["nodes"][0]["id"], "mere");
        assert_eq!(value["nodes"][0]["pushed_at"], "2026-07-30T05:44:23Z");
        assert_eq!(value["edges"][1]["id"], "turnstone-hosts-mere");
        for arrangement in value["arrangements"].as_array().expect("arrangements") {
            assert_eq!(
                arrangement["nodes"].as_array().expect("scene nodes").len(),
                3
            );
            assert_eq!(arrangement["nodes"][0]["id"], "mere");
        }
    }

    #[test]
    fn arrangement_catalog_is_deterministic() {
        let first = layout_graph_json(SAMPLE).expect("first layout");
        let second = layout_graph_json(SAMPLE).expect("second layout");
        assert_eq!(first, second);
    }

    #[test]
    fn unknown_edge_endpoint_is_rejected() {
        let invalid = SAMPLE.replace("\"target\":\"genet\"", "\"target\":\"missing\"");
        let error = layout_graph_json(&invalid).expect_err("unknown endpoint should fail");
        assert!(error.contains("unknown endpoint"));
    }

    #[test]
    fn archived_graph_without_mere_uses_its_first_public_node_as_focus() {
        let archived = r#"{
          "schema":"mer3ly.repo-graph/v1",
          "nodes":[{"id":"graphshell","name":"Graphshell","class":"product","status":"archived","pushed_at":"2026-02-21T13:32:51-05:00"}],
          "edges":[]
        }"#;

        let encoded = layout_graph_json(archived).expect("layout archived graph");
        let value: serde_json::Value = serde_json::from_str(&encoded).expect("parse layout");
        assert_eq!(value["focus"], "graphshell");
        for arrangement in value["arrangements"].as_array().expect("arrangements") {
            assert_eq!(arrangement["nodes"][0]["id"], "graphshell");
        }
    }

    #[test]
    fn every_registered_arrangement_is_selectable_or_explained() {
        let encoded = layout_graph_json(SAMPLE).expect("layout graph catalog");
        let value: serde_json::Value = serde_json::from_str(&encoded).expect("parse layout");
        let mut arrangement_ids = value["arrangements"]
            .as_array()
            .expect("arrangements")
            .iter()
            .map(|arrangement| arrangement["id"].as_str().expect("arrangement id"))
            .collect::<Vec<_>>();
        arrangement_ids.extend(
            value["unavailable_arrangements"]
                .as_array()
                .expect("unavailable arrangements")
                .iter()
                .map(|arrangement| arrangement["id"].as_str().expect("arrangement id")),
        );
        arrangement_ids.sort_unstable();
        assert_eq!(
            arrangement_ids,
            vec![
                "graph_layout:grid",
                "graph_layout:kanban",
                "graph_layout:lsystem",
                "graph_layout:penrose",
                "graph_layout:phyllotaxis",
                "graph_layout:radial",
                "graph_layout:semantic_embedding",
                "graph_layout:stack",
                "graph_layout:timeline",
            ]
        );
    }

    #[test]
    fn timeline_uses_one_proportional_axis_with_collision_free_strips() {
        let placed = place_timeline_strips(vec![
            ("oldest".to_owned(), Point2D::new(0.0, 0.0)),
            ("quarter".to_owned(), Point2D::new(25.0, 0.0)),
            ("newest".to_owned(), Point2D::new(100.0, 0.0)),
        ]);
        let points = placed
            .iter()
            .map(|(_, point)| (point.x.round() as i32, point.y.round() as i32))
            .collect::<HashSet<_>>();

        assert_eq!(points.len(), 3);
        assert_eq!(placed[0].0, "oldest");
        assert_eq!(placed[0].1.x, -310.0);
        assert_eq!(placed[1].1.x, -155.0);
        assert_eq!(placed[2].0, "newest");
        assert_eq!(placed[2].1.x, 310.0);
    }

    #[test]
    fn timestamp_coordinate_preserves_time_of_day_and_midnight_distance() {
        let before = timestamp_coordinate("2026-07-30T23:59:59Z").expect("before midnight");
        let after = timestamp_coordinate("2026-07-31T00:00:01Z").expect("after midnight");
        assert_eq!(after - before, 2.0);
    }

    #[test]
    fn portable_projection_is_a_real_scenograph_score_scene_and_trace() {
        let json = portable_projection_json(SAMPLE).expect("portable projection");
        let artifact: PortableProjectionArtifact =
            serde_json::from_str(&json).expect("portable projection JSON");
        assert_eq!(artifact.schema, PORTABLE_PROJECTION_SCHEMA);
        assert_eq!(artifact.score.items.len(), 3);
        assert_eq!(artifact.snapshot.active_item_count(), 3);
        assert_eq!(artifact.snapshot.tables.relations.len(), 2);
        assert_eq!(artifact.default_trace.len(), 7);
        assert_eq!(artifact.default_trace[1].label, "Move Turnstone");
        assert!(artifact.default_trace[1].diff.is_some());

        let receipt = consume_portable_projection_json(&json).expect("native receipt");
        assert_eq!(receipt.initial_revision, 1);
        assert_eq!(receipt.final_revision, 5);
        assert_eq!(receipt.active_items, 3);
        assert_eq!(receipt.active_relations, 1);
        assert_eq!(receipt.picked_source, "mere");
        assert_eq!(receipt.honored_holds, 0);
    }

    /// The sandbox's own shared-scene shape, extra fields and all, so the test
    /// proves the seam accepts what the live path really emits.
    const SHARED_SCENE: &str = r#"{
      "schema": "mer3ly.graphshell-scene-state/v1",
      "dataset": "public-repos",
      "source": {"source":"mer3ly/specimen","commit":"authored","committed_at":"static"},
      "reading": "neighbors",
      "arrangement": "graph_layout:radial",
      "motion": "free",
      "backdrop": {"kind":"ambient","collidable":false},
      "physics": "settled",
      "selection": "mere",
      "pins": [{"id":"genet","x":-120.5,"y":64.25}]
    }"#;

    #[test]
    fn the_expected_generation_is_the_score_generation() {
        // The checkability invariant: what a citation carries as
        // expects.generation must be what the solved score stamps, or the
        // check reports drift on every link ever written.
        let expected = authority_generation(SAMPLE).expect("generation");
        let artifact = portable_projection(SAMPLE).expect("portable projection");
        assert_eq!(expected, artifact.score.generation.to_string());
        // And it moves when the authority moves, or it checks nothing.
        let altered = SAMPLE.replace("Turnstone", "Ternstone");
        assert_ne!(
            authority_generation(&altered).expect("generation"),
            expected
        );
    }

    #[test]
    fn a_visitor_pin_reaches_the_score_and_the_solver_honors_it() {
        let json = portable_projection_with_placement_json(SAMPLE, SHARED_SCENE)
            .expect("portable projection with placement");
        let artifact: PortableProjectionArtifact = serde_json::from_str(&json).unwrap();

        assert_eq!(artifact.score.holds.len(), 1, "the pin reached the score");
        let held = &artifact.score.holds[0];
        assert_eq!(held.source.id, "genet");
        assert_eq!(held.hold, Hold::Pinned);

        // Spiral is the arrangement, and Spiral is exactly the family that used
        // to discard an authored coordinate without saying so.
        let instance = instance_for_source(&artifact.snapshot, "genet").expect("genet is placed");
        let at = artifact
            .snapshot
            .active_item(instance)
            .unwrap()
            .transform
            .translate;
        assert_eq!((at.x, at.y), (-120.5, 64.25));

        let receipt = consume_portable_projection_json(&json).expect("receipt");
        assert_eq!(receipt.honored_holds, 1);
    }

    #[test]
    fn anchored_motion_records_a_softer_hold() {
        let anchored = SHARED_SCENE.replace(r#""motion": "free""#, r#""motion": "anchored""#);
        let artifact = portable_projection_holding(
            SAMPLE,
            &serde_json::from_str::<PlacementDelta>(&anchored).unwrap(),
        )
        .expect("anchored projection");
        assert_eq!(artifact.score.holds[0].hold, Hold::Anchored);
    }

    #[test]
    fn an_unpinned_share_projects_exactly_as_the_plain_path() {
        let no_pins = SHARED_SCENE.replace(
            r#""pins": [{"id":"genet","x":-120.5,"y":64.25}]"#,
            r#""pins": []"#,
        );
        let held = portable_projection_with_placement_json(SAMPLE, &no_pins).unwrap();
        let plain = portable_projection_json(SAMPLE).unwrap();
        assert_eq!(held, plain, "an empty delta must not perturb the receipt");
    }

    #[test]
    fn a_pin_naming_an_unknown_node_is_refused() {
        let ghost = SHARED_SCENE.replace(r#""id":"genet""#, r#""id":"no-such-repo""#);
        let error = portable_projection_with_placement_json(SAMPLE, &ghost)
            .expect_err("a pin on a node the authority lacks is a broken citation");
        assert!(error.contains("no-such-repo"), "{error}");
    }

    #[test]
    fn consuming_catches_a_hold_the_scene_did_not_honor() {
        // Forge the failure the seam exists to make impossible: an artifact
        // claiming a pin the scene does not actually satisfy.
        let json = portable_projection_with_placement_json(SAMPLE, SHARED_SCENE).unwrap();
        let mut artifact: PortableProjectionArtifact = serde_json::from_str(&json).unwrap();
        artifact.score.holds[0].at = Vec2::new(999.0, 999.0);
        let forged = serde_json::to_string(&artifact).unwrap();
        let error = consume_portable_projection_json(&forged)
            .expect_err("an unhonored hold must not pass consumption");
        assert!(error.contains("was not honored"), "{error}");
    }

    #[test]
    fn sandbox_binds_arrangement_slots_to_real_seiche_motion_and_props() {
        let mut physics = graph_physics(SAMPLE).expect("sandbox physics");
        let positions = serde_json::to_string(&vec![
            GraphNodePosition {
                id: "mere".to_owned(),
                x: -120.0,
                y: 0.0,
            },
            GraphNodePosition {
                id: "genet".to_owned(),
                x: 0.0,
                y: 0.0,
            },
            GraphNodePosition {
                id: "turnstone".to_owned(),
                x: 120.0,
                y: 0.0,
            },
        ])
        .expect("positions");

        physics
            .apply_arrangement(&positions, "anchored")
            .expect("anchor slots");
        assert_eq!(physics.simulation.anchor_count(), 3);
        physics
            .apply_backdrop("props", true)
            .expect("collidable props");
        assert_eq!(physics.simulation.scene_body_count(), 6);

        let frame: serde_json::Value =
            serde_json::from_str(&physics.frame_json().expect("physics frame")).expect("JSON");
        assert_eq!(frame["schema"], "mer3ly.graph-physics-frame/v1");
        assert_eq!(frame["nodes"].as_array().expect("nodes").len(), 3);
        assert_eq!(frame["props"].as_array().expect("props").len(), 6);

        physics
            .apply_arrangement(&positions, "free")
            .expect("free motion");
        assert_eq!(physics.simulation.anchor_count(), 0);
        assert_eq!(
            physics
                .apply_arrangement(&positions, "frozen")
                .expect_err("interactive graphs reject frozen motion"),
            "unsupported mobility frozen"
        );
    }

    #[test]
    fn radial_focus_is_host_configurable() {
        let mut value: serde_json::Value = serde_json::from_str(SAMPLE).expect("sample");
        value["focus"] = serde_json::Value::String("turnstone".to_owned());
        let encoded = layout_graph_json(&value.to_string()).expect("focused layout");
        let layout: serde_json::Value = serde_json::from_str(&encoded).expect("layout JSON");
        assert_eq!(layout["focus"], "turnstone");
    }

    #[test]
    fn sandbox_exports_meres_primitive_and_behavior_registry() {
        let encoded = representation_registry().expect("representation registry");
        let registry: serde_json::Value =
            serde_json::from_str(&encoded).expect("representation registry JSON");
        assert_eq!(registry["schema"], "mere.graph-representation-registry/v2");
        assert_eq!(registry["profiles"][0]["primitive"]["body"], "hexagon");
        assert_eq!(
            registry["profiles"][0]["behaviors"][0]["behavior"],
            "inspect"
        );
    }

    #[test]
    fn sandbox_exports_meres_reading_registry() {
        let encoded = reading_registry().expect("reading registry");
        let registry: serde_json::Value =
            serde_json::from_str(&encoded).expect("reading registry JSON");
        assert_eq!(registry["schema"], "mere.graph-reading-registry/v1");
        assert_eq!(registry["profiles"].as_array().expect("profiles").len(), 5);
        assert_eq!(registry["profiles"][3]["id"], "neighbors");
        assert_eq!(
            registry["profiles"][3]["actor_scope"],
            "focus_and_neighbors"
        );
    }

    #[test]
    fn neighbors_is_a_real_actor_projection_independent_of_arrangement() {
        let current: serde_json::Value = serde_json::from_str(SAMPLE).expect("sample graph");
        let request = serde_json::json!({
            "reading": "neighbors",
            "current": current,
            "focus": "genet"
        });
        let encoded = project_reading_json(&request.to_string()).expect("neighbors reading");
        let reading: serde_json::Value = serde_json::from_str(&encoded).expect("neighbors graph");
        let ids = reading["nodes"]
            .as_array()
            .expect("nodes")
            .iter()
            .map(|node| node["id"].as_str().expect("node id"))
            .collect::<HashSet<_>>();
        assert_eq!(ids, HashSet::from(["mere", "genet"]));
        assert_eq!(reading["edges"].as_array().expect("edges").len(), 1);
        assert_eq!(reading["focus"], "genet");
    }

    #[test]
    fn changes_is_computed_by_the_native_reading_consumer() {
        let current: serde_json::Value = serde_json::from_str(SAMPLE).expect("sample graph");
        let mut previous = current.clone();
        previous["nodes"]
            .as_array_mut()
            .expect("previous nodes")
            .retain(|node| node["id"] != "turnstone");
        previous["edges"]
            .as_array_mut()
            .expect("previous edges")
            .retain(|edge| edge["source"] != "turnstone");
        previous["nodes"][0]["pushed_at"] = serde_json::json!("2026-07-29T05:44:23Z");
        let request = serde_json::json!({
            "reading": "changes",
            "current": current,
            "previous": previous
        });
        let encoded = project_reading_json(&request.to_string()).expect("changes reading");
        let reading: serde_json::Value = serde_json::from_str(&encoded).expect("changes graph");
        let change_by_id = reading["nodes"]
            .as_array()
            .expect("nodes")
            .iter()
            .map(|node| {
                (
                    node["id"].as_str().expect("node id"),
                    node["change"].as_str().expect("change"),
                )
            })
            .collect::<HashMap<_, _>>();
        assert_eq!(change_by_id["mere"], "updated");
        assert_eq!(change_by_id["genet"], "stable");
        assert_eq!(change_by_id["turnstone"], "added");
    }

    fn sample_matrix_request() -> MatrixProjectionRequest {
        let authority: GraphInput = serde_json::from_str(SAMPLE).expect("sample graph");
        MatrixProjectionRequest {
            rows: MatrixAxisRequest {
                dataset: "live".into(),
                record: "rev:1".into(),
                reading: "neighbors".into(),
                current: authority.clone(),
                previous: None,
                focus: Some("mere".into()),
            },
            columns: MatrixAxisRequest {
                dataset: "live".into(),
                record: "rev:1".into(),
                reading: "graph".into(),
                current: authority,
                previous: None,
                focus: None,
            },
        }
    }

    #[test]
    fn two_independent_readings_form_a_provenance_carrying_matrix_capture() {
        let artifact = matrix_projection(&sample_matrix_request()).expect("Matrix projection");
        let receipt = consume_matrix_projection(&artifact).expect("Matrix receipt");
        assert_eq!(artifact.rows.reading, "neighbors");
        assert_eq!(artifact.columns.reading, "graph");
        assert_eq!(receipt.cells, receipt.row_sources * receipt.column_sources);
        assert!(
            receipt.relation_cells >= 2,
            "cross-scope relations remain cells"
        );
        assert!(receipt.accessible_table);

        let mere_source = artifact
            .capture
            .scene
            .tables
            .sources
            .iter()
            .position(|source| {
                source.as_ref().is_some_and(|source| {
                    source.adapter == PROJECTION_ADAPTER && source.id == "mere"
                })
            })
            .expect("Mere source");
        let appearances = artifact
            .capture
            .scene
            .tables
            .items
            .iter()
            .flatten()
            .filter(|item| item.source.0 as usize == mere_source)
            .count();
        assert!(appearances >= 2, "one source backs both Matrix headings");

        assert!(
            artifact
                .accessible_html
                .contains("<table data-projection-family=\"matrix\"")
        );
        assert!(artifact.accessible_html.contains("scope=\"row\""));
        assert!(artifact.accessible_html.contains("scope=\"col\""));
    }

    fn scene_source_mapping(snapshot: &scenotime::SceneSnapshot) -> Vec<(InstanceId, SourceRef)> {
        snapshot
            .tables
            .items
            .iter()
            .enumerate()
            .filter_map(|(index, item)| {
                let item = item.as_ref()?;
                if !item.visible {
                    return None;
                }
                let source = snapshot
                    .tables
                    .sources
                    .get(item.source.0 as usize)?
                    .as_ref()?;
                Some((InstanceId(index as u32), source.clone()))
            })
            .collect()
    }

    #[derive(Clone)]
    struct MatrixSnapshotEndpoint {
        snapshot: chirograph::ProjectionSnapshot,
        resources: BTreeMap<chirograph::ContentHash, Vec<u8>>,
    }

    fn matrix_presentation(
        artifact: &MatrixProjectionArtifact,
    ) -> (
        chirograph::PresentationManifest,
        BTreeMap<chirograph::ContentHash, Vec<u8>>,
    ) {
        let labels = artifact
            .rows
            .sources
            .iter()
            .chain(&artifact.columns.sources)
            .map(|source| source.name.clone())
            .chain(artifact.cells.iter().map(|cell| cell.description.clone()))
            .collect::<Vec<_>>();
        assert_eq!(
            labels.len(),
            scene_source_mapping(&artifact.capture.scene).len(),
            "Matrix construction and presentation keep the same row-column-cell order"
        );
        let mut presentation = chirograph::PresentationManifest::default();
        let mut resources = BTreeMap::new();
        for ((instance, _), label) in scene_source_mapping(&artifact.capture.scene)
            .into_iter()
            .zip(labels)
        {
            let glyph = chirograph::NativeGlyphV1 {
                label: label.clone(),
                icon: None,
                color: None,
            };
            let bytes = serde_json::to_vec(&glyph).expect("encode Matrix glyph");
            let resource = chirograph::ContentHash::of(&bytes);
            let key = chirograph::PresentationKey(format!("matrix:{}", instance.0));
            presentation.bindings.push(chirograph::PresentationBinding {
                instance,
                key: key.clone(),
            });
            presentation.offers.insert(
                key,
                vec![chirograph::PresentationOffer {
                    codec: chirograph::PresentationCodec::NativeGlyphV1,
                    resource,
                    byte_size: bytes.len() as u64,
                    requires: chirograph::PresentationCapability::NativeGlyph,
                    semantics: chirograph::PresentationSemantics {
                        label,
                        role: chirograph::SemanticRole::Graphic,
                        bounds: chirograph::BoundsRelationship::FillFootprint,
                        actions: Vec::new(),
                    },
                }],
            );
            resources.insert(resource, bytes);
        }
        assert_eq!(
            presentation.bindings.len(),
            artifact.capture.scene.active_item_count(),
            "every remote Matrix instance has presentation semantics"
        );
        (presentation, resources)
    }

    impl graphshell_endpoint::ProjectionCatalog for MatrixSnapshotEndpoint {
        fn describe(&self) -> chirograph::EndpointDescriptor {
            chirograph::EndpointDescriptor {
                label: "Mer3ly Matrix receipt".into(),
                projections: vec![chirograph::ProjectionOffer {
                    label: "Two-reading Matrix".into(),
                    request: chirograph::ProjectionRequest {
                        version: chirograph::ProtocolVersion::V1,
                        session: self.snapshot.session.clone(),
                        score: Score::new(SceneArrangement::Grid(sceno::Grid::default())),
                    },
                }],
            }
        }
    }

    impl graphshell_endpoint::ProjectionSource for MatrixSnapshotEndpoint {
        type Error = String;

        fn snapshot(
            &mut self,
            request: chirograph::ProjectionRequest,
        ) -> Result<chirograph::ProjectionSnapshot, Self::Error> {
            if request.session != self.snapshot.session {
                return Err("unknown Matrix projection".into());
            }
            Ok(self.snapshot.clone())
        }
    }

    impl graphshell_endpoint::PresentationSource for MatrixSnapshotEndpoint {
        type Error = String;

        fn resource(
            &mut self,
            request: chirograph::ResourceRequest,
        ) -> Result<chirograph::ResourceResponse, Self::Error> {
            if request.session != self.snapshot.session {
                return Err("unknown Matrix projection".into());
            }
            let bytes = self
                .resources
                .get(&request.resource)
                .cloned()
                .ok_or_else(|| "unknown Matrix presentation resource".to_owned())?;
            Ok(chirograph::ResourceResponse::new(request.session, bytes))
        }
    }

    impl graphshell_endpoint::IntentSink for MatrixSnapshotEndpoint {
        type Error = String;

        fn invoke(
            &mut self,
            _: chirograph::IntentInvocation,
        ) -> Result<chirograph::IntentResult, Self::Error> {
            Err("the Matrix receipt is read-only".into())
        }
    }

    impl graphshell_endpoint::ProjectionNoticeSource for MatrixSnapshotEndpoint {
        type Error = String;

        fn poll_notice(&mut self) -> Result<Option<chirograph::CarrierNotice>, Self::Error> {
            Ok(None)
        }
    }

    const FT7_NETWORK: notochord::NetworkId = notochord::NetworkId([3; 32]);
    const FT7_ROOT_AUTHORITY: [u8; 32] = [7; 32];
    const FT7_NOW_MS: u64 = 50;

    fn ft7_owner() -> personae::InMemoryProvider {
        personae::InMemoryProvider::from_seed([1; 32])
    }

    fn ft7_viewer() -> personae::InMemoryProvider {
        personae::InMemoryProvider::from_seed([4; 32])
    }

    fn ft7_profile_ref() -> notochord::ProfileRef {
        notochord::ProfileRef {
            id: "mere.base".into(),
            revision: 1,
        }
    }

    fn ft7_grant(subject: [u8; 32]) -> personae::delegation::SignedDelegationCertificate {
        use personae::IdentityProvider as _;

        let owner = ft7_owner();
        personae::delegation::SignedDelegationCertificate::issue(
            &owner,
            personae::delegation::DelegationCertificate::new(
                personae::delegation::DelegationParent::Root(FT7_ROOT_AUTHORITY),
                owner.master_public_key().to_bytes(),
                subject,
                personae::delegation::CapabilityScope {
                    domain: graphshell::admission::GRAPHSHELL_DOMAIN.into(),
                    resource: FT7_NETWORK.0.to_vec(),
                    path_prefix: graphshell::admission::PROJECTION_SERVICE.into(),
                    actions: [graphshell::admission::CONNECT_ACTION.to_owned()]
                        .into_iter()
                        .collect(),
                },
                5,
                10,
                Some(FT7_NOW_MS + 3_600_000),
                1,
                [1; 32],
            ),
        )
        .expect("issue Matrix viewer certificate")
    }

    fn ft7_policy() -> notochord::LocalNetworkPolicy {
        use personae::IdentityProvider as _;

        graphshell::carrier::projection_policy(
            FT7_NETWORK,
            vec![notochord::TrustedRoot {
                authority: FT7_ROOT_AUTHORITY,
                issuer: ft7_owner().master_public_key().to_bytes(),
            }],
            vec![ft7_profile_ref()],
            None,
        )
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn one_matrix_mapping_survives_local_remote_and_frozen_realizations() {
        use chirograph::Carrier;
        use graphshell::admission::open_session;
        use graphshell::carrier::accept_projection_session;
        use graphshell::lifecycle::SessionAuthority;
        use graphshell::network_carrier::{
            CarrierRuntime, NetworkCarrier, dial_projection_session, projection_binding,
        };
        use graphshell::session_notices::serve_admitted_session_notifying;
        use graphshell_client::{RetainedEndpointSession, frozen::FrozenScene};
        use notochord::{RevocationLedger, TrafficClass};
        use personae::IdentityProvider as _;
        use std::sync::RwLock;
        use std::time::Duration;
        use transport::{PeerID, memory::MemoryTransport};

        let artifact = matrix_projection(&sample_matrix_request()).expect("Matrix projection");
        let source_mapping = scene_source_mapping(&artifact.capture.scene);
        assert!(
            source_mapping
                .iter()
                .enumerate()
                .any(|(left, (_, source))| source_mapping[left + 1..]
                    .iter()
                    .any(|(_, candidate)| candidate == source)),
            "the receipt keeps a genuinely repeated source instance"
        );

        let (presentation, resources) = matrix_presentation(&artifact);
        let snapshot = chirograph::ProjectionSnapshot {
            version: chirograph::ProtocolVersion::V1,
            session: chirograph::ProjectionSession("mer3ly.matrix.ft7".into()),
            scene: artifact.capture.scene.clone(),
            presentation,
            cache_policy: chirograph::CachePolicy::default(),
        };
        let local_carrier = graphshell_local::LocalCarrier::new(
            MatrixSnapshotEndpoint {
                snapshot: snapshot.clone(),
                resources: resources.clone(),
            },
            |_: &mut MatrixSnapshotEndpoint, _| Err("the frozen receipt does not resume".into()),
        );
        let mut local_viewer = RetainedEndpointSession::over(
            Box::new(local_carrier),
            chirograph::CapabilityProfile::default(),
        )
        .expect("local Graphshell host discovers the Matrix endpoint");
        let local_session = local_viewer.mount(0).expect("local host mounts Matrix");
        assert_eq!(
            local_viewer
                .resolve_all(&local_session)
                .expect("local host realizes every Matrix item")
                .len(),
            source_mapping.len()
        );
        let local_mapping = scene_source_mapping(
            &local_viewer
                .client()
                .mounted(&local_session)
                .expect("local host owns the disclosed snapshot")
                .scene,
        );
        assert_eq!(local_mapping, source_mapping);
        // The in-process carrier has no session plane; dropping the handle is
        // its complete lifecycle rather than pretending it can answer Close.
        drop(local_viewer);

        let viewer_identity = ft7_viewer();
        let subject = viewer_identity.master_public_key().to_bytes();
        let client_peer = PeerID::from_bytes(&subject).expect("Matrix viewer peer");
        let server_peer = PeerID::from_bytes(&ft7_owner().master_public_key().to_bytes())
            .expect("Matrix owner peer");
        let (server_transport, client_transport) = MemoryTransport::pair(server_peer, client_peer);

        let serving = tokio::spawn(async move {
            let mut admitted = accept_projection_session(
                &server_transport,
                &ft7_policy(),
                &RevocationLedger::default(),
                FT7_NOW_MS,
                0,
            )
            .await
            .expect("Matrix accept path")
            .expect("the Matrix viewer is admitted");
            let authority = SessionAuthority::retain_admitted(&admitted);
            let mut endpoint = MatrixSnapshotEndpoint {
                snapshot,
                resources,
            };
            let mut resume = |_: &mut MatrixSnapshotEndpoint, _: chirograph::ResumeRequest| {
                Err("the fixed Matrix receipt does not resume".to_owned())
            };
            serve_admitted_session_notifying(
                &mut admitted,
                &authority,
                &RwLock::new(RevocationLedger::default()),
                &mut endpoint,
                &mut resume,
                || FT7_NOW_MS,
                Duration::from_millis(10),
            )
            .await
            .expect("serve admitted Matrix session")
        });

        let handle = tokio::runtime::Handle::current();
        let accessible_html = artifact.accessible_html.clone();
        tokio::task::spawn_blocking(move || {
            let binding = projection_binding(client_peer);
            let hello = open_session(
                &viewer_identity,
                FT7_NETWORK,
                ft7_profile_ref(),
                TrafficClass::Interactive,
                [5; 32],
                &binding,
                vec![ft7_grant(subject)],
            )
            .expect("issue Matrix session hello");
            let stream = handle
                .block_on(dial_projection_session(
                    &client_transport,
                    server_peer,
                    &hello,
                    &ft7_policy().limits,
                ))
                .expect("dial Matrix projection service")
                .expect("the owner admits the Matrix viewer");
            let mut carrier =
                NetworkCarrier::over(stream, CarrierRuntime::borrowed(handle.clone()));
            let opened = carrier
                .request(chirograph::CarrierRequestBody::Open(Box::new(
                    chirograph::SessionOpen {
                        version: chirograph::ProtocolVersion::V1,
                        capabilities: chirograph::CapabilityProfile::default(),
                    },
                )))
                .expect("viewer opens the admitted Graphshell session");
            match opened {
                chirograph::CarrierResponseBody::Opened(opened) => {
                    assert_eq!(opened.status, chirograph::SessionStatus::Live);
                    assert_eq!(opened.descriptor.projections.len(), 1);
                    assert_eq!(opened.descriptor.projections[0].label, "Two-reading Matrix");
                }
                other => panic!("expected an opened Matrix session, got {other:?}"),
            }

            let mut viewer = RetainedEndpointSession::over(
                Box::new(carrier),
                chirograph::CapabilityProfile::default(),
            )
            .expect("source-free viewer discovers the Matrix endpoint");
            let session = viewer.mount(0).expect("remote viewer mounts Matrix");
            let resolved = viewer
                .resolve_all(&session)
                .expect("remote viewer realizes every Matrix item");
            assert_eq!(
                resolved
                    .iter()
                    .map(|(instance, _)| *instance)
                    .collect::<Vec<_>>(),
                local_mapping
                    .iter()
                    .map(|(instance, _)| *instance)
                    .collect::<Vec<_>>(),
                "the remote presentation covers the same instance table"
            );

            {
                let mounted = viewer
                    .client()
                    .mounted(&session)
                    .expect("viewer owns the disclosed snapshot");
                let remote_mapping = scene_source_mapping(&mounted.scene);
                assert_eq!(
                    remote_mapping, local_mapping,
                    "the admitted Graphshell session preserves every Matrix binding"
                );

                let names = resolved
                    .iter()
                    .map(|(instance, presentation)| {
                        (*instance, presentation.semantics.label.clone())
                    })
                    .collect::<HashMap<_, _>>();
                let frozen =
                    FrozenScene::freeze_snapshot(&mounted.scene, "Two-reading Matrix", &names);
                let frozen_mapping = frozen
                    .instances
                    .iter()
                    .map(|instance| (instance.instance, instance.source.clone()))
                    .collect::<Vec<_>>();
                assert_eq!(
                    frozen_mapping, local_mapping,
                    "the frozen semantic document preserves the remote mapping"
                );
                let frozen_html = frozen.to_html("mer3ly-matrix");
                for (instance, source) in &local_mapping {
                    let carried = format!(
                        "data-projection-instance=\"{}\" data-source-adapter=\"{}\" data-source-id=\"{}\"",
                        instance.0, source.adapter, source.id
                    );
                    assert!(
                        frozen_html.contains(&carried),
                        "frozen navigation lost {carried}"
                    );
                    assert!(
                        accessible_html.contains(&carried),
                        "the Matrix table lost {carried}"
                    );
                }
                assert!(frozen_html.contains("<table class=\"frozen-alternate\""));
            }

            viewer.close().expect("viewer closes the Graphshell session");
        })
        .await
        .expect("Matrix viewer thread");

        let summary = serving.await.expect("Matrix server task");
        assert!(
            summary.answered > 3,
            "the admitted session served the Matrix"
        );
    }

    #[test]
    fn composed_shelfmark_restores_view_state_without_rewriting_authority() {
        let mut matrix = sample_matrix_request();
        let mut specimen: GraphInput = serde_json::from_str(SAMPLE).expect("specimen graph");
        specimen.nodes[0].status = "reviewed".into();
        matrix.columns.dataset = "specimen".into();
        matrix.columns.record = "authored".into();
        matrix.columns.current = specimen.clone();

        let mut selection = CoordinatedSelection::new(SelectionResolution::Crossfilter);
        selection.set(
            chirograph::SelectionRole::Focus,
            chirograph::Selection::one("spatial", "node", "mere"),
        );
        selection.set(
            chirograph::SelectionRole::Brush,
            chirograph::Selection::one("matrix", "node", "turnstone"),
        );
        let request = ComposedShelfmarkRequest {
            matrix: matrix.clone(),
            spatial: SpatialShelfmarkRequest {
                dataset: "live".into(),
                record: matrix.rows.record.clone(),
                reading: "neighbors".into(),
                current: matrix.rows.current.clone(),
                previous: matrix.rows.previous.clone(),
                focus: Some("mere".into()),
                arrangement: "graph_layout:grid".into(),
            },
            selection,
            instances: vec![MatrixInstanceDelta {
                instance: ProjectedInstanceAddress {
                    view: "deck".into(),
                    source: SourceRef::new(PROJECTION_ADAPTER, "mere"),
                    facet: "summary".into(),
                },
                visible: false,
            }],
            placement: vec![HeldPlacement {
                source: SourceRef::new(PROJECTION_ADAPTER, "mere"),
                at: Vec2::new(12.0, -8.0),
                hold: Hold::Pinned,
            }],
            motion: Some("free".into()),
            backdrop: Some(BackdropDelta {
                kind: "props".into(),
                collidable: true,
            }),
            facets: vec![ProjectedInstanceAddress {
                view: "deck".into(),
                source: SourceRef::new(PROJECTION_ADAPTER, "mere"),
                facet: "summary".into(),
            }],
            camera: Some(CameraDelta {
                x: 40.0,
                y: -20.0,
                zoom: 1.25,
            }),
            carried_delta: BTreeMap::from([("other.reader".into(), "opaque".into())]),
        };
        let artifact = matrix_projection(&matrix).expect("composed Matrix");
        let shelfmark = composed_matrix_shelfmark(&request, &artifact).expect("shelfmark");
        assert_ne!(
            shelfmark.inputs["rows"].expects_generation,
            shelfmark.inputs["columns"].expects_generation
        );
        assert_eq!(
            shelfmark.inputs["rows"].authority.record, shelfmark.inputs["spatial"].authority.record,
            "view state must not rewrite the cited authority bytes"
        );
        let mut altered_view = request.clone();
        altered_view.selection = CoordinatedSelection::new(SelectionResolution::Single);
        altered_view.placement.clear();
        altered_view.backdrop = Some(BackdropDelta {
            kind: "ambient".into(),
            collidable: false,
        });
        altered_view.facets.clear();
        altered_view.camera = Some(CameraDelta {
            x: -160.0,
            y: 120.0,
            zoom: 0.8,
        });
        let altered_shelfmark =
            composed_matrix_shelfmark(&altered_view, &artifact).expect("altered view shelfmark");
        for role in ["rows", "columns", "spatial"] {
            let before = &shelfmark.inputs[role];
            let after = &altered_shelfmark.inputs[role];
            assert_eq!(before.authority.record, after.authority.record);
            assert_eq!(before.reading, after.reading);
            assert_eq!(before.reading_parameters, after.reading_parameters);
            assert_eq!(before.arrangement, after.arrangement);
            assert_eq!(before.expects_generation, after.expects_generation);
        }
        assert_ne!(shelfmark.delta, altered_shelfmark.delta);

        let resolution = MatrixShelfmarkResolutionRequest {
            shelfmark: shelfmark.clone(),
            datasets: BTreeMap::from([
                (
                    "live".into(),
                    ResolvedMatrixDataset {
                        current: matrix.rows.current,
                        previous: None,
                    },
                ),
                (
                    "specimen".into(),
                    ResolvedMatrixDataset {
                        current: specimen,
                        previous: None,
                    },
                ),
            ]),
        };
        let receipt = resolve_matrix_shelfmark_value(&resolution).expect("resolve shelfmark");
        assert_eq!(
            receipt.selection_resolution,
            SelectionResolution::Crossfilter
        );
        assert_eq!(receipt.honored_instance_deltas, 1);
        assert_eq!(receipt.honored_placements, 1);
        assert_eq!(receipt.honored_facets, 1);
        assert_eq!(receipt.camera.zoom, 1.25);
        assert_eq!(receipt.input_generations.len(), 3);
        assert_eq!(shelfmark.delta["other.reader"], "opaque");

        let wire = serde_json::to_string(&shelfmark).expect("stable shelfmark wire");
        let far_side: ShelfmarkV1 = serde_json::from_str(&wire).expect("decode shelfmark");
        assert_eq!(far_side, shelfmark);
    }
}
