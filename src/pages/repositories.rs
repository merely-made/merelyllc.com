use std::collections::BTreeMap;
use std::path::Path;

use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::repositories::{
    AuthorityError, PublicOrganizationEvent, PublicRepositoryMetadata, PublicSiteData,
    RelationRecord, RepositoryClass, RepositoryRecord,
};
use crate::repository_history::{
    GitAuthorityHistoryProjection, RepositoryGraph, public_history_projection,
};
use crate::site::{
    ActivePage, PageMetadata, SiteView, element, external_link, link, render_with_body_end,
    section_heading, shell, txt,
};

pub const METADATA: PageMetadata = PageMetadata {
    title: "Repository family | Merely",
    description: "Explore Merely's public repositories, their current status, and the concrete relationships among them.",
    canonical_url: "https://mer3ly.net/repos/",
};

const REPO_GRAPH_LOADER: &[u8] = include_bytes!("../../assets/repo-graph.js");
const REPO_GRAPH_WASM_GLUE: &[u8] = include_bytes!("../../assets/mer3ly_repo_graph.js");
const REPO_GRAPH_WASM: &[u8] = include_bytes!("../../assets/mer3ly_repo_graph_bg.wasm");
const HISTORY_POINT_LIMIT: usize = 24;

pub fn document(root: &Path) -> Result<String, AuthorityError> {
    let data = PublicSiteData::load(root)?;
    let graph = RepositoryGraph::from_parts(
        &data.authority.repositories,
        &data.authority.relations,
        &data.metadata,
    )
    .map_err(AuthorityError::from_message)?;
    let history = public_history_projection(root, graph, HISTORY_POINT_LIMIT)
        .map_err(AuthorityError::from_message)?;
    Ok(document_with_history(&data, Some(&history)))
}

pub fn document_for(data: &PublicSiteData) -> String {
    document_with_history(data, None)
}

fn document_with_history(
    data: &PublicSiteData,
    history: Option<&GitAuthorityHistoryProjection>,
) -> String {
    let bootstrap = graph_bootstrap(data, history);
    render_with_body_end(&METADATA, move || view(data), &bootstrap)
}

pub fn view(data: &PublicSiteData) -> SiteView {
    shell(
        ActivePage::Repositories,
        element(
            "main",
            &[("id", "main"), ("class", "repositories-main")],
            vec![
                hero(data),
                repository_graph(data),
                organization_activity(data),
                repository_index(data),
                source_note(data),
            ],
        ),
    )
}

fn hero(data: &PublicSiteData) -> SiteView {
    let repositories = &data.authority.repositories.repository;
    let relations = &data.authority.relations.relation;
    let curated = relations
        .iter()
        .filter(|relation| relation.provenance.label() == "curated")
        .count();
    let derived = relations.len() - curated;

    element(
        "section",
        &[
            ("class", "hero repositories-hero"),
            ("aria-labelledby", "repositories-title"),
        ],
        vec![
            element(
                "p",
                &[("class", "eyebrow")],
                vec![txt("Public work · one typed authority")],
            ),
            element(
                "h1",
                &[("id", "repositories-title")],
                vec![txt("The repositories, and how they fit together.")],
            ),
            element(
                "p",
                &[("class", "hero-copy")],
                vec![txt(
                    "Every project below is public. The links among them are shown as ordinary text first, so the family remains legible without JavaScript, WebGPU, or a graph canvas.",
                )],
            ),
            element(
                "dl",
                &[("class", "repository-overview")],
                vec![
                    overview_stat("repositories", repositories.len()),
                    overview_stat("relationships", relations.len()),
                    overview_stat("derived edges", derived),
                    overview_stat("curated edges", curated),
                ],
            ),
        ],
    )
}

fn repository_graph(data: &PublicSiteData) -> SiteView {
    element(
        "section",
        &[
            ("class", "content-section repository-graph-section"),
            ("aria-label", "Live repository relationship map"),
        ],
        vec![
            section_heading("01", "live relationship map"),
            element(
                "div",
                &[("class", "repository-graph-heading")],
                vec![
                    element(
                        "h3",
                        &[],
                        vec![txt("A small Mere canvas for the public project family.")],
                    ),
                    element(
                        "p",
                        &[],
                        vec![txt(
                            "GitHub supplies the live membership and latest push dates; Mer3ly supplies the project roles and relationships. Choose Timeline to arrange the family by its latest public pushes.",
                        )],
                    ),
                ],
            ),
            element(
                "div",
                &[
                    ("class", "repository-graph-shell"),
                    ("data-repository-graph", ""),
                    ("data-graph-state", "pending"),
                ],
                vec![
                    element(
                        "p",
                        &[
                            ("class", "repository-graph-fallback"),
                            ("data-graph-fallback", ""),
                        ],
                        vec![txt(
                            "The interactive map requires WebGPU and WebAssembly. The complete repository index remains available below.",
                        )],
                    ),
                    element(
                        "div",
                        &[
                            ("class", "repository-graph-interface"),
                            ("data-graph-interface", ""),
                            ("hidden", "hidden"),
                        ],
                        vec![
                            graph_toolbar(),
                            element(
                                "div",
                                &[
                                    ("class", "repository-graph-stage"),
                                    ("data-graph-stage", ""),
                                ],
                                vec![
                                    element(
                                        "canvas",
                                        &[
                                            ("class", "repository-graph-canvas"),
                                            ("aria-hidden", "true"),
                                        ],
                                        vec![],
                                    ),
                                    element(
                                        "div",
                                        &[
                                            ("class", "repository-graph-nodes"),
                                            ("data-graph-nodes", ""),
                                            ("role", "group"),
                                            ("aria-label", "Repository graph nodes"),
                                        ],
                                        vec![],
                                    ),
                                ],
                            ),
                            graph_legend(data),
                        ],
                    ),
                    element(
                        "p",
                        &[
                            ("class", "repository-graph-status sr-only"),
                            ("data-graph-status", ""),
                            ("aria-live", "polite"),
                        ],
                        vec![txt("Interactive repository map not initialized.")],
                    ),
                ],
            ),
        ],
    )
}

fn organization_activity(data: &PublicSiteData) -> SiteView {
    let repositories_by_slug: BTreeMap<_, _> = data
        .authority
        .repositories
        .repository
        .iter()
        .map(|repository| (repository.github_slug.as_str(), repository))
        .collect();
    let events = data
        .metadata
        .event
        .iter()
        .take(12)
        .filter_map(|event| {
            repositories_by_slug
                .get(event.repository.as_str())
                .map(|repository| activity_item(event, repository))
        })
        .collect();

    element(
        "section",
        &[
            ("class", "content-section repository-activity-section"),
            ("aria-label", "Recent merely-made GitHub activity"),
        ],
        vec![
            section_heading("02", "recent organization activity"),
            element(
                "p",
                &[("class", "repository-activity-intro")],
                vec![txt(
                    "A reduced view of the public merely-made GitHub feed. Each deployment refreshes repository membership, metadata, and these events before rebuilding the graph.",
                )],
            ),
            element("ol", &[("class", "repository-activity-feed")], events),
        ],
    )
}

fn activity_item(event: &PublicOrganizationEvent, repository: &RepositoryRecord) -> SiteView {
    let project_href = format!("/projects/{}/", repository.id);
    element(
        "li",
        &[
            ("class", "repository-activity-item"),
            ("data-activity-kind", event.kind.as_str()),
            ("data-activity-repository", repository.id.as_str()),
        ],
        vec![
            element(
                "span",
                &[("class", "repository-activity-statement")],
                vec![
                    link(
                        &project_href,
                        &repository.name,
                        "repository-activity-project",
                    ),
                    txt(format!(" {}", activity_label(&event.kind))),
                ],
            ),
            element(
                "time",
                &[
                    ("class", "repository-activity-time"),
                    ("datetime", event.created_at.as_str()),
                ],
                vec![txt(format_timestamp(&event.created_at))],
            ),
        ],
    )
}

fn activity_label(kind: &str) -> &'static str {
    match kind {
        "PushEvent" => "pushed code",
        "PullRequestEvent" => "updated a pull request",
        "CreateEvent" => "created a branch or tag",
        "DeleteEvent" => "deleted a branch or tag",
        "ReleaseEvent" => "published a release",
        "IssuesEvent" => "updated an issue",
        "IssueCommentEvent" => "commented on an issue",
        "ForkEvent" => "created a fork",
        "WatchEvent" => "received a star",
        _ => "recorded public activity",
    }
}

fn graph_toolbar() -> SiteView {
    element(
        "div",
        &[
            ("class", "repository-graph-toolbar"),
            ("data-graph-controls", ""),
            ("aria-label", "Repository map controls"),
        ],
        vec![
            element(
                "label",
                &[("class", "repository-graph-arrangement-picker")],
                vec![
                    element("span", &[], vec![txt("Arrangement")]),
                    element(
                        "select",
                        &[
                            ("data-graph-arrangement", ""),
                            ("aria-label", "Repository graph arrangement"),
                        ],
                        vec![],
                    ),
                ],
            ),
            element(
                "div",
                &[
                    ("class", "repository-graph-history-picker"),
                    ("data-graph-history-controls", ""),
                    ("role", "group"),
                    ("aria-label", "Repository graph source history"),
                    ("hidden", "hidden"),
                ],
                vec![
                    element("span", &[], vec![txt("Source time")]),
                    element(
                        "input",
                        &[
                            ("type", "range"),
                            ("min", "0"),
                            ("max", "0"),
                            ("value", "0"),
                            ("step", "1"),
                            ("data-graph-history", ""),
                            ("aria-label", "Repository graph source time"),
                        ],
                        vec![],
                    ),
                    element(
                        "output",
                        &[("data-graph-history-status", "")],
                        vec![txt("Live authority")],
                    ),
                    graph_button("return-live", "Return to live authority", "live"),
                ],
            ),
            graph_button("share", "Copy shareable repository scene link", "share"),
            element(
                "p",
                &[
                    ("class", "repository-graph-scene-caption"),
                    ("data-graph-scene-caption", ""),
                    ("aria-live", "polite"),
                ],
                vec![txt(
                    "Constellation medallions · relationships remain fully drawn",
                )],
            ),
            element(
                "div",
                &[
                    ("class", "repository-graph-control-group"),
                    ("role", "group"),
                    ("aria-label", "Zoom controls"),
                ],
                vec![
                    graph_button("zoom-out", "Zoom out", "−"),
                    graph_button("fit", "Fit the graph", "fit"),
                    graph_button("zoom-in", "Zoom in", "+"),
                ],
            ),
            element(
                "div",
                &[
                    ("class", "repository-graph-control-group graph-pan-controls"),
                    ("role", "group"),
                    ("aria-label", "Pan controls"),
                ],
                vec![
                    graph_button("pan-left", "Pan left", "←"),
                    graph_button("pan-up", "Pan up", "↑"),
                    graph_button("pan-down", "Pan down", "↓"),
                    graph_button("pan-right", "Pan right", "→"),
                ],
            ),
            graph_button("open", "Open selected project profile", "open selected"),
        ],
    )
}

fn graph_button(action: &str, label: &str, text: &str) -> SiteView {
    element(
        "button",
        &[
            ("type", "button"),
            ("class", "repository-graph-control"),
            ("data-graph-action", action),
            ("aria-label", label),
        ],
        vec![txt(text)],
    )
}

fn graph_legend(data: &PublicSiteData) -> SiteView {
    let mut relation_kinds = data
        .authority
        .relations
        .relation
        .iter()
        .map(|relation| relation.kind)
        .collect::<Vec<_>>();
    relation_kinds.sort_by_key(|kind| kind.slug());
    relation_kinds.dedup();

    element(
        "aside",
        &[
            ("class", "repository-graph-legend"),
            ("aria-label", "Repository map legend"),
        ],
        vec![
            element(
                "div",
                &[],
                vec![
                    element("h3", &[], vec![txt("Repository role")]),
                    element(
                        "ul",
                        &[],
                        [
                            RepositoryClass::Product,
                            RepositoryClass::Platform,
                            RepositoryClass::Foundation,
                            RepositoryClass::Tool,
                        ]
                        .into_iter()
                        .map(|class| {
                            element(
                                "li",
                                &[],
                                vec![
                                    element(
                                        "span",
                                        &[(
                                            "class",
                                            &format!("graph-legend-node class-{}", class.slug()),
                                        )],
                                        vec![],
                                    ),
                                    txt(class.label()),
                                ],
                            )
                        })
                        .collect(),
                    ),
                ],
            ),
            element(
                "div",
                &[],
                vec![
                    element("h3", &[], vec![txt("Relationship")]),
                    element(
                        "ul",
                        &[],
                        relation_kinds
                            .into_iter()
                            .map(|kind| {
                                element(
                                    "li",
                                    &[],
                                    vec![
                                        element(
                                            "span",
                                            &[(
                                                "class",
                                                &format!("graph-legend-edge kind-{}", kind.slug()),
                                            )],
                                            vec![],
                                        ),
                                        txt(kind.label()),
                                    ],
                                )
                            })
                            .collect(),
                    ),
                ],
            ),
            element(
                "div",
                &[],
                vec![
                    element("h3", &[], vec![txt("Evidence")]),
                    element(
                        "ul",
                        &[],
                        vec![
                            element(
                                "li",
                                &[],
                                vec![
                                    element(
                                        "span",
                                        &[("class", "provenance-badge provenance-derived")],
                                        vec![txt("derived")],
                                    ),
                                    txt(" manifests"),
                                ],
                            ),
                            element(
                                "li",
                                &[],
                                vec![
                                    element(
                                        "span",
                                        &[("class", "provenance-badge provenance-curated")],
                                        vec![txt("curated")],
                                    ),
                                    txt(" documentation"),
                                ],
                            ),
                        ],
                    ),
                ],
            ),
        ],
    )
}

fn overview_stat(label: &str, value: usize) -> SiteView {
    element(
        "div",
        &[("class", "overview-stat")],
        vec![
            element("dt", &[], vec![txt(label)]),
            element("dd", &[], vec![txt(value.to_string())]),
        ],
    )
}

fn repository_index(data: &PublicSiteData) -> SiteView {
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

fn repository_filters(repositories: &[RepositoryRecord], cards: Vec<SiteView>) -> SiteView {
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

fn class_count(repositories: &[RepositoryRecord], class: RepositoryClass) -> usize {
    repositories
        .iter()
        .filter(|repository| repository.public && repository.class == class)
        .count()
}

fn filter_input(value: &str, label: &str, count: usize, checked: bool) -> Vec<SiteView> {
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

fn relation_key() -> SiteView {
    element(
        "aside",
        &[
            ("class", "relation-key"),
            ("aria-label", "Relationship key"),
        ],
        vec![
            element(
                "p",
                &[],
                vec![
                    element(
                        "span",
                        &[("class", "provenance-badge provenance-derived")],
                        vec![txt("derived")],
                    ),
                    txt(" read from dependency manifests"),
                ],
            ),
            element(
                "p",
                &[],
                vec![
                    element(
                        "span",
                        &[("class", "provenance-badge provenance-curated")],
                        vec![txt("curated")],
                    ),
                    txt(" recorded from project documentation"),
                ],
            ),
        ],
    )
}

fn repository_card(
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

fn repository_facts(
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

fn fact(value: impl Into<String>) -> SiteView {
    element("li", &[], vec![txt(value)])
}

fn repository_topics(metadata: &PublicRepositoryMetadata) -> SiteView {
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

fn repository_links(repository: &RepositoryRecord, github_url: &str) -> SiteView {
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

fn relationship_block(
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

fn relationship_item(
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

fn source_note(data: &PublicSiteData) -> SiteView {
    element(
        "section",
        &[
            ("class", "repository-source-note"),
            ("aria-labelledby", "repository-source-title"),
        ],
        vec![
            element(
                "p",
                &[("class", "eyebrow")],
                vec![txt("public data boundary")],
            ),
            element(
                "h2",
                &[("id", "repository-source-title")],
                vec![txt("Two inputs. One public projection.")],
            ),
            element(
                "p",
                &[],
                vec![txt(format!(
                    "GitHub owns public organization membership, repository facts, push dates, and activity. Mer3ly owns the editorial roles, summaries, statuses, and meaningful relationships layered over that roster. Both the semantic index and the Mere canvas consume the reconciled result. Refreshed {}. If a future refresh fails, the last validated snapshot remains in place.",
                    format_timestamp(&data.metadata.generated_at_utc)
                ))],
            ),
        ],
    )
}

fn graph_bootstrap(
    data: &PublicSiteData,
    history: Option<&GitAuthorityHistoryProjection>,
) -> String {
    let authority = RepositoryGraph::from_parts(
        &data.authority.repositories,
        &data.authority.relations,
        &data.metadata,
    )
    .expect("validated public site data projects a repository graph");
    let schema =
        serde_json::to_string(&authority.schema).expect("repository graph schema is serializable");
    let nodes = serialize_json_records(&authority.nodes);
    let edges = serialize_json_records(&authority.edges);
    let feed = serialize_json_records(&data.metadata.event);
    let mut json = format!(
        "{{\n\"schema\":{schema},\n\"nodes\":[\n{nodes}\n],\n\"edges\":[\n{edges}\n],\n\"feed\":[\n{feed}\n]"
    );
    if let Some(history) = history {
        let history_schema = serde_json::to_string(&history.schema)
            .expect("repository history schema is serializable");
        let checkpoints = serialize_json_records(&history.checkpoints);
        json.push_str(&format!(
            ",\n\"history\":{{\n\"schema\":{history_schema},\n\"checkpoints\":[\n{checkpoints}\n]\n}}"
        ));
    }
    json.push_str("\n}");
    let json = json
        .replace('<', "\\u003c")
        .replace('>', "\\u003e")
        .replace('&', "\\u0026");
    let graph_runtime_href = graph_runtime_href();
    format!(
        "<script id=\"repository-graph-data\" type=\"application/json\">{json}</script>\n\
<script type=\"module\" src=\"{graph_runtime_href}\"></script>"
    )
}

fn serialize_json_records<T: Serialize>(records: &[T]) -> String {
    records
        .iter()
        .map(|record| {
            serde_json::to_string(record)
                .expect("repository graph authority contains serializable records")
        })
        .collect::<Vec<_>>()
        .join(",\n")
}

fn graph_runtime_href() -> String {
    let mut digest = Sha256::new();
    digest.update(REPO_GRAPH_LOADER);
    digest.update(REPO_GRAPH_WASM_GLUE);
    digest.update(REPO_GRAPH_WASM);
    let digest = format!("{:x}", digest.finalize());
    format!("/repo-graph.js?v={}", &digest[..12])
}

fn format_timestamp(value: &str) -> String {
    format!("{} {} UTC", &value[..10], &value[11..16])
}
