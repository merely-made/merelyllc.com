use super::*;

#[derive(Clone, Debug, Deserialize)]
pub(super) struct ReadingRequest {
    pub(super) reading: String,
    pub(super) current: GraphInput,
    #[serde(default)]
    pub(super) previous: Option<GraphInput>,
    #[serde(default)]
    pub(super) focus: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) struct MatrixAxisRequest {
    pub(super) dataset: String,
    pub(super) record: String,
    pub(super) reading: String,
    pub(super) current: GraphInput,
    #[serde(default)]
    pub(super) previous: Option<GraphInput>,
    #[serde(default)]
    pub(super) focus: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) struct MatrixProjectionRequest {
    pub(super) rows: MatrixAxisRequest,
    pub(super) columns: MatrixAxisRequest,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) struct MatrixAxisSource {
    pub(super) source: SourceRef,
    pub(super) name: String,
    pub(super) class: String,
    pub(super) status: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) struct MatrixAxis {
    pub(super) dataset: String,
    pub(super) record: String,
    pub(super) reading: String,
    pub(super) focus: Option<String>,
    pub(super) authority_sha256: String,
    pub(super) generation: String,
    pub(super) sources: Vec<MatrixAxisSource>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum MatrixCellKind {
    Relation,
    IdentityMatch,
    Absence,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) struct MatrixContributor {
    pub(super) authority: String,
    pub(super) source: SourceRef,
    pub(super) provenance: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) struct MatrixCell {
    pub(super) instance: InstanceId,
    pub(super) row: SourceRef,
    pub(super) column: SourceRef,
    pub(super) source: SourceRef,
    pub(super) kind: MatrixCellKind,
    pub(super) value: String,
    pub(super) description: String,
    pub(super) contributors: Vec<MatrixContributor>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) struct MatrixProjectionArtifact {
    pub(super) schema: String,
    pub(super) rows: MatrixAxis,
    pub(super) columns: MatrixAxis,
    pub(super) cells: Vec<MatrixCell>,
    pub(super) capture: ProjectionCaptureV1,
    pub(super) accessible_html: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub(super) struct MatrixReceipt {
    pub(super) schema: String,
    pub(super) row_sources: usize,
    pub(super) column_sources: usize,
    pub(super) cells: usize,
    pub(super) relation_cells: usize,
    pub(super) scene_instances: usize,
    pub(super) capture_bytes: usize,
    pub(super) accessible_table: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub(super) struct ProjectedInstanceAddress {
    pub(super) view: String,
    pub(super) source: SourceRef,
    pub(super) facet: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub(super) struct MatrixInstanceDelta {
    pub(super) instance: ProjectedInstanceAddress,
    pub(super) visible: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) struct ComposedShelfmarkRequest {
    pub(super) matrix: MatrixProjectionRequest,
    pub(super) spatial: SpatialShelfmarkRequest,
    pub(super) selection: CoordinatedSelection,
    #[serde(default)]
    pub(super) instances: Vec<MatrixInstanceDelta>,
    #[serde(default)]
    pub(super) placement: Vec<HeldPlacement>,
    #[serde(default)]
    pub(super) motion: Option<String>,
    #[serde(default)]
    pub(super) backdrop: Option<BackdropDelta>,
    #[serde(default)]
    pub(super) facets: Vec<ProjectedInstanceAddress>,
    #[serde(default)]
    pub(super) camera: Option<CameraDelta>,
    #[serde(default)]
    pub(super) carried_delta: BTreeMap<String, String>,
}

/// The adjacent spatial projection belongs beside a Matrix citation, rather
/// than being inferred from the Matrix axes.  It cites the same authority
/// shape but keeps its reading and arrangement as an independently checkable
/// Shelfmark input.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) struct SpatialShelfmarkRequest {
    pub(super) dataset: String,
    pub(super) record: String,
    pub(super) reading: String,
    pub(super) current: GraphInput,
    #[serde(default)]
    pub(super) previous: Option<GraphInput>,
    #[serde(default)]
    pub(super) focus: Option<String>,
    pub(super) arrangement: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) struct BackdropDelta {
    pub(super) kind: String,
    pub(super) collidable: bool,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub(super) struct CameraDelta {
    pub(super) x: f32,
    pub(super) y: f32,
    pub(super) zoom: f32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) struct ResolvedMatrixDataset {
    pub(super) current: GraphInput,
    #[serde(default)]
    pub(super) previous: Option<GraphInput>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) struct MatrixShelfmarkResolutionRequest {
    pub(super) shelfmark: ShelfmarkV1,
    pub(super) datasets: BTreeMap<String, ResolvedMatrixDataset>,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub(super) struct MatrixShelfmarkReceipt {
    pub(super) matrix: MatrixReceipt,
    pub(super) input_generations: BTreeMap<String, String>,
    pub(super) selection_resolution: SelectionResolution,
    pub(super) honored_instance_deltas: usize,
    pub(super) honored_placements: usize,
    pub(super) honored_facets: usize,
    pub(super) camera: CameraDelta,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) struct MatrixAuthorityRecord {
    pub(super) dataset: String,
    pub(super) record: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(super) struct MatrixReadingParameters {
    #[serde(default)]
    pub(super) focus: Option<String>,
}
