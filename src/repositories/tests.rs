use super::*;

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

#[test]
fn committed_authority_is_valid() {
    let authority = Authority::load(workspace_root()).expect("load authority files");
    if let Err(errors) = authority.validate() {
        panic!("authority validation failed:\n{}", errors.join("\n"));
    }
}

#[test]
fn inventory_targets_never_contain_absolute_paths() {
    let authority = Authority::load(workspace_root()).expect("load authority files");
    for target in authority.inventory_targets() {
        if let Some(locator) = target.local_locator {
            assert!(!locator.contains(':'));
            assert!(!locator.contains('\\'));
            assert!(!locator.starts_with('/'));
        }
    }
}

#[test]
fn live_github_roster_adds_fallbacks_and_removes_stale_relations() {
    let root = workspace_root();
    let authority = Authority::load(&root).expect("load authority files");
    let mut metadata =
        PublicMetadataCache::load(root.join(PUBLIC_METADATA_PATH)).expect("load public metadata");
    metadata
        .repository
        .retain(|repository| repository.id != "mere");
    metadata.repository.push(PublicRepositoryMetadata {
        id: "new-public-tool".to_owned(),
        github_slug: "merely-made/new-public-tool".to_owned(),
        name: "new-public-tool".to_owned(),
        description: "A newly discovered public tool.".to_owned(),
        homepage: None,
        license: Some("mit".to_owned()),
        updated_at: "2026-08-11T00:00:00Z".to_owned(),
        pushed_at: "2026-08-11T00:00:00Z".to_owned(),
        primary_language: Some("Rust".to_owned()),
        stargazer_count: 0,
        archived: false,
        fork: false,
        topics: Vec::new(),
    });

    let reconciled = authority.with_live_github_repositories(&metadata);
    let ids = reconciled
        .repositories
        .repository
        .iter()
        .map(|repository| repository.id.as_str())
        .collect::<BTreeSet<_>>();
    assert!(!ids.contains("mere"));
    assert!(ids.contains("new-public-tool"));
    let discovered = reconciled
        .repositories
        .repository
        .iter()
        .find(|repository| repository.id == "new-public-tool")
        .expect("new GitHub repository gets a fallback profile");
    assert_eq!(discovered.class, RepositoryClass::Tool);
    assert_eq!(discovered.summary, "A newly discovered public tool.");
    assert!(reconciled.relations.relation.iter().all(|relation| {
        ids.contains(relation.source.as_str()) && ids.contains(relation.target.as_str())
    }));
}
