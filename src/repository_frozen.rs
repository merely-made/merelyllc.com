// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! The repositories page's first view, frozen at build time (site canvas
//! plan, Rulings 3 and 157).
//!
//! /repos/ opens on three static readings, each made by mere's shared
//! readers rather than a site copy (Ruling 1):
//! - **the scene**: the default reading (every repository and relationship of
//!   the live authority) solved into a scene, read by graphshell-client's
//!   frozen reader. A fold would list as its group (S5); the default reading
//!   has none;
//! - **the matrix**: the sandbox's default matrix, crossed by scenomise's
//!   `derive_matrix` through cartography's `two-reading-matrix` adapter and
//!   frozen as a `FrozenGrid` (S2). The site supplies its own default focus,
//!   Mere (Ruling 147);
//! - **the history**: every merged checkpoint of the v2 history, classified
//!   by scenomise's evaluator against the one before it, under the narrowed
//!   fields the envelope declares (S3; Rulings 145-146).
//!
//! The live sandbox loads on first interaction, as the projection proof's
//! replay does (Ruling 140).

use std::collections::{BTreeMap, HashMap};

use cartography::matrix::{
    Matrix, MatrixCellKind, ReadingAxis, TWO_READING_MATRIX, project_two_reading_matrix,
};
use cartography::reading::default_graph_reading_registry;
use chirograph::ProjectionCaptureV2;
use graphshell_client::frozen::{
    FrozenGrid, FrozenGridCell, FrozenGridHeading, FrozenGridRow, FrozenScene,
};
use mer3ly_repo_graph::projection_capture_with_placement_json;
use sceno::InstanceId;
use scenomise::history::Change;
use scenomise::host_dataset::{HostDatasetV2, parse_host_history};
use scenomise::projection::ProjectionDataset;

use crate::host_history::{
    SiteGraph, disclose_graph, repository_host_history_json, repository_source,
};
use crate::repository_history::{GitAuthorityCursor, GitAuthorityHistoryProjection};
use crate::site::{SiteView, element, txt};

/// How the frozen view opens when no script runs: the existing wording.
pub const NO_SCRIPT_LEAD: &str = "The sandbox requires WebAssembly. Without it, this";

/// How it opens while the script runs and the sandbox waits for a first
/// interaction (Ruling 140's two openings).
pub const SCRIPTED_LEAD: &str =
    "Interact with the graph to load the live sandbox. Until then, this";

/// How it opens when the sandbox could not load.
pub const FAILED_LEAD: &str = "The live sandbox could not load, so this";

/// The default matrix's row focus. cartography never guesses one (Ruling
/// 147); the site's default is Mere, else the first repository.
pub const DEFAULT_MATRIX_FOCUS: &str = "mere";

/// The authored specimen's record, as the sandbox cites it.
pub const SPECIMEN_RECORD: &str =
    r#"{"source":"mer3ly/specimen","commit":"authored","committed_at":"static"}"#;

/// Where the frozen readings are spliced into the rendered page: the site's
/// view tree has no raw-markup node, and the shared readers render markup.
pub const SCENE_MARKER: &str = "@@mer3ly-frozen-scene@@";
pub const GRID_MARKER: &str = "@@mer3ly-frozen-matrix@@";

/// The frozen first view and what it was read from.
#[derive(Clone, Debug)]
pub struct FrozenFirstView {
    pub scene: FrozenScene,
    pub matrix: Matrix,
    pub grid: FrozenGrid,
    /// The published v2 history, as mere's reader parsed it.
    pub history: HostDatasetV2,
    /// Its published bytes.
    pub history_json: String,
}

impl FrozenFirstView {
    /// Read the live authority, the specimen and the checkpoint history.
    pub fn build(
        current: &impl serde::Serialize,
        specimen: &serde_json::Value,
        history: &GitAuthorityHistoryProjection,
    ) -> Result<Self, String> {
        let history_json = repository_host_history_json(history)?;
        let history = parse_host_history(&history_json)
            .map_err(|error| format!("mere's reader refuses the repository history: {error}"))?;
        let scene = frozen_scene(current, history.current().dataset.clone())?;
        let matrix = default_matrix(&history, specimen)?;
        let grid = frozen_grid(&matrix)?;
        Ok(Self {
            scene,
            matrix,
            grid,
            history,
            history_json,
        })
    }

    /// The scene's and the grid's markup, as graphshell-client renders them.
    pub fn scene_html(&self) -> String {
        self.scene.to_html("repos-frozen-scene")
    }

    pub fn grid_html(&self) -> String {
        self.grid.to_html("repos-frozen-matrix")
    }

    /// The opening sentence after the lead.
    pub fn summary(&self) -> String {
        format!(
            " is its default reading, frozen at build time: {} repositories and {} relationships, the matrix it opens on, and {} checkpoints of source history as text. The semantic repository index remains available below.",
            self.scene.instances.len()
                + self
                    .scene
                    .folds
                    .iter()
                    .map(|fold| fold.members.len())
                    .sum::<usize>(),
            self.scene.relations.len(),
            self.history.revisions.len(),
        )
    }

    /// The history as accessible text: one item per merged checkpoint,
    /// worded from the evaluator's classification.
    pub fn history_view(&self) -> SiteView {
        let items = (0..self.history.revisions.len())
            .map(|index| self.history_item(index))
            .collect();
        element(
            "ol",
            &[
                ("class", "graph-sandbox-frozen-history"),
                ("aria-label", "Source history, oldest first"),
            ],
            items,
        )
    }

    fn history_item(&self, index: usize) -> SiteView {
        let entry = &self.history.revisions[index];
        let cursor: Option<GitAuthorityCursor> = serde_json::from_str(entry.revision.as_str()).ok();
        let (date, label) = match &cursor {
            Some(cursor) => (
                cursor.committed_at.clone(),
                format!(
                    " · {} ({})",
                    &cursor.commit[..cursor.commit.len().min(7)],
                    cursor.source
                ),
            ),
            None => (String::new(), entry.revision.as_str().to_owned()),
        };
        let changes = self
            .history
            .changes_at(index)
            .expect("an index within the history");
        let sentence = if index == 0 {
            format!(
                ": the first checkpoint, {} and {}.",
                count(
                    entry.dataset.occurrences.len(),
                    "repository",
                    "repositories"
                ),
                count(entry.relationships.len(), "relationship", "relationships"),
            )
        } else {
            let occurrences = changes.counts();
            let of = |change| occurrences.get(&change).copied().unwrap_or(0);
            let mut relationships = BTreeMap::<Change, usize>::new();
            for relationship in &changes.relationships {
                *relationships.entry(relationship.change).or_default() += 1;
            }
            let related = [Change::Added, Change::Updated, Change::Removed]
                .into_iter()
                .filter_map(|change| {
                    relationships
                        .get(&change)
                        .map(|n| format!("{n} {}", change_word(change)))
                })
                .collect::<Vec<_>>();
            let mut sentence = format!(
                ": {} added, {} updated, {} stable and {} removed repositories; {}.",
                of(Change::Added),
                of(Change::Updated),
                of(Change::Stable),
                of(Change::Removed),
                if related.is_empty() {
                    "relationships unchanged".to_owned()
                } else {
                    format!("relationships {}", related.join(", "))
                },
            );
            let previous = &self.history.revisions[index - 1].dataset;
            for (change, word) in [(Change::Added, "Added"), (Change::Removed, "Removed")] {
                let names = changes
                    .occurrences
                    .iter()
                    .filter(|entry| entry.change == change)
                    .map(|change| label_of(&change.occurrence_id, &[&entry.dataset, previous]))
                    .collect::<Vec<_>>();
                if !names.is_empty() {
                    sentence.push_str(&format!(" {word}: {}.", names.join(", ")));
                }
            }
            sentence
        };
        element(
            "li",
            &[("data-sandbox-history-revision", &entry.sequence.to_string())],
            vec![
                element(
                    "time",
                    &[("datetime", date.as_str())],
                    vec![txt(date.get(..10).unwrap_or(&date).to_owned())],
                ),
                txt(format!("{label}{sentence}")),
            ],
        )
    }
}

fn label_of(id: &str, datasets: &[&ProjectionDataset]) -> String {
    datasets
        .iter()
        .flat_map(|dataset| &dataset.occurrences)
        .find(|occurrence| occurrence.occurrence_id == id)
        .and_then(|occurrence| occurrence.values.get("label"))
        .and_then(|value| value.text())
        .unwrap_or(id)
        .to_owned()
}

fn change_word(change: Change) -> &'static str {
    match change {
        Change::Added => "added",
        Change::Updated => "updated",
        Change::Stable => "stable",
        Change::Removed => "removed",
    }
}

fn count(n: usize, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

/// The default reading solved into a scene by the graph crate's portable path
/// and read by graphshell-client's frozen reader, named from the history's
/// current disclosure.
fn frozen_scene(
    current: &impl serde::Serialize,
    disclosed: ProjectionDataset,
) -> Result<FrozenScene, String> {
    let graph = serde_json::to_string(current)
        .map_err(|error| format!("the live authority is not serializable: {error}"))?;
    let capture = projection_capture_with_placement_json(&graph, "{}")?;
    let capture = ProjectionCaptureV2::decode(capture.as_bytes())
        .map_err(|error| format!("the default reading's capture is invalid: {error}"))?;
    let scene = capture.scene;
    let mut names = HashMap::new();
    let mut details = HashMap::new();
    for (index, item) in scene.tables.items.iter().enumerate() {
        let Some(item) = item else { continue };
        let Some(Some(source)) = scene.tables.sources.get(item.source.0 as usize) else {
            continue;
        };
        let occurrence = disclosed
            .occurrences
            .iter()
            .find(|occurrence| occurrence.source == *source)
            .ok_or_else(|| format!("the live disclosure does not name {}", source.id))?;
        let text = |field: &str| {
            occurrence
                .values
                .get(field)
                .and_then(|value| value.text())
                .unwrap_or_default()
                .to_owned()
        };
        let instance = InstanceId(index as u32);
        names.insert(instance, text("label"));
        details.insert(instance, format!("{}, {}", text("class"), text("status")));
    }
    let frozen = FrozenScene::freeze_snapshot_with_details(
        &scene,
        "The repository family, default reading",
        &names,
        &details,
    );
    if frozen.unnamed != 0 {
        return Err(format!(
            "{} repositories in the default reading have no name",
            frozen.unnamed
        ));
    }
    Ok(frozen)
}

/// The sandbox's default matrix: Mere's neighbors in the live authority's
/// current checkpoint as rows, the authored specimen's changes as columns.
fn default_matrix(history: &HostDatasetV2, specimen: &serde_json::Value) -> Result<Matrix, String> {
    let registry = default_graph_reading_registry();
    let reading = |id: &str| {
        registry
            .resolve(id)
            .ok_or_else(|| format!("mere's registry has no {id} reading"))
    };
    let current = history.current();
    let focus = if current
        .dataset
        .occurrences
        .iter()
        .any(|occurrence| occurrence.occurrence_id == DEFAULT_MATRIX_FOCUS)
    {
        DEFAULT_MATRIX_FOCUS
    } else {
        current
            .dataset
            .occurrences
            .first()
            .map(|occurrence| occurrence.occurrence_id.as_str())
            .ok_or("the live checkpoint has no repositories")?
    };
    let specimen = SiteGraph::of(specimen)?;
    let (specimen_dataset, specimen_relationships) =
        disclose_graph(&specimen, &specimen_source(), SPECIMEN_RECORD);
    let rows = ReadingAxis {
        authority: "live",
        record: current.revision.as_str(),
        profile: reading("neighbors")?,
        focus: Some(focus),
        dataset: &current.dataset,
        relationships: &current.relationships,
        previous: None,
        label_field: "label",
    };
    let columns = ReadingAxis {
        authority: "specimen",
        record: SPECIMEN_RECORD,
        profile: reading("changes")?,
        focus: None,
        dataset: &specimen_dataset,
        relationships: &specimen_relationships,
        previous: None,
        label_field: "label",
    };
    // The profile the matrix is offered as in the sandbox's reading cycle.
    reading(TWO_READING_MATRIX)?;
    project_two_reading_matrix(&rows, &columns)
        .map_err(|error| format!("the default matrix is refused: {error}"))
}

/// The specimen discloses under its own source, as its own authority.
fn specimen_source() -> scenograph::SourceBinding {
    let mut source = repository_source();
    source.resource = "specimen".into();
    source
}

/// The matrix frozen as a two-axis table, each heading and cell carrying the
/// scene instance `Matrix::scene` draws it as.
fn frozen_grid(matrix: &Matrix) -> Result<FrozenGrid, String> {
    let columns = matrix
        .columns
        .sources
        .iter()
        .enumerate()
        .map(|(index, source)| FrozenGridHeading {
            instance: Some(matrix.column_instance(index)),
            source: source.source.clone(),
            name: source.label.clone(),
        })
        .collect();
    let width = matrix.columns.sources.len();
    let rows = matrix
        .rows
        .sources
        .iter()
        .enumerate()
        .map(|(row, source)| FrozenGridRow {
            heading: FrozenGridHeading {
                instance: Some(matrix.row_instance(row)),
                source: source.source.clone(),
                name: source.label.clone(),
            },
            cells: (0..width)
                .map(|column| {
                    let index = row * width + column;
                    let cell = &matrix.cells[index];
                    FrozenGridCell {
                        instance: Some(matrix.cell_instance(index)),
                        source: cell.source.clone(),
                        text: cell.value.clone(),
                        description: cell.description.clone(),
                    }
                })
                .collect(),
        })
        .collect();
    let focus = matrix.rows.focus.as_deref().unwrap_or("the focus");
    let focus = matrix
        .rows
        .sources
        .iter()
        .find(|source| source.source.id == focus)
        .map_or(focus, |source| source.label.as_str());
    FrozenGrid::new(
        format!(
            "Neighbors of {focus} in the live authority, by changes in the authored specimen: {} relation cells",
            matrix
                .cells
                .iter()
                .filter(|cell| cell.kind == MatrixCellKind::Relation)
                .count()
        ),
        "rows / columns",
        columns,
        rows,
    )
    .map_err(|error| format!("the default matrix does not freeze: {error:?}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pages::repositories;
    use crate::repositories::PublicSiteData;
    use crate::repository_history::RepositoryGraph;

    fn root() -> &'static std::path::Path {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
    }

    fn page() -> repositories::RepositoriesPage {
        repositories::page(root()).expect("repository page")
    }

    #[test]
    fn the_frozen_scene_is_the_default_reading_named_from_the_disclosure() {
        let data = PublicSiteData::load(root()).unwrap();
        let graph = RepositoryGraph::from_parts(
            &data.authority.repositories,
            &data.authority.relations,
            &data.metadata,
        )
        .unwrap();
        let frozen = page().frozen;
        assert_eq!(frozen.scene.instances.len(), graph.nodes.len());
        assert_eq!(frozen.scene.relations.len(), graph.edges.len());
        assert_eq!(frozen.scene.unnamed, 0);
        // The default reading folds nothing; a fold would list as its group.
        assert!(frozen.scene.folds.is_empty());
        for node in &graph.nodes {
            let instance = frozen
                .scene
                .instances
                .iter()
                .find(|instance| instance.source.id == node.id)
                .unwrap();
            assert_eq!(instance.name, node.name);
            assert_eq!(
                instance.detail.as_deref(),
                Some(format!("{}, {}", node.class.slug(), node.status.slug()).as_str())
            );
        }
    }

    #[test]
    fn the_default_matrix_crosses_meres_neighbors_with_the_specimens_changes() {
        let frozen = page().frozen;
        let matrix = &frozen.matrix;
        assert_eq!(matrix.rows.reading, "neighbors");
        assert_eq!(matrix.rows.focus.as_deref(), Some(DEFAULT_MATRIX_FOCUS));
        assert_eq!(matrix.rows.authority, "live");
        assert_eq!(
            matrix.rows.record,
            frozen.history.current().revision.as_str()
        );
        assert_eq!(matrix.columns.reading, "changes");
        assert_eq!(matrix.columns.record, SPECIMEN_RECORD);
        let specimen = repositories::specimen();
        let specimen_ids = specimen["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .map(|node| node["id"].as_str().unwrap())
            .collect::<Vec<_>>();
        let columns = matrix
            .columns
            .sources
            .iter()
            .map(|source| source.source.id.as_str())
            .collect::<Vec<_>>();
        assert_eq!(columns, specimen_ids);
        assert!(
            matrix
                .rows
                .sources
                .iter()
                .any(|source| source.source.id == DEFAULT_MATRIX_FOCUS)
        );
        assert_eq!(
            matrix.cells.len(),
            matrix.rows.sources.len() * matrix.columns.sources.len()
        );
        // Mere and Turnstone are on both axes; the specimen's two
        // relationships into Mere and its host relationship meet live rows.
        let count = |kind| matrix.cells.iter().filter(|cell| cell.kind == kind).count();
        assert_eq!(count(MatrixCellKind::IdentityMatch), 2);
        assert!(count(MatrixCellKind::Relation) > 0);
        // The grid freezes every cell with its sentence, row by row.
        assert_eq!(frozen.grid.rows.len(), matrix.rows.sources.len());
        for (row, frozen_row) in frozen.grid.rows.iter().enumerate() {
            for (column, cell) in frozen_row.cells.iter().enumerate() {
                let source = matrix.cell(row, column).unwrap();
                assert_eq!(cell.text, source.value);
                assert_eq!(cell.description, source.description);
            }
        }
    }

    #[test]
    fn without_mere_the_matrix_focuses_the_first_repository() {
        let mut history = page().frozen.history;
        let current = history.revisions.last_mut().unwrap();
        current
            .dataset
            .occurrences
            .retain(|occurrence| occurrence.occurrence_id != DEFAULT_MATRIX_FOCUS);
        current.relationships.retain(|relationship| {
            relationship.from_occurrence != DEFAULT_MATRIX_FOCUS
                && relationship.to_occurrence != DEFAULT_MATRIX_FOCUS
        });
        let first = current.dataset.occurrences[0].occurrence_id.clone();
        let matrix = default_matrix(&history, &repositories::specimen()).unwrap();
        assert_eq!(matrix.rows.focus.as_deref(), Some(first.as_str()));
    }

    #[test]
    fn the_page_carries_both_openings_and_the_shared_readers_markup() {
        let page = page();
        assert!(page.html.contains(&format!(
            "<span data-sandbox-reading-lead=\"\" data-scripted-lead=\"{SCRIPTED_LEAD}\" data-failed-lead=\"{FAILED_LEAD}\">{NO_SCRIPT_LEAD}</span> is its default reading, frozen at build time:"
        )));
        assert!(page.html.contains(&page.frozen.scene_html()));
        assert!(page.html.contains(&page.frozen.grid_html()));
        assert!(!page.html.contains(SCENE_MARKER));
        assert!(!page.html.contains(GRID_MARKER));
        assert_eq!(
            page.html.matches("data-sandbox-history-revision=").count(),
            page.frozen.history.revisions.len()
        );
        // The frozen view comes before the hidden live interface.
        let frozen = page.html.find("data-sandbox-frozen").unwrap();
        let interface = page.html.find("data-sandbox-interface").unwrap();
        assert!(frozen < interface);
    }
}
