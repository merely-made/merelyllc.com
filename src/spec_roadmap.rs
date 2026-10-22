// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! The roadmap: the spec registry read as features. Each `unlocks` entry is a
//! feature; its chain is the claim that unlocks it plus the prerequisites the
//! same repository claims. Every link names its depth, its registry line and,
//! for fetched counts, the source commit. The same model becomes the page's
//! frozen scene (through the repository graph's solver) and the Graphshell
//! dataset the live map loads.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use chirograph::ProjectionCaptureV2;
use graphshell_client::frozen::FrozenScene;
use mer3ly_repo_graph::projection_capture_with_placement_json;
use sceno::{InstanceId, SourceRef};
use scenograph::SourceBinding;
use scenomise::projection::{
    DisclosedRelationship, ProjectionDataset, ProjectionFieldType, ProjectionOccurrence,
    ProjectionValue, RelationshipProvenance,
};
use serde::Serialize;
use serde_json::json;
use sha2::{Digest, Sha256};

use crate::repositories::Authority;
use crate::spec_coverage::{CatalogSnapshot, requirements};
use crate::spec_results::Fetched;
use crate::specs::{ClaimRecord, Depth, SpecRegistry};

pub const ROADMAP_DATASET_FILE: &str = "spec-roadmap-dataset.json";
const DATASET_SCHEMA: &str = "scenomise.host-dataset/v1";
const ADAPTER: &str = "mer3ly.spec-roadmap/v1";
const REGISTRY_URL: &str =
    "https://github.com/merely-made/merelyllc.com/blob/main/content/specs.toml";

pub struct Roadmap {
    /// The latest date in the inputs: a claim's verification or a count's commit.
    pub on: String,
    pub features: Vec<Feature>,
    pub measured: Vec<MeasuredRow>,
    pub repos: Vec<RepoRow>,
    nodes: Vec<Node>,
    edges: Vec<Edge>,
}

pub struct Feature {
    pub text: String,
    pub consumer: String,
    pub consumer_url: Option<String>,
    pub links: Vec<Link>,
}

impl Feature {
    pub fn complete(&self) -> usize {
        self.links
            .iter()
            .filter(|link| link.depth == "full")
            .count()
    }
}

pub struct Link {
    pub spec: String,
    pub spec_name: String,
    pub repo: String,
    pub depth: &'static str,
    /// The spec this one is a prerequisite of; `None` for the claim that unlocks.
    pub via: Option<String>,
    pub measured: Option<String>,
    pub registry_url: String,
}

pub struct MeasuredRow {
    pub spec_name: String,
    pub repo: String,
    pub count: String,
    pub on: String,
    pub source_label: String,
    /// `None` where the source repository is not one the site may link to.
    pub source_url: Option<String>,
}

pub struct RepoRow {
    pub repo: String,
    pub claims: usize,
    pub full: usize,
    pub partial: usize,
}

struct Node {
    id: String,
    name: String,
    class: &'static str,
    status: String,
    summary: String,
    detail_url: String,
    source_url: String,
}

struct Edge {
    id: String,
    source: String,
    target: String,
    kind: &'static str,
    label: String,
    explanation: String,
}

/// The site publishes GitHub links only to its catalogued merely-made
/// repositories (the artifact gate's rule); any other URL passes.
fn publishable(url: &str, allowed: &BTreeSet<String>) -> bool {
    let Some(rest) = url.strip_prefix("https://github.com/") else {
        return true;
    };
    let mut parts = rest.split('/');
    let owner = parts.next().unwrap_or_default().to_ascii_lowercase();
    let repo = parts.next().map(str::to_ascii_lowercase);
    owner == "merely-made" && repo.is_none_or(|repo| allowed.contains(&format!("{owner}/{repo}")))
}

fn count_text(claim: &ClaimRecord) -> Option<String> {
    let m = claim.measured.as_ref()?;
    let pair = |a: Option<u64>, b: Option<u64>, unit: &str| {
        a.zip(b).map(|(a, b)| format!("{a} / {b} {unit}"))
    };
    pair(m.wpt_pass, m.wpt_total, "passing")
        .or_else(|| pair(m.files_all_pass, m.files, "files passing"))
        .or_else(|| pair(m.css_props_named, m.css_props_total, "CSS properties named"))
        .or_else(|| pair(m.idl_named, m.idl_total, "interfaces named"))
}

/// The line each `[[spec]]` id and each `[[claim]]` (by index) starts on.
fn registry_lines(text: &str) -> (HashMap<String, usize>, Vec<usize>) {
    let mut specs = HashMap::new();
    let mut claims = Vec::new();
    for (number, line) in text.lines().enumerate() {
        if line == "[[claim]]" {
            claims.push(number + 1);
        } else if let Some(id) = line
            .strip_prefix("id = \"")
            .and_then(|r| r.strip_suffix('"'))
        {
            specs.insert(id.to_owned(), number + 1);
        }
    }
    (specs, claims)
}

impl Roadmap {
    pub fn build(
        registry: &SpecRegistry,
        provenance: &BTreeMap<usize, Fetched>,
        catalog: &CatalogSnapshot,
        registry_text: &str,
        authority: &Authority,
    ) -> Self {
        let ledger: BTreeMap<&str, _> = authority
            .migration
            .migration
            .iter()
            .map(|record| (record.id.as_str(), record))
            .collect();
        let allowed: BTreeSet<String> = authority
            .repositories
            .repository
            .iter()
            .filter(|repository| repository.public)
            .map(|repository| repository.github_slug.to_ascii_lowercase())
            .collect();
        let publish = |url: String| publishable(&url, &allowed).then_some(url);
        let (spec_lines, claim_lines) = registry_lines(registry_text);
        let names: BTreeMap<&str, &str> = registry
            .spec
            .iter()
            .map(|s| (s.id.as_str(), s.name.as_str()))
            .collect();
        let spec_urls: BTreeMap<&str, &str> = registry
            .spec
            .iter()
            .filter_map(|s| s.url.as_deref().map(|url| (s.id.as_str(), url)))
            .collect();
        let claim_url = |index: usize| {
            claim_lines.get(index).map_or_else(
                || REGISTRY_URL.to_owned(),
                |line| format!("{REGISTRY_URL}#L{line}"),
            )
        };
        // A repository's strongest claim on each spec, across crates.
        let mut best: BTreeMap<(&str, &str), usize> = BTreeMap::new();
        for (index, claim) in registry.claim.iter().enumerate() {
            let key = (claim.repo.as_str(), claim.spec.as_str());
            let stronger = best
                .get(&key)
                .is_none_or(|&other| claim.depth < registry.claim[other].depth);
            if stronger {
                best.insert(key, index);
            }
        }
        let requires = requirements(registry, catalog);
        let link = |index: usize, via: Option<&str>| {
            let claim = &registry.claim[index];
            Link {
                spec: claim.spec.clone(),
                spec_name: names
                    .get(claim.spec.as_str())
                    .copied()
                    .unwrap_or(&claim.spec)
                    .to_owned(),
                repo: claim.repo.clone(),
                depth: claim.depth.as_str(),
                via: via.map(str::to_owned),
                measured: count_text(claim),
                registry_url: claim_url(index),
            }
        };
        let consumer_url = |consumer: &str| {
            let (repo, rest) = consumer.split_once(':')?;
            let record = ledger.get(repo)?;
            let path = rest.split_whitespace().next()?;
            if !path.contains('/') {
                return None;
            }
            publish(format!(
                "https://github.com/{}/blob/{}/{path}",
                record.current_slug, record.default_branch
            ))
        };

        let mut features: Vec<Feature> = Vec::new();
        for (index, claim) in registry.claim.iter().enumerate() {
            for unlock in &claim.unlocks {
                let position = features
                    .iter()
                    .position(|f| f.text == unlock.feature && f.consumer == unlock.consumer)
                    .unwrap_or_else(|| {
                        features.push(Feature {
                            text: unlock.feature.clone(),
                            consumer: unlock.consumer.clone(),
                            consumer_url: consumer_url(&unlock.consumer),
                            links: Vec::new(),
                        });
                        features.len() - 1
                    });
                let feature = &mut features[position];
                let mut add = |link: Link| {
                    if !feature
                        .links
                        .iter()
                        .any(|l| l.spec == link.spec && l.repo == link.repo)
                    {
                        feature.links.push(link);
                    }
                };
                add(link(index, None));
                for required in requires.get(claim.spec.as_str()).into_iter().flatten() {
                    if let Some(&other) = best.get(&(claim.repo.as_str(), *required)) {
                        add(link(other, Some(&claim.spec)));
                    }
                }
            }
        }

        let mut measured = Vec::new();
        for (index, claim) in registry.claim.iter().enumerate() {
            let (Some(count), Some(m)) = (count_text(claim), claim.measured.as_ref()) else {
                continue;
            };
            let (source_label, source_url) = match provenance.get(&index) {
                Some(f) => (
                    format!(
                        "{} at {}",
                        f.source.slug,
                        &f.commit[..f.commit.len().min(10)]
                    ),
                    publish(format!(
                        "https://github.com/{}/blob/{}/{}",
                        f.source.slug, f.commit, f.source.path
                    )),
                ),
                None => (
                    "copied into the registry".to_owned(),
                    Some(claim_url(index)),
                ),
            };
            measured.push(MeasuredRow {
                spec_name: names
                    .get(claim.spec.as_str())
                    .copied()
                    .unwrap_or(&claim.spec)
                    .to_owned(),
                repo: claim.repo.clone(),
                count,
                on: m.on.clone(),
                source_label,
                source_url,
            });
        }

        let mut repos: BTreeMap<&str, RepoRow> = BTreeMap::new();
        for claim in &registry.claim {
            let row = repos.entry(claim.repo.as_str()).or_insert_with(|| RepoRow {
                repo: claim.repo.clone(),
                claims: 0,
                full: 0,
                partial: 0,
            });
            row.claims += 1;
            row.full += usize::from(claim.depth == Depth::Full);
            row.partial += usize::from(claim.depth == Depth::Partial);
        }

        let on = registry
            .claim
            .iter()
            .map(|claim| claim.verified_on.as_str())
            .chain(provenance.values().map(|f| f.committed_on.as_str()))
            .max()
            .unwrap_or_default()
            .to_owned();

        // The graph the scene and the live map draw: repositories, the specs on
        // each feature's chain, and the features.
        let mut nodes: BTreeMap<String, Node> = BTreeMap::new();
        let mut edges: BTreeMap<String, Edge> = BTreeMap::new();
        for (number, feature) in features.iter().enumerate() {
            let feature_id = format!("feature:{number:03}");
            nodes.insert(
                feature_id.clone(),
                Node {
                    id: feature_id.clone(),
                    name: feature.text.clone(),
                    class: "product",
                    status: format!("{} of {} complete", feature.complete(), feature.links.len()),
                    summary: format!("For {}.", feature.consumer),
                    detail_url: feature
                        .consumer_url
                        .clone()
                        .unwrap_or_else(|| REGISTRY_URL.into()),
                    source_url: feature.links[0].registry_url.clone(),
                },
            );
            for link in &feature.links {
                let spec_id = format!("spec:{}", link.spec);
                nodes.entry(spec_id.clone()).or_insert_with(|| Node {
                    id: spec_id.clone(),
                    name: link.spec_name.clone(),
                    class: "foundation",
                    status: link.depth.into(),
                    summary: link
                        .measured
                        .clone()
                        .unwrap_or_else(|| format!("{} in {}.", link.depth, link.repo)),
                    detail_url: spec_urls
                        .get(link.spec.as_str())
                        .and_then(|u| publish((*u).to_owned()))
                        .unwrap_or_else(|| link.registry_url.clone()),
                    source_url: spec_lines.get(&link.spec).map_or_else(
                        || REGISTRY_URL.into(),
                        |line| format!("{REGISTRY_URL}#L{line}"),
                    ),
                });
                let repo_id = format!("repo:{}", link.repo);
                nodes.entry(repo_id.clone()).or_insert_with(|| {
                    let github = ledger
                        .get(link.repo.as_str())
                        .and_then(|r| publish(format!("https://github.com/{}", r.current_slug)))
                        .unwrap_or_else(|| REGISTRY_URL.into());
                    Node {
                        id: repo_id.clone(),
                        name: link.repo.clone(),
                        class: "platform",
                        status: "repository".into(),
                        summary: format!("Claims in the registry for {}.", link.repo),
                        detail_url: github.clone(),
                        source_url: github,
                    }
                });
                let claim_id = format!("claim:{}:{}", link.repo, link.spec);
                edges.entry(claim_id.clone()).or_insert_with(|| Edge {
                    id: claim_id,
                    source: repo_id.clone(),
                    target: spec_id.clone(),
                    kind: "claims",
                    label: format!("{} claims {} ({})", link.repo, link.spec_name, link.depth),
                    explanation: link.measured.clone().map_or_else(
                        || format!("Depth {}.", link.depth),
                        |m| format!("Depth {}; {m}.", link.depth),
                    ),
                });
                match &link.via {
                    None => {
                        let id = format!("unlocks:{}:{feature_id}", link.spec);
                        edges.insert(
                            id.clone(),
                            Edge {
                                id,
                                source: spec_id,
                                target: feature_id.clone(),
                                kind: "unlocks",
                                label: format!("{} unlocks {}", link.spec_name, feature.text),
                                explanation: format!("Named by {}.", feature.consumer),
                            },
                        );
                    }
                    Some(via) => {
                        let id = format!("requires:{via}:{}", link.spec);
                        edges.entry(id.clone()).or_insert_with(|| Edge {
                            id,
                            source: format!("spec:{via}"),
                            target: spec_id,
                            kind: "requires",
                            label: format!("{via} requires {}", link.spec_name),
                            explanation: "A normative reference or a registry `requires` edge."
                                .into(),
                        });
                    }
                }
            }
        }

        Self {
            on,
            features,
            measured,
            repos: repos.into_values().collect(),
            nodes: nodes.into_values().collect(),
            edges: edges.into_values().collect(),
        }
    }

    pub fn spec_count(&self) -> usize {
        self.nodes
            .iter()
            .filter(|node| node.class == "foundation")
            .count()
    }

    /// The graph in the repository graph's input shape, for its solver.
    fn graph_json(&self) -> String {
        json!({
            "schema": "mer3ly.repo-graph/v1",
            "nodes": self.nodes.iter().map(|n| json!({
                "id": n.id, "name": n.name, "class": n.class, "status": n.status,
                "pushed_at": self.on, "summary": n.summary,
            })).collect::<Vec<_>>(),
            "edges": self.edges.iter().map(|e| json!({
                "id": e.id, "source": e.source, "target": e.target, "kind": e.kind,
                "provenance": "registry",
            })).collect::<Vec<_>>(),
        })
        .to_string()
    }

    /// The roadmap solved into a scene and frozen for the no-script first view.
    pub fn frozen_scene(&self) -> Result<FrozenScene, String> {
        let capture = projection_capture_with_placement_json(&self.graph_json(), "{}")?;
        let capture = ProjectionCaptureV2::decode(capture.as_bytes())
            .map_err(|error| format!("the roadmap's capture is invalid: {error}"))?;
        let scene = capture.scene;
        let by_id: HashMap<&str, &Node> = self.nodes.iter().map(|n| (n.id.as_str(), n)).collect();
        let mut names = HashMap::new();
        let mut details = HashMap::new();
        for (index, item) in scene.tables.items.iter().enumerate() {
            let Some(item) = item else { continue };
            let Some(Some(source)) = scene.tables.sources.get(item.source.0 as usize) else {
                continue;
            };
            let node = by_id
                .get(source.id.as_str())
                .ok_or_else(|| format!("the roadmap scene names unknown node {}", source.id))?;
            names.insert(InstanceId(index as u32), node.name.clone());
            details.insert(InstanceId(index as u32), node.status.clone());
        }
        Ok(FrozenScene::freeze_snapshot_with_details(
            &scene,
            "The spec roadmap: repositories, specs and the features they unlock",
            &names,
            &details,
        ))
    }

    /// The live map's Graphshell dataset (`scenomise.host-dataset/v1`).
    pub fn dataset_json(&self) -> Result<String, String> {
        let source = SourceBinding {
            authority: "https://merelyllc.com".into(),
            domain: "spec-registry".into(),
            resource: "spec-roadmap".into(),
        };
        let mut fields: BTreeMap<String, ProjectionFieldType> = [
            "occurrence_id",
            "label",
            "summary",
            "class",
            "status",
            "detail_url",
            "source_url",
        ]
        .into_iter()
        .map(|name| (name.to_owned(), ProjectionFieldType::Text))
        .collect();
        fields.insert("order".into(), ProjectionFieldType::Number);
        let occurrences: Vec<ProjectionOccurrence> = self
            .nodes
            .iter()
            .enumerate()
            .map(|(order, node)| {
                let text = |value: &str| ProjectionValue::Text(value.to_owned());
                let values = BTreeMap::from([
                    ("occurrence_id".into(), text(&node.id)),
                    ("label".into(), text(&node.name)),
                    ("summary".into(), text(&node.summary)),
                    ("class".into(), text(node.class)),
                    ("status".into(), text(&node.status)),
                    ("detail_url".into(), text(&node.detail_url)),
                    ("source_url".into(), text(&node.source_url)),
                    ("order".into(), ProjectionValue::Number(order as f64)),
                ]);
                ProjectionOccurrence {
                    occurrence_id: node.id.clone(),
                    source: SourceRef::new(ADAPTER, &node.id),
                    values,
                }
            })
            .collect();
        let mut relationships: Vec<DisclosedRelationship> = self
            .edges
            .iter()
            .map(|edge| DisclosedRelationship {
                id: edge.id.clone(),
                from_occurrence: edge.source.clone(),
                to_occurrence: edge.target.clone(),
                kind: edge.kind.into(),
                label: edge.label.clone(),
                explanation: edge.explanation.clone(),
                provenance: RelationshipProvenance {
                    source: source.clone(),
                    source_revision: "".into(),
                    method: format!("mer3ly.spec-roadmap.{}", edge.kind),
                    method_version: 1,
                    provider: "mer3ly spec roadmap".into(),
                    evidence: vec![SourceRef::new(ADAPTER, &edge.id)],
                },
            })
            .collect();
        let material = serde_json::to_vec(&(
            DATASET_SCHEMA,
            &source,
            &fields,
            &occurrences,
            &relationships,
        ))
        .map_err(|error| format!("could not canonicalize the roadmap dataset: {error}"))?;
        let revision = format!("sha256:{:x}", Sha256::digest(material));
        for relationship in &mut relationships {
            relationship.provenance.source_revision = revision.clone().into();
        }
        #[derive(Serialize)]
        struct Envelope {
            schema: &'static str,
            dataset: ProjectionDataset,
            relationships: Vec<DisclosedRelationship>,
            revisions: Vec<serde_json::Value>,
        }
        serde_json::to_string_pretty(&Envelope {
            schema: DATASET_SCHEMA,
            dataset: ProjectionDataset {
                source,
                revision: revision.clone().into(),
                fields,
                occurrences,
            },
            relationships,
            revisions: vec![json!({ "sequence": 1, "revision": revision })],
        })
        .map_err(|error| format!("could not serialize the roadmap dataset: {error}"))
    }
}
