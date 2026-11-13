use super::*;

pub(super) fn project_reading_json(input: &str) -> Result<String, String> {
    let request: ReadingRequest =
        serde_json::from_str(input).map_err(|error| format!("invalid reading request: {error}"))?;
    let projection = project_reading_request(request)?;
    serde_json::to_string(&projection)
        .map_err(|error| format!("could not encode graph reading: {error}"))
}

pub(super) fn project_reading_request(request: ReadingRequest) -> Result<GraphInput, String> {
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

pub(super) fn decorate_current(
    mut current: GraphInput,
    previous: Option<&GraphInput>,
) -> GraphInput {
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

pub(super) fn diff_graphs(previous: Option<&GraphInput>, current: &GraphInput) -> GraphInput {
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

pub(super) fn focus_and_neighbors(current: &GraphInput, focus: &str) -> GraphInput {
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

pub(super) fn node_signature(node: &GraphNodeInput) -> (&str, &str, &str, &str) {
    (&node.name, &node.class, &node.status, &node.pushed_at)
}

pub(super) fn incident_edge_signatures(edges: &[GraphEdge]) -> HashMap<String, String> {
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

pub(super) fn humanize_identifier(value: &str) -> String {
    value.replace(['_', '-'], " ")
}

/// The authority's content identity: its SHA-256 and the score generation
/// derived from that digest's first eight bytes.
///
/// One recipe, shared by the portable path and the citation check, because a
/// citation is only checkable if the generation it carries was computed the
/// same way the resolver recomputes it.
pub(super) fn authority_identity(input: &GraphInput) -> Result<(String, u64), String> {
    let digest = Sha256::digest(
        serde_json::to_vec(input)
            .map_err(|error| format!("could not canonicalize graph authority: {error}"))?,
    );
    let generation = u64::from_be_bytes(
        digest[..8]
            .try_into()
            .expect("SHA-256 prefix is eight bytes"),
    );
    Ok((base16ct::lower::encode_string(&digest), generation))
}

pub(super) fn next_diff(snapshot: &SceneSnapshot, operations: Vec<SceneOp>) -> SceneDiff {
    SceneDiff {
        epoch: snapshot.epoch,
        base: snapshot.revision,
        revision: Revision(snapshot.revision.0 + 1),
        operations,
    }
}

pub(super) fn move_diff(
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
