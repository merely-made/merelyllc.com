use sha2::{Digest, Sha256};

use crate::site::{
    ActivePage, PageMetadata, SiteView, element, external_link, render_with_body_end,
    section_heading, shell, txt,
};

const MESSAGE_PATH_LAB: &[u8] = include_bytes!("../../assets/message-path-lab.js");

pub const METADATA: PageMetadata = PageMetadata {
    title: "Community radio | Merely",
    description: "A low-cost, open-source LoRa radio pilot for community-owned backup communications across the FIVCO counties.",
    canonical_url: "https://mer3ly.net/radio.html",
};

pub fn document() -> String {
    let digest = format!("{:x}", Sha256::digest(MESSAGE_PATH_LAB));
    let bootstrap = format!(
        "<script type=\"module\" src=\"/message-path-lab.js?v={}\"></script>",
        &digest[..12]
    );
    render_with_body_end(&METADATA, view, &bootstrap)
}

pub fn view() -> SiteView {
    shell(
        ActivePage::Radio,
        element(
            "main",
            &[("id", "main"), ("class", "radio-main")],
            vec![
                hero(),
                problem_solution(),
                mesh(),
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

fn mesh() -> SiteView {
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
                    ("data-blocked", "true"),
                    ("data-step", "5"),
                ],
                vec![
                    message_path_header(),
                    message_path_controls(),
                    element(
                        "div",
                        &[("class", "message-path-workbench")],
                        vec![message_path_topology(), message_path_projections()],
                    ),
                    element(
                        "figcaption",
                        &[],
                        vec![txt(
                            "When a direct path is unavailable, a message follows the relays that can still hear one another.",
                        )],
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
                        vec![txt("Pull the mesh. Follow the message.")],
                    ),
                    element(
                        "p",
                        &[("id", "message-path-description")],
                        vec![txt(
                            "Move any radio, change the direct path, then send or scrub through the exchange.",
                        )],
                    ),
                ],
            ),
            element(
                "p",
                &[("class", "message-path-boundary")],
                vec![txt(
                    "Deterministic model · not a live traffic or radio-range receipt",
                )],
            ),
        ],
    )
}

fn message_path_controls() -> SiteView {
    element(
        "div",
        &[
            ("class", "message-path-controls"),
            ("aria-label", "Message path controls"),
        ],
        vec![
            element(
                "button",
                &[
                    ("class", "button button-primary"),
                    ("type", "button"),
                    ("data-path-action", "send"),
                ],
                vec![txt("Send message")],
            ),
            element(
                "button",
                &[
                    ("class", "button button-quiet"),
                    ("type", "button"),
                    ("data-path-action", "previous"),
                    ("aria-label", "Previous exchange step"),
                ],
                vec![txt("Previous")],
            ),
            element(
                "button",
                &[
                    ("class", "button button-quiet"),
                    ("type", "button"),
                    ("data-path-action", "next"),
                    ("aria-label", "Next exchange step"),
                ],
                vec![txt("Next")],
            ),
            element(
                "label",
                &[("class", "message-path-scrubber")],
                vec![
                    element(
                        "span",
                        &[],
                        vec![
                            txt("Exchange step "),
                            element(
                                "output",
                                &[("data-path-step-output", "")],
                                vec![txt("6 of 6")],
                            ),
                        ],
                    ),
                    element(
                        "input",
                        &[
                            ("type", "range"),
                            ("min", "0"),
                            ("max", "5"),
                            ("step", "1"),
                            ("value", "5"),
                            ("data-path-step", ""),
                        ],
                        vec![],
                    ),
                ],
            ),
            element(
                "label",
                &[("class", "message-path-toggle")],
                vec![
                    element(
                        "input",
                        &[
                            ("type", "checkbox"),
                            ("checked", "checked"),
                            ("data-path-blocked", ""),
                        ],
                        vec![],
                    ),
                    element("span", &[], vec![txt("Direct path blocked")]),
                ],
            ),
            element(
                "button",
                &[
                    ("class", "button button-quiet"),
                    ("type", "button"),
                    ("data-path-action", "share"),
                ],
                vec![txt("Share scene")],
            ),
            element(
                "span",
                &[
                    ("class", "message-path-status"),
                    ("data-path-status", ""),
                    ("role", "status"),
                    ("aria-live", "polite"),
                ],
                vec![txt("Message delivered by three relays.")],
            ),
        ],
    )
}

fn message_path_topology() -> SiteView {
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
                        vec![txt("Reroute · fire → church → water → garage")],
                    ),
                ],
            ),
            element(
                "div",
                &[("class", "message-path-stage"), ("data-path-stage", "")],
                vec![
                    element(
                        "svg",
                        &[
                            ("class", "message-path-links"),
                            ("aria-hidden", "true"),
                            ("data-path-links", ""),
                        ],
                        vec![
                            message_path_edge("fire-church", "fire", "church", true, false),
                            message_path_edge("church-water", "church", "water", true, false),
                            message_path_edge("water-ridge", "water", "ridge", false, false),
                            message_path_edge("water-garage", "water", "garage", true, false),
                            message_path_edge("fire-water", "fire", "water", false, true),
                            element(
                                "circle",
                                &[
                                    ("class", "message-path-packet"),
                                    ("r", "6"),
                                    ("data-path-packet", ""),
                                    ("hidden", "hidden"),
                                ],
                                vec![],
                            ),
                        ],
                    ),
                    message_path_node("fire", "Fire station", "17", "72"),
                    message_path_node("church", "Church steeple", "34", "24"),
                    message_path_node("water", "Water tower", "57", "55"),
                    message_path_node("ridge", "Ridgeline", "78", "20"),
                    message_path_node("garage", "County garage", "83", "76"),
                ],
            ),
            element(
                "p",
                &[("class", "message-path-help")],
                vec![txt(
                    "Drag a radio, or focus it and use the arrow keys. Every edge follows.",
                )],
            ),
            element(
                "p",
                &[("class", "message-path-fallback")],
                vec![txt(
                    "Static route: fire station → church steeple → water tower → county garage. The direct fire-to-water path is blocked.",
                )],
            ),
        ],
    )
}

fn message_path_edge(id: &str, from: &str, to: &str, on_route: bool, blocked: bool) -> SiteView {
    let class = match (on_route, blocked) {
        (true, false) => "message-path-edge is-route",
        (false, true) => "message-path-edge is-blocked",
        _ => "message-path-edge",
    };
    element(
        "line",
        &[
            ("class", class),
            ("data-lab-edge", id),
            ("data-from", from),
            ("data-to", to),
        ],
        vec![],
    )
}

fn message_path_node(id: &str, label: &str, x: &str, y: &str) -> SiteView {
    let style = format!("left:{x}%;top:{y}%");
    let attrs = [
        ("class", "message-path-node"),
        ("type", "button"),
        ("data-lab-node", id),
        ("data-x", x),
        ("data-y", y),
        ("style", style.as_str()),
        ("aria-label", label),
    ];
    element(
        "button",
        &attrs,
        vec![
            element("span", &[("class", "message-path-node-mark")], vec![]),
            element(
                "span",
                &[("class", "message-path-node-label")],
                vec![txt(label)],
            ),
        ],
    )
}

fn message_path_projections() -> SiteView {
    element(
        "aside",
        &[
            ("class", "message-path-projections"),
            ("aria-label", "Synchronized message views"),
        ],
        vec![message_path_radio(), message_path_ledger()],
    )
}

fn message_path_radio() -> SiteView {
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
                        &[("class", "eyebrow")],
                        vec![txt("attached-host radio view")],
                    ),
                    element("p", &[("class", "message-path-led")], vec![txt("TX")]),
                ],
            ),
            element(
                "div",
                &[
                    ("class", "message-path-oled"),
                    ("aria-label", "Attached host radio screen"),
                ],
                vec![
                    element(
                        "div",
                        &[("class", "message-path-oled-header")],
                        vec![
                            element(
                                "span",
                                &[("data-path-screen-header", "")],
                                vec![txt("RET · DELIVERED")],
                            ),
                            element("span", &[("data-path-screen-count", "")], vec![txt("6/6")]),
                        ],
                    ),
                    message_path_screen_row("STATE", "RX FRAME", "state"),
                    message_path_screen_row("HOP", "GARAGE", "hop"),
                    message_path_screen_row("SEQ", "05", "sequence"),
                    message_path_screen_row("HOST", "ATTACHED", "host"),
                ],
            ),
            element(
                "p",
                &[("class", "message-path-radio-note")],
                vec![txt(
                    "The host supplies route context; the radio reports frame traffic.",
                )],
            ),
        ],
    )
}

fn message_path_screen_row(label: &str, value: &str, id: &str) -> SiteView {
    element(
        "p",
        &[("class", "message-path-screen-row")],
        vec![
            element("span", &[], vec![txt(label)]),
            element("strong", &[("data-path-screen-row", id)], vec![txt(value)]),
        ],
    )
}

fn message_path_ledger() -> SiteView {
    let events = [
        "Route selected through church and water",
        "Fire station queues the message",
        "Church steeple receives the frame",
        "Water tower receives the relay",
        "Water tower forwards to the garage",
        "County garage confirms delivery",
    ];
    element(
        "section",
        &[("class", "message-path-ledger")],
        vec![
            element(
                "div",
                &[("class", "message-path-panel-heading")],
                vec![
                    element("p", &[("class", "eyebrow")], vec![txt("event ledger")]),
                    element(
                        "p",
                        &[("data-path-ledger-count", "")],
                        vec![txt("06 events")],
                    ),
                ],
            ),
            element(
                "ol",
                &[],
                events
                    .iter()
                    .enumerate()
                    .map(|(index, event)| {
                        let index_text = format!("{index:02}");
                        let data_index = index.to_string();
                        let class = if index == 5 {
                            "message-path-event is-current"
                        } else {
                            "message-path-event is-complete"
                        };
                        let attrs = [("class", class), ("data-lab-event", data_index.as_str())];
                        element(
                            "li",
                            &attrs,
                            vec![
                                element(
                                    "span",
                                    &[("class", "message-path-event-index")],
                                    vec![txt(index_text)],
                                ),
                                element("span", &[("data-path-event-copy", "")], vec![txt(*event)]),
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
