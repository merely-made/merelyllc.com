use std::collections::BTreeMap;
use std::path::Path;

use serde::Serialize;
use serde_json::json;
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

const GRAPH_SANDBOX_LOADER: &[u8] = include_bytes!("../../assets/graph-sandbox.js");
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
                graph_sandbox(),
                organization_activity(data),
                repository_index(data),
                source_note(data),
            ],
        ),
    )
}

fn graph_sandbox() -> SiteView {
    element(
        "section",
        &[
            ("class", "content-section graph-sandbox-section"),
            ("aria-labelledby", "graph-sandbox-title"),
            ("data-graph-sandbox", ""),
            ("data-sandbox-state", "pending"),
        ],
        vec![
            section_heading("01", "graphshell sandbox"),
            element(
                "div",
                &[("class", "graph-sandbox-heading")],
                vec![
                    element(
                        "h3",
                        &[("id", "graph-sandbox-title")],
                        vec![txt("The graph is also its own control surface.")],
                    ),
                    element(
                        "p",
                        &[],
                        vec![txt(
                            "Switch the graph from inside the graph. Reading changes membership and face; arrangement changes placement; anchored or free motion decides what happens after you move a node. The typed actors and relations remain the same beneath every projection.",
                        )],
                    ),
                ],
            ),
            element(
                "p",
                &[
                    ("class", "graph-sandbox-fallback"),
                    ("data-sandbox-fallback", ""),
                ],
                vec![txt(
                    "The sandbox requires WebAssembly. Its semantic repository index remains available below.",
                )],
            ),
            element(
                "div",
                &[
                    ("class", "graph-sandbox-shell"),
                    ("data-sandbox-interface", ""),
                    ("hidden", "hidden"),
                ],
                vec![
                    element(
                        "div",
                        &[
                            ("class", "graph-sandbox-stage"),
                            ("data-sandbox-stage", ""),
                            ("data-sandbox-scene", "graph"),
                            ("data-sandbox-backdrop", "ambient"),
                        ],
                        vec![
                            element(
                                "canvas",
                                &[
                                    ("class", "graph-sandbox-canvas"),
                                    ("data-sandbox-canvas", ""),
                                    ("aria-hidden", "true"),
                                ],
                                vec![],
                            ),
                            element(
                                "div",
                                &[
                                    ("class", "graph-sandbox-nodes"),
                                    ("data-sandbox-nodes", ""),
                                    ("role", "group"),
                                    ("aria-label", "Graph sandbox nodes"),
                                ],
                                vec![],
                            ),
                            sandbox_graph_controls(),
                            sandbox_scene_tools(),
                            element(
                                "p",
                                &[
                                    ("class", "graph-sandbox-caption"),
                                    ("data-sandbox-caption", ""),
                                ],
                                vec![txt("Loading the graph runtime…")],
                            ),
                        ],
                    ),
                    sandbox_receipt_views(),
                ],
            ),
            element(
                "p",
                &[
                    ("class", "sr-only"),
                    ("data-sandbox-status", ""),
                    ("aria-live", "polite"),
                ],
                vec![txt("Graphshell sandbox not initialized.")],
            ),
            element(
                "p",
                &[("class", "graph-sandbox-range-note")],
                vec![txt(
                    "A Mermaid diagram or spreadsheet chart can be another projection of the same graph. Their boxes, bars, axes, lanes, and labels are faces and scene marks; a frozen export simply omits the interaction layer.",
                )],
            ),
        ],
    )
}

fn sandbox_receipt_views() -> SiteView {
    element(
        "aside",
        &[
            ("class", "graph-sandbox-receipts"),
            ("aria-label", "Coordinated projection receipts"),
        ],
        vec![
            element(
                "section",
                &[("class", "graph-sandbox-receipt")],
                vec![
                    element("h4", &[], vec![txt("Matrix")]),
                    element(
                        "p",
                        &[("class", "graph-sandbox-receipt-note")],
                        vec![txt("Neighborhood rows crossed with change columns.")],
                    ),
                    element(
                        "button",
                        &[
                            ("type", "button"),
                            ("class", "graph-sandbox-clear-filter"),
                            ("data-sandbox-clear-matrix", ""),
                        ],
                        vec![txt("clear Matrix filter")],
                    ),
                    element(
                        "button",
                        &[
                            ("type", "button"),
                            ("class", "graph-sandbox-clear-filter"),
                            ("data-sandbox-clear-facets", ""),
                        ],
                        vec![txt("clear selected facets")],
                    ),
                    element(
                        "div",
                        &[
                            ("class", "graph-sandbox-matrix"),
                            ("data-sandbox-matrix", ""),
                        ],
                        vec![],
                    ),
                ],
            ),
            element(
                "section",
                &[("class", "graph-sandbox-receipt")],
                vec![
                    element("h4", &[], vec![txt("Scatter")]),
                    element(
                        "div",
                        &[
                            ("class", "graph-sandbox-scatter"),
                            ("data-sandbox-scatter", ""),
                            ("role", "group"),
                            ("aria-label", "Scatter appearances"),
                        ],
                        vec![],
                    ),
                ],
            ),
            element(
                "section",
                &[("class", "graph-sandbox-receipt")],
                vec![
                    element("h4", &[], vec![txt("Deck")]),
                    element(
                        "div",
                        &[
                            ("class", "graph-sandbox-deck"),
                            ("data-sandbox-deck", ""),
                            ("role", "list"),
                        ],
                        vec![],
                    ),
                ],
            ),
            element(
                "p",
                &[("class", "sr-only"), ("id", "graph-source-description")],
                vec![txt(
                    "Each appearance addresses the same repository source in a distinct projected view.",
                )],
            ),
        ],
    )
}

fn sandbox_graph_controls() -> SiteView {
    element(
        "div",
        &[
            ("class", "graph-sandbox-control-actors"),
            ("role", "group"),
            ("aria-label", "Graph controls"),
        ],
        vec![
            sandbox_control_actor("dataset", "Source", "merely-made feed"),
            sandbox_control_actor("reading", "Reading", "Graph"),
            sandbox_control_actor("arrangement", "Arrangement", "Stack"),
            sandbox_control_actor("mobility", "Motion", "Anchored"),
            sandbox_control_actor("environment", "Field", "Ambient"),
        ],
    )
}

fn sandbox_control_actor(name: &str, label: &str, value: &str) -> SiteView {
    element(
        "button",
        &[
            ("type", "button"),
            ("class", "graph-sandbox-control-actor"),
            ("data-sandbox-cycle", name),
        ],
        vec![
            element(
                "span",
                &[("class", "graph-sandbox-control-kind")],
                vec![txt(label)],
            ),
            element(
                "span",
                &[
                    ("class", "graph-sandbox-control-value"),
                    ("data-sandbox-cycle-value", ""),
                ],
                vec![txt(value)],
            ),
        ],
    )
}

fn sandbox_scene_tools() -> SiteView {
    element(
        "div",
        &[("class", "graph-sandbox-scene-tools")],
        vec![
            element(
                "label",
                &[
                    ("class", "graph-sandbox-history"),
                    ("data-sandbox-history-control", ""),
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
                            ("data-sandbox-history", ""),
                            ("aria-label", "Sandbox source time"),
                        ],
                        vec![],
                    ),
                    element(
                        "output",
                        &[("data-sandbox-history-status", "")],
                        vec![txt("loading source")],
                    ),
                ],
            ),
            element(
                "div",
                &[
                    ("class", "graph-sandbox-camera-tools"),
                    ("role", "group"),
                    ("aria-label", "View camera"),
                ],
                vec![
                    element("span", &[], vec![txt("Camera")]),
                    camera_control("pan-left", "Pan left"),
                    camera_control("pan-right", "Pan right"),
                    camera_control("pan-up", "Pan up"),
                    camera_control("pan-down", "Pan down"),
                    camera_control("zoom-out", "Zoom out"),
                    camera_control("zoom-in", "Zoom in"),
                    camera_control("reset", "Reset camera"),
                ],
            ),
            element(
                "button",
                &[
                    ("type", "button"),
                    ("class", "graph-sandbox-share"),
                    ("data-sandbox-share", ""),
                ],
                vec![txt("share scene")],
            ),
            element(
                "span",
                &[
                    ("class", "graph-sandbox-share-status"),
                    ("data-sandbox-share-status", ""),
                ],
                vec![txt("state stays in the link")],
            ),
            element(
                "button",
                &[
                    ("type", "button"),
                    ("class", "graph-sandbox-share"),
                    ("data-sandbox-export", ""),
                ],
                vec![txt("export projection")],
            ),
            element(
                "span",
                &[
                    ("class", "graph-sandbox-share-status"),
                    ("data-sandbox-export-status", ""),
                    ("aria-live", "polite"),
                ],
                vec![txt("the realized scene, not the link")],
            ),
        ],
    )
}

fn camera_control(action: &'static str, label: &'static str) -> SiteView {
    element(
        "button",
        &[
            ("type", "button"),
            ("class", "graph-sandbox-camera-control"),
            ("data-sandbox-camera", action),
        ],
        vec![txt(label)],
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
    let sandbox_runtime_href = graph_sandbox_runtime_href();
    let sandbox_json = graph_sandbox_json();
    format!(
        "<script id=\"repository-graph-data\" type=\"application/json\">{json}</script>\n\
<script id=\"graph-sandbox-data\" type=\"application/json\">{sandbox_json}</script>\n\
<script type=\"module\" src=\"{sandbox_runtime_href}\"></script>"
    )
}

fn graph_sandbox_json() -> String {
    serde_json::to_string_pretty(&json!({
        "schema": "mer3ly.repo-graph/v1",
        "focus": "merecat",
        "nodes": [
            {"id":"merecat","name":"merecat","class":"product","status":"live","pushed_at":"2026-08-11T16:20:00Z","change":"updated","summary":"A graph browser hosted on the local device."},
            {"id":"mere","name":"Mere","class":"platform","status":"live","pushed_at":"2026-08-10T09:12:00Z","change":"updated","summary":"The modular graph GUI and canvas library."},
            {"id":"turnstone","name":"Turnstone","class":"device","status":"present","pushed_at":"2026-08-07T14:30:00Z","change":"stable","summary":"A physical host in the trusted device group."},
            {"id":"ashland","name":"Ashland","class":"place","status":"present","pushed_at":"2026-07-29T12:00:00Z","change":"stable","summary":"A place node, drawn and collided as a circle."},
            {"id":"merely-made","name":"merely-made","class":"community","status":"live","pushed_at":"2026-08-11T15:42:00Z","change":"added","summary":"A public organization and software community."},
            {"id":"mark","name":"Mark","class":"person","status":"present","pushed_at":"2026-08-11T15:00:00Z","change":"stable","summary":"A person actor, distinct from their devices and projects."},
            {"id":"field-notes","name":"Field notes","class":"document","status":"draft","pushed_at":"2026-08-09T18:05:00Z","change":"updated","summary":"A square document primitive carrying observations."},
            {"id":"radio-session","name":"Radio session","class":"event","status":"past","pushed_at":"2026-08-05T20:00:00Z","change":"stable","summary":"A time-bound event represented as a diamond."},
            {"id":"relay","name":"Neighborhood relay","class":"device","status":"present","pushed_at":"2026-08-06T08:30:00Z","change":"added","summary":"A peer radio carrying local messages."},
            {"id":"message","name":"Shared message","class":"note","status":"received","pushed_at":"2026-08-06T08:33:00Z","change":"added","summary":"A small piece of content moving between peers."},
            {"id":"strophe","name":"Strophe","class":"software","status":"research","pushed_at":"2026-08-08T11:14:00Z","change":"stable","summary":"A sibling software system sharing the same surface stack."},
            {"id":"old-mock","name":"Old mock","class":"page","status":"retired","pushed_at":"2026-07-18T10:10:00Z","change":"removed","summary":"A removed representation retained in the changes scene."}
        ],
        "edges": [
            {"id":"merecat-uses-mere","source":"merecat","target":"mere","kind":"uses","provenance":"curated"},
            {"id":"turnstone-hosts-merecat","source":"turnstone","target":"merecat","kind":"hosts","provenance":"curated"},
            {"id":"mark-builds-mere","source":"mark","target":"mere","kind":"builds","provenance":"curated"},
            {"id":"mark-member-merely","source":"mark","target":"merely-made","kind":"member_of","provenance":"curated"},
            {"id":"merely-publishes-mere","source":"merely-made","target":"mere","kind":"publishes","provenance":"derived"},
            {"id":"strophe-shares-mere","source":"strophe","target":"mere","kind":"shares_stack","provenance":"curated"},
            {"id":"session-occurs-ashland","source":"radio-session","target":"ashland","kind":"occurs_in","provenance":"curated"},
            {"id":"mark-attends-session","source":"mark","target":"radio-session","kind":"participates","provenance":"curated"},
            {"id":"session-produces-notes","source":"radio-session","target":"field-notes","kind":"produces","provenance":"curated"},
            {"id":"relay-carries-message","source":"relay","target":"message","kind":"carries","provenance":"derived"},
            {"id":"message-recorded-notes","source":"message","target":"field-notes","kind":"recorded_in","provenance":"curated"},
            {"id":"relay-near-ashland","source":"relay","target":"ashland","kind":"located_in","provenance":"curated"},
            {"id":"old-mock-replaced-merecat","source":"old-mock","target":"merecat","kind":"replaced_by","provenance":"curated"}
        ],
        "sandbox": {
            "schema": "mer3ly.graphshell-sandbox/v5",
            "scene_state_schema": "mere.shelfmark/1",
            "reading_registry_schema": "mere.graph-reading-registry/v1",
            "representation_registry_schema": "mere.graph-representation-registry/v2",
            "reading_rule": "Mere selects actor scope, surface, emphasis, and an initial arrangement",
            "face_rule": "the host may give the same typed actor a different face for each reading",
            "primitive_rule": "Mere's registry maps class to one body shared by paint and collision",
            "behavior_rule": "named host bindings remain distinct from endpoint domain actions",
            "motion_rule": "interactive actors are anchored or free; frozen belongs to static renderers",
            "views": ["graph", "changes", "activity", "neighbors", "matrix"]
        }
    }))
    .expect("graph sandbox data is serializable")
    .replace('<', "\\u003c")
    .replace('>', "\\u003e")
    .replace('&', "\\u0026")
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

fn graph_sandbox_runtime_href() -> String {
    let mut digest = Sha256::new();
    digest.update(GRAPH_SANDBOX_LOADER);
    digest.update(REPO_GRAPH_WASM_GLUE);
    digest.update(REPO_GRAPH_WASM);
    let digest = format!("{:x}", digest.finalize());
    format!("/graph-sandbox.js?v={}", &digest[..12])
}

fn format_timestamp(value: &str) -> String {
    format!("{} {} UTC", &value[..10], &value[11..16])
}
