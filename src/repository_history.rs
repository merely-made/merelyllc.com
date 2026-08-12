//! Public repository-graph snapshots sourced from immutable Git authority
//! checkpoints.
//!
//! This is deliberately a build-host adapter. The browser receives only the
//! reduced [`HistoricalRepositoryGraph`] records it needs to render; it never
//! receives a checkout path, a Git client, or a GitHub credential.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::process::Command;

use serde::{Deserialize, Serialize};

use crate::repositories::{
    PublicMetadataCache, RelationKind, RelationManifest, RelationProvenance, RepositoryClass,
    RepositoryManifest, RepositoryStatus, reconcile_live_github_repositories,
};

pub const REPOSITORY_GRAPH_SCHEMA: &str = "mer3ly.repo-graph/v1";
pub const GIT_HISTORY_SCHEMA: &str = "mer3ly.repository-git-history/v1";

const REPOSITORIES_PATH: &str = "content/repositories.toml";
const RELATIONS_PATH: &str = "content/relations.toml";
const METADATA_PATH: &str = "content/github-metadata.json";
const HISTORICAL_TIMELINE_PATH: &str = "content/repository-history.toml";
const HISTORICAL_TIMELINE_SCHEMA: &str = "mer3ly.repository-history-source/v1";

/// The reduced, public graph shape shared by the current authority projection
/// and an immutable historical checkpoint.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RepositoryGraph {
    pub schema: String,
    pub nodes: Vec<RepositoryGraphNode>,
    pub edges: Vec<RepositoryGraphEdge>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RepositoryGraphNode {
    pub id: String,
    pub name: String,
    pub class: RepositoryClass,
    pub status: RepositoryStatus,
    pub pushed_at: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RepositoryGraphEdge {
    pub id: String,
    pub source: String,
    pub target: String,
    pub kind: RelationKind,
    pub provenance: RelationProvenance,
}

impl RepositoryGraph {
    /// Project just the public graph fields from validated authority inputs.
    ///
    /// This keeps historical data deliberately reduced. A card's private
    /// operational data, GitHub refresh token, and local checkout do not cross
    /// this boundary.
    pub fn from_parts(
        repositories: &RepositoryManifest,
        relations: &RelationManifest,
        metadata: &PublicMetadataCache,
    ) -> Result<Self, String> {
        let (repositories, relations) =
            reconcile_live_github_repositories(repositories, relations, metadata);
        let mut metadata_by_id = BTreeMap::new();
        for entry in &metadata.repository {
            if metadata_by_id.insert(entry.id.as_str(), entry).is_some() {
                return Err(format!("duplicate public metadata record {}", entry.id));
            }
        }

        let mut node_ids = BTreeSet::new();
        let mut nodes = Vec::new();
        for repository in repositories
            .repository
            .iter()
            .filter(|repository| repository.public)
        {
            if !node_ids.insert(repository.id.as_str()) {
                return Err(format!("duplicate public repository {}", repository.id));
            }
            let metadata = metadata_by_id.get(repository.id.as_str()).ok_or_else(|| {
                format!("public repository {} is missing metadata", repository.id)
            })?;
            if metadata.github_slug != repository.github_slug {
                return Err(format!(
                    "public repository {} has mismatched metadata slug",
                    repository.id
                ));
            }
            nodes.push(RepositoryGraphNode {
                id: repository.id.clone(),
                name: repository.name.clone(),
                class: repository.class,
                status: repository.status,
                pushed_at: metadata.pushed_at.clone(),
            });
        }

        let mut edge_ids = BTreeSet::new();
        let mut edges = Vec::new();
        for relation in &relations.relation {
            if !edge_ids.insert(relation.id.as_str()) {
                return Err(format!("duplicate public relation {}", relation.id));
            }
            if !node_ids.contains(relation.source.as_str())
                || !node_ids.contains(relation.target.as_str())
            {
                return Err(format!(
                    "public relation {} has an endpoint outside the graph",
                    relation.id
                ));
            }
            edges.push(RepositoryGraphEdge {
                id: relation.id.clone(),
                source: relation.source.clone(),
                target: relation.target.clone(),
                kind: relation.kind,
                provenance: relation.provenance,
            });
        }

        let graph = Self {
            schema: REPOSITORY_GRAPH_SCHEMA.to_owned(),
            nodes,
            edges,
        };
        graph.validate()?;
        Ok(graph)
    }

    fn validate(&self) -> Result<(), String> {
        if self.schema != REPOSITORY_GRAPH_SCHEMA {
            return Err("repository graph schema mismatch".to_owned());
        }
        let node_ids = self
            .nodes
            .iter()
            .map(|node| node.id.as_str())
            .collect::<BTreeSet<_>>();
        if node_ids.len() != self.nodes.len()
            || self
                .nodes
                .iter()
                .any(|node| node.id.is_empty() || node.name.is_empty() || node.pushed_at.is_empty())
        {
            return Err("repository graph has invalid nodes".to_owned());
        }
        let edge_ids = self
            .edges
            .iter()
            .map(|edge| edge.id.as_str())
            .collect::<BTreeSet<_>>();
        if edge_ids.len() != self.edges.len()
            || self.edges.iter().any(|edge| {
                edge.id.is_empty()
                    || !node_ids.contains(edge.source.as_str())
                    || !node_ids.contains(edge.target.as_str())
            })
        {
            return Err("repository graph has invalid relations".to_owned());
        }
        Ok(())
    }
}

/// An immutable source cursor. Source and commit identity are the cursor, not
/// a mutable branch name or wall-clock timestamp.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct GitAuthorityCursor {
    /// The public repository whose immutable Git object supplied this snapshot.
    pub source: String,
    pub commit: String,
    pub committed_at: String,
}

/// Ordered source cursors available from one authority repository.
#[derive(Clone, Debug)]
pub struct GitAuthorityHistory {
    checkpoints: Vec<GitAuthorityCursor>,
}

impl GitAuthorityHistory {
    /// Discover committed checkpoints without inspecting the working tree.
    pub fn discover(repository: impl AsRef<Path>) -> Result<Self, String> {
        let repository = repository.as_ref();
        let source = git_repository_source(repository)?;
        let output = git_output(
            repository,
            ["log", "--reverse", "--format=%H%x09%cI"],
            "could not list Git authority checkpoints",
        )?;
        let checkpoints = output
            .lines()
            .map(|line| {
                let (commit, committed_at) = line
                    .split_once('\t')
                    .ok_or_else(|| "Git checkpoint is missing its timestamp".to_owned())?;
                if commit.is_empty() || committed_at.is_empty() {
                    return Err("Git checkpoint is incomplete".to_owned());
                }
                Ok(GitAuthorityCursor {
                    source: source.clone(),
                    commit: commit.to_owned(),
                    committed_at: committed_at.to_owned(),
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        if checkpoints.is_empty() {
            return Err("Git authority history has no committed checkpoints".to_owned());
        }
        Ok(Self { checkpoints })
    }

    pub fn checkpoints(&self) -> &[GitAuthorityCursor] {
        &self.checkpoints
    }

    /// The first immutable source checkpoint available to this history.
    pub fn earliest_cursor(&self) -> &GitAuthorityCursor {
        self.checkpoints
            .first()
            .expect("GitAuthorityHistory rejects an empty checkpoint list")
    }

    /// Stable, evenly distributed immutable cursors for a compact scrubber.
    /// A one-point control lands at live; larger controls include both ends
    /// when this authority has more than one checkpoint.
    pub fn cursor_ticks(&self, max_points: usize) -> Vec<&GitAuthorityCursor> {
        if max_points == 0 {
            return Vec::new();
        }
        let final_index = self.checkpoints.len() - 1;
        if max_points == 1 || final_index == 0 {
            return vec![self.live_cursor()];
        }
        let points = max_points.min(self.checkpoints.len());
        let denominator = points - 1;
        (0..points)
            .map(|index| &self.checkpoints[index * final_index / denominator])
            .collect()
    }

    pub fn live_cursor(&self) -> &GitAuthorityCursor {
        self.checkpoints
            .last()
            .expect("GitAuthorityHistory rejects an empty checkpoint list")
    }

    /// Load the reduced graph from exactly this committed source tree.
    pub fn snapshot_at(
        &self,
        repository: impl AsRef<Path>,
        cursor: &GitAuthorityCursor,
    ) -> Result<HistoricalRepositoryGraph, String> {
        if !self.checkpoints.iter().any(|known| known == cursor) {
            return Err("Git checkpoint does not belong to this authority history".to_owned());
        }
        let repository = repository.as_ref();
        let repositories = git_toml(repository, &cursor.commit, REPOSITORIES_PATH)?;
        let relations = git_toml(repository, &cursor.commit, RELATIONS_PATH)?;
        let metadata = git_json(repository, &cursor.commit, METADATA_PATH)?;
        let graph = RepositoryGraph::from_parts(&repositories, &relations, &metadata)?;
        Ok(HistoricalRepositoryGraph {
            schema: GIT_HISTORY_SCHEMA.to_owned(),
            cursor: cursor.clone(),
            graph,
        })
    }

    /// Materialize a bounded set of scrubber ticks for a static public
    /// artifact. A missing or invalid historical authority input remains an
    /// unavailable checkpoint so callers can report the source boundary.
    pub fn sampled_projection(
        &self,
        repository: impl AsRef<Path>,
        max_points: usize,
    ) -> GitAuthorityHistoryProjection {
        let repository = repository.as_ref();
        let checkpoints = self
            .cursor_ticks(max_points)
            .into_iter()
            .map(|cursor| match self.snapshot_at(repository, cursor) {
                Ok(snapshot) => GitAuthorityCheckpointProjection::Available {
                    cursor: snapshot.cursor,
                    graph: snapshot.graph,
                },
                Err(reason) => GitAuthorityCheckpointProjection::Unavailable {
                    cursor: cursor.clone(),
                    reason,
                },
            })
            .collect();
        GitAuthorityHistoryProjection {
            schema: GIT_HISTORY_SCHEMA.to_owned(),
            checkpoints,
        }
    }
}

/// Build the public timeline from two sources: checked-in, reduced older
/// checkpoints whose provenance is a different public repository, then the
/// Mer3ly authority's own committed checkpoints. The static site ships the
/// result only; it never needs another checkout or credentials at runtime.
pub fn public_history_projection(
    repository: impl AsRef<Path>,
    current: RepositoryGraph,
    max_points: usize,
) -> Result<GitAuthorityHistoryProjection, String> {
    let repository = repository.as_ref();
    let mut checkpoints = HistoricalAuthorityManifest::load(repository)?.project()?;
    let history = GitAuthorityHistory::discover(repository)?;
    let live_cursor = history.live_cursor().clone();
    let current_checkpoint = GitAuthorityCheckpointProjection::Available {
        cursor: live_cursor.clone(),
        graph: current,
    };

    for checkpoint in history
        .sampled_projection(repository, max_points)
        .checkpoints
    {
        match &checkpoint {
            GitAuthorityCheckpointProjection::Available { cursor, .. }
                if cursor == &live_cursor =>
            {
                checkpoints.push(current_checkpoint.clone());
            }
            _ => checkpoints.push(checkpoint),
        }
    }
    checkpoints.sort_by(|left, right| {
        let left = checkpoint_cursor(left);
        let right = checkpoint_cursor(right);
        left.committed_at
            .cmp(&right.committed_at)
            .then_with(|| left.source.cmp(&right.source))
            .then_with(|| left.commit.cmp(&right.commit))
    });

    let mut identities = BTreeSet::new();
    for checkpoint in &checkpoints {
        let cursor = checkpoint_cursor(checkpoint);
        if cursor.source.is_empty() || cursor.commit.is_empty() || cursor.committed_at.is_empty() {
            return Err("repository history has an incomplete source cursor".to_owned());
        }
        if !identities.insert((&cursor.source, &cursor.commit)) {
            return Err("repository history repeats a source commit".to_owned());
        }
        if let GitAuthorityCheckpointProjection::Available { graph, .. } = checkpoint {
            graph.validate()?;
        }
    }

    Ok(GitAuthorityHistoryProjection {
        schema: GIT_HISTORY_SCHEMA.to_owned(),
        checkpoints,
    })
}

/// The public artifact record for one Git authority checkpoint.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct HistoricalRepositoryGraph {
    pub schema: String,
    pub cursor: GitAuthorityCursor,
    pub graph: RepositoryGraph,
}

/// A sampled, public history payload for a static artifact. Checkpoints whose
/// authority files predate this graph are retained as explicit unavailability,
/// never projected as an empty graph.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct GitAuthorityHistoryProjection {
    pub schema: String,
    pub checkpoints: Vec<GitAuthorityCheckpointProjection>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "availability", rename_all = "snake_case")]
pub enum GitAuthorityCheckpointProjection {
    Available {
        cursor: GitAuthorityCursor,
        graph: RepositoryGraph,
    },
    Unavailable {
        cursor: GitAuthorityCursor,
        reason: String,
    },
}

#[derive(Debug, Deserialize)]
struct HistoricalAuthorityManifest {
    schema: String,
    #[serde(default)]
    checkpoint: Vec<HistoricalAuthorityCheckpoint>,
}

#[derive(Debug, Deserialize)]
struct HistoricalAuthorityCheckpoint {
    source: String,
    commit: String,
    committed_at: String,
    #[serde(default, rename = "node")]
    nodes: Vec<RepositoryGraphNode>,
    #[serde(default, rename = "edge")]
    edges: Vec<RepositoryGraphEdge>,
}

impl HistoricalAuthorityManifest {
    fn load(repository: &Path) -> Result<Self, String> {
        let path = repository.join(HISTORICAL_TIMELINE_PATH);
        let text = std::fs::read_to_string(&path)
            .map_err(|error| format!("could not read historical repository authority: {error}"))?;
        let manifest: Self = toml::from_str(&text)
            .map_err(|error| format!("historical repository authority is invalid TOML: {error}"))?;
        if manifest.schema != HISTORICAL_TIMELINE_SCHEMA || manifest.checkpoint.is_empty() {
            return Err(
                "historical repository authority has an invalid schema or no checkpoints"
                    .to_owned(),
            );
        }
        Ok(manifest)
    }

    fn project(self) -> Result<Vec<GitAuthorityCheckpointProjection>, String> {
        self.checkpoint
            .into_iter()
            .map(|checkpoint| {
                let graph = RepositoryGraph {
                    schema: REPOSITORY_GRAPH_SCHEMA.to_owned(),
                    nodes: checkpoint.nodes,
                    edges: checkpoint.edges,
                };
                graph.validate()?;
                Ok(GitAuthorityCheckpointProjection::Available {
                    cursor: GitAuthorityCursor {
                        source: checkpoint.source,
                        commit: checkpoint.commit,
                        committed_at: checkpoint.committed_at,
                    },
                    graph,
                })
            })
            .collect()
    }
}

fn checkpoint_cursor(checkpoint: &GitAuthorityCheckpointProjection) -> &GitAuthorityCursor {
    match checkpoint {
        GitAuthorityCheckpointProjection::Available { cursor, .. }
        | GitAuthorityCheckpointProjection::Unavailable { cursor, .. } => cursor,
    }
}

fn git_toml<T>(repository: &Path, commit: &str, path: &str) -> Result<T, String>
where
    T: for<'de> Deserialize<'de>,
{
    let text = git_file(repository, commit, path)?;
    toml::from_str(&text).map_err(|error| format!("Git checkpoint {path} is invalid TOML: {error}"))
}

fn git_json<T>(repository: &Path, commit: &str, path: &str) -> Result<T, String>
where
    T: for<'de> Deserialize<'de>,
{
    let text = git_file(repository, commit, path)?;
    serde_json::from_str(&text)
        .map_err(|error| format!("Git checkpoint {path} is invalid JSON: {error}"))
}

fn git_file(repository: &Path, commit: &str, path: &str) -> Result<String, String> {
    let revision = format!("{commit}:{path}");
    git_output(
        repository,
        ["show", "--no-textconv", revision.as_str()],
        "could not read Git authority checkpoint file",
    )
}

fn git_repository_source(repository: &Path) -> Result<String, String> {
    let remote = git_output(
        repository,
        ["remote", "get-url", "origin"],
        "could not identify Git authority origin",
    )?;
    let remote = remote.trim().trim_end_matches(".git");
    let source = remote
        .strip_prefix("https://github.com/")
        .or_else(|| remote.strip_prefix("git@github.com:"))
        .unwrap_or(remote);
    if source.is_empty() {
        return Err("Git authority origin is empty".to_owned());
    }
    Ok(source.to_owned())
}

fn git_output<const N: usize>(
    repository: &Path,
    arguments: [&str; N],
    context: &str,
) -> Result<String, String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repository)
        .args(arguments)
        .output()
        .map_err(|error| format!("{context}: {error}"))?;
    if !output.status.success() {
        return Err(context.to_owned());
    }
    String::from_utf8(output.stdout)
        .map_err(|error| format!("{context}: non-UTF-8 output: {error}"))
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::path::{Path, PathBuf};

    use super::{
        GitAuthorityCheckpointProjection, GitAuthorityCursor, GitAuthorityHistory, METADATA_PATH,
        RepositoryGraph, git_json, public_history_projection,
    };
    use crate::repositories::{PublicMetadataCache, PublicSiteData};

    fn workspace_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
    }

    #[test]
    fn committed_authority_cursor_replays_the_committed_public_graph() {
        let root = workspace_root();
        let history = GitAuthorityHistory::discover(&root).expect("Git authority history");
        let cursor = history.live_cursor().clone();

        let historical = history
            .snapshot_at(&root, &cursor)
            .expect("the committed live authority snapshot loads");
        assert_eq!(historical.cursor, cursor);
        let committed_metadata: PublicMetadataCache =
            git_json(&root, &historical.cursor.commit, METADATA_PATH)
                .expect("committed public metadata loads");
        let committed_ids = committed_metadata
            .repository
            .iter()
            .map(|repository| repository.id.as_str())
            .collect::<BTreeSet<_>>();
        let graph_ids = historical
            .graph
            .nodes
            .iter()
            .map(|repository| repository.id.as_str())
            .collect::<BTreeSet<_>>();
        assert_eq!(graph_ids, committed_ids);
        assert_eq!(
            historical.graph.nodes.len(),
            committed_metadata.repository.len()
        );
    }

    #[test]
    fn a_cursor_from_another_history_is_rejected_before_git_is_invoked() {
        let root = workspace_root();
        let history = GitAuthorityHistory::discover(&root).expect("Git authority history");
        let foreign = GitAuthorityCursor {
            source: "other/repository".to_owned(),
            commit: "not-a-commit".to_owned(),
            committed_at: "1970-01-01T00:00:00Z".to_owned(),
        };

        assert!(history.snapshot_at(&root, &foreign).is_err());
    }

    #[test]
    fn cursor_ticks_include_earliest_and_live_without_needing_timestamps() {
        let history = GitAuthorityHistory {
            checkpoints: (0..5)
                .map(|index| GitAuthorityCursor {
                    source: "fixture/history".to_owned(),
                    commit: format!("commit-{index}"),
                    committed_at: format!("not-used-for-order-{index}"),
                })
                .collect(),
        };

        assert!(history.cursor_ticks(0).is_empty());
        assert_eq!(history.cursor_ticks(1), vec![history.live_cursor()]);
        assert_eq!(history.cursor_ticks(3)[0], history.earliest_cursor());
        assert_eq!(history.cursor_ticks(3)[1].commit, "commit-2");
        assert_eq!(history.cursor_ticks(3)[2], history.live_cursor());
        assert_eq!(history.cursor_ticks(9).len(), 5);
    }

    #[test]
    fn sampled_projection_marks_unavailable_authority_without_erasing_it() {
        let root = workspace_root();
        let history = GitAuthorityHistory::discover(&root).expect("Git authority history");
        let projection = history.sampled_projection(&root, 2);

        assert_eq!(projection.schema, super::GIT_HISTORY_SCHEMA);
        assert_eq!(projection.checkpoints.len(), 2);
        assert!(matches!(
            projection.checkpoints.first(),
            Some(GitAuthorityCheckpointProjection::Unavailable { .. })
        ));
        assert!(matches!(
            projection.checkpoints.last(),
            Some(GitAuthorityCheckpointProjection::Available { cursor, .. })
                if cursor == history.live_cursor()
        ));
    }

    #[test]
    fn public_projection_keeps_archived_lineage_and_fresh_live_authority() {
        let root = workspace_root();
        let data = PublicSiteData::load(&root).expect("validated current public authority");
        let current = RepositoryGraph::from_parts(
            &data.authority.repositories,
            &data.authority.relations,
            &data.metadata,
        )
        .expect("current authority projects a public graph");
        let projection = public_history_projection(&root, current.clone(), 24)
            .expect("public history authority projects");
        let available = projection
            .checkpoints
            .iter()
            .filter_map(|checkpoint| match checkpoint {
                GitAuthorityCheckpointProjection::Available { cursor, graph } => {
                    Some((cursor, graph))
                }
                GitAuthorityCheckpointProjection::Unavailable { .. } => None,
            })
            .collect::<Vec<_>>();

        assert!(available.len() >= 6, "timeline retains public source eras");
        assert_eq!(available[0].0.source, "merely-made/mere");
        assert!(
            available[0]
                .1
                .nodes
                .iter()
                .any(|node| node.id == "graphshell")
        );
        assert!(
            available
                .iter()
                .any(|(_, graph)| graph.nodes.iter().any(|node| node.id == "webrender-wgpu"))
        );
        let graphshell_position = available
            .iter()
            .position(|(_, graph)| graph.nodes.iter().any(|node| node.id == "graphshell"))
            .expect("archived Graphshell source checkpoint");
        assert!(
            available
                .iter()
                .skip(graphshell_position + 1)
                .any(|(_, graph)| !graph.nodes.iter().any(|node| node.id == "graphshell"))
        );
        assert_eq!(available.last().expect("live checkpoint").1, &current);
    }
}
