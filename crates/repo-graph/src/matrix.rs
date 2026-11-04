use super::*;

pub(super) fn matrix_projection_json(input: &str) -> Result<String, String> {
    let request: MatrixProjectionRequest = serde_json::from_str(input)
        .map_err(|error| format!("invalid Matrix projection request: {error}"))?;
    let artifact = matrix_projection(&request)?;
    serde_json::to_string(&artifact)
        .map_err(|error| format!("could not encode Matrix projection: {error}"))
}

pub(super) fn matrix_axis(request: &MatrixAxisRequest) -> Result<(MatrixAxis, GraphInput), String> {
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

pub(super) fn matrix_projection(
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

pub(super) fn matrix_cell_contributors(
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

pub(super) fn matrix_accessible_html(
    rows: &MatrixAxis,
    columns: &MatrixAxis,
    cells: &[MatrixCell],
) -> String {
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

pub(super) fn escape_html(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

pub(super) fn consume_matrix_projection(
    artifact: &MatrixProjectionArtifact,
) -> Result<MatrixReceipt, String> {
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
