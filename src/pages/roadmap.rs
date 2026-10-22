// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! /roadmap/: the spec registry as features, what each needs, and how far
//! along it is. Built on every deploy from `content/specs.toml` and the counts
//! fetched from the repositories that run each suite. The first view is a
//! frozen scene; the live Graphshell map loads on request.

use std::path::Path;

use crate::repositories::{Authority, AuthorityError};
use crate::site::{
    ActivePage, PageMetadata, SiteView, element, external_link, render_with_body_end,
    section_heading, shell, txt,
};
use crate::spec_coverage::CatalogSnapshot;
use crate::spec_results;
use crate::spec_roadmap::{Feature, ROADMAP_DATASET_FILE, Roadmap};
use crate::specs::{SPEC_REGISTRY_PATH, SpecRegistry};

pub const METADATA: PageMetadata = PageMetadata {
    title: "Spec roadmap | Merely",
    description: "The specifications Merely's stack implements, the features each gap holds back, and how far each one is, updated from the repositories on every deploy.",
    canonical_url: "https://merelyllc.com/roadmap/",
};
pub const MOUNT_FILE: &str = "roadmap-mount.js";
pub const MOUNT_SCRIPT: &[u8] = include_bytes!("../../assets/roadmap-mount.js");
const SCENE_MARKER: &str = "@@mer3ly-roadmap-scene@@";
const SOURCE: &str = "https://github.com/merely-made/merelyllc.com/blob/main";

pub struct RoadmapPage {
    pub html: String,
    pub dataset_json: String,
}

/// Load the registry, fill fetched counts, and build the page and its dataset.
pub fn page(root: &Path) -> Result<RoadmapPage, AuthorityError> {
    let authority = Authority::load(root)?;
    let text = std::fs::read_to_string(root.join(SPEC_REGISTRY_PATH)).map_err(|error| {
        AuthorityError::from_message(format!("read {SPEC_REGISTRY_PATH}: {error}"))
    })?;
    let mut registry = SpecRegistry::parse(&text)?;
    let catalog = CatalogSnapshot::load(root)?;
    let provenance = spec_results::apply(&mut registry, &authority, root)
        .map_err(|errors| AuthorityError::from_message(errors.join("\n")))?;
    let roadmap = Roadmap::build(&registry, &provenance, &catalog, &text, &authority);
    let scene = roadmap
        .frozen_scene()
        .map_err(AuthorityError::from_message)?;
    let dataset_json = roadmap
        .dataset_json()
        .map_err(AuthorityError::from_message)?;
    let bootstrap = format!("<script type=\"module\" src=\"/{MOUNT_FILE}\"></script>");
    let html = render_with_body_end(&METADATA, || view(&roadmap), &bootstrap).replacen(
        SCENE_MARKER,
        &scene.to_html("roadmap-frozen-scene"),
        1,
    );
    Ok(RoadmapPage { html, dataset_json })
}

fn paragraph(text: impl Into<String>) -> SiteView {
    element("p", &[("class", "section-intro")], vec![txt(text)])
}

fn view(roadmap: &Roadmap) -> SiteView {
    let complete = roadmap
        .features
        .iter()
        .filter(|f| f.complete() == f.links.len())
        .count();
    shell(
        ActivePage::Roadmap,
        element(
            "main",
            &[("id", "main"), ("class", "roadmap-main")],
            vec![
                element(
                    "section",
                    &[("class", "hero"), ("aria-labelledby", "roadmap-title")],
                    vec![
                        element("p", &[("class", "eyebrow")], vec![txt("spec roadmap")]),
                        element(
                            "h1",
                            &[("id", "roadmap-title")],
                            vec![txt(
                                "What the stack implements, and what each gap holds back.",
                            )],
                        ),
                        element(
                            "p",
                            &[("class", "hero-copy")],
                            vec![txt(format!(
                                "{} features named by the family's own plans, resting on {} specifications. {complete} have every spec on their chain complete. Counts come from the repositories that run each suite, as of {}.",
                                roadmap.features.len(),
                                roadmap.spec_count(),
                                roadmap.on
                            ))],
                        ),
                    ],
                ),
                map_section(),
                features_section(roadmap),
                measured_section(roadmap),
                repos_section(roadmap),
                updates_section(),
            ],
        ),
    )
}

fn map_section() -> SiteView {
    let runtime = crate::viewer::href("viewer.js");
    let style = crate::viewer::href("styles.css");
    let dataset = format!("/{ROADMAP_DATASET_FILE}");
    element(
        "section",
        &[
            ("class", "content-section roadmap-map"),
            ("aria-labelledby", "roadmap-map-title"),
            ("data-roadmap-map", ""),
            ("data-viewer-runtime", &runtime),
            ("data-viewer-style", &style),
            ("data-dataset-src", &dataset),
        ],
        vec![
            section_heading("01", "the map"),
            element(
                "h3",
                &[("id", "roadmap-map-title")],
                vec![txt(
                    "Repositories claim specs; specs require specs; specs unlock features.",
                )],
            ),
            paragraph(
                "The scene below is solved and frozen at build time. The live map is Graphshell, loading the same graph as a dataset; select a node to follow its links to the spec, its registry line or the plan that names it.",
            ),
            element(
                "button",
                &[
                    ("type", "button"),
                    ("class", "graph-sandbox-open"),
                    ("data-roadmap-open", ""),
                ],
                vec![txt("Open the live map")],
            ),
            element(
                "p",
                &[("data-roadmap-status", ""), ("role", "status")],
                vec![],
            ),
            element(
                "div",
                &[("data-roadmap-surface", ""), ("hidden", "hidden")],
                vec![],
            ),
            element(
                "div",
                &[("class", "graph-sandbox-frozen")],
                vec![element(
                    "div",
                    &[("class", "graph-sandbox-frozen-scene")],
                    vec![txt(SCENE_MARKER)],
                )],
            ),
        ],
    )
}

fn feature_view(feature: &Feature) -> SiteView {
    let consumer = match &feature.consumer_url {
        Some(url) => external_link(url, feature.consumer.clone(), "text-link"),
        None => txt(feature.consumer.clone()),
    };
    let rows = feature
        .links
        .iter()
        .map(|link| {
            let role = match &link.via {
                None => format!("{}, unlocks it", link.repo),
                Some(via) => format!("{}, needed by {via}", link.repo),
            };
            let detail = link
                .measured
                .as_ref()
                .map_or_else(|| link.depth.to_owned(), |m| format!("{}, {m}", link.depth));
            element(
                "tr",
                &[("data-depth", link.depth)],
                vec![
                    element(
                        "th",
                        &[("scope", "row")],
                        vec![
                            external_link(&link.registry_url, link.spec_name.clone(), "text-link"),
                            element(
                                "span",
                                &[("class", "roadmap-role")],
                                vec![txt(format!(" · {role}"))],
                            ),
                        ],
                    ),
                    element("td", &[], vec![txt(detail)]),
                ],
            )
        })
        .collect();
    element(
        "article",
        &[("class", "roadmap-feature")],
        vec![
            element("h3", &[], vec![txt(feature.text.clone())]),
            element(
                "p",
                &[("class", "roadmap-feature-meta")],
                vec![
                    txt(format!(
                        "{} of {} specs complete · for ",
                        feature.complete(),
                        feature.links.len()
                    )),
                    consumer,
                ],
            ),
            element(
                "div",
                &[("class", "table-wrap")],
                vec![element("table", &[], vec![element("tbody", &[], rows)])],
            ),
        ],
    )
}

fn features_section(roadmap: &Roadmap) -> SiteView {
    let mut children = vec![
        section_heading("02", "features"),
        paragraph(
            "Each feature is named by a plan or a consumer in the family. Its chain is the spec that unlocks it and that spec's prerequisites in the same repository; each row links to its line in the registry.",
        ),
    ];
    children.extend(roadmap.features.iter().map(feature_view));
    element("section", &[("class", "content-section")], children)
}

fn measured_section(roadmap: &Roadmap) -> SiteView {
    let rows = roadmap
        .measured
        .iter()
        .map(|row| {
            element(
                "tr",
                &[],
                vec![
                    element(
                        "th",
                        &[("scope", "row")],
                        vec![txt(format!("{} · {}", row.spec_name, row.repo))],
                    ),
                    element(
                        "td",
                        &[],
                        vec![
                            txt(format!("{} on {} · ", row.count, row.on)),
                            match &row.source_url {
                                Some(url) => {
                                    external_link(url, row.source_label.clone(), "text-link")
                                }
                                None => txt(row.source_label.clone()),
                            },
                        ],
                    ),
                ],
            )
        })
        .collect();
    element(
        "section",
        &[("class", "content-section")],
        vec![
            section_heading("03", "measured"),
            paragraph(
                "Every count on this page, and where it came from. A commit link means the count was read from that repository's own results file at that commit.",
            ),
            element(
                "div",
                &[("class", "table-wrap")],
                vec![element("table", &[], vec![element("tbody", &[], rows)])],
            ),
        ],
    )
}

fn repos_section(roadmap: &Roadmap) -> SiteView {
    let rows = roadmap
        .repos
        .iter()
        .map(|row| {
            element(
                "tr",
                &[],
                vec![
                    element("th", &[("scope", "row")], vec![txt(row.repo.clone())]),
                    element(
                        "td",
                        &[],
                        vec![txt(format!(
                            "{} claims · {} full · {} partial",
                            row.claims, row.full, row.partial
                        ))],
                    ),
                ],
            )
        })
        .collect();
    element(
        "section",
        &[("class", "content-section")],
        vec![
            section_heading("04", "by repository"),
            element(
                "div",
                &[("class", "table-wrap")],
                vec![element("table", &[], vec![element("tbody", &[], rows)])],
            ),
        ],
    )
}

fn updates_section() -> SiteView {
    element(
        "section",
        &[("class", "content-section")],
        vec![
            section_heading("05", "how this updates"),
            paragraph(
                "Nothing here is typed into the page. Every deploy rebuilds it from the family spec registry and refetches each suite's results file at its latest commit, so a commit that changes a count or a claim shows up on the next deploy, which runs on every push and daily.",
            ),
            element(
                "ul",
                &[("class", "roadmap-sources")],
                [
                    ("the registry", format!("{SOURCE}/content/specs.toml")),
                    (
                        "the fetched results and their commits",
                        format!("{SOURCE}/content/spec-results/sources.json"),
                    ),
                    (
                        "the fetch",
                        format!("{SOURCE}/scripts/refresh-spec-results.py"),
                    ),
                    (
                        "the registry's plan",
                        format!("{SOURCE}/docs/2026-10-18_family_spec_registry_plan.md"),
                    ),
                ]
                .into_iter()
                .map(|(label, url)| {
                    element("li", &[], vec![external_link(&url, label, "text-link")])
                })
                .collect(),
            ),
        ],
    )
}
