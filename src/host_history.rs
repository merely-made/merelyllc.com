// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! The repository family's source history as a `scenomise.host-dataset/v2`
//! envelope (site canvas plan, Rulings 144-148 and 157).
//!
//! One revision per merged authority checkpoint: identical neighbours merge
//! into the later one, as the sandbox's slider always merged them, and
//! checkpoints whose authority files predate the graph are left out. Each
//! revision discloses the reduced public graph the slider draws: every
//! repository with its name, class, status and push time, and every
//! relationship with its endpoints, kind and provenance.
//!
//! The envelope narrows what counts as change to the site's slider fields
//! (name, class, status, pushed_at; Ruling 145) and relationships to their
//! endpoints and kind (Ruling 146). Its revision identity is the checkpoint's
//! cursor as JSON, the same record string the sandbox cites, so a share link
//! names the checkpoint it was made at.
//!
//! The existing `repository-host-dataset.json` (v1) is unchanged for its
//! consumers. This file is read through mere's shared reader before it is
//! written, and `validate-artifact` compares it byte for byte.

use std::collections::BTreeMap;

use sceno::SourceRef;
use scenograph::SourceBinding;
use scenomise::host_dataset::{
    HOST_DATASET_SCHEMA_V2, HostDatasetRevisionV2, HostDatasetV2, parse_host_history,
};
use scenomise::projection::{
    DisclosedRelationship, ProjectionDataset, ProjectionFieldType, ProjectionOccurrence,
    ProjectionValue, RelationshipProvenance,
};
use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::repository_history::{
    GitAuthorityCheckpointProjection, GitAuthorityCursor, GitAuthorityHistoryProjection,
};

pub const HOST_HISTORY_FILE: &str = "repository-host-history.json";

/// Ruling 157: the history the sandbox fetches on first interaction is at most
/// 128 KiB gzip.
pub const HOST_HISTORY_GZIP_LIMIT: usize = 128 * 1024;

/// The occurrence fields whose change the slider reports (Ruling 145).
pub const COMPARED_FIELDS: [&str; 4] = ["label", "class", "status", "pushed_at"];

/// The relationship parts whose change the slider reports (Ruling 146).
pub const COMPARED_RELATIONSHIP_FIELDS: [&str; 2] = ["endpoints", "kind"];

pub const REPOSITORY_ADAPTER: &str = "mer3ly.repository-graph/v1";
const RELATION_ADAPTER: &str = "mer3ly.repository-relation/v1";
/// A relationship's method is this prefix and the site's edge provenance
/// (`curated` or `derived`), as the v1 envelope words it.
pub const RELATION_METHOD_PREFIX: &str = "mer3ly.repository-relations.";
const FIELDS: [&str; 5] = ["occurrence_id", "label", "class", "status", "pushed_at"];

/// The reduced public graph either authority discloses: a checkpoint's
/// `mer3ly.repo-graph/v1` record or the sandbox's authored specimen. Fields
/// beyond these are not disclosed.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct SiteGraph {
    pub nodes: Vec<SiteNode>,
    pub edges: Vec<SiteEdge>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct SiteNode {
    pub id: String,
    pub name: String,
    pub class: String,
    pub status: String,
    pub pushed_at: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct SiteEdge {
    pub id: String,
    pub source: String,
    pub target: String,
    pub kind: String,
    pub provenance: String,
}

impl SiteGraph {
    /// Read the shared fields of any serializable site graph.
    pub fn of(graph: &impl serde::Serialize) -> Result<Self, String> {
        serde_json::to_value(graph)
            .and_then(serde_json::from_value)
            .map_err(|error| format!("a site graph is not readable: {error}"))
    }

    /// Two graphs are one checkpoint when they hold the same repositories
    /// and relationships, in any order: the sandbox's own merge rule.
    fn same_content(&self, other: &Self) -> bool {
        let sorted = |graph: &Self| {
            let mut nodes = graph.nodes.clone();
            let mut edges = graph.edges.clone();
            nodes.sort();
            edges.sort();
            (nodes, edges)
        };
        sorted(self) == sorted(other)
    }
}

/// The site's repository source, which every revision discloses.
pub fn repository_source() -> SourceBinding {
    SourceBinding {
        authority: "https://merelyllc.com".into(),
        domain: "public-repositories".into(),
        resource: "repository-graph".into(),
    }
}

/// Disclose one site graph at `revision` as a dataset and its relationships.
pub fn disclose_graph(
    graph: &SiteGraph,
    source: &SourceBinding,
    revision: &str,
) -> (ProjectionDataset, Vec<DisclosedRelationship>) {
    let fields = FIELDS
        .into_iter()
        .map(|name| (name.to_owned(), ProjectionFieldType::Text))
        .collect::<BTreeMap<_, _>>();
    let names = graph
        .nodes
        .iter()
        .map(|node| (node.id.as_str(), node.name.as_str()))
        .collect::<BTreeMap<_, _>>();
    let occurrences = graph
        .nodes
        .iter()
        .map(|node| ProjectionOccurrence {
            occurrence_id: node.id.clone(),
            source: SourceRef::new(REPOSITORY_ADAPTER, &node.id),
            values: BTreeMap::from([
                (
                    "occurrence_id".to_owned(),
                    ProjectionValue::Text(node.id.clone()),
                ),
                ("label".to_owned(), ProjectionValue::Text(node.name.clone())),
                (
                    "class".to_owned(),
                    ProjectionValue::Text(node.class.clone()),
                ),
                (
                    "status".to_owned(),
                    ProjectionValue::Text(node.status.clone()),
                ),
                (
                    "pushed_at".to_owned(),
                    ProjectionValue::Text(node.pushed_at.clone()),
                ),
            ]),
        })
        .collect();
    let relationships = graph
        .edges
        .iter()
        .map(|edge| {
            let name = |id: &str| names.get(id).copied().unwrap_or(id).to_owned();
            DisclosedRelationship {
                id: edge.id.clone(),
                from_occurrence: edge.source.clone(),
                to_occurrence: edge.target.clone(),
                kind: edge.kind.clone(),
                label: format!(
                    "{} {} {}",
                    name(&edge.source),
                    edge.kind.replace('_', " "),
                    name(&edge.target)
                ),
                explanation: format!(
                    "A {} relationship in the site's repository authority.",
                    edge.provenance
                ),
                provenance: RelationshipProvenance {
                    source: source.clone(),
                    source_revision: revision.to_owned().into(),
                    method: format!("{RELATION_METHOD_PREFIX}{}", edge.provenance),
                    method_version: 1,
                    provider: "mer3ly authority exporter".into(),
                    evidence: vec![SourceRef::new(RELATION_ADAPTER, &edge.id)],
                },
            }
        })
        .collect();
    (
        ProjectionDataset {
            source: source.clone(),
            revision: revision.to_owned().into(),
            fields,
            occurrences,
        },
        relationships,
    )
}

/// A checkpoint's revision identity: its cursor as JSON, the record string the
/// sandbox cites.
pub fn cursor_revision(cursor: &GitAuthorityCursor) -> Result<String, String> {
    serde_json::to_string(cursor).map_err(|error| format!("a cursor is not serializable: {error}"))
}

/// The available checkpoints, oldest first, with identical neighbours merged
/// into the later one.
pub fn merged_checkpoints(
    history: &GitAuthorityHistoryProjection,
) -> Result<Vec<(GitAuthorityCursor, SiteGraph)>, String> {
    let mut merged: Vec<(GitAuthorityCursor, SiteGraph)> = Vec::new();
    for checkpoint in &history.checkpoints {
        let GitAuthorityCheckpointProjection::Available { cursor, graph } = checkpoint else {
            continue;
        };
        let graph = SiteGraph::of(graph)?;
        match merged.last_mut() {
            Some(last) if last.1.same_content(&graph) => *last = (cursor.clone(), graph),
            _ => merged.push((cursor.clone(), graph)),
        }
    }
    Ok(merged)
}

/// The history envelope.
pub fn repository_host_history(
    history: &GitAuthorityHistoryProjection,
) -> Result<HostDatasetV2, String> {
    let source = repository_source();
    let revisions = merged_checkpoints(history)?
        .into_iter()
        .enumerate()
        .map(|(index, (cursor, graph))| {
            let revision = cursor_revision(&cursor)?;
            let (dataset, relationships) = disclose_graph(&graph, &source, &revision);
            Ok(HostDatasetRevisionV2 {
                sequence: index as u64 + 1,
                revision: revision.into(),
                dataset,
                relationships,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    if revisions.is_empty() {
        return Err("the repository history has no available checkpoint".into());
    }
    Ok(HostDatasetV2 {
        schema: HOST_DATASET_SCHEMA_V2.to_owned(),
        revisions,
        compared_fields: Some(COMPARED_FIELDS.map(str::to_owned).to_vec()),
        compared_relationship_fields: Some(
            COMPARED_RELATIONSHIP_FIELDS.map(str::to_owned).to_vec(),
        ),
    })
}

/// The published bytes: compact JSON, accepted by mere's shared reader.
pub fn repository_host_history_json(
    history: &GitAuthorityHistoryProjection,
) -> Result<String, String> {
    let json = serde_json::to_string(&repository_host_history(history)?)
        .map_err(|error| format!("could not serialize the repository history: {error}"))?;
    parse_host_history(&json)
        .map_err(|error| format!("mere's reader refuses the repository history: {error}"))?;
    Ok(json)
}

/// Exact bytes, as the v1 envelope is checked: the writer is the authority.
pub fn validate_repository_host_history(
    actual: &str,
    history: &GitAuthorityHistoryProjection,
) -> Result<(), String> {
    if actual != repository_host_history_json(history)? {
        return Err(format!(
            "{HOST_HISTORY_FILE} differs from the current public authority history"
        ));
    }
    Ok(())
}

/// A content-versioned URL for the history.
pub fn host_history_href(json: &str) -> String {
    format!(
        "/{HOST_HISTORY_FILE}?v={}",
        &format!("{:x}", Sha256::digest(json.as_bytes()))[..12]
    )
}

#[cfg(test)]
mod tests {
    use scenomise::history::{Change, ComparedFields, Comparison};

    use super::*;
    use crate::repository_history::{REPOSITORY_GRAPH_SCHEMA, RepositoryGraph};

    fn graph(nodes: &[(&str, &str)], edges: &[(&str, &str, &str)]) -> RepositoryGraph {
        serde_json::from_value(serde_json::json!({
            "schema": REPOSITORY_GRAPH_SCHEMA,
            "nodes": nodes.iter().map(|(id, pushed_at)| serde_json::json!({
                "id": id, "name": id.to_uppercase(), "class": "platform",
                "status": "active", "pushed_at": pushed_at,
            })).collect::<Vec<_>>(),
            "edges": edges.iter().map(|(id, source, target)| serde_json::json!({
                "id": id, "source": source, "target": target,
                "kind": "depends_on", "provenance": "derived",
            })).collect::<Vec<_>>(),
        }))
        .unwrap()
    }

    fn checkpoint(commit: &str, graph: RepositoryGraph) -> GitAuthorityCheckpointProjection {
        GitAuthorityCheckpointProjection::Available {
            cursor: GitAuthorityCursor {
                source: "merely-made/merelyllc.com".into(),
                commit: commit.into(),
                committed_at: format!("2026-10-0{}T00:00:00Z", commit.len()),
            },
            graph,
        }
    }

    fn history(
        checkpoints: Vec<GitAuthorityCheckpointProjection>,
    ) -> GitAuthorityHistoryProjection {
        GitAuthorityHistoryProjection {
            schema: "mer3ly.repository-git-history/v1".into(),
            checkpoints,
        }
    }

    #[test]
    fn identical_neighbours_merge_into_the_later_checkpoint() {
        let a = graph(&[("mere", "1"), ("genet", "1")], &[("e", "mere", "genet")]);
        // The same content in another order is the same checkpoint.
        let a_reordered = graph(&[("genet", "1"), ("mere", "1")], &[("e", "mere", "genet")]);
        let b = graph(&[("mere", "2"), ("genet", "1")], &[("e", "mere", "genet")]);
        let projection = history(vec![
            checkpoint("a", a.clone()),
            GitAuthorityCheckpointProjection::Unavailable {
                cursor: GitAuthorityCursor {
                    source: "merely-made/merelyllc.com".into(),
                    commit: "gone".into(),
                    committed_at: "2026-10-01T00:00:00Z".into(),
                },
                reason: "predates the graph".into(),
            },
            checkpoint("aa", a_reordered),
            checkpoint("bbb", b),
            checkpoint("bbbb", a),
        ]);
        let merged = merged_checkpoints(&projection).unwrap();
        let commits = merged
            .iter()
            .map(|(cursor, _)| cursor.commit.as_str())
            .collect::<Vec<_>>();
        assert_eq!(commits, ["aa", "bbb", "bbbb"]);
        // The later checkpoint's own order is kept.
        assert_eq!(merged[0].1.nodes[0].id, "genet");
    }

    #[test]
    fn the_history_reads_through_meres_reader_with_the_sites_narrowed_comparison() {
        let projection = history(vec![
            checkpoint(
                "a",
                graph(
                    &[("mere", "1"), ("genet", "1"), ("hocket", "1")],
                    &[
                        ("mere-genet", "mere", "genet"),
                        ("hocket-mere", "hocket", "mere"),
                    ],
                ),
            ),
            checkpoint(
                "bb",
                graph(
                    &[("mere", "2"), ("genet", "1")],
                    &[("mere-genet", "mere", "genet")],
                ),
            ),
        ]);
        let json = repository_host_history_json(&projection).unwrap();
        let parsed = parse_host_history(&json).unwrap();
        assert_eq!(parsed.revisions.len(), 2);
        assert_eq!(
            parsed.revisions[1].revision.as_str(),
            r#"{"source":"merely-made/merelyllc.com","commit":"bb","committed_at":"2026-10-02T00:00:00Z"}"#
        );
        let comparison = Comparison::of(&parsed);
        assert_eq!(
            comparison.fields,
            ComparedFields::Only(COMPARED_FIELDS.map(str::to_owned).into_iter().collect())
        );
        assert_eq!(
            comparison.relationship_fields,
            ComparedFields::Only(
                COMPARED_RELATIONSHIP_FIELDS
                    .map(str::to_owned)
                    .into_iter()
                    .collect()
            )
        );
        let changes = parsed.changes();
        assert_eq!(changes.occurrence("mere"), Some(Change::Updated));
        assert_eq!(changes.occurrence("genet"), Some(Change::Stable));
        assert_eq!(changes.occurrence("hocket"), Some(Change::Removed));
        // A relationship that left with its endpoint is told by the
        // endpoint's removal (Ruling 147).
        assert!(
            changes
                .relationships
                .iter()
                .all(|entry| entry.change != Change::Removed)
        );
        assert!(validate_repository_host_history(&json, &projection).is_ok());
        assert!(validate_repository_host_history(&format!("{json} "), &projection).is_err());
    }
}
