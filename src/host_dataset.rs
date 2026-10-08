// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! Build-host disclosure of the site's reconciled public repository authority.
//!
//! The envelope follows S1's `scenomise.host-dataset/v1` boundary. Its dataset
//! and relationships use the published scene-family types at the site's Mere
//! pin. This writer neither parses host input nor changes stored projections.

use std::collections::BTreeMap;

use sceno::SourceRef;
use scenograph::SourceBinding;
use scenomise::projection::{
    DisclosedRelationship, ProjectionDataset, ProjectionFieldType, ProjectionOccurrence,
    ProjectionValue, RelationshipProvenance,
};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::repositories::{Authority, PublicMetadataCache, reconcile_live_github_repositories};
use crate::repository_history::RepositoryGraph;

pub const HOST_DATASET_FILE: &str = "repository-host-dataset.json";
const HOST_DATASET_SCHEMA: &str = "scenomise.host-dataset/v1";
const REPOSITORY_ADAPTER: &str = "mer3ly.repository-graph/v1";
const RELATION_ADAPTER: &str = "mer3ly.repository-relation/v1";
const RELATIONSHIP_SCOPE: &str = "Published public repository-family relationships from the site authority; excludes a resolved Cargo dependency closure.";

/// Serialization only: the shared S1 reader owns this envelope's validation.
#[derive(Serialize)]
struct ExportEnvelope {
    schema: &'static str,
    dataset: ProjectionDataset,
    relationships: Vec<DisclosedRelationship>,
    revisions: Vec<ExportRevision>,
}

#[derive(Serialize)]
struct ExportRevision {
    sequence: u64,
    revision: String,
}

/// Export one complete current disclosure, with a content-derived public
/// revision. No previous revision is claimed until historical disclosures are
/// exported; this sequence contains exactly the current published material.
pub fn repository_host_dataset_json(
    authority: &Authority,
    metadata: &PublicMetadataCache,
) -> Result<String, String> {
    serde_json::to_string_pretty(&export(authority, metadata)?)
        .map_err(|error| format!("could not serialize repository host dataset: {error}"))
}

/// Artifact integrity uses the authority writer, not a second host-input
/// parser. Exact bytes also preserve canonical ordering and every explanation.
pub fn validate_repository_host_dataset(
    actual: &str,
    authority: &Authority,
    metadata: &PublicMetadataCache,
) -> Result<(), String> {
    let expected = repository_host_dataset_json(authority, metadata)?;
    if actual != expected {
        return Err("repository host dataset differs from the current public authority".into());
    }
    Ok(())
}

fn export(authority: &Authority, metadata: &PublicMetadataCache) -> Result<ExportEnvelope, String> {
    let (repositories, relations) =
        reconcile_live_github_repositories(&authority.repositories, &authority.relations, metadata);
    let mut graph = RepositoryGraph::from_parts(&repositories, &relations, metadata)?;
    // The exporter authors presentation order by canonical project id. This
    // is neither historical manifest order nor a dependency rank or metric.
    graph.nodes.sort_by(|left, right| left.id.cmp(&right.id));
    graph.edges.sort_by(|left, right| left.id.cmp(&right.id));
    let repository_by_id = repositories
        .repository
        .iter()
        .map(|repository| (repository.id.as_str(), repository))
        .collect::<BTreeMap<_, _>>();
    let relation_by_id = relations
        .relation
        .iter()
        .map(|relation| (relation.id.as_str(), relation))
        .collect::<BTreeMap<_, _>>();
    let source = SourceBinding {
        authority: "https://merelyllc.com".into(),
        domain: "public-repositories".into(),
        resource: "repository-graph".into(),
    };
    let text_fields = [
        "occurrence_id",
        "label",
        "summary",
        "class",
        "status",
        "pushed_at",
        "github_url",
        "project_url",
        "homepage",
        "license",
        "relationship_scope",
    ];
    let mut fields = text_fields
        .into_iter()
        .map(|name| (name.to_owned(), ProjectionFieldType::Text))
        .collect::<BTreeMap<_, _>>();
    fields.insert("order".into(), ProjectionFieldType::Number);
    let occurrences = graph
        .nodes
        .iter()
        .enumerate()
        .map(|(order, node)| {
            let repository = repository_by_id[node.id.as_str()];
            let mut values = BTreeMap::from([
                (
                    "occurrence_id".into(),
                    ProjectionValue::Text(node.id.clone()),
                ),
                ("label".into(), ProjectionValue::Text(node.name.clone())),
                (
                    "summary".into(),
                    ProjectionValue::Text(repository.summary.clone()),
                ),
                (
                    "class".into(),
                    ProjectionValue::Text(node.class.slug().into()),
                ),
                (
                    "status".into(),
                    ProjectionValue::Text(node.status.slug().into()),
                ),
                (
                    "pushed_at".into(),
                    ProjectionValue::Text(node.pushed_at.clone()),
                ),
                (
                    "github_url".into(),
                    ProjectionValue::Text(format!("https://github.com/{}", repository.github_slug)),
                ),
                (
                    "project_url".into(),
                    ProjectionValue::Text(format!("https://merelyllc.com/projects/{}/", node.id)),
                ),
                (
                    "homepage".into(),
                    ProjectionValue::Text(repository.homepage.clone()),
                ),
                (
                    "license".into(),
                    ProjectionValue::Text(repository.license.clone()),
                ),
                (
                    "relationship_scope".into(),
                    ProjectionValue::Text(RELATIONSHIP_SCOPE.into()),
                ),
            ]);
            values.insert("order".into(), ProjectionValue::Number(order as f64));
            ProjectionOccurrence {
                occurrence_id: node.id.clone(),
                source: SourceRef::new(REPOSITORY_ADAPTER, &node.id),
                values,
            }
        })
        .collect::<Vec<_>>();
    let mut relationships = Vec::with_capacity(graph.edges.len());
    for edge in &graph.edges {
        let relation = relation_by_id[edge.id.as_str()];
        if edge.source == edge.target || relation.evidence.trim().is_empty() {
            return Err(format!(
                "repository relationship {} has no distinct endpoints or evidence",
                edge.id
            ));
        }
        let kind = serde_json::to_value(edge.kind)
            .map_err(|error| format!("could not serialize relation kind: {error}"))?;
        let kind = kind.as_str().ok_or("relation kind is not a string")?;
        relationships.push(DisclosedRelationship {
            id: edge.id.clone(),
            from_occurrence: edge.source.clone(),
            to_occurrence: edge.target.clone(),
            kind: kind.into(),
            label: format!(
                "{} {} {}",
                repository_by_id[edge.source.as_str()].name,
                relation.kind.label(),
                repository_by_id[edge.target.as_str()].name
            ),
            explanation: format!(
                "{} Verification recorded on {} ({}).",
                relation.evidence,
                relation.verified_on,
                relation.provenance.label()
            ),
            provenance: RelationshipProvenance {
                source: source.clone(),
                // Filled after the complete public disclosure has been hashed.
                source_revision: "".into(),
                method: format!(
                    "mer3ly.repository-relations.{}",
                    relation.provenance.label()
                ),
                method_version: 1,
                provider: "mer3ly authority exporter".into(),
                evidence: vec![SourceRef::new(RELATION_ADAPTER, &edge.id)],
            },
        });
    }
    // Hash the complete, canonically ordered disclosure before inserting its
    // self-reference. Private migration fields and refresh timestamps are not
    // part of this material and cannot change this identity.
    let material = serde_json::to_vec(&(
        HOST_DATASET_SCHEMA,
        &source,
        &fields,
        &occurrences,
        &relationships,
    ))
    .map_err(|error| format!("could not canonicalize repository host dataset: {error}"))?;
    let revision = format!("sha256:{:x}", Sha256::digest(material));
    for relationship in &mut relationships {
        relationship.provenance.source_revision = revision.clone().into();
    }
    Ok(ExportEnvelope {
        schema: HOST_DATASET_SCHEMA,
        dataset: ProjectionDataset {
            source,
            revision: revision.clone().into(),
            fields,
            occurrences,
        },
        relationships,
        revisions: vec![ExportRevision {
            sequence: 1,
            revision,
        }],
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repositories::PublicSiteData;

    fn data() -> PublicSiteData {
        PublicSiteData::load(env!("CARGO_MANIFEST_DIR")).expect("validated public site data")
    }

    #[test]
    fn disclosure_preserves_live_authority_occurrences_and_relationships() {
        let data = data();
        let exported = export(&data.authority, &data.metadata).unwrap();
        let expected = RepositoryGraph::from_parts(
            &data.authority.repositories,
            &data.authority.relations,
            &data.metadata,
        )
        .unwrap();
        let ids = exported
            .dataset
            .occurrences
            .iter()
            .map(|item| item.occurrence_id.as_str())
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(
            ids,
            expected.nodes.iter().map(|node| node.id.as_str()).collect()
        );
        assert!(!ids.contains("hocket"));
        assert_eq!(exported.relationships.len(), expected.edges.len());
        for edge in &expected.edges {
            let relation = exported
                .relationships
                .iter()
                .find(|relation| relation.id == edge.id)
                .unwrap();
            assert_eq!(relation.from_occurrence, edge.source);
            assert_eq!(relation.to_occurrence, edge.target);
            assert_eq!(
                relation.kind,
                serde_json::to_value(edge.kind).unwrap().as_str().unwrap()
            );
            assert_eq!(relation.provenance.source, exported.dataset.source);
            assert_eq!(
                relation.provenance.source_revision,
                exported.dataset.revision
            );
            let original = data
                .authority
                .relations
                .relation
                .iter()
                .find(|record| record.id == edge.id)
                .unwrap();
            assert!(relation.explanation.contains(&original.evidence));
            assert!(relation.explanation.contains(&original.verified_on));
            assert_eq!(
                relation.provenance.evidence,
                [SourceRef::new(RELATION_ADAPTER, &edge.id)]
            );
        }
        for (index, occurrence) in exported.dataset.occurrences.iter().enumerate() {
            let original = data
                .authority
                .repositories
                .repository
                .iter()
                .find(|record| record.id == occurrence.occurrence_id)
                .unwrap();
            assert_eq!(
                occurrence.source,
                SourceRef::new(REPOSITORY_ADAPTER, &original.id)
            );
            assert_eq!(
                occurrence.values["label"],
                ProjectionValue::Text(original.name.clone())
            );
            assert_eq!(
                occurrence.values["summary"],
                ProjectionValue::Text(original.summary.clone())
            );
            assert_eq!(
                occurrence.values["order"],
                ProjectionValue::Number(index as f64)
            );
            assert_eq!(
                occurrence.values["relationship_scope"],
                ProjectionValue::Text(RELATIONSHIP_SCOPE.into())
            );
        }
        assert_eq!(exported.revisions.len(), 1);
        assert_eq!(
            exported.revisions[0].revision,
            exported.dataset.revision.as_str()
        );
    }

    #[test]
    fn order_and_operational_metadata_do_not_change_the_disclosure() {
        let mut data = data();
        let expected = repository_host_dataset_json(&data.authority, &data.metadata).unwrap();
        data.authority.repositories.repository.reverse();
        data.authority.relations.relation.reverse();
        data.metadata.repository.reverse();
        data.metadata.generated_at_utc = "2099-01-01T00:00:00Z".into();
        data.metadata.event.clear();
        data.authority.migration.inventory_receipt = "private operational change".into();
        assert_eq!(
            repository_host_dataset_json(&data.authority, &data.metadata).unwrap(),
            expected
        );
    }

    #[test]
    fn changed_live_roster_is_reconciled_before_export() {
        let mut data = data();
        let mut added = data.metadata.repository[0].clone();
        added.id = "receipt-added".into();
        added.github_slug = "merely-made/receipt-added".into();
        added.name = "Receipt addition".into();
        added.description = "Public metadata without an editorial entry.".into();
        data.metadata.repository.push(added);
        data.metadata
            .repository
            .retain(|repository| repository.id != "mere");
        let exported = export(&data.authority, &data.metadata).unwrap();
        assert!(
            exported
                .dataset
                .occurrences
                .iter()
                .all(|item| item.occurrence_id != "mere")
        );
        assert!(
            exported
                .relationships
                .iter()
                .all(|relationship| relationship.from_occurrence != "mere"
                    && relationship.to_occurrence != "mere")
        );
        let added = exported
            .dataset
            .occurrences
            .iter()
            .find(|item| item.occurrence_id == "receipt-added")
            .unwrap();
        assert_eq!(
            added.values["label"],
            ProjectionValue::Text("Receipt addition".into())
        );
        assert_eq!(
            added.values["summary"],
            ProjectionValue::Text("Public metadata without an editorial entry.".into())
        );
        assert!(
            exported
                .dataset
                .occurrences
                .iter()
                .any(|item| item.occurrence_id == "turnstone")
        );
    }

    #[test]
    fn published_content_and_evidence_change_the_revision() {
        let mut data = data();
        let first = export(&data.authority, &data.metadata)
            .unwrap()
            .dataset
            .revision;
        data.authority.repositories.repository[0]
            .summary
            .push_str(" An additional public fact.");
        let second = export(&data.authority, &data.metadata)
            .unwrap()
            .dataset
            .revision;
        assert_ne!(first, second);
        data.authority.relations.relation[0]
            .evidence
            .push_str(" Further public evidence.");
        assert_ne!(
            second,
            export(&data.authority, &data.metadata)
                .unwrap()
                .dataset
                .revision
        );
    }

    #[test]
    fn artifact_validation_refuses_a_changed_endpoint_or_revision() {
        let data = data();
        let json = repository_host_dataset_json(&data.authority, &data.metadata).unwrap();
        validate_repository_host_dataset(&json, &data.authority, &data.metadata).unwrap();
        for pointer in ["/relationships/0/to_occurrence", "/dataset/revision"] {
            let mut changed: serde_json::Value = serde_json::from_str(&json).unwrap();
            *changed.pointer_mut(pointer).unwrap() = "missing".into();
            assert!(
                validate_repository_host_dataset(
                    &serde_json::to_string_pretty(&changed).unwrap(),
                    &data.authority,
                    &data.metadata
                )
                .is_err()
            );
        }
    }
}
