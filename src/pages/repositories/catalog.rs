use super::*;

pub(super) fn repository_index(data: &PublicSiteData) -> SiteView {
    let repositories = &data.authority.repositories.repository;
    let metadata_by_id: BTreeMap<_, _> = data
        .metadata
        .repository
        .iter()
        .map(|metadata| (metadata.id.as_str(), metadata))
        .collect();
    let repositories_by_id: BTreeMap<_, _> = repositories
        .iter()
        .map(|repository| (repository.id.as_str(), repository))
        .collect();

    let cards = repositories
        .iter()
        .filter(|repository| repository.public)
        .map(|repository| {
            repository_card(
                repository,
                metadata_by_id
                    .get(repository.id.as_str())
                    .expect("validated metadata record"),
                &repositories_by_id,
                &data.authority.relations.relation,
            )
        })
        .collect();

    element(
        "section",
        &[
            ("class", "content-section repository-index"),
            ("aria-label", "Repository index"),
        ],
        vec![
            section_heading("03", "repository index"),
            element(
                "p",
                &[("class", "index-intro")],
                vec![txt(
                    "Filter by working role. Status, license, topics, and both directions of every recorded relationship remain on each visible card.",
                )],
            ),
            element(
                "div",
                &[("class", "repository-filter-region")],
                vec![repository_filters(repositories, cards)],
            ),
        ],
    )
}

pub(super) fn repository_filters(
    repositories: &[RepositoryRecord],
    cards: Vec<SiteView>,
) -> SiteView {
    let mut children = vec![element(
        "legend",
        &[("class", "sr-only")],
        vec![txt("Filter repositories by class")],
    )];
    children.extend(filter_input("all", "All", repositories.len(), true));
    children.extend(filter_input(
        "product",
        "Products",
        class_count(repositories, RepositoryClass::Product),
        false,
    ));
    children.extend(filter_input(
        "platform",
        "Platforms",
        class_count(repositories, RepositoryClass::Platform),
        false,
    ));
    children.extend(filter_input(
        "foundation",
        "Foundations",
        class_count(repositories, RepositoryClass::Foundation),
        false,
    ));
    children.extend(filter_input(
        "tool",
        "Tools",
        class_count(repositories, RepositoryClass::Tool),
        false,
    ));
    children.push(relation_key());
    children.push(element("div", &[("class", "repository-list")], cards));
    element(
        "fieldset",
        &[("class", "repository-filter-shell")],
        children,
    )
}

pub(super) fn class_count(repositories: &[RepositoryRecord], class: RepositoryClass) -> usize {
    repositories
        .iter()
        .filter(|repository| repository.public && repository.class == class)
        .count()
}

pub(super) fn filter_input(value: &str, label: &str, count: usize, checked: bool) -> Vec<SiteView> {
    let id = format!("repository-filter-{value}");
    let mut attributes = vec![
        ("type", "radio"),
        ("name", "repository-class"),
        ("value", value),
        ("id", id.as_str()),
        ("class", "repository-filter-input"),
    ];
    if checked {
        attributes.push(("checked", "checked"));
    }
    vec![
        element("input", &attributes, vec![]),
        element(
            "label",
            &[("for", id.as_str()), ("class", "repository-filter-label")],
            vec![
                txt(label),
                element(
                    "span",
                    &[("aria-hidden", "true")],
                    vec![txt(count.to_string())],
                ),
            ],
        ),
    ]
}

pub(super) fn repository_card(
    repository: &RepositoryRecord,
    metadata: &PublicRepositoryMetadata,
    repositories_by_id: &BTreeMap<&str, &RepositoryRecord>,
    relations: &[RelationRecord],
) -> SiteView {
    let article_id = format!("repo-{}", repository.id);
    let class = format!(
        "repository-card class-{} status-{}",
        repository.class.slug(),
        repository.status.slug()
    );
    let github_url = format!("https://github.com/{}", repository.github_slug);
    let profile_href = format!("/projects/{}/", repository.id);
    let outgoing: Vec<_> = relations
        .iter()
        .filter(|relation| relation.source == repository.id)
        .collect();
    let incoming: Vec<_> = relations
        .iter()
        .filter(|relation| relation.target == repository.id)
        .collect();

    element(
        "article",
        &[
            ("id", article_id.as_str()),
            ("class", class.as_str()),
            ("data-repository-id", repository.id.as_str()),
            ("data-project-href", profile_href.as_str()),
            ("data-class", repository.class.slug()),
            ("data-status", repository.status.slug()),
        ],
        vec![
            element(
                "header",
                &[("class", "repository-card-header")],
                vec![
                    element(
                        "div",
                        &[],
                        vec![
                            element(
                                "p",
                                &[("class", "repository-slug")],
                                vec![txt(&repository.github_slug)],
                            ),
                            element(
                                "h2",
                                &[],
                                vec![link(
                                    &profile_href,
                                    &repository.name,
                                    "repository-profile-link",
                                )],
                            ),
                        ],
                    ),
                    element(
                        "div",
                        &[
                            ("class", "repository-badges"),
                            ("aria-label", "Repository classification"),
                        ],
                        vec![
                            element(
                                "span",
                                &[("class", "repository-class-badge")],
                                vec![txt(repository.class.label())],
                            ),
                            element(
                                "span",
                                &[("class", "repository-status-badge")],
                                vec![txt(repository.status.label())],
                            ),
                        ],
                    ),
                ],
            ),
            element(
                "p",
                &[("class", "repository-summary")],
                vec![txt(&repository.summary)],
            ),
            repository_facts(repository, metadata),
            repository_topics(metadata),
            repository_links(repository, &github_url),
            element(
                "div",
                &[("class", "relationship-grid")],
                vec![
                    relationship_block(
                        repository,
                        "outgoing",
                        "Outgoing relationships",
                        &outgoing,
                        repositories_by_id,
                    ),
                    relationship_block(
                        repository,
                        "incoming",
                        "Incoming relationships",
                        &incoming,
                        repositories_by_id,
                    ),
                ],
            ),
        ],
    )
}

pub(super) fn repository_facts(
    repository: &RepositoryRecord,
    metadata: &PublicRepositoryMetadata,
) -> SiteView {
    let mut facts = vec![
        fact(format!(
            "GitHub updated {}",
            format_timestamp(&metadata.updated_at)
        )),
        fact(format!("license {}", repository.license)),
        fact(format!(
            "{} star{}",
            metadata.stargazer_count,
            if metadata.stargazer_count == 1 {
                ""
            } else {
                "s"
            }
        )),
    ];
    if let Some(language) = &metadata.primary_language {
        facts.insert(1, fact(language));
    }
    if metadata.fork {
        facts.push(fact("maintained fork"));
    }
    if metadata.archived {
        facts.push(fact("archived"));
    }
    element(
        "ul",
        &[
            ("class", "repository-facts"),
            ("aria-label", "Public repository metadata"),
        ],
        facts,
    )
}

pub(super) fn fact(value: impl Into<String>) -> SiteView {
    element("li", &[], vec![txt(value)])
}

pub(super) fn repository_topics(metadata: &PublicRepositoryMetadata) -> SiteView {
    element(
        "div",
        &[("class", "repository-topics")],
        vec![
            element("h3", &[("class", "sr-only")], vec![txt("GitHub topics")]),
            element(
                "ul",
                &[],
                metadata
                    .topics
                    .iter()
                    .map(|topic| element("li", &[], vec![txt(topic)]))
                    .collect(),
            ),
        ],
    )
}

pub(super) fn repository_links(repository: &RepositoryRecord, github_url: &str) -> SiteView {
    let profile_href = format!("/projects/{}/", repository.id);
    let mut links = vec![
        link(&profile_href, "Merely profile", "repository-link"),
        external_link(github_url, "GitHub ↗", "repository-link"),
    ];
    if repository.homepage != github_url {
        links.push(external_link(
            &repository.homepage,
            "Project site ↗",
            "repository-link",
        ));
    }
    element(
        "nav",
        &[
            ("class", "repository-links"),
            ("aria-label", "Repository links"),
        ],
        links,
    )
}

pub(super) fn relationship_block(
    repository: &RepositoryRecord,
    direction: &str,
    heading: &str,
    relations: &[&RelationRecord],
    repositories_by_id: &BTreeMap<&str, &RepositoryRecord>,
) -> SiteView {
    let heading_id = format!("repo-{}-{direction}", repository.id);
    let content = if relations.is_empty() {
        vec![element(
            "p",
            &[("class", "relationship-empty")],
            vec![txt("No recorded relationships in this direction.")],
        )]
    } else {
        vec![element(
            "ul",
            &[("class", "relationship-list")],
            relations
                .iter()
                .map(|relation| relationship_item(relation, direction, repositories_by_id))
                .collect(),
        )]
    };
    let mut children = vec![element(
        "h3",
        &[("id", heading_id.as_str())],
        vec![txt(heading)],
    )];
    children.extend(content);
    element(
        "section",
        &[
            ("class", "relationship-block"),
            ("aria-labelledby", heading_id.as_str()),
        ],
        children,
    )
}

pub(super) fn relationship_item(
    relation: &RelationRecord,
    direction: &str,
    repositories_by_id: &BTreeMap<&str, &RepositoryRecord>,
) -> SiteView {
    let other_id = if direction == "outgoing" {
        relation.target.as_str()
    } else {
        relation.source.as_str()
    };
    let other = repositories_by_id
        .get(other_id)
        .expect("validated relation repository");
    let href = format!("#repo-{}", other.id);
    let verb = if direction == "outgoing" {
        relation.kind.label()
    } else {
        relation.kind.incoming_label()
    };
    let provenance_class = format!(
        "provenance-badge provenance-{}",
        relation.provenance.label()
    );

    let sentence = if direction == "outgoing" {
        vec![
            element("span", &[("class", "relationship-verb")], vec![txt(verb)]),
            element("a", &[("href", href.as_str())], vec![txt(&other.name)]),
        ]
    } else {
        vec![
            element("a", &[("href", href.as_str())], vec![txt(&other.name)]),
            element("span", &[("class", "relationship-verb")], vec![txt(verb)]),
        ]
    };
    let mut children = sentence;
    children.push(element(
        "span",
        &[("class", provenance_class.as_str())],
        vec![txt(relation.provenance.label())],
    ));
    element(
        "li",
        &[
            ("data-relation-id", relation.id.as_str()),
            ("data-provenance", relation.provenance.label()),
        ],
        children,
    )
}
