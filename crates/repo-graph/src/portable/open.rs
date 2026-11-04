use super::*;

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
pub(super) fn read_selection(
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

pub(super) fn source_of(
    snapshot: &SceneSnapshot,
    instance: sceno::InstanceId,
) -> Result<String, String> {
    let item = snapshot
        .active_item(instance)
        .ok_or_else(|| format!("instance {} is a tombstone", instance.0))?;
    Ok(snapshot.tables.sources[item.source.0 as usize]
        .as_ref()
        .ok_or_else(|| format!("instance {} has no source", instance.0))?
        .id
        .clone())
}
