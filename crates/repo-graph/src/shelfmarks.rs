use super::*;

pub(super) fn composed_matrix_shelfmark_json(input: &str) -> Result<String, String> {
    let request: ComposedShelfmarkRequest = serde_json::from_str(input)
        .map_err(|error| format!("invalid composed shelfmark request: {error}"))?;
    let artifact = matrix_projection(&request.matrix)?;
    let shelfmark = composed_matrix_shelfmark(&request, &artifact)?;
    serde_json::to_string(&shelfmark)
        .map_err(|error| format!("could not encode composed shelfmark: {error}"))
}

pub(super) fn composed_matrix_shelfmark(
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

pub(super) fn resolve_matrix_shelfmark_json(input: &str) -> Result<String, String> {
    let request: MatrixShelfmarkResolutionRequest = serde_json::from_str(input)
        .map_err(|error| format!("invalid Matrix shelfmark resolution request: {error}"))?;
    let receipt = resolve_matrix_shelfmark_value(&request)?;
    serde_json::to_string(&receipt)
        .map_err(|error| format!("could not encode Matrix shelfmark receipt: {error}"))
}

pub(super) fn resolve_matrix_shelfmark_value(
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
    let placement = read_placement(
        request
            .shelfmark
            .delta
            .get("placement")
            .ok_or_else(|| "Matrix shelfmark lacks placement state".to_owned())?,
    )?;
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

/// Parse a shared link's placement section.
///
/// mer3ly has written only `Pinned` since Ruling 103. An older link's
/// "Anchored" was a hard pin in the live path, so it reads as pinned (Ruling
/// 104); this bare read never passes through `Score::read`'s version gate.
pub(super) fn read_placement(section: &str) -> Result<Vec<HeldPlacement>, String> {
    let mut placement: Vec<HeldPlacement> = serde_json::from_str(section)
        .map_err(|error| format!("invalid placement section: {error}"))?;
    for held in &mut placement {
        if held.hold == sceno::Hold::Anchored {
            held.hold = sceno::Hold::Pinned;
        }
    }
    Ok(placement)
}

pub(super) fn validate_view_delta(
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

pub(super) fn resolve_matrix_axis(
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
