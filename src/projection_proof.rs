//! The Mere profile's projection proof, over the stack's shared artifacts
//! (site canvas plan, P1 and P2; Rulings 130-136).
//!
//! Four sibling files carry it, and the page reads all four:
//! - the V2 capture, the trace and the shelfmark, written by
//!   `mer3ly_repo_graph::portable_projection` from the proof's authority;
//! - `repository-host-dataset.json`, the S1 envelope, which supplies every
//!   name, class, status and relationship label (Ruling 136).
//!
//! This module joins them at build time, for the no-script reading and for
//! `validate-artifact`. The join is strict: every captured item must be a
//! disclosed occurrence, every captured relation must match exactly one
//! disclosed relationship by its endpoints and kind, and every selection in
//! the trace must name one of them.

use std::collections::BTreeMap;

use mer3ly_repo_graph::read_portable_projection;
use scenomise::host_dataset::parse_host_dataset;
use scenomise::projection::ProjectionValue;
use sha2::{Digest, Sha256};

pub use mer3ly_repo_graph::{
    CAPTURE_FILE, PROJECTION_STEP_BOUND, PortableProjection, ProjectionReading, SHELFMARK_FILE,
    TRACE_FILE, portable_projection,
};

use crate::host_dataset::{HOST_DATASET_FILE, repository_host_dataset_json};
use crate::repositories::{Authority, PublicMetadataCache, PublicSiteData};
use crate::repository_history::RepositoryGraph;
use crate::site::{SiteView, element, txt};

/// One captured project, labelled from the host dataset.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProofNode {
    pub id: String,
    pub label: String,
    pub class: String,
    pub status: String,
}

/// One captured relation, identified and labelled from the host dataset.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProofRelation {
    pub index: u32,
    pub id: String,
    pub label: String,
}

/// The proof's artifacts, consumed and joined to the host dataset.
#[derive(Clone, Debug)]
pub struct ProjectionProof {
    pub artifacts: PortableProjection,
    pub reading: ProjectionReading,
    pub nodes: Vec<ProofNode>,
    pub relations: Vec<ProofRelation>,
}

/// The repository authority the proof projects: Mere's direct relations and
/// the projects at their ends.
pub fn projection_authority_json(authority: &Authority, metadata: &PublicMetadataCache) -> String {
    let authority =
        RepositoryGraph::from_parts(&authority.repositories, &authority.relations, metadata)
            .expect("validated public site data projects a repository graph");
    let edges = authority
        .edges
        .into_iter()
        .filter(|edge| edge.source == "mere" || edge.target == "mere")
        .collect::<Vec<_>>();
    let node_ids = edges
        .iter()
        .flat_map(|edge| [edge.source.clone(), edge.target.clone()])
        .collect::<std::collections::BTreeSet<_>>();
    let nodes = authority
        .nodes
        .into_iter()
        .filter(|node| node_ids.contains(&node.id))
        .collect::<Vec<_>>();
    let graph = RepositoryGraph {
        schema: authority.schema,
        nodes,
        edges,
    };
    serde_json::to_string(&graph).expect("Mere projection proof authority is serializable")
}

impl ProjectionProof {
    /// Export the proof's artifacts from the site's data and join them.
    pub fn build(data: &PublicSiteData) -> Result<Self, String> {
        Self::derive(&data.authority, &data.metadata)
    }

    /// [`build`](Self::build), from the authority and metadata alone.
    pub fn derive(authority: &Authority, metadata: &PublicMetadataCache) -> Result<Self, String> {
        let artifacts = portable_projection(&projection_authority_json(authority, metadata))?;
        let dataset = repository_host_dataset_json(authority, metadata)?;
        Self::read(artifacts, &dataset)
    }

    /// Consume published artifacts and join them to a host dataset.
    pub fn read(artifacts: PortableProjection, dataset: &str) -> Result<Self, String> {
        let reading = read_portable_projection(&artifacts)?;
        let dataset = parse_host_dataset(dataset)
            .map_err(|error| format!("{HOST_DATASET_FILE} is refused: {error}"))?;

        let text = |values: &BTreeMap<String, ProjectionValue>, id: &str, field: &str| match values
            .get(field)
        {
            Some(ProjectionValue::Text(value)) if !value.trim().is_empty() => Ok(value.clone()),
            _ => Err(format!(
                "{HOST_DATASET_FILE} gives captured project {id} no {field}"
            )),
        };
        let mut nodes = Vec::with_capacity(reading.items.len());
        for id in &reading.items {
            let occurrence = dataset
                .dataset
                .occurrences
                .iter()
                .find(|occurrence| &occurrence.occurrence_id == id)
                .ok_or_else(|| {
                    format!("{HOST_DATASET_FILE} does not disclose captured project {id}")
                })?;
            nodes.push(ProofNode {
                id: id.clone(),
                label: text(&occurrence.values, id, "label")?,
                class: text(&occurrence.values, id, "class")?,
                status: text(&occurrence.values, id, "status")?,
            });
        }

        let mut relations = Vec::with_capacity(reading.relations.len());
        for relation in &reading.relations {
            let matches = dataset
                .relationships
                .iter()
                .filter(|disclosed| {
                    disclosed.from_occurrence == relation.from
                        && disclosed.to_occurrence == relation.to
                        && disclosed.kind == relation.kind
                })
                .collect::<Vec<_>>();
            let [disclosed] = matches.as_slice() else {
                return Err(format!(
                    "captured relation {} ({} {} {}) matches {} disclosed relationships, not one",
                    relation.index,
                    relation.from,
                    relation.kind,
                    relation.to,
                    matches.len()
                ));
            };
            relations.push(ProofRelation {
                index: relation.index,
                id: disclosed.id.clone(),
                label: disclosed.label.clone(),
            });
        }

        for (index, step) in reading.steps.iter().enumerate() {
            let Some((kind, id)) = &step.selection else {
                continue;
            };
            let known = match kind.as_str() {
                "node" => nodes.iter().any(|node| &node.id == id),
                _ => relations.iter().any(|relation| &relation.id == id),
            };
            if !known {
                return Err(format!(
                    "trace step {index} selects {kind} {id}, which the proof does not carry"
                ));
            }
        }

        Ok(Self {
            artifacts,
            reading,
            nodes,
            relations,
        })
    }

    /// The published files, by artifact path.
    pub fn files(&self) -> [(&'static str, &[u8]); 3] {
        [
            (CAPTURE_FILE, &self.artifacts.capture),
            (TRACE_FILE, &self.artifacts.trace),
            (SHELFMARK_FILE, &self.artifacts.shelfmark),
        ]
    }

    /// A content-versioned URL for one artifact, so a cached copy is never
    /// read beside a newer sibling.
    pub fn href(name: &str, bytes: &[u8]) -> String {
        format!(
            "/{name}?v={}",
            &format!("{:x}", Sha256::digest(bytes))[..12]
        )
    }

    fn label_of(&self, kind: &str, id: &str) -> String {
        match kind {
            "node" => self
                .nodes
                .iter()
                .find(|node| node.id == id)
                .map(|node| node.label.clone()),
            _ => self
                .relations
                .iter()
                .find(|relation| relation.id == id)
                .map(|relation| relation.label.clone()),
        }
        .unwrap_or_else(|| id.to_owned())
    }

    /// The sentence the figure states, derived from the capture and trace.
    pub fn summary(&self) -> String {
        let receipt = &self.reading.receipt;
        format!(
            "{} public projects and {} validated relationships enter one Scenograph capture, with the score that solved it. Its scene trace has {} steps, from revision {} to revision {}; a native receipt and both page projections replay the same capture and trace.",
            count_words(self.nodes.len()),
            self.relations.len(),
            receipt.trace_steps,
            receipt.initial_revision,
            receipt.final_revision,
        )
    }

    /// The no-script reading: what the capture holds and each trace step.
    pub fn static_reading(&self) -> SiteView {
        let receipt = &self.reading.receipt;
        let steps = self
            .reading
            .steps
            .iter()
            .enumerate()
            .map(|(index, step)| {
                let what = match &step.selection {
                    Some((kind, id)) => format!("selects {}", self.label_of(kind, id)),
                    None if step.changes_scene => format!("scene revision {}", step.revision),
                    None => "no scene change".to_owned(),
                };
                element(
                    "li",
                    &[("data-projection-reading-step", &(index + 1).to_string())],
                    vec![txt(format!("{}: {what}.", step.label))],
                )
            })
            .collect();
        element(
            "div",
            &[
                ("class", "projection-proof-fallback"),
                ("data-projection-fallback", ""),
            ],
            vec![
                element(
                    "p",
                    &[],
                    vec![txt(format!(
                        "The synchronized scene requires JavaScript. Without it, this is the trace read at build time: {} projects and {} relationships captured at revision {}, then {} steps ending at revision {} with {} relationships still in the scene. The relationship lists above carry the same public projects and relationships as ordinary text.",
                        self.nodes.len(),
                        self.relations.len(),
                        receipt.initial_revision,
                        receipt.trace_steps,
                        receipt.final_revision,
                        receipt.active_relations,
                    ))],
                ),
                element("ol", &[("class", "projection-proof-reading")], steps),
            ],
        )
    }
}

fn count_words(count: usize) -> String {
    const WORDS: [&str; 13] = [
        "Zero", "One", "Two", "Three", "Four", "Five", "Six", "Seven", "Eight", "Nine", "Ten",
        "Eleven", "Twelve",
    ];
    WORDS
        .get(count)
        .map_or_else(|| count.to_string(), |word| (*word).to_owned())
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::*;

    fn data() -> PublicSiteData {
        PublicSiteData::load(env!("CARGO_MANIFEST_DIR")).expect("validated public site data")
    }

    fn dataset(data: &PublicSiteData) -> Value {
        serde_json::from_str(
            &repository_host_dataset_json(&data.authority, &data.metadata).expect("dataset"),
        )
        .expect("dataset JSON")
    }

    fn artifacts(data: &PublicSiteData) -> PortableProjection {
        portable_projection(&projection_authority_json(&data.authority, &data.metadata))
            .expect("artifacts")
    }

    #[test]
    fn every_label_comes_from_the_host_dataset() {
        let data = data();
        let proof = ProjectionProof::build(&data).expect("proof");
        let dataset = dataset(&data);
        for node in &proof.nodes {
            let occurrence = dataset["dataset"]["occurrences"]
                .as_array()
                .unwrap()
                .iter()
                .find(|occurrence| occurrence["occurrence_id"] == node.id.as_str())
                .unwrap();
            assert_eq!(occurrence["values"]["label"]["value"], node.label.as_str());
            assert_eq!(occurrence["values"]["class"]["value"], node.class.as_str());
            assert_eq!(
                occurrence["values"]["status"]["value"],
                node.status.as_str()
            );
        }
        let ids = proof
            .relations
            .iter()
            .map(|relation| relation.id.as_str())
            .collect::<Vec<_>>();
        assert!(ids.contains(&"turnstone-hosts-mere"));
        assert!(ids.contains(&"turnstone-depends-on-mere"));
        assert_eq!(proof.relations.len(), proof.reading.relations.len());
    }

    #[test]
    fn a_dataset_that_cannot_label_the_capture_is_refused() {
        let data = data();
        let mut unlabelled = dataset(&data);
        let occurrence = unlabelled["dataset"]["occurrences"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|occurrence| occurrence["occurrence_id"] == "turnstone")
            .unwrap();
        occurrence["values"]["status"] = json!({"kind": "text", "value": " "});
        let refused = ProjectionProof::read(artifacts(&data), &unlabelled.to_string()).unwrap_err();
        assert!(refused.contains("turnstone no status"), "{refused}");

        let mut ambiguous = dataset(&data);
        let relationships = ambiguous["relationships"].as_array_mut().unwrap();
        let mut twin = relationships
            .iter()
            .find(|relation| relation["id"] == "turnstone-hosts-mere")
            .unwrap()
            .clone();
        twin["id"] = json!("turnstone-hosts-mere-twin");
        relationships.push(twin);
        assert!(ProjectionProof::read(artifacts(&data), &ambiguous.to_string()).is_err());
    }

    #[test]
    fn the_static_reading_states_each_step_and_its_selection() {
        let data = data();
        let proof = ProjectionProof::build(&data).expect("proof");
        let mere = data
            .authority
            .repositories
            .repository
            .iter()
            .find(|repository| repository.id == "mere")
            .unwrap();
        let html = crate::pages::projects::document_for(&data, mere);
        assert_eq!(
            html.matches("data-projection-reading-step=").count(),
            proof.reading.steps.len()
        );
        assert!(html.contains("Select the Turnstone host relationship: selects Turnstone"));
        assert!(html.contains(&format!(
            "revision {}",
            proof.reading.receipt.final_revision
        )));
    }
}
