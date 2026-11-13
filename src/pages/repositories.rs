use std::collections::BTreeMap;
use std::path::Path;

use serde::Serialize;
use serde_json::json;
use sha2::{Digest, Sha256};

use crate::host_history::host_history_href;
use crate::repositories::{
    Authority, AuthorityError, PublicMetadataCache, PublicOrganizationEvent,
    PublicRepositoryMetadata, PublicSiteData, RelationRecord, RepositoryClass, RepositoryRecord,
};
use crate::repository_frozen::{
    FAILED_LEAD, FrozenFirstView, GRID_MARKER, NO_SCRIPT_LEAD, SCENE_MARKER, SCRIPTED_LEAD,
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
    canonical_url: "https://merelyllc.com/repos/",
};

const GRAPH_SANDBOX_LOADER: &[u8] = include_bytes!("../../assets/graph-sandbox.js");
const GRAPH_SANDBOX_MOUNT: &[u8] = include_bytes!("../../assets/graph-sandbox-mount.js");
const REPO_GRAPH_WASM_GLUE: &[u8] = include_bytes!("../../assets/mer3ly_repo_graph.js");
const REPO_GRAPH_WASM: &[u8] = include_bytes!("../../assets/mer3ly_repo_graph_bg.wasm");
const HISTORY_POINT_LIMIT: usize = 24;

/// The published page and the history file it cites.
pub struct RepositoriesPage {
    pub html: String,
    pub frozen: FrozenFirstView,
}

/// The authority's checkpoint history, sampled as the sandbox's slider is.
pub fn site_history(
    root: &Path,
    authority: &Authority,
    metadata: &PublicMetadataCache,
) -> Result<GitAuthorityHistoryProjection, AuthorityError> {
    let graph =
        RepositoryGraph::from_parts(&authority.repositories, &authority.relations, metadata)
            .map_err(AuthorityError::from_message)?;
    public_history_projection(root, graph, HISTORY_POINT_LIMIT)
        .map_err(AuthorityError::from_message)
}

/// Build the page, its frozen first view, and the v2 history it cites.
pub fn page(root: &Path) -> Result<RepositoriesPage, AuthorityError> {
    let data = PublicSiteData::load(root)?;
    let history = site_history(root, &data.authority, &data.metadata)?;
    page_for(&data, &history)
}

/// [`page`], from loaded data and a history.
pub fn page_for(
    data: &PublicSiteData,
    history: &GitAuthorityHistoryProjection,
) -> Result<RepositoriesPage, AuthorityError> {
    let frozen = frozen_first_view(&data.authority, &data.metadata, history)
        .map_err(AuthorityError::from_message)?;
    let bootstrap = graph_bootstrap(data);
    let html = render_with_body_end(&METADATA, || view(data, &frozen), &bootstrap)
        .replacen(SCENE_MARKER, &frozen.scene_html(), 1)
        .replacen(GRID_MARKER, &frozen.grid_html(), 1);
    Ok(RepositoriesPage { html, frozen })
}

/// The frozen first view the page and `validate-artifact` both derive.
pub fn frozen_first_view(
    authority: &Authority,
    metadata: &PublicMetadataCache,
    history: &GitAuthorityHistoryProjection,
) -> Result<FrozenFirstView, String> {
    let current =
        RepositoryGraph::from_parts(&authority.repositories, &authority.relations, metadata)?;
    let disclosure = crate::host_dataset::current_dataset(authority, metadata)?;
    FrozenFirstView::build(&current, &specimen(), history, &disclosure)
}

pub fn document(root: &Path) -> Result<String, AuthorityError> {
    page(root).map(|page| page.html)
}

pub fn view(data: &PublicSiteData, frozen: &FrozenFirstView) -> SiteView {
    shell(
        ActivePage::Repositories,
        element(
            "main",
            &[("id", "main"), ("class", "repositories-main")],
            vec![
                hero(data),
                graph_viewer(frozen),
                organization_activity(data),
                repository_index(data),
                source_note(data),
            ],
        ),
    )
}

fn graph_viewer(frozen: &FrozenFirstView) -> SiteView {
    let runtime = crate::viewer::href("viewer.js");
    let style = crate::viewer::href("styles.css");
    let history = host_history_href(&frozen.history_json);
    element(
        "section",
        &[
            ("class", "content-section graph-sandbox-section"),
            ("aria-labelledby", "graph-sandbox-title"),
            ("data-graph-sandbox", ""),
            ("data-sandbox-state", "frozen"),
            ("data-sandbox-runtime", &runtime),
            ("data-viewer-style", &style),
            ("data-sandbox-history-src", &history),
        ],
        vec![
            section_heading("01", "repository graph"),
            element(
                "h3",
                &[("id", "graph-sandbox-title")],
                vec![txt("Explore the projects and their relationships.")],
            ),
            element(
                "p",
                &[],
                vec![txt(
                    "Select a project to inspect it. Drag the canvas to pan, scroll to zoom, or use the graph’s commands. The static graph, matrix and source history remain available below.",
                )],
            ),
            element(
                "div",
                &[("data-sandbox-interface", ""), ("hidden", "hidden")],
                vec![],
            ),
            frozen_view(frozen),
            element(
                "p",
                &[
                    ("data-sandbox-status", ""),
                    ("role", "status"),
                    ("aria-live", "polite"),
                ],
                vec![txt("Static reading is available.")],
            ),
        ],
    )
}

/// The first view, frozen at build time (Ruling 157): the default reading, its
/// matrix and the source history, until someone interacts with it.
fn frozen_view(frozen: &FrozenFirstView) -> SiteView {
    element(
        "div",
        &[
            ("class", "graph-sandbox-frozen"),
            ("data-sandbox-frozen", ""),
        ],
        vec![element(
            "details",
            &[("data-static-reading", ""), ("open", "open")],
            vec![
                element("summary", &[], vec![txt("Static project map")]),
                element(
                    "p",
                    &[
                        ("class", "graph-sandbox-fallback"),
                        ("data-sandbox-fallback", ""),
                    ],
                    vec![
                        // Static reading is the initial surface. The mount script
                        // offers an explicit live-viewer route when supported.
                        element(
                            "span",
                            &[
                                ("data-sandbox-reading-lead", ""),
                                ("data-scripted-lead", SCRIPTED_LEAD),
                                ("data-failed-lead", FAILED_LEAD),
                            ],
                            vec![txt(NO_SCRIPT_LEAD)],
                        ),
                        txt(frozen.summary()),
                    ],
                ),
                element(
                    "button",
                    &[
                        ("type", "button"),
                        ("class", "graph-sandbox-open"),
                        ("data-sandbox-open", ""),
                        ("hidden", "hidden"),
                    ],
                    vec![txt("Open the live graph")],
                ),
                element(
                    "div",
                    &[("class", "graph-sandbox-frozen-scene")],
                    vec![txt(SCENE_MARKER)],
                ),
                element(
                    "details",
                    &[("data-static-matrix", "")],
                    vec![
                        element("summary", &[], vec![txt("Relationship matrix")]),
                        element(
                            "div",
                            &[
                                ("class", "graph-sandbox-frozen-matrix"),
                                ("role", "region"),
                                ("aria-labelledby", "repos-frozen-matrix-caption"),
                                ("tabindex", "0"),
                            ],
                            vec![txt(GRID_MARKER)],
                        ),
                    ],
                ),
                element(
                    "details",
                    &[("data-static-history", "")],
                    vec![
                        element("summary", &[], vec![txt("Source history")]),
                        frozen.history_view(),
                    ],
                ),
            ],
        )],
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

fn graph_bootstrap(data: &PublicSiteData) -> String {
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
    // The checkpoint history no longer rides inline: the viewer fetches the
    // v2 history on first interaction (Ruling 157).
    let json = format!(
        "{{\n\"schema\":{schema},\n\"nodes\":[\n{nodes}\n],\n\"edges\":[\n{edges}\n],\n\"feed\":[\n{feed}\n]\n}}"
    );
    let json = script_safe(&json);
    let mount_href = graph_sandbox_mount_href();
    format!(
        "<script id=\"repository-graph-data\" type=\"application/json\">{json}</script>\n\
<script type=\"module\" src=\"{mount_href}\"></script>"
    )
}

fn script_safe(json: &str) -> String {
    json.replace('<', "\\u003c")
        .replace('>', "\\u003e")
        .replace('&', "\\u0026")
}

/// The sandbox's authored specimen: a heterogeneous graph beside the live one.
pub fn specimen() -> serde_json::Value {
    json!({
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
    })
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

/// The only script /repos/ loads up front: it swaps in the scripted opening
/// and imports the viewer on first interaction.
pub fn graph_sandbox_mount_href() -> String {
    format!(
        "/graph-sandbox-mount.js?v={}",
        &base16ct::lower::encode_string(&Sha256::digest(GRAPH_SANDBOX_MOUNT))[..12]
    )
}

/// The version query the graph runtime is fetched under. The sandbox loader
/// passes its own query on to the glue and the Wasm, and the Mere profile's
/// projection proof imports them under the same query, so a visitor's cache
/// holds one copy for both pages (Ruling 140).
pub fn graph_runtime_version() -> String {
    let mut digest = Sha256::new();
    digest.update(GRAPH_SANDBOX_LOADER);
    digest.update(REPO_GRAPH_WASM_GLUE);
    digest.update(REPO_GRAPH_WASM);
    let digest = base16ct::lower::encode_string(&digest.finalize());
    format!("?v={}", &digest[..12])
}

/// The graph runtime's glue and Wasm, at [`graph_runtime_version`].
pub fn graph_runtime_hrefs() -> (String, String) {
    let version = graph_runtime_version();
    (
        format!("/mer3ly_repo_graph.js{version}"),
        format!("/mer3ly_repo_graph_bg.wasm{version}"),
    )
}

fn format_timestamp(value: &str) -> String {
    format!("{} {} UTC", &value[..10], &value[11..16])
}

mod catalog;

use catalog::repository_index;
