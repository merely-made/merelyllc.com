use super::*;

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
