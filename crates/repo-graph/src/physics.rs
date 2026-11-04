use super::*;

pub(super) fn graph_physics(input: &str) -> Result<GraphPhysics, String> {
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

pub(super) fn collider_for_class(class: &str) -> NodeCollider {
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

pub(super) fn sandbox_props_scene() -> SceneSpec {
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

pub(super) fn physics_prop(
    position: Point2D<f32>,
    rotation: f32,
    collider: NodeCollider,
) -> PhysicsProp {
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
