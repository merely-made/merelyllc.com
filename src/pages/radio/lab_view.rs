use super::*;

pub(super) fn message_path_header() -> SiteView {
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

pub(super) fn quiet_button(action: &str, label: &str, aria_label: &str) -> SiteView {
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

pub(super) fn message_path_controls(state: &LabState<'_>) -> SiteView {
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

pub(super) fn message_path_topology(state: &LabState<'_>) -> SiteView {
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

pub(super) fn message_path_edge(a: &str, b: &str, class: &str) -> SiteView {
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

pub(super) fn message_path_node(
    layout: &NodeLayout,
    transit: bool,
    active: bool,
    selected: bool,
) -> SiteView {
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

pub(super) fn layout_label(name: &str) -> &str {
    layout(name).map_or(name, |layout| layout.label)
}

pub(super) fn message_path_radio(state: &LabState<'_>) -> SiteView {
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

pub(super) fn message_path_ledger(state: &LabState<'_>) -> SiteView {
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
