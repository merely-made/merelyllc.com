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
            GitAuthorityCheckpointProjection::Available { cursor, graph } => Some((cursor, graph)),
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
