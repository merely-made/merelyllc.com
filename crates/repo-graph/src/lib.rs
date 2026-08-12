use std::collections::{HashMap, HashSet};

use arrangements::camera::CanvasViewport;
use arrangements::scene::{CanvasEdge, CanvasNode, CanvasSceneInput};
use arrangements::{
    AxisValue, Layout, LayoutExtras, LayoutRegistry, Radial, RadialAngularPolicy, RadialConfig,
    RadialUnreachablePolicy, StaticLayoutState, Timeline, TimelineConfig,
};
use euclid::default::Point2D;
use sceno::{
    Arrangement as SceneArrangement, Footprint, Placement, Representation, RoutedRelation, Score,
    ScoreItem, SourceRef, Spiral, Vec2,
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
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct GraphEdge {
    id: String,
    source: String,
    target: String,
    kind: String,
    provenance: String,
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
/// Arrangements supply slots. This adapter decides whether those slots are
/// frozen positions, anchor springs, or initial conditions for free physics.
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
        if self.mobility != "frozen" {
            self.simulation.unpin(key);
        }
        Ok(())
    }

    #[wasm_bindgen(js_name = isPinned)]
    pub fn is_pinned(&self, id: &str) -> bool {
        self.manually_pinned.contains(id) || self.mobility == "frozen"
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
    match class {
        "document" | "page" | "note" => NodeCollider::Square { half: 22.0 },
        "device" | "tool" => NodeCollider::RoundedSquare {
            half: 24.0,
            border: 7.0,
        },
        "event" => NodeCollider::Hull {
            points: vec![(0.0, -27.0), (27.0, 0.0), (0.0, 27.0), (-27.0, 0.0)],
            fallback: 24.0,
        },
        "place" | "person" | "community" => NodeCollider::Ball { radius: 25.0 },
        _ => NodeCollider::Ball { radius: 22.0 },
    }
}

impl GraphPhysics {
    fn apply_arrangement(&mut self, positions: &str, mobility: &str) -> Result<(), String> {
        if !matches!(mobility, "frozen" | "anchored" | "free") {
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
        if mobility == "frozen" {
            for (key, point) in &targets {
                self.simulation.pin(*key, *point);
            }
        } else {
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
}

pub fn portable_projection_json(input: &str) -> Result<String, String> {
    let artifact = portable_projection(input)?;
    serde_json::to_string(&artifact)
        .map_err(|error| format!("could not serialize portable projection: {error}"))
}

pub fn portable_projection(input: &str) -> Result<PortableProjectionArtifact, String> {
    let input: GraphInput =
        serde_json::from_str(input).map_err(|error| format!("invalid graph JSON: {error}"))?;
    validate(&input)?;

    let authority_digest = Sha256::digest(
        serde_json::to_vec(&input)
            .map_err(|error| format!("could not canonicalize graph authority: {error}"))?,
    );
    let authority_sha256 = format!("{authority_digest:x}");
    let generation = u64::from_be_bytes(
        authority_digest[..8]
            .try_into()
            .expect("SHA-256 prefix is eight bytes"),
    );

    let mut score = Score::new(SceneArrangement::Spiral(Spiral::default()));
    score.generation = generation;
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

    let scene = CanvasSceneInput {
        nodes: input
            .nodes
            .iter()
            .map(|node| CanvasNode {
                id: node.id.clone(),
                position: Point2D::origin(),
                radius: 24.0,
                label: Some(node.name.clone()),
            })
            .collect(),
        edges: input
            .edges
            .iter()
            .map(|edge| CanvasEdge::untagged(edge.source.clone(), edge.target.clone()))
            .collect(),
    };
    let registry = LayoutRegistry::<String>::default();
    let mut arrangements = Vec::with_capacity(ARRANGEMENT_ORDER.len());
    for arrangement_id in ARRANGEMENT_ORDER {
        let provider = registry
            .resolve(arrangement_id)
            .ok_or_else(|| format!("Mere arrangement registry is missing {arrangement_id}"))?;
        let capability = provider.capability();
        let positions = arrangement_positions(&input, &scene, arrangement_id, &provider, focus)?;
        arrangements.push(GraphArrangement {
            id: capability.id.clone(),
            name: capability.display_name,
            description: arrangement_description(&capability.id, capability.description),
            engine: capability.id,
            nodes: positions,
        });
    }
    let unavailable_arrangements = UNAVAILABLE_ARRANGEMENTS
        .iter()
        .map(|(arrangement_id, reason)| {
            let capability = registry
                .resolve(arrangement_id)
                .ok_or_else(|| format!("Mere arrangement registry is missing {arrangement_id}"))?
                .capability();
            Ok(UnavailableArrangement {
                id: capability.id,
                name: capability.display_name,
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

fn arrangement_positions(
    input: &GraphInput,
    scene: &CanvasSceneInput<String>,
    arrangement_id: &str,
    provider: &std::sync::Arc<dyn arrangements::LayoutProvider<String>>,
    focus: &str,
) -> Result<Vec<GraphNodePosition>, String> {
    let mut extras = LayoutExtras::default();
    match arrangement_id {
        "graph_layout:timeline" => {
            for node in &input.nodes {
                extras.axis_value_by_node.insert(
                    node.id.clone(),
                    AxisValue::Numeric(timestamp_coordinate(&node.pushed_at)?),
                );
            }
        }
        "graph_layout:kanban" => {
            for node in &input.nodes {
                extras
                    .axis_value_by_node
                    .insert(node.id.clone(), AxisValue::Categorical(node.status.clone()));
            }
        }
        _ => {}
    }

    let deltas = if arrangement_id == DEFAULT_ARRANGEMENT {
        let mut layout = Radial::new(RadialConfig {
            focus: Some(focus.to_owned()),
            center: Point2D::origin(),
            ring_spacing: 190.0,
            angular_policy: RadialAngularPolicy::DegreeWeighted,
            rotation_offset: 0.0,
            unreachable_policy: RadialUnreachablePolicy::LeaveInPlace,
        });
        layout.step(
            scene,
            &mut StaticLayoutState::default(),
            0.0,
            &CanvasViewport::default(),
            &extras,
        )
    } else if arrangement_id == "graph_layout:timeline" {
        let mut layout = Timeline::new(TimelineConfig {
            row_gap: 120.0,
            ..TimelineConfig::default()
        });
        layout.step(
            scene,
            &mut StaticLayoutState::default(),
            0.0,
            &CanvasViewport::default(),
            &extras,
        )
    } else {
        let mut layout = provider.create_default();
        let mut state = layout.default_state_erased();
        layout.step_dyn(scene, &mut state, 0.0, &CanvasViewport::default(), &extras)
    };

    let mut host_positions = HashMap::new();
    if arrangement_id == DEFAULT_ARRANGEMENT {
        let unreachable = input
            .nodes
            .iter()
            .filter(|node| node.id != focus && !deltas.contains_key(&node.id))
            .collect::<Vec<_>>();
        let lane_center = unreachable.len().saturating_sub(1) as f32 * 0.5;
        for (index, node) in unreachable.into_iter().enumerate() {
            host_positions.insert(
                node.id.clone(),
                Point2D::new(-470.0, (index as f32 - lane_center) * 190.0),
            );
        }
    }

    let raw_positions = input
        .nodes
        .iter()
        .map(|node| {
            let point = host_positions.get(&node.id).copied().unwrap_or_else(|| {
                deltas
                    .get(&node.id)
                    .map_or_else(Point2D::origin, |delta| Point2D::origin() + *delta)
            });
            (node.id.clone(), point)
        })
        .collect::<Vec<_>>();
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

fn arrangement_description(id: &str, registry_description: Option<String>) -> String {
    match id {
        "graph_layout:radial" => "Neighborhood rings around the selected node.".to_owned(),
        "graph_layout:stack" => {
            "Directed relations arranged into readable topology layers.".to_owned()
        }
        "graph_layout:timeline" => {
            "Repositories grouped by their last public push date.".to_owned()
        }
        "graph_layout:kanban" => "Repositories grouped by public project status.".to_owned(),
        _ => registry_description.unwrap_or_else(|| "Mere positional arrangement.".to_owned()),
    }
}

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
    }

    #[test]
    fn radial_focus_is_host_configurable() {
        let mut value: serde_json::Value = serde_json::from_str(SAMPLE).expect("sample");
        value["focus"] = serde_json::Value::String("turnstone".to_owned());
        let encoded = layout_graph_json(&value.to_string()).expect("focused layout");
        let layout: serde_json::Value = serde_json::from_str(&encoded).expect("layout JSON");
        assert_eq!(layout["focus"], "turnstone");
    }
}
