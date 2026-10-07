use std::collections::BTreeMap;
use std::path::Path;

use crate::repositories::{AuthorityError, PublicSiteData, RepositoryRecord, ShowcaseRecord};
use crate::site::{
    ActivePage, PageMetadata, SiteView, element, link, render_with, section_heading, shell, txt,
};

pub const METADATA: PageMetadata = PageMetadata {
    title: "Merely | Local-first software and community radio",
    description: "Merely builds local-first software, graph-shaped web tools, and community-owned radio infrastructure in Ashland, Kentucky.",
    canonical_url: "https://merelyllc.com/",
};

pub fn document(root: &Path) -> Result<String, AuthorityError> {
    let data = PublicSiteData::load(root)?;
    Ok(document_for(&data))
}

pub fn document_for(data: &PublicSiteData) -> String {
    render_with(&METADATA, || view(data))
}

pub fn view(data: &PublicSiteData) -> SiteView {
    shell(
        ActivePage::Home,
        element(
            "main",
            &[("id", "main")],
            vec![
                hero(),
                radio_feature(),
                project_showcases(data),
                principle(),
            ],
        ),
    )
}

fn hero() -> SiteView {
    element(
        "section",
        &[
            ("class", "hero home-hero"),
            ("aria-labelledby", "home-title"),
        ],
        vec![
            element(
                "p",
                &[("class", "eyebrow")],
                vec![txt("Merely LLC · Ashland, Kentucky")],
            ),
            element(
                "h1",
                &[("id", "home-title")],
                vec![txt("Tools for people who are their own infrastructure.")],
            ),
            element(
                "p",
                &[("class", "hero-copy")],
                vec![txt(
                    "We build local-first software, graph-shaped web tools, and community-owned radio systems. The work is open, inspectable, and made to keep working on your own terms.",
                )],
            ),
            element(
                "div",
                &[("class", "hero-actions")],
                vec![
                    link(
                        "/radio.html",
                        "Read about community radio",
                        "button button-primary",
                    ),
                    link(
                        "/repos/",
                        "Explore the repository map",
                        "button button-quiet",
                    ),
                ],
            ),
            element(
                "div",
                &[("class", "signal-rule"), ("aria-hidden", "true")],
                vec![],
            ),
        ],
    )
}

fn radio_feature() -> SiteView {
    element(
        "section",
        &[("class", "content-section")],
        vec![
            section_heading("01", "community radio"),
            element(
                "article",
                &[("class", "feature-card")],
                vec![
                    element(
                        "div",
                        &[("class", "feature-copy")],
                        vec![
                            element(
                                "p",
                                &[("class", "card-kicker")],
                                vec![txt("Retinue · field status")],
                            ),
                            element(
                                "h3",
                                &[],
                                vec![txt("Radios that work when the towers do not.")],
                            ),
                            element(
                                "p",
                                &[],
                                vec![txt(
                                    "Small LoRa radios relay encrypted messages device to device without cell towers, internet service, or a monthly bill. Three units are exchanging data on the workbench today. The next field target is a ten-site FIVCO pilot.",
                                )],
                            ),
                            element(
                                "div",
                                &[("class", "feature-links")],
                                vec![
                                    link("/radio.html", "Read the pilot brief", "text-link"),
                                    link("/devices/", "Explore the hardware catalog", "text-link"),
                                ],
                            ),
                        ],
                    ),
                    element(
                        "dl",
                        &[("class", "status-list")],
                        vec![
                            status("radios on air", "3"),
                            status("mesh standards", "3"),
                            status("unit cost", "~ a tank of gas"),
                            status("monthly bill", "none"),
                        ],
                    ),
                ],
            ),
        ],
    )
}

fn status(term: &str, value: &str) -> SiteView {
    element(
        "div",
        &[("class", "status-row")],
        vec![
            element("dt", &[], vec![txt(term)]),
            element("dd", &[], vec![txt(value)]),
        ],
    )
}

fn project_showcases(data: &PublicSiteData) -> SiteView {
    let repositories = data
        .authority
        .repositories
        .repository
        .iter()
        .filter(|repository| repository.public)
        .map(|repository| (repository.id.as_str(), repository))
        .collect::<BTreeMap<_, _>>();
    let showcases = data
        .showcases
        .ordered()
        .into_iter()
        .map(|showcase| {
            let repository = repositories
                .get(showcase.repository.as_str())
                .expect("validated showcase repository");
            project_showcase(showcase, repository)
        })
        .collect();

    element(
        "section",
        &[("class", "content-section home-showcase-section")],
        vec![
            section_heading("02", "software in view"),
            element(
                "p",
                &[("class", "section-intro")],
                vec![txt(
                    "A few current surfaces from the same public repository family. Each profile names the working boundary, its state, and the projects around it.",
                )],
            ),
            element("div", &[("class", "home-showcase-list")], showcases),
            link(
                "/repos/",
                "Explore every repository and relationship",
                "button button-quiet showcase-index-link",
            ),
        ],
    )
}

fn project_showcase(showcase: &ShowcaseRecord, repository: &RepositoryRecord) -> SiteView {
    let image_src = format!("/{}", showcase.image);
    let profile_href = format!("/projects/{}/", repository.id);
    element(
        "article",
        &[("class", "home-showcase-card")],
        vec![
            element(
                "figure",
                &[("class", "home-showcase-figure")],
                vec![element(
                    "img",
                    &[
                        ("src", image_src.as_str()),
                        ("alt", showcase.alt.as_str()),
                        ("loading", "lazy"),
                        ("decoding", "async"),
                    ],
                    vec![],
                )],
            ),
            element(
                "div",
                &[("class", "home-showcase-copy")],
                vec![
                    element(
                        "p",
                        &[("class", "card-kicker")],
                        vec![txt(format!(
                            "{} · {} · {}",
                            repository.name,
                            repository.class.label(),
                            repository.status.label()
                        ))],
                    ),
                    element("h3", &[], vec![txt(&showcase.headline)]),
                    element("p", &[], vec![txt(&showcase.copy)]),
                    link(&profile_href, "Read the project profile", "text-link"),
                ],
            ),
        ],
    )
}

fn principle() -> SiteView {
    element(
        "section",
        &[
            ("class", "closing-note"),
            ("aria-label", "Working principle"),
        ],
        vec![
            element("p", &[("class", "eyebrow")], vec![txt("working principle")]),
            element(
                "p",
                &[("class", "closing-copy")],
                vec![txt(
                    "Your tools should explain themselves, preserve your data, and remain useful when somebody else's service is unavailable.",
                )],
            ),
        ],
    )
}
