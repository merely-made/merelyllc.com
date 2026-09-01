use std::collections::BTreeMap;
use std::path::Path;

use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};

use crate::repositories::{
    AuthorityError, PublicRepositoryMetadata, PublicSiteData, RelationRecord, RepositoryRecord,
    ShowcaseRecord,
};
use crate::repository_history::RepositoryGraph;
use crate::site::{
    ActivePage, DEFAULT_SOCIAL_IMAGE_ALT, DEFAULT_SOCIAL_IMAGE_URL, DocumentMetadata,
    ORGANIZATION_ID, SiteView, SocialImage, WEBSITE_ID, base_schema_graph, element, external_link,
    json_ld_for_script, link, render_with_dynamic_and_body_end, section_heading, shell, txt,
};

const PROJECTION_PROOF: &[u8] = include_bytes!("../../assets/projection-proof.js");

pub fn documents(data: &PublicSiteData) -> Vec<(String, String)> {
    data.authority
        .repositories
        .repository
        .iter()
        .filter(|repository| repository.public)
        .map(|repository| (repository.id.clone(), document_for(data, repository)))
        .collect()
}

pub fn document(root: &Path, repository_id: &str) -> Result<String, AuthorityError> {
    let data = PublicSiteData::load(root)?;
    let repository = data
        .authority
        .repositories
        .repository
        .iter()
        .find(|repository| repository.public && repository.id == repository_id)
        .ok_or_else(|| {
            AuthorityError::from_message(format!(
                "unknown public project repository {repository_id}"
            ))
        })?;
    Ok(document_for(&data, repository))
}

pub fn document_for(data: &PublicSiteData, repository: &RepositoryRecord) -> String {
    let title = format!("{} | Merely", repository.name);
    let canonical = format!("https://mer3ly.net/projects/{}/", repository.id);
    let repository_metadata = data
        .metadata
        .repository
        .iter()
        .find(|metadata| metadata.id == repository.id);
    let showcase = data.showcases.for_repository(&repository.id);
    let image_url = showcase.map_or_else(
        || DEFAULT_SOCIAL_IMAGE_URL.to_owned(),
        |showcase| format!("https://mer3ly.net/{}", showcase.image),
    );
    let image_type = if showcase.is_some() {
        "image/png"
    } else {
        "image/jpeg"
    };
    let image_alt = showcase.map_or(DEFAULT_SOCIAL_IMAGE_ALT, |showcase| showcase.alt.as_str());
    let json_ld = project_json_ld(repository, repository_metadata, &canonical);
    let metadata = DocumentMetadata {
        title: &title,
        description: &repository.summary,
        canonical_url: &canonical,
        social_image: SocialImage {
            url: &image_url,
            mime_type: image_type,
            alt: image_alt,
        },
        json_ld: &json_ld,
    };
    let bootstrap = if repository.id == "mere" {
        projection_bootstrap(data)
    } else {
        String::new()
    };
    render_with_dynamic_and_body_end(&metadata, || view(data, repository), &bootstrap)
}

fn project_json_ld(
    repository: &RepositoryRecord,
    metadata: Option<&PublicRepositoryMetadata>,
    canonical: &str,
) -> String {
    let repository_url = format!("https://github.com/{}", repository.github_slug);
    let entity_id = format!("{canonical}#repository");
    let entity_type = if repository.id == "org-profile" {
        "CreativeWork"
    } else {
        "SoftwareSourceCode"
    };
    let mut entity = Map::from_iter([
        ("@type".to_owned(), Value::String(entity_type.to_owned())),
        ("@id".to_owned(), Value::String(entity_id.clone())),
        ("name".to_owned(), Value::String(repository.name.clone())),
        (
            "description".to_owned(),
            Value::String(repository.summary.clone()),
        ),
        ("url".to_owned(), Value::String(canonical.to_owned())),
        (
            "sameAs".to_owned(),
            Value::Array(vec![Value::String(repository_url.clone())]),
        ),
        ("publisher".to_owned(), json!({ "@id": ORGANIZATION_ID })),
    ]);
    if entity_type == "SoftwareSourceCode" {
        entity.insert("codeRepository".to_owned(), Value::String(repository_url));
        if let Some(language) = metadata.and_then(|metadata| metadata.primary_language.as_ref()) {
            entity.insert(
                "programmingLanguage".to_owned(),
                Value::String(language.clone()),
            );
        }
    }
    if let Some(metadata) = metadata
        && !metadata.topics.is_empty()
    {
        entity.insert(
            "keywords".to_owned(),
            Value::Array(
                metadata
                    .topics
                    .iter()
                    .map(|topic| Value::String(topic.clone()))
                    .collect(),
            ),
        );
    }

    let mut graph = base_schema_graph();
    graph.push(json!({
        "@type": "WebPage",
        "@id": canonical,
        "url": canonical,
        "name": repository.name,
        "description": repository.summary,
        "isPartOf": { "@id": WEBSITE_ID },
        "about": { "@id": entity_id }
    }));
    graph.push(Value::Object(entity));
    json_ld_for_script(&json!({
        "@context": "https://schema.org",
        "@graph": graph
    }))
}

pub fn view(data: &PublicSiteData, repository: &RepositoryRecord) -> SiteView {
    let metadata = data
        .metadata
        .repository
        .iter()
        .find(|metadata| metadata.id == repository.id);
    let showcase = data.showcases.for_repository(&repository.id);

    let mut sections = vec![
        hero(repository),
        showcase_section(showcase),
        place_in_family(data, repository),
    ];
    if repository.id == "mere" {
        sections.push(projection_proof_section());
    }
    sections.push(project_facts(
        if repository.id == "mere" { "04" } else { "03" },
        repository,
        metadata,
    ));
    sections.push(profile_closing(repository));

    shell(
        ActivePage::Repositories,
        element(
            "main",
            &[
                ("id", "main"),
                ("class", "project-profile-main"),
                ("data-project-id", repository.id.as_str()),
            ],
            sections,
        ),
    )
}

fn projection_proof_section() -> SiteView {
    element(
        "section",
        &[
            ("class", "content-section projection-proof-section"),
            ("aria-labelledby", "projection-proof-title"),
        ],
        vec![
            section_heading("03", "portable scene"),
            element(
                "div",
                &[("class", "projection-proof-heading")],
                vec![
                    element(
                        "div",
                        &[],
                        vec![
                            element(
                                "h2",
                                &[("id", "projection-proof-title")],
                                vec![txt("One portable scene. Two working projections.")],
                            ),
                            element(
                                "p",
                                &[],
                                vec![txt(
                                    "Move or select a project in either view. Remove a relationship from the scene, fold Mere's dependencies, scrub the revisioned trace, and both projections follow the same serialized state.",
                                )],
                            ),
                        ],
                    ),
                    element(
                        "p",
                        &[("class", "projection-proof-boundary")],
                        vec![txt(
                            "Scenograph supplies the score, solved scene, stable slots, and revisioned diffs · Mer3ly supplies the public-authority adapter",
                        )],
                    ),
                ],
            ),
            element(
                "figure",
                &[
                    ("class", "projection-proof"),
                    ("data-projection-proof", ""),
                    ("data-ready", "false"),
                    ("data-state", "pending"),
                    ("data-cursor", "0"),
                ],
                vec![
                    element(
                        "p",
                        &[
                            ("class", "projection-proof-fallback"),
                            ("data-projection-fallback", ""),
                        ],
                        vec![txt(
                            "The synchronized scene requires JavaScript. The relationship lists above preserve the same public nodes and edges as ordinary text.",
                        )],
                    ),
                    element(
                        "div",
                        &[
                            ("class", "projection-proof-interface"),
                            ("data-projection-interface", ""),
                            ("hidden", "hidden"),
                        ],
                        vec![
                            projection_proof_controls(),
                            element(
                                "div",
                                &[("class", "projection-proof-views")],
                                vec![
                                    projection_view(
                                        "canvas",
                                        "Canvas projection",
                                        "A full working view of the Mere repository neighborhood.",
                                    ),
                                    projection_view(
                                        "swatch",
                                        "Swatch projection",
                                        "The same scene in a compact, independently operable view.",
                                    ),
                                ],
                            ),
                            element(
                                "figcaption",
                                &[],
                                vec![txt(
                                    "Eight public projects and nine validated relationships enter one serialized Scenograph score and scene snapshot. A native receipt and both page projections consume the same artifact and revisioned trace.",
                                )],
                            ),
                        ],
                    ),
                    element(
                        "p",
                        &[
                            ("class", "projection-proof-status sr-only"),
                            ("data-projection-status", ""),
                            ("role", "status"),
                            ("aria-live", "polite"),
                        ],
                        vec![txt("Portable scene not initialized.")],
                    ),
                ],
            ),
        ],
    )
}

fn projection_proof_controls() -> SiteView {
    element(
        "div",
        &[
            ("class", "projection-proof-controls"),
            ("aria-label", "Portable scene controls"),
        ],
        vec![
            element(
                "button",
                &[
                    ("class", "button button-primary"),
                    ("type", "button"),
                    ("data-projection-action", "replay"),
                ],
                vec![txt("Replay changes")],
            ),
            element(
                "button",
                &[
                    ("class", "button button-quiet"),
                    ("type", "button"),
                    ("data-projection-action", "fold"),
                ],
                vec![txt("Fold dependencies")],
            ),
            element(
                "button",
                &[
                    ("class", "button button-quiet"),
                    ("type", "button"),
                    ("data-projection-action", "edge"),
                    ("disabled", "disabled"),
                ],
                vec![txt("Select an edge")],
            ),
            element(
                "button",
                &[
                    ("class", "button button-quiet"),
                    ("type", "button"),
                    ("data-projection-action", "reset"),
                ],
                vec![txt("Reset trace")],
            ),
            element(
                "label",
                &[("class", "projection-proof-scrubber")],
                vec![
                    element(
                        "span",
                        &[],
                        vec![
                            txt("Scene history "),
                            element(
                                "output",
                                &[("data-projection-cursor-output", "")],
                                vec![txt("0 of 0")],
                            ),
                        ],
                    ),
                    element(
                        "input",
                        &[
                            ("type", "range"),
                            ("min", "0"),
                            ("max", "0"),
                            ("step", "1"),
                            ("value", "0"),
                            ("data-projection-cursor", ""),
                        ],
                        vec![],
                    ),
                ],
            ),
            element(
                "button",
                &[
                    ("class", "button button-quiet"),
                    ("type", "button"),
                    ("data-projection-action", "share"),
                ],
                vec![txt("Share scene")],
            ),
            element(
                "p",
                &[("class", "projection-proof-readout")],
                vec![
                    element("span", &[], vec![txt("Selected")]),
                    element(
                        "strong",
                        &[("data-projection-readout", "")],
                        vec![txt("Mere")],
                    ),
                ],
            ),
        ],
    )
}

fn projection_view(kind: &str, heading: &str, description: &str) -> SiteView {
    element(
        "section",
        &[
            ("class", "projection-proof-view"),
            ("data-projection-view", kind),
            ("aria-label", heading),
        ],
        vec![
            element(
                "header",
                &[("class", "projection-proof-view-heading")],
                vec![
                    element("h3", &[], vec![txt(heading)]),
                    element(
                        "p",
                        &[("data-projection-selection", "")],
                        vec![txt("Mere selected")],
                    ),
                ],
            ),
            element("p", &[("class", "sr-only")], vec![txt(description)]),
            element(
                "div",
                &[
                    ("class", "projection-proof-stage"),
                    ("data-projection-stage", kind),
                ],
                vec![
                    element(
                        "svg",
                        &[
                            ("class", "projection-proof-edges"),
                            ("data-projection-edges", ""),
                            ("aria-hidden", "true"),
                        ],
                        vec![],
                    ),
                    element(
                        "div",
                        &[
                            ("class", "projection-proof-edge-controls"),
                            ("data-projection-edge-controls", ""),
                        ],
                        vec![],
                    ),
                    element(
                        "div",
                        &[
                            ("class", "projection-proof-nodes"),
                            ("data-projection-nodes", ""),
                            ("role", "group"),
                            ("aria-label", "Project nodes"),
                        ],
                        vec![],
                    ),
                ],
            ),
        ],
    )
}

fn hero(repository: &RepositoryRecord) -> SiteView {
    let github_url = format!("https://github.com/{}", repository.github_slug);
    let project_href = format!("/projects/{}/", repository.id);
    let mut links = vec![
        external_link(&github_url, "Open repository ↗", "button button-primary"),
        link(
            "/repos/",
            "See the complete repository map",
            "button button-quiet",
        ),
    ];
    if repository.homepage != github_url
        && repository.homepage != format!("https://mer3ly.net{project_href}")
    {
        links.push(external_link(
            &repository.homepage,
            "Visit project site ↗",
            "button button-quiet",
        ));
    }

    element(
        "section",
        &[
            ("class", "hero project-profile-hero"),
            ("aria-labelledby", "project-title"),
        ],
        vec![
            element(
                "p",
                &[("class", "eyebrow")],
                vec![txt(format!(
                    "Merely project · {} · {}",
                    repository.class.label(),
                    repository.status.label()
                ))],
            ),
            element(
                "h1",
                &[("id", "project-title")],
                vec![txt(&repository.name)],
            ),
            element(
                "p",
                &[("class", "hero-copy")],
                vec![txt(&repository.summary)],
            ),
            element("div", &[("class", "hero-actions")], links),
            element(
                "div",
                &[("class", "signal-rule"), ("aria-hidden", "true")],
                vec![],
            ),
        ],
    )
}

fn showcase_section(showcase: Option<&ShowcaseRecord>) -> SiteView {
    let Some(showcase) = showcase else {
        return element(
            "section",
            &[("class", "content-section project-no-image")],
            vec![
                section_heading("01", "current public description"),
                element(
                    "p",
                    &[("class", "project-no-image-copy")],
                    vec![txt(
                        "This profile is intentionally text-first. Merely has not selected a current visual that would clarify the project without overstating its state.",
                    )],
                ),
            ],
        );
    };

    let visual = if showcase.images.is_empty() {
        showcase_figure(
            &showcase.image,
            &showcase.alt,
            &showcase.caption,
            &showcase.source_url,
            &showcase.source_license,
            "eager",
            None,
        )
    } else {
        showcase_carousel(showcase)
    };
    element(
        "section",
        &[("class", "content-section project-showcase-section")],
        vec![
            section_heading("01", "current view"),
            element(
                "div",
                &[("class", "project-showcase-layout")],
                vec![
                    element(
                        "div",
                        &[("class", "project-showcase-copy")],
                        vec![
                            element("h2", &[], vec![txt(&showcase.headline)]),
                            element("p", &[], vec![txt(&showcase.copy)]),
                        ],
                    ),
                    visual,
                ],
            ),
        ],
    )
}

/// One approved capture as a `<figure>`: image, caption, source attribution.
/// `slide_id` is present only inside a rotation, where the dot anchors need a
/// jump target.
#[allow(clippy::too_many_arguments)]
fn showcase_figure(
    image: &str,
    alt: &str,
    caption: &str,
    source_url: &str,
    source_license: &str,
    loading: &str,
    slide_id: Option<&str>,
) -> SiteView {
    let image_src = format!("/{image}");
    let mut attrs = vec![("class", "project-showcase-figure")];
    if let Some(id) = slide_id {
        attrs.push(("id", id));
    }
    element(
        "figure",
        &attrs,
        vec![
            element(
                "img",
                &[
                    ("src", image_src.as_str()),
                    ("alt", alt),
                    ("loading", loading),
                    ("decoding", "async"),
                ],
                vec![],
            ),
            element(
                "figcaption",
                &[],
                vec![
                    txt(format!("{caption} Source image: ")),
                    external_link(source_url, "repository ↗", "text-link"),
                    txt(format!(". License: {source_license}.")),
                ],
            ),
        ],
    )
}

/// The multi-image rotation: every approved capture is in the initial HTML in
/// order, side by side in a scroll-snap strip, with anchor dots to jump
/// between them. No script is involved, so the no-script reading order is the
/// same figures top to bottom.
fn showcase_carousel(showcase: &ShowcaseRecord) -> SiteView {
    let count = showcase.images.len() + 1;
    let slide_ids = (1..=count)
        .map(|position| format!("showcase-{}-{position}", showcase.repository))
        .collect::<Vec<_>>();
    let mut slides = vec![showcase_figure(
        &showcase.image,
        &showcase.alt,
        &showcase.caption,
        &showcase.source_url,
        &showcase.source_license,
        "eager",
        Some(slide_ids[0].as_str()),
    )];
    for (index, extra) in showcase.images.iter().enumerate() {
        slides.push(showcase_figure(
            &extra.image,
            &extra.alt,
            &extra.caption,
            &extra.source_url,
            &extra.source_license,
            "lazy",
            Some(slide_ids[index + 1].as_str()),
        ));
    }
    let dots = slide_ids
        .iter()
        .enumerate()
        .map(|(index, id)| {
            let href = format!("#{id}");
            let label = format!("Image {} of {count}", index + 1);
            element(
                "a",
                &[
                    ("class", "project-showcase-dot"),
                    ("href", href.as_str()),
                    ("aria-label", label.as_str()),
                ],
                vec![txt((index + 1).to_string())],
            )
        })
        .collect::<Vec<_>>();
    element(
        "div",
        &[("class", "project-showcase-rotation")],
        vec![
            element("div", &[("class", "project-showcase-strip")], slides),
            element(
                "nav",
                &[
                    ("class", "project-showcase-dots"),
                    ("aria-label", "Showcase images"),
                ],
                dots,
            ),
        ],
    )
}

fn place_in_family(data: &PublicSiteData, repository: &RepositoryRecord) -> SiteView {
    let repositories = data
        .authority
        .repositories
        .repository
        .iter()
        .filter(|repository| repository.public)
        .map(|repository| (repository.id.as_str(), repository))
        .collect::<BTreeMap<_, _>>();
    let outgoing = data
        .authority
        .relations
        .relation
        .iter()
        .filter(|relation| relation.source == repository.id)
        .collect::<Vec<_>>();
    let incoming = data
        .authority
        .relations
        .relation
        .iter()
        .filter(|relation| relation.target == repository.id)
        .collect::<Vec<_>>();

    element(
        "section",
        &[("class", "content-section project-relations-section")],
        vec![
            section_heading("02", "place in the family"),
            element(
                "div",
                &[("class", "project-relation-columns")],
                vec![
                    relation_group("This project uses", &outgoing, true, &repositories),
                    relation_group("Other projects use this", &incoming, false, &repositories),
                ],
            ),
        ],
    )
}

fn relation_group(
    heading: &str,
    relations: &[&RelationRecord],
    outgoing: bool,
    repositories: &BTreeMap<&str, &RepositoryRecord>,
) -> SiteView {
    let body = if relations.is_empty() {
        vec![element(
            "p",
            &[("class", "project-relation-empty")],
            vec![txt("No public relationship is currently recorded.")],
        )]
    } else {
        vec![element(
            "ul",
            &[("class", "project-relation-list")],
            relations
                .iter()
                .map(|relation| {
                    let other_id = if outgoing {
                        relation.target.as_str()
                    } else {
                        relation.source.as_str()
                    };
                    let other = repositories
                        .get(other_id)
                        .expect("validated relation repository");
                    let href = format!("/projects/{}/", other.id);
                    let label = if outgoing {
                        relation.kind.label()
                    } else {
                        relation.kind.incoming_label()
                    };
                    element(
                        "li",
                        &[("data-relation-id", relation.id.as_str())],
                        vec![
                            element("span", &[("class", "relation-verb")], vec![txt(label)]),
                            link(&href, &other.name, "project-relation-link"),
                            element(
                                "span",
                                &[("class", "relation-provenance")],
                                vec![txt(relation.provenance.label())],
                            ),
                        ],
                    )
                })
                .collect(),
        )]
    };

    element(
        "section",
        &[("class", "project-relation-group")],
        vec![
            element("h3", &[], vec![txt(heading)]),
            element("div", &[], body),
        ],
    )
}

fn project_facts(
    number: &str,
    repository: &RepositoryRecord,
    metadata: Option<&PublicRepositoryMetadata>,
) -> SiteView {
    let mut facts = vec![
        fact("working role", repository.class.label()),
        fact("status", repository.status.label()),
        fact("license", &repository.license),
    ];
    if let Some(metadata) = metadata {
        if let Some(language) = &metadata.primary_language {
            facts.push(fact("primary language", language));
        }
        facts.push(fact(
            "metadata refreshed",
            &format_date(&metadata.updated_at),
        ));
    }

    let topics = metadata.map_or_else(Vec::new, |metadata| {
        metadata
            .topics
            .iter()
            .map(|topic| element("li", &[], vec![txt(topic)]))
            .collect()
    });

    element(
        "section",
        &[("class", "content-section project-facts-section")],
        vec![
            section_heading(number, "project facts"),
            element(
                "div",
                &[("class", "project-facts-layout")],
                vec![
                    element("dl", &[("class", "project-facts")], facts),
                    element(
                        "section",
                        &[
                            ("class", "project-topics"),
                            ("aria-label", "Repository topics"),
                        ],
                        vec![
                            element("h3", &[], vec![txt("Public topics")]),
                            if topics.is_empty() {
                                element(
                                    "p",
                                    &[],
                                    vec![txt("No public topics are currently recorded.")],
                                )
                            } else {
                                element("ul", &[], topics)
                            },
                        ],
                    ),
                ],
            ),
        ],
    )
}

fn fact(term: &str, description: &str) -> SiteView {
    element(
        "div",
        &[("class", "project-fact")],
        vec![
            element("dt", &[], vec![txt(term)]),
            element("dd", &[], vec![txt(description)]),
        ],
    )
}

fn profile_closing(repository: &RepositoryRecord) -> SiteView {
    let github_url = format!("https://github.com/{}", repository.github_slug);
    element(
        "section",
        &[("class", "closing-note project-profile-closing")],
        vec![
            element("p", &[("class", "eyebrow")], vec![txt("source of truth")]),
            element(
                "p",
                &[("class", "closing-copy")],
                vec![txt(
                    "This profile projects committed Mer3ly authority and validated public GitHub metadata. The repository remains authoritative for implementation and current project documentation.",
                )],
            ),
            external_link(&github_url, "Read the repository ↗", "text-link"),
        ],
    )
}

fn format_date(timestamp: &str) -> String {
    timestamp.get(..10).unwrap_or(timestamp).to_owned()
}

pub fn projection_artifact_json(data: &PublicSiteData) -> String {
    let authority = RepositoryGraph::from_parts(
        &data.authority.repositories,
        &data.authority.relations,
        &data.metadata,
    )
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
    let graph_json =
        serde_json::to_string(&graph).expect("Mere projection proof authority is serializable");
    mer3ly_repo_graph::portable_projection_json(&graph_json)
        .expect("validated Mere authority projects through Scenograph")
}

fn projection_bootstrap(data: &PublicSiteData) -> String {
    let artifact = projection_artifact_json(data);
    let embedded = artifact
        .replace('<', "\\u003c")
        .replace('>', "\\u003e")
        .replace('&', "\\u0026");
    let mut digest = Sha256::new();
    digest.update(PROJECTION_PROOF);
    digest.update(artifact.as_bytes());
    let digest = format!("{:x}", digest.finalize());
    format!(
        "<script id=\"mere-projection-artifact\" type=\"application/json\">{embedded}</script>\n\
<script type=\"module\" src=\"/projection-proof.js?v={}\"></script>",
        &digest[..12]
    )
}
