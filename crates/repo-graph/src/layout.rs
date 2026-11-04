use super::*;

pub(super) fn layout_graph_json(input: &str) -> Result<String, String> {
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
pub(super) fn arrangement_positions(
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

pub(super) fn focal_node(input: &GraphInput) -> &str {
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

pub(super) fn place_timeline_strips(
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

pub(super) fn timeline_lane_offset(lane: usize) -> f32 {
    if lane == 0 {
        return 0.0;
    }
    let distance = lane.div_ceil(2) as f32 * TIMELINE_LANE_GAP;
    if lane % 2 == 1 { -distance } else { distance }
}

pub(super) fn normalize_positions(
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

pub(super) fn timestamp_coordinate(value: &str) -> Result<f64, String> {
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

pub(super) fn validate(input: &GraphInput) -> Result<(), String> {
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
