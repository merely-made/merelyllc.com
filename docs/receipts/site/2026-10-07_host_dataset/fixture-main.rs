// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

use std::collections::{BTreeMap, BTreeSet};
use std::{env, fs};

use sceno::Size2;
use scenograph::{ProjectionInputBinding, RevisionEvidence};
use scenomise::host_dataset::{HostDatasetError, parse_host_dataset};
use scenomise::projection::{
    AUTHORED_ORDER_FACET, EXPLAINED_RELATIONSHIPS_FACET, ItemSizes, OCCURRENCE_LABELS_FACET,
    ProjectionCompiler, ProjectionValue, RelationshipDataset, RelationshipSnapshot,
    relationship_recipe,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args().skip(1);
    let dataset_path = args.next().ok_or("expected DATASET_JSON REPOS_HTML")?;
    let authority_path = args.next().ok_or("expected DATASET_JSON REPOS_HTML")?;
    if args.next().is_some() {
        return Err("expected DATASET_JSON REPOS_HTML".into());
    }
    let input = fs::read_to_string(&dataset_path)?;
    let envelope = parse_host_dataset(&input)?;
    let html = fs::read_to_string(&authority_path)?;
    let marker = "<script id=\"repository-graph-data\" type=\"application/json\">";
    let (_, remainder) = html
        .split_once(marker)
        .ok_or("repository authority is absent")?;
    let (authority, _) = remainder
        .split_once("</script>")
        .ok_or("repository authority is incomplete")?;
    let authority: Value = serde_json::from_str(authority)?;
    let nodes = authority["nodes"]
        .as_array()
        .ok_or("authority nodes are absent")?;
    let edges = authority["edges"]
        .as_array()
        .ok_or("authority edges are absent")?;
    let ids = envelope
        .dataset
        .occurrences
        .iter()
        .map(|occurrence| occurrence.occurrence_id.as_str())
        .collect::<BTreeSet<_>>();
    let authority_ids = nodes
        .iter()
        .map(|node| node["id"].as_str().ok_or("invalid authority id"))
        .collect::<Result<BTreeSet<_>, _>>()?;
    if ids != authority_ids || envelope.dataset.occurrences.len() != nodes.len() {
        return Err("exported occurrence identities differ from the authority".into());
    }
    for node in nodes {
        let id = node["id"].as_str().ok_or("invalid authority id")?;
        let occurrence = envelope
            .dataset
            .occurrences
            .iter()
            .find(|occurrence| occurrence.occurrence_id == id)
            .ok_or("authority node was dropped")?;
        if occurrence.source.id != id
            || occurrence.source.adapter != "mer3ly.repository-graph/v1"
            || occurrence.values.get("label")
                != Some(&ProjectionValue::Text(
                    node["name"]
                        .as_str()
                        .ok_or("invalid authority label")?
                        .into(),
                ))
        {
            return Err(format!("source or label for {id} differs from the authority").into());
        }
    }
    if envelope.relationships.len() != edges.len() {
        return Err("exported relation count differs from the authority".into());
    }
    for edge in edges {
        let id = edge["id"].as_str().ok_or("invalid authority relation")?;
        let relationship = envelope
            .relationships
            .iter()
            .find(|relationship| relationship.id == id)
            .ok_or("authority relation was dropped")?;
        if Some(relationship.from_occurrence.as_str()) != edge["source"].as_str()
            || Some(relationship.to_occurrence.as_str()) != edge["target"].as_str()
            || Some(relationship.kind.as_str()) != edge["kind"].as_str()
        {
            return Err(format!("relationship {id} differs from the authority").into());
        }
    }
    let source = envelope.dataset.source.clone();
    let revision = envelope.dataset.revision.clone();
    let snapshot = RelationshipSnapshot {
        recipe: relationship_recipe(
            "site-integration",
            "Public repository relationships",
            "site integration fixture",
            "fixture-v1",
            BTreeMap::from([(
                "site".into(),
                ProjectionInputBinding {
                    source: source.clone(),
                    expects_generation: Some(revision.clone()),
                    revision_evidence: RevisionEvidence::PublicGeneration,
                },
            )]),
        ),
        source_name: "site".into(),
        selected_occurrence: None,
        selected_relationship: None,
    };
    let disclosed = RelationshipDataset {
        dataset: envelope.dataset,
        facets: BTreeSet::from([
            AUTHORED_ORDER_FACET.into(),
            OCCURRENCE_LABELS_FACET.into(),
            EXPLAINED_RELATIONSHIPS_FACET.into(),
        ]),
        relationships: envelope.relationships,
    };
    let compiler = ProjectionCompiler::new(ItemSizes {
        card: Size2 { w: 164.0, h: 68.0 },
    });
    let compiled = compiler
        .compile_relationship_snapshot(&snapshot, &disclosed)
        .map_err(|issues| format!("source compilation refused: {issues:?}"))?;
    if compiled.projection.scene.items.len() != nodes.len()
        || compiled.projection.scene.relations.len() != edges.len()
        || compiled.relationships.len() != edges.len()
    {
        return Err("compiled scene drops authority items or relations".into());
    }
    let mut sources = Vec::new();
    for occurrence in &disclosed.dataset.occurrences {
        let instance = compiled
            .projection
            .instance_by_occurrence
            .get(&occurrence.occurrence_id)
            .ok_or("compiled occurrence mapping is absent")?;
        let item = &compiled.projection.scene.items[instance.0 as usize];
        let actual = &compiled.projection.scene.sources[item.source.0 as usize];
        if actual != &occurrence.source {
            return Err(format!("compiled source {} changed", occurrence.occurrence_id).into());
        }
        sources.push(json!({"occurrence": occurrence.occurrence_id, "source": actual, "instance": instance.0}));
    }
    let mut relationships = Vec::new();
    for original in &disclosed.relationships {
        let matching = compiled
            .relationships
            .iter()
            .filter(|relationship| relationship.disclosure.id == original.id)
            .collect::<Vec<_>>();
        if matching.len() != 1 {
            return Err(format!(
                "compiled relationship {} was dropped or duplicated",
                original.id
            )
            .into());
        }
        let relationship = matching[0];
        if &relationship.disclosure != original {
            return Err(format!(
                "compiled relationship {} changed its disclosure",
                original.id
            )
            .into());
        }
        let from = compiled.projection.instance_by_occurrence[&original.from_occurrence];
        let to = compiled.projection.instance_by_occurrence[&original.to_occurrence];
        if relationship.from != from || relationship.to != to {
            return Err(format!("compiled relationship {} changed endpoints", original.id).into());
        }
        let rendered = compiled
            .projection
            .scene
            .relations
            .iter()
            .find(|relation| {
                relation.from == from
                    && relation.to == to
                    && relation.kind.as_deref() == Some(original.kind.as_str())
            })
            .ok_or("compiled relationship is absent from the scene")?;
        if rendered.points.len() < 2 {
            return Err("compiled relationship has no route".into());
        }
        relationships.push(json!({"id": original.id, "from": original.from_occurrence, "to": original.to_occurrence, "kind": original.kind}));
    }
    let original: Value = serde_json::from_str(&input)?;
    let mut unknown = original.clone();
    unknown["dataset"]["occurrences"][0]["unrecognized"] = json!(true);
    if !matches!(
        parse_host_dataset(&unknown.to_string()),
        Err(HostDatasetError::UnknownKey { .. })
    ) {
        return Err("unknown-key control was not refused by the boundary".into());
    }
    let mut stale = original.clone();
    stale["revisions"]
        .as_array_mut()
        .ok_or("no revision sequence")?
        .push(json!({"sequence": 2, "revision": "control:newer"}));
    if !matches!(
        parse_host_dataset(&stale.to_string()),
        Err(HostDatasetError::StaleRevision { .. })
    ) {
        return Err("stale-revision control was not refused by the boundary".into());
    }
    let mut endpoint = original;
    endpoint["relationships"][0]["to_occurrence"] = json!("control:undisclosed");
    if !matches!(
        parse_host_dataset(&endpoint.to_string()),
        Err(HostDatasetError::Relationships(_))
    ) {
        return Err("missing-endpoint control was not refused by the boundary".into());
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "schema": "mer3ly.host-dataset-native-integration-receipt/v1",
            "qualification": "Native integration fixture over local unmerged S1; no browser or practice acceptance claim",
            "dataset_file": dataset_path,
            "authority_file": authority_path,
            "dataset_sha256": format!("{:x}", Sha256::digest(input.as_bytes())),
            "dataset_bytes": input.len(), "source": source, "revision": revision,
            "authority_occurrences": nodes.len(), "compiled_items": compiled.projection.scene.items.len(),
            "authority_relationships": edges.len(), "compiled_relationships": compiled.relationships.len(),
            "occurrence_sources_preserved": true,
            "typed_relationship_disclosures_preserved": true,
            "occurrence_sources": sources, "relationship_endpoints": relationships,
            "controls": {"unknown_nested_key": "refused", "stale_revision": "refused", "undisclosed_endpoint": "refused"},
        }))?
    );
    Ok(())
}
