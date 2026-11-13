use sha2::{Digest, Sha256};

use crate::message_path::{
    self, DefaultView, MANIFEST_ID, Milestone, NodeLayout, layout, milestones, seconds,
};
use crate::retinue_traces::{TraceEvent, TraceScenario, TraceSet};
use crate::site::{
    ActivePage, PageMetadata, SiteView, element, external_link, render_with_body_end,
    section_heading, shell, txt,
};

const MESSAGE_PATH_LAB: &[u8] = include_bytes!("../../assets/message-path-lab.js");
const RADIO_MIRROR_WASM_GLUE: &[u8] = include_bytes!("../../assets/radio_mirror.js");
const RADIO_MIRROR_WASM: &[u8] = include_bytes!("../../assets/radio_mirror_bg.wasm");

pub const METADATA: PageMetadata = PageMetadata {
    title: "Community radio | Merely",
    description: "A low-cost, open-source LoRa radio pilot for community-owned backup communications across the FIVCO counties.",
    canonical_url: "https://merelyllc.com/radio.html",
};

/// The committed traces, which the build cannot do without.
fn traces() -> TraceSet {
    TraceSet::embedded().unwrap_or_else(|errors| {
        panic!(
            "the committed retinue traces are invalid:\n{}",
            errors.join("\n")
        )
    })
}

pub fn document() -> String {
    let traces = traces();
    let manifest = message_path::manifest_embedded(&traces)
        .unwrap_or_else(|error| panic!("the message path manifest: {error}"));
    let bootstrap = format!(
        "<script id=\"{MANIFEST_ID}\" type=\"application/json\">{manifest}</script>\n\
<script type=\"module\" src=\"{}\"></script>",
        message_path_lab_href()
    );
    render_with_body_end(&METADATA, || view_with(&traces), &bootstrap)
}

/// The lab module's address, versioned by the loader and the radio-mirror
/// runtime it imports.
pub fn message_path_lab_href() -> String {
    let mut digest = Sha256::new();
    digest.update(MESSAGE_PATH_LAB);
    digest.update(RADIO_MIRROR_WASM_GLUE);
    digest.update(RADIO_MIRROR_WASM);
    let digest = base16ct::lower::encode_string(&digest.finalize());
    format!("/message-path-lab.js?v={}", &digest[..12])
}

pub fn view() -> SiteView {
    view_with(&traces())
}

fn view_with(traces: &TraceSet) -> SiteView {
    shell(
        ActivePage::Radio,
        element(
            "main",
            &[("id", "main"), ("class", "radio-main")],
            vec![
                hero(),
                problem_solution(),
                mesh(traces),
                pilot(),
                costs(),
                partnership(),
            ],
        ),
    )
}

fn hero() -> SiteView {
    element(
        "header",
        &[("class", "hero radio-hero")],
        vec![
            element(
                "p",
                &[("class", "eyebrow")],
                vec![txt("Retinue · community radio")],
            ),
            element(
                "h1",
                &[],
                vec![txt("Resilient communities, connected peer-to-peer.")],
            ),
            element(
                "p",
                &[("class", "hero-copy")],
                vec![txt(
                    "Low-cost, community-owned radio networks that keep people messaging when cell towers and internet go down.",
                )],
            ),
        ],
    )
}

fn problem_solution() -> SiteView {
    element(
        "section",
        &[("class", "two-up"), ("aria-label", "Problem and approach")],
        vec![
            numbered_card(
                "01",
                "the problem",
                "Storms, floods, and ice take down power and communications in our region, sometimes for days. Families cannot reach each other, and volunteer responders lose coordination exactly when they need it most.",
            ),
            numbered_card(
                "02",
                "the approach",
                "Small LoRa radios relay data device to device at long range. Hosted at fire stations, churches, ridgelines, and public facilities, they form local networks that can link with their neighbors.",
            ),
        ],
    )
}

fn numbered_card(number: &str, heading: &str, copy: &str) -> SiteView {
    element(
        "article",
        &[("class", "info-card")],
        vec![
            element(
                "p",
                &[("class", "card-kicker")],
                vec![txt(format!("{number} · {heading}"))],
            ),
            element("p", &[], vec![txt(copy)]),
        ],
    )
}

/// What the lab shows before any script runs: the default scenario's last
/// event, on the default node. The script starts from the same state.
struct LabState<'a> {
    traces: &'a TraceSet,
    scenario: &'a TraceScenario,
    view: DefaultView,
    milestones: Vec<Milestone>,
}

impl<'a> LabState<'a> {
    fn new(traces: &'a TraceSet) -> Self {
        message_path::check_layout(traces).unwrap_or_else(|error| panic!("{error}"));
        let view = message_path::default_view(traces)
            .unwrap_or_else(|error| panic!("the message path default: {error}"));
        let scenario = traces
            .scenario(&view.scenario)
            .expect("the default scenario is present");
        Self {
            traces,
            scenario,
            milestones: milestones(scenario),
            view,
        }
    }

    fn event(&self) -> &TraceEvent {
        &self.scenario.trace.events[self.view.step]
    }

    /// The milestone at or before the step: the ledger's current row.
    fn current_milestone(&self) -> Option<&Milestone> {
        self.milestones
            .iter()
            .rev()
            .find(|row| row.event <= self.view.step)
    }

    /// The latest delivery at or before the step.
    fn delivered_path(&self) -> Option<&[String]> {
        self.scenario.trace.events[..=self.view.step]
            .iter()
            .rev()
            .find_map(|event| match event {
                TraceEvent::Delivered { path, .. } => Some(path.as_slice()),
                _ => None,
            })
    }

    fn edge_is_cut(&self, a: &str, b: &str) -> bool {
        self.scenario.trace.events[..=self.view.step]
            .iter()
            .any(|event| matches!(event, TraceEvent::Cut { a: x, b: y, .. } if (x == a && y == b) || (x == b && y == a)))
    }
}

/// The step reading: the event's time, and its ledger text when it has one.
pub fn step_reading(row: &Milestone) -> String {
    format!("{} · {}", seconds(row.t), row.text)
}

fn mesh(traces: &TraceSet) -> SiteView {
    let state = LabState::new(traces);
    let step = state.view.step.to_string();
    element(
        "section",
        &[("class", "content-section")],
        vec![
            section_heading("03", "how the mesh works"),
            element(
                "figure",
                &[
                    ("class", "mesh-card message-path-lab"),
                    ("data-message-path-lab", ""),
                    ("data-ready", "false"),
                    ("data-scenario", state.view.scenario.as_str()),
                    ("data-step", step.as_str()),
                    ("data-node", state.view.node.as_str()),
                ],
                vec![
                    message_path_header(),
                    element(
                        "p",
                        &[
                            ("class", "message-path-notice"),
                            ("data-path-notice", ""),
                            ("role", "status"),
                            ("hidden", ""),
                        ],
                        vec![],
                    ),
                    message_path_controls(&state),
                    element(
                        "div",
                        &[("class", "message-path-workbench")],
                        vec![
                            message_path_topology(&state),
                            element(
                                "aside",
                                &[
                                    ("class", "message-path-projections"),
                                    ("aria-label", "The selected radio and the trace ledger"),
                                ],
                                vec![message_path_radio(&state), message_path_ledger(&state)],
                            ),
                        ],
                    ),
                    element(
                        "figcaption",
                        &[("data-path-source", "")],
                        vec![
                            txt(message_path::source_statement(traces)),
                            txt(" "),
                            external_link(
                                &format!(
                                    "https://github.com/merely-made/murm/tree/{}/crates/retinue-sim",
                                    traces.provenance.revision
                                ),
                                "Read retinue-sim at that revision.",
                                "message-path-source-link",
                            ),
                        ],
                    ),
                    element(
                        "p",
                        &[],
                        vec![txt(concat!(
                            "Each unit costs about as much as a tank of gas. ",
                            "Nodes can run from USB-C, battery, or solar power; the pilot will ",
                            "measure off-grid runtime under local conditions. The network remains ",
                            "useful as long as working radios retain a path between them.",
                        ))],
                    ),
                ],
            ),
        ],
    )
}

fn pilot() -> SiteView {
    element(
        "section",
        &[("class", "content-section")],
        vec![
            section_heading("04", "the FIVCO pilot"),
            element(
                "div",
                &[("class", "pilot-grid")],
                vec![
                    element(
                        "figure",
                        &[("class", "county-card")],
                        vec![
                            county_map(),
                            element(
                                "figcaption",
                                &[],
                                vec![txt("ten proposed sites · stylized, not to scale")],
                            ),
                        ],
                    ),
                    element(
                        "div",
                        &[("class", "pilot-copy")],
                        vec![
                            element(
                                "p",
                                &[],
                                vec![txt(
                                    "Ten sites across Boyd, Carter, Elliott, Greenup, and Lawrence counties, hosted by local organizations and public facilities, with high ground prioritized for useful range.",
                                )],
                            ),
                            element(
                                "p",
                                &[("class", "list-lead")],
                                vec![txt("The pilot produces three things:")],
                            ),
                            element(
                                "ol",
                                &[("class", "deliverable-list")],
                                vec![
                                    element(
                                        "li",
                                        &[],
                                        vec![txt(
                                            "A working backup messaging layer for participating communities.",
                                        )],
                                    ),
                                    element("li", &[], vec![txt("A measured coverage map.")]),
                                    element(
                                        "li",
                                        &[],
                                        vec![txt(
                                            "A costed, step-by-step playbook other Appalachian counties can copy.",
                                        )],
                                    ),
                                ],
                            ),
                            element(
                                "aside",
                                &[("class", "callout")],
                                vec![txt(
                                    "Three working radios, built in a single day, are exchanging data over the air now. A county-scale pilot is chiefly a materials, siting, and training problem.",
                                )],
                            ),
                        ],
                    ),
                ],
            ),
        ],
    )
}

fn county_map() -> SiteView {
    element(
        "svg",
        &[
            ("class", "county-map"),
            ("viewBox", "0 0 300 320"),
            ("role", "img"),
            ("aria-labelledby", "county-map-title county-map-description"),
        ],
        vec![
            element(
                "title",
                &[("id", "county-map-title")],
                vec![txt("FIVCO pilot network")],
            ),
            element(
                "desc",
                &[("id", "county-map-description")],
                vec![txt(
                    "A stylized map of Boyd, Carter, Elliott, Greenup, and Lawrence counties connected by ten proposed radio sites.",
                )],
            ),
            county_shape(
                "M60 18 L150 10 L172 52 L150 108 L74 116 L48 60 Z",
                "county-shape county-shape-sage",
            ),
            county_shape("M150 10 L236 24 L252 96 L172 52 Z", "county-shape"),
            county_shape("M172 52 L252 96 L244 178 L150 108 Z", "county-shape"),
            county_shape(
                "M74 116 L150 108 L244 178 L200 260 L96 244 Z",
                "county-shape county-shape-sage",
            ),
            county_shape("M96 244 L200 260 L212 308 L108 312 Z", "county-shape"),
            svg_text("104", "66", "county-label county-label-sage", "GREENUP"),
            svg_text("204", "52", "county-label", "BOYD"),
            svg_text("206", "122", "county-label", "CARTER"),
            svg_text("152", "196", "county-label county-label-sage", "ELLIOTT"),
            svg_text("158", "290", "county-label", "LAWRENCE"),
            svg_line("110", "44", "206", "36", "county-link"),
            svg_line("206", "36", "216", "100", "county-link"),
            svg_line("110", "44", "126", "90", "county-link"),
            svg_line("126", "90", "216", "100", "county-link"),
            svg_line("126", "90", "140", "170", "county-link"),
            svg_line("216", "100", "196", "150", "county-link"),
            svg_line("140", "170", "196", "150", "county-link"),
            svg_line("140", "170", "128", "228", "county-link"),
            svg_line("128", "228", "176", "276", "county-link"),
            svg_line("196", "150", "176", "276", "county-link"),
            svg_line("90", "140", "126", "90", "county-link"),
            svg_line("232", "210", "196", "150", "county-link"),
            svg_circle("110", "44", "6", "county-site"),
            svg_circle("206", "36", "6", "county-site"),
            svg_circle("216", "100", "6", "county-site"),
            svg_circle("126", "90", "6", "county-site"),
            svg_circle("140", "170", "6", "county-site"),
            svg_circle("196", "150", "6", "county-site"),
            svg_circle("128", "228", "6", "county-site"),
            svg_circle("176", "276", "6", "county-site"),
            svg_circle("90", "140", "6", "county-site"),
            svg_circle("232", "210", "6", "county-site"),
        ],
    )
}

fn county_shape(path: &str, class: &str) -> SiteView {
    element("path", &[("d", path), ("class", class)], vec![])
}

fn svg_line(x1: &str, y1: &str, x2: &str, y2: &str, class: &str) -> SiteView {
    element(
        "line",
        &[
            ("x1", x1),
            ("y1", y1),
            ("x2", x2),
            ("y2", y2),
            ("class", class),
        ],
        vec![],
    )
}

fn svg_circle(cx: &str, cy: &str, radius: &str, class: &str) -> SiteView {
    element(
        "circle",
        &[("cx", cx), ("cy", cy), ("r", radius), ("class", class)],
        vec![],
    )
}

fn svg_text(x: &str, y: &str, class: &str, label: &str) -> SiteView {
    element(
        "text",
        &[
            ("x", x),
            ("y", y),
            ("text-anchor", "middle"),
            ("class", class),
        ],
        vec![txt(label)],
    )
}

fn costs() -> SiteView {
    element(
        "section",
        &[("class", "content-section")],
        vec![
            section_heading("05", "what it costs"),
            element(
                "div",
                &[("class", "table-wrap")],
                vec![element(
                    "table",
                    &[],
                    vec![
                        element(
                            "caption",
                            &[("class", "sr-only")],
                            vec![txt("Typical community radio hardware costs")],
                        ),
                        element(
                            "thead",
                            &[],
                            vec![element(
                                "tr",
                                &[],
                                vec![
                                    element("th", &[("scope", "col")], vec![txt("Item")]),
                                    element("th", &[("scope", "col")], vec![txt("Estimate")]),
                                ],
                            )],
                        ),
                        element(
                            "tbody",
                            &[],
                            vec![
                                cost("Heltec V4 radio, assembled and programmed", "~ $50"),
                                cost("T114 radio", "~ $30"),
                                cost("All-in-one solar node", "~ $100"),
                                cost("Battery, solar panel, or wall power", "varies by site"),
                                cost("Monthly service fees or subscriptions", "none"),
                                cost(
                                    "Ten-site county pilot",
                                    "materials + installation + training",
                                ),
                            ],
                        ),
                    ],
                )],
            ),
            element(
                "p",
                &[("class", "aside-copy")],
                vec![txt(
                    "Exact site costs depend on the host and placement. Measuring them is part of the pilot.",
                )],
            ),
        ],
    )
}

fn cost(item: &str, estimate: &str) -> SiteView {
    element(
        "tr",
        &[],
        vec![
            element("th", &[("scope", "row")], vec![txt(item)]),
            element("td", &[], vec![txt(estimate)]),
        ],
    )
}

fn partnership() -> SiteView {
    element(
        "section",
        &[("class", "two-up closing-grid")],
        vec![
            element(
                "article",
                &[("class", "info-card")],
                vec![
                    element(
                        "p",
                        &[("class", "card-kicker")],
                        vec![txt("06 · partnership")],
                    ),
                    element(
                        "p",
                        &[],
                        vec![txt(
                            "Merely is the technical partner: building, installing, and maintaining equipment, then training local hosts. An eligible public or nonprofit partner holds grant funds. The community owns its network.",
                        )],
                    ),
                ],
            ),
            element(
                "article",
                &[("class", "night-card")],
                vec![
                    element(
                        "p",
                        &[("class", "card-kicker")],
                        vec![txt("07 · open source")],
                    ),
                    element(
                        "p",
                        &[],
                        vec![txt(concat!(
                            "Retinue is our open-source Rust implementation of Reticulum. ",
                            "Stock Reticulum applications recognize our radios today, and the ",
                            "same radio family interoperates with Meshtastic and MeshCore.",
                        ))],
                    ),
                    external_link(
                        "https://github.com/merely-made/murm",
                        "Read the Retinue source ↗",
                        "button button-night",
                    ),
                ],
            ),
        ],
    )
}

mod lab_view;

use lab_view::{
    message_path_controls, message_path_header, message_path_ledger, message_path_radio,
    message_path_topology,
};
