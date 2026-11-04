use std::collections::BTreeMap;
use std::path::Path;

use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};

use crate::host_dataset::HOST_DATASET_FILE;
use crate::projection_proof::ProjectionProof;
use crate::repositories::{
    AuthorityError, PublicRepositoryMetadata, PublicSiteData, RelationRecord, RepositoryRecord,
    ShowcaseRecord,
};
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
    let canonical = format!("https://merelyllc.com/projects/{}/", repository.id);
    let repository_metadata = data
        .metadata
        .repository
        .iter()
        .find(|metadata| metadata.id == repository.id);
    let showcase = data.showcases.for_repository(&repository.id);
    let image_url = showcase.map_or_else(
        || DEFAULT_SOCIAL_IMAGE_URL.to_owned(),
        |showcase| format!("https://merelyllc.com/{}", showcase.image),
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
    let proof = proof_for(data, repository);
    let bootstrap = proof.as_ref().map(projection_bootstrap).unwrap_or_default();
    render_with_dynamic_and_body_end(
        &metadata,
        || view_with(data, repository, proof.as_ref()),
        &bootstrap,
    )
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
    view_with(data, repository, proof_for(data, repository).as_ref())
}

fn view_with(
    data: &PublicSiteData,
    repository: &RepositoryRecord,
    proof: Option<&ProjectionProof>,
) -> SiteView {
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
    if let Some(proof) = proof {
        sections.push(projection_proof_section(proof));
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
        && repository.homepage != format!("https://merelyllc.com{project_href}")
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

mod projection;

use projection::{projection_bootstrap, projection_proof_section, proof_for};

mod showcase;

use showcase::showcase_section;
