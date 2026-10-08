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
    let digest = format!("{:x}", digest.finalize());
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
                                    "{}/tree/{}/crates/retinue-sim",
                                    traces.provenance.repository, traces.provenance.revision
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

fn message_path_header() -> SiteView {
    element(
        "header",
        &[("class", "message-path-header")],
        vec![
            element(
                "div",
                &[],
                vec![
                    element("p", &[("class", "eyebrow")], vec![txt("message path lab")]),
                    element(
                        "h3",
                        &[("id", "message-path-title")],
                        vec![txt("Cut a link. Follow the trace.")],
                    ),
                    element(
                        "p",
                        &[("id", "message-path-description")],
                        vec![txt(
                            "Step through what five Retinue radios did: the cut, each send, every frame on the air and who heard it, the requests that expired, and the deliveries.",
                        )],
                    ),
                ],
            ),
            element(
                "p",
                &[("class", "message-path-boundary")],
                vec![txt(
                    "Generated route trace · not a live traffic or radio-range receipt",
                )],
            ),
        ],
    )
}

fn quiet_button(action: &str, label: &str, aria_label: &str) -> SiteView {
    element(
        "button",
        &[
            ("class", "button button-quiet"),
            ("type", "button"),
            ("data-path-action", action),
            ("aria-label", aria_label),
        ],
        vec![txt(label)],
    )
}

fn message_path_controls(state: &LabState<'_>) -> SiteView {
    let events = state.scenario.trace.events.len();
    let max = (events - 1).to_string();
    let value = state.view.step.to_string();
    let reading = state
        .current_milestone()
        .map(step_reading)
        .unwrap_or_default();
    let options = state
        .traces
        .scenarios
        .iter()
        .map(|scenario| {
            let label = message_path::scenario_label(scenario);
            let mut attrs = vec![("value", scenario.id.as_str())];
            if scenario.id == state.view.scenario {
                attrs.push(("selected", "selected"));
            }
            element("option", &attrs, vec![txt(label)])
        })
        .collect();
    element(
        "div",
        &[
            ("class", "message-path-controls"),
            ("role", "group"),
            ("aria-label", "Message path controls"),
        ],
        vec![
            element(
                "label",
                &[("class", "message-path-scenario")],
                vec![
                    element("span", &[], vec![txt("Trace")]),
                    element("select", &[("data-path-scenario", "")], options),
                ],
            ),
            element(
                "button",
                &[
                    ("class", "button button-primary"),
                    ("type", "button"),
                    ("data-path-action", "play"),
                ],
                vec![txt("Play trace")],
            ),
            quiet_button("previous", "Previous", "Previous trace event"),
            quiet_button("next", "Next", "Next trace event"),
            quiet_button("previous-milestone", "◀ Ledger", "Previous ledger row"),
            quiet_button("next-milestone", "Ledger ▶", "Next ledger row"),
            element(
                "label",
                &[("class", "message-path-scrubber")],
                vec![
                    element(
                        "span",
                        &[],
                        vec![
                            txt("Trace event "),
                            element(
                                "output",
                                &[("data-path-step-output", "")],
                                vec![txt(format!(
                                    "{} of {events} · {}",
                                    state.view.step + 1,
                                    seconds(state.event().t())
                                ))],
                            ),
                        ],
                    ),
                    element(
                        "input",
                        &[
                            ("type", "range"),
                            ("min", "0"),
                            ("max", max.as_str()),
                            ("step", "1"),
                            ("value", value.as_str()),
                            ("data-path-step", ""),
                        ],
                        vec![],
                    ),
                ],
            ),
            quiet_button("share", "Share step", "Share this trace step"),
            element(
                "p",
                &[
                    ("class", "message-path-status"),
                    ("data-path-status", ""),
                    ("role", "status"),
                    ("aria-live", "polite"),
                ],
                vec![txt(reading)],
            ),
        ],
    )
}

fn message_path_topology(state: &LabState<'_>) -> SiteView {
    let trace = &state.scenario.trace;
    let route = state.delivered_path();
    let on_route = |a: &str, b: &str| {
        route.is_some_and(|path| {
            path.windows(2)
                .any(|pair| (pair[0] == a && pair[1] == b) || (pair[0] == b && pair[1] == a))
        })
    };
    let active = match state.event() {
        TraceEvent::Transmit { node, .. }
        | TraceEvent::Receive { node, .. }
        | TraceEvent::LinkRequestExpired { node, .. }
        | TraceEvent::Delivered { node, .. } => Some(node.as_str()),
        TraceEvent::Send { from, .. } | TraceEvent::SendRefused { from, .. } => Some(from.as_str()),
        TraceEvent::Cut { .. } => None,
    };
    let mut drawing = trace
        .edges
        .iter()
        .map(|edge| {
            let mut class = "message-path-edge".to_owned();
            if on_route(&edge.a, &edge.b) {
                class.push_str(" is-route");
            }
            if state.edge_is_cut(&edge.a, &edge.b) {
                class.push_str(" is-cut");
            }
            message_path_edge(&edge.a, &edge.b, &class)
        })
        .collect::<Vec<_>>();
    drawing.push(element(
        "circle",
        &[
            ("class", "message-path-packet"),
            ("r", "6"),
            ("data-path-packet", ""),
            ("hidden", "hidden"),
        ],
        vec![],
    ));
    let mut stage = vec![element(
        "svg",
        &[
            ("class", "message-path-links"),
            ("aria-hidden", "true"),
            ("data-path-links", ""),
        ],
        drawing,
    )];
    for node in &trace.nodes {
        let layout = layout(&node.name).expect("the layout draws every trace node");
        stage.push(message_path_node(
            layout,
            node.transit,
            active == Some(layout.name),
            state.view.node == layout.name,
        ));
    }
    element(
        "section",
        &[
            ("class", "message-path-topology"),
            (
                "aria-labelledby",
                "message-path-title message-path-description",
            ),
        ],
        vec![
            element(
                "div",
                &[("class", "message-path-topology-header")],
                vec![
                    element("p", &[("class", "eyebrow")], vec![txt("topology")]),
                    element(
                        "p",
                        &[("data-path-route", "")],
                        vec![txt(message_path::route_sentence(state.scenario))],
                    ),
                ],
            ),
            element(
                "div",
                &[("class", "message-path-stage"), ("data-path-stage", "")],
                stage,
            ),
            element(
                "p",
                &[("class", "message-path-help")],
                vec![txt(
                    "Choose a radio to see its own screen. Solid lines are links the trace runs over; a dashed line is cut. Positions are illustrative, not geography or radio range.",
                )],
            ),
        ],
    )
}

fn message_path_edge(a: &str, b: &str, class: &str) -> SiteView {
    let id = format!("{a}-{b}");
    element(
        "line",
        &[
            ("class", class),
            ("data-lab-edge", id.as_str()),
            ("data-from", a),
            ("data-to", b),
        ],
        vec![],
    )
}

fn message_path_node(layout: &NodeLayout, transit: bool, active: bool, selected: bool) -> SiteView {
    let style = format!("left:{}%;top:{}%", layout.x, layout.y);
    let role = if transit { "relays" } else { "does not relay" };
    let aria = format!("{}, {role}. Show its screen.", layout.label);
    let mut class = "message-path-node".to_owned();
    if active {
        class.push_str(" is-active");
    }
    if selected {
        class.push_str(" is-selected");
    }
    let transit_attr = transit.to_string();
    element(
        "button",
        &[
            ("class", class.as_str()),
            ("type", "button"),
            ("data-lab-node", layout.name),
            ("data-transit", transit_attr.as_str()),
            ("style", style.as_str()),
            ("aria-label", aria.as_str()),
            ("aria-pressed", if selected { "true" } else { "false" }),
        ],
        vec![
            element("span", &[("class", "message-path-node-mark")], vec![]),
            element(
                "span",
                &[("class", "message-path-node-label")],
                vec![
                    txt(layout.label),
                    element(
                        "small",
                        &[],
                        vec![txt(if transit { "relay" } else { "leaf" })],
                    ),
                ],
            ),
        ],
    )
}

fn layout_label(name: &str) -> &str {
    layout(name).map_or(name, |layout| layout.label)
}

fn message_path_radio(state: &LabState<'_>) -> SiteView {
    let screen = message_path::static_screen(state.traces)
        .unwrap_or_else(|error| panic!("the message path screen: {error}"));
    let src = format!("/{}", screen.path);
    element(
        "section",
        &[("class", "message-path-radio")],
        vec![
            element(
                "div",
                &[("class", "message-path-panel-heading")],
                vec![
                    element(
                        "p",
                        &[("class", "eyebrow"), ("data-path-screen-node", "")],
                        vec![txt(format!(
                            "{} · its own screen",
                            layout_label(&state.view.node)
                        ))],
                    ),
                    element(
                        "p",
                        &[("class", "message-path-page")],
                        vec![txt(screen.screen.to_uppercase())],
                    ),
                ],
            ),
            element(
                "div",
                &[
                    ("class", "message-path-oled"),
                    ("data-path-screen", ""),
                    ("data-screen-name", screen.screen.as_str()),
                ],
                vec![
                    // The text list below is the reading; the pixels repeat it.
                    element(
                        "img",
                        &[
                            ("src", src.as_str()),
                            ("alt", ""),
                            ("width", "128"),
                            ("height", "64"),
                            ("data-path-static", ""),
                        ],
                        vec![],
                    ),
                    element(
                        "canvas",
                        &[
                            ("width", "128"),
                            ("height", "64"),
                            ("aria-hidden", "true"),
                            ("hidden", ""),
                            ("data-path-canvas", ""),
                        ],
                        vec![],
                    ),
                ],
            ),
            element(
                "p",
                &[("class", "message-path-reading-label")],
                vec![txt("What its screen says")],
            ),
            element(
                "ul",
                &[
                    ("class", "message-path-screen-text"),
                    ("data-path-screen-text", ""),
                ],
                screen
                    .lines
                    .iter()
                    .map(|line| element("li", &[], vec![txt(line.as_str())]))
                    .collect(),
            ),
            element(
                "p",
                &[("class", "message-path-radio-note")],
                vec![txt(
                    "radio-mirror draws this radio's own TRAFFIC page from its state at the step. Firmware has no route or delivery screen: a radio knows its next hop, not the whole route, so the route and the deliveries are written in the ledger instead.",
                )],
            ),
        ],
    )
}

fn message_path_ledger(state: &LabState<'_>) -> SiteView {
    let current = state.current_milestone().map(|row| row.event);
    let count = format!(
        "{} rows · {} events",
        state.milestones.len(),
        state.scenario.trace.events.len()
    );
    element(
        "section",
        &[("class", "message-path-ledger")],
        vec![
            element(
                "div",
                &[("class", "message-path-panel-heading")],
                vec![
                    element("p", &[("class", "eyebrow")], vec![txt("trace ledger")]),
                    element("p", &[("data-path-ledger-count", "")], vec![txt(count)]),
                ],
            ),
            element(
                "ol",
                &[("data-path-ledger", "")],
                state
                    .milestones
                    .iter()
                    .map(|row| {
                        let event = row.event.to_string();
                        let class = if Some(row.event) == current {
                            "message-path-event is-current"
                        } else if Some(row.event) < current {
                            "message-path-event is-complete"
                        } else {
                            "message-path-event"
                        };
                        let mut attrs = vec![
                            ("class", class),
                            ("data-lab-event", event.as_str()),
                            ("data-kind", row.kind),
                        ];
                        if Some(row.event) == current {
                            attrs.push(("aria-current", "step"));
                        }
                        element(
                            "li",
                            &attrs,
                            vec![
                                element(
                                    "span",
                                    &[("class", "message-path-event-index")],
                                    vec![txt(seconds(row.t))],
                                ),
                                element(
                                    "span",
                                    &[("data-path-event-copy", "")],
                                    vec![txt(row.text.as_str())],
                                ),
                            ],
                        )
                    })
                    .collect(),
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
                        "https://github.com/merely-made/retinue",
                        "Read the Retinue source ↗",
                        "button button-night",
                    ),
                ],
            ),
        ],
    )
}
