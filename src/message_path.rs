//! The message path lab's reading of the committed retinue-sim traces.
//!
//! Everything the lab states about a route, a cut, a send, a frame or a
//! delivery is derived here from a trace, at site build time. The only site
//! data is [`LAYOUT`]: a label and a drawing position for each node, which are
//! illustrative and say nothing about geography or radio range. The firmware
//! has no ROUTE or DELIVERED screen, so route and delivery are site prose in
//! the ledger; the screen is radio-mirror's TRAFFIC page for one node.

use serde_json::json;

use crate::retinue_traces::{ARTIFACT_DIRECTORY, TraceEvent, TraceScenario, TraceSet, sha256_hex};

/// The inline document the lab's script reads.
pub const MANIFEST_SCHEMA: &str = "mer3ly.message-path-traces/v1";
/// The id of the `<script type="application/json">` holding it.
pub const MANIFEST_ID: &str = "message-path-traces";
/// The scenario a reader without script sees, and the script's default.
pub const DEFAULT_SCENARIO: &str = "cold";

/// How the lab draws one trace node. Site layout, not trace data.
#[derive(Clone, Copy, Debug)]
pub struct NodeLayout {
    pub name: &'static str,
    pub label: &'static str,
    /// Percent of the stage's width and height.
    pub x: u8,
    pub y: u8,
}

/// Labels and drawing positions for the trace's five radios. The positions
/// only keep the drawing legible; they are not where anything stands.
pub const LAYOUT: [NodeLayout; 5] = [
    NodeLayout {
        name: "fire",
        label: "Fire station",
        x: 17,
        y: 72,
    },
    NodeLayout {
        name: "church",
        label: "Church steeple",
        x: 34,
        y: 24,
    },
    NodeLayout {
        name: "water",
        label: "Water tower",
        x: 57,
        y: 55,
    },
    NodeLayout {
        name: "ridge",
        label: "Ridgeline",
        x: 78,
        y: 20,
    },
    NodeLayout {
        name: "garage",
        label: "County garage",
        x: 83,
        y: 76,
    },
];

pub fn layout(name: &str) -> Option<&'static NodeLayout> {
    LAYOUT.iter().find(|layout| layout.name == name)
}

/// Checks that the layout draws exactly the traces' nodes.
pub fn check_layout(traces: &TraceSet) -> Result<(), String> {
    for scenario in &traces.scenarios {
        let names = scenario
            .trace
            .nodes
            .iter()
            .map(|node| node.name.as_str())
            .collect::<Vec<_>>();
        let drawn = LAYOUT.iter().map(|layout| layout.name).collect::<Vec<_>>();
        if names != drawn {
            return Err(format!(
                "the lab draws {drawn:?}, but trace {} runs {names:?}",
                scenario.id
            ));
        }
    }
    Ok(())
}

/// A node as prose mid-sentence: "the fire station".
fn the(name: &str) -> String {
    match layout(name) {
        Some(layout) => format!("the {}", layout.label.to_lowercase()),
        None => format!("the {name} radio"),
    }
}

fn capitalized(text: String) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => text,
    }
}

fn listed(items: &[String]) -> String {
    match items {
        [] => String::new(),
        [one] => one.clone(),
        [init @ .., last] => format!("{} and {last}", init.join(", ")),
    }
}

/// Simulated milliseconds as the lab prints them: `10.9 s`.
pub fn seconds(t: u64) -> String {
    if t.is_multiple_of(100) {
        format!("{}.{} s", t / 1000, (t % 1000) / 100)
    } else {
        format!("{}.{:03} s", t / 1000, t % 1000)
    }
}

/// A message by its send's payload ("message 1"), or by its trace id.
pub fn message_name(scenario: &TraceScenario, id: u32) -> String {
    let trace = &scenario.trace;
    trace
        .messages
        .get(id as usize)
        .and_then(|message| {
            trace.sends.iter().find(|send| {
                send.at == message.sent_at && send.from == message.from && send.to == message.to
            })
        })
        .map_or_else(|| format!("message #{id}"), |send| send.payload.clone())
}

fn path_text(path: &[String]) -> String {
    path.iter()
        .map(|name| layout(name).map_or(name.clone(), |layout| layout.label.to_lowercase()))
        .collect::<Vec<_>>()
        .join(" → ")
}

fn relays_text(path: &[String]) -> String {
    match path.len().saturating_sub(2) {
        0 => "no relay".to_owned(),
        1 => "1 relay".to_owned(),
        count => format!("{count} relays"),
    }
}

fn packet_name(packet_type: &str) -> String {
    match packet_type {
        "announce" => "an announce".to_owned(),
        "link_request" => "a link request".to_owned(),
        "proof" => "a link proof".to_owned(),
        "data" => "a data frame".to_owned(),
        other => format!("a {other} frame"),
    }
}

/// One ledger row: an event the lab names in prose.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Milestone {
    pub event: usize,
    pub t: u64,
    pub kind: &'static str,
    pub text: String,
}

/// The trace's milestones, in order: cuts, announce rounds, sends and
/// refusals, every frame that is not an announce, expiries, and deliveries.
pub fn milestones(scenario: &TraceScenario) -> Vec<Milestone> {
    let trace = &scenario.trace;
    let mut rows = Vec::new();
    let mut announced_at = None;
    for (index, event) in trace.events.iter().enumerate() {
        let row = |kind, text: String| Milestone {
            event: index,
            t: event.t(),
            kind,
            text,
        };
        match event {
            TraceEvent::Cut { a, b, .. } => {
                rows.push(row(
                    "cut",
                    capitalized(format!(
                        "{}–{} link is cut: neither radio hears the other.",
                        the(a),
                        the(b).trim_start_matches("the ")
                    )),
                ));
            }
            TraceEvent::Transmit {
                t, origin, packet, ..
            } if packet.packet_type == "announce" => {
                // One row per announce round: the first radio's own announce.
                if origin == "announce" && announced_at != Some(*t) {
                    announced_at = Some(*t);
                    let names = trace
                        .events
                        .iter()
                        .filter_map(|other| match other {
                            TraceEvent::Transmit {
                                t: when,
                                node,
                                origin,
                                packet,
                                ..
                            } if when == t
                                && origin == "announce"
                                && packet.packet_type == "announce" =>
                            {
                                Some(the(node))
                            }
                            _ => None,
                        })
                        .collect::<Vec<_>>();
                    rows.push(row(
                        "announce",
                        capitalized(format!(
                            "{} each transmit their own announce, and the relays forward them. The announce interval is {}.",
                            listed(&names),
                            seconds(trace.timing.announce_interval)
                        )),
                    ));
                }
            }
            TraceEvent::Send {
                message,
                from,
                to,
                via,
                hops,
                ..
            } => {
                let route = match (via, hops) {
                    (Some(via), Some(hops)) => format!(
                        " Its route names {} as first relay, with hop count {hops}.",
                        the(via)
                    ),
                    (None, Some(hops)) => {
                        format!(
                            " Its route reaches {} directly, with hop count {hops}.",
                            the(to)
                        )
                    }
                    (Some(via), None) => format!(" It addresses {} as first relay.", the(via)),
                    (None, None) => " It has no route.".to_owned(),
                };
                rows.push(row(
                    "send",
                    capitalized(format!(
                        "{} sends {} to {}.{route}",
                        the(from),
                        message_name(scenario, *message),
                        the(to)
                    )),
                ));
            }
            TraceEvent::SendRefused {
                message,
                from,
                to,
                reason,
                ..
            } => {
                let why = match reason.as_str() {
                    "unknown_destination" => format!("it has not heard {} announce", the(to)),
                    "pending_full" => "its pending-link table is full".to_owned(),
                    other => other.replace('_', " "),
                };
                rows.push(row(
                    "refused",
                    capitalized(format!(
                        "{} will not open a link for {}: {why}.",
                        the(from),
                        message_name(scenario, *message)
                    )),
                ));
            }
            TraceEvent::Transmit {
                node,
                origin,
                packet,
                heard_by,
                blocked,
                ..
            } => {
                let frame = packet_name(&packet.packet_type);
                let mut text = match origin.as_str() {
                    "forward" => {
                        format!("{} relays {}", the(node), frame.replacen("a ", "the ", 1))
                    }
                    "reply" => format!("{} answers with {frame}", the(node)),
                    _ => format!("{} transmits {frame}", the(node)),
                };
                if packet.packet_type == "link_request"
                    && let Some(destination) = &packet.destination_node
                {
                    text.push_str(&format!(" for {}", the(destination)));
                }
                if let Some(transport) = &packet.transport {
                    text.push_str(&format!(", addressed through {}", the(transport)));
                }
                text.push('.');
                let heard = heard_by.iter().map(|name| the(name)).collect::<Vec<_>>();
                if heard.is_empty() {
                    text.push_str(" No radio hears it.");
                } else {
                    text.push_str(&format!(" Heard by {}.", listed(&heard)));
                }
                let behind = blocked.iter().map(|name| the(name)).collect::<Vec<_>>();
                if !behind.is_empty() {
                    text.push_str(&format!(
                        " Behind the cut, {} {} not hear it.",
                        listed(&behind),
                        if behind.len() == 1 { "does" } else { "do" }
                    ));
                }
                rows.push(row("transmit", capitalized(text)));
            }
            TraceEvent::LinkRequestExpired { node, message, .. } => {
                let what = message.map_or_else(
                    || "a link request".to_owned(),
                    |message| format!("the link request for {}", message_name(scenario, message)),
                );
                rows.push(row(
                    "expired",
                    capitalized(format!(
                        "{} drops {what}: no proof came back by its deadline.",
                        the(node)
                    )),
                ));
            }
            TraceEvent::Delivered {
                message,
                node,
                path,
                ..
            } => {
                rows.push(row(
                    "delivered",
                    capitalized(format!(
                        "{} reaches {}. Path: {}, {}.",
                        message_name(scenario, *message),
                        the(node),
                        path_text(path),
                        relays_text(path)
                    )),
                ));
            }
            TraceEvent::Receive { .. } => {}
        }
    }
    rows
}

/// A scenario's name as the lab offers it, from its id and its cut.
pub fn scenario_label(scenario: &TraceScenario) -> String {
    let kind = match scenario.id.as_str() {
        "cold" => "Cold start",
        "warm" => "Warm cut",
        other => other,
    };
    match scenario.trace.cuts.first() {
        Some(cut) if cut.at == 0 => format!(
            "{kind}: {}–{} cut from the start",
            layout(&cut.a).map_or(cut.a.as_str(), |layout| layout.label),
            the(&cut.b).trim_start_matches("the ")
        ),
        Some(cut) => format!(
            "{kind}: {}–{} cut at {}",
            layout(&cut.a).map_or(cut.a.as_str(), |layout| layout.label),
            the(&cut.b).trim_start_matches("the "),
            seconds(cut.at)
        ),
        None => kind.to_owned(),
    }
}

/// The static reading's route sentence: the scenario's first message, as
/// the trace delivered it.
pub fn route_sentence(scenario: &TraceScenario) -> String {
    let Some(message) = scenario.trace.messages.first() else {
        return "This trace sends no message.".to_owned();
    };
    let name = message_name(scenario, message.id);
    match &message.delivered {
        Some(delivered) => capitalized(format!(
            "{name} delivered at {}: {}, {}.",
            seconds(delivered.t),
            path_text(&delivered.path),
            relays_text(&delivered.path)
        )),
        None => capitalized(format!("{name} was not delivered.")),
    }
}

/// What the reader without script sees, and where the script starts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DefaultView {
    pub scenario: String,
    pub step: usize,
    pub node: String,
}

/// The cold trace's last event, on the first message's destination.
pub fn default_view(traces: &TraceSet) -> Result<DefaultView, String> {
    let scenario = traces
        .scenario(DEFAULT_SCENARIO)
        .ok_or("the default scenario is missing")?;
    let node = scenario
        .trace
        .messages
        .first()
        .map(|message| message.to.clone())
        .ok_or("the default scenario sends no message")?;
    let step = scenario
        .trace
        .events
        .len()
        .checked_sub(1)
        .ok_or("the default scenario has no events")?;
    Ok(DefaultView {
        scenario: scenario.id.clone(),
        step,
        node,
    })
}

/// The no-script screen: the default node's TRAFFIC page at the default step,
/// from its face-track documents, drawn by radio-mirror.
pub fn static_screen(traces: &TraceSet) -> Result<mer3ly_radio_mirror::StaticScreen, String> {
    let view = default_view(traces)?;
    let scenario = traces
        .scenario(&view.scenario)
        .ok_or("the default scenario is missing")?;
    let path = format!(
        "radio-mirror/message-path-{}-{}.png",
        view.scenario, view.node
    );
    match scenario.face_at(&view.node, view.step) {
        Some(entry) => mer3ly_radio_mirror::lab_screen(
            path,
            &entry.local.to_string(),
            Some(&entry.host.to_string()),
        ),
        None => {
            mer3ly_radio_mirror::lab_screen(path, r#"{"schema":"radio-mirror.local/v1"}"#, None)
        }
    }
}

/// A published trace file's address, versioned by its bytes.
pub fn trace_href(file: &str, bytes: &[u8]) -> String {
    format!(
        "/{ARTIFACT_DIRECTORY}/{file}?v={}",
        &sha256_hex(bytes)[..12]
    )
}

/// The inline document the lab's script reads: where each trace is, what it
/// is called, its ledger rows, and where the traces came from.
pub fn manifest_json(traces: &TraceSet) -> Result<String, String> {
    check_layout(traces)?;
    let view = default_view(traces)?;
    let provenance = &traces.provenance;
    let scenarios = traces
        .scenarios
        .iter()
        .zip(&provenance.trace)
        .map(|(scenario, source)| {
            json!({
                "id": scenario.id,
                "name": scenario.trace.scenario,
                "label": scenario_label(scenario),
                "route": trace_href(&scenario.route_file, &scenario.route_bytes),
                "face": trace_href(&scenario.face_file, &scenario.face_bytes),
                "trace_sha256": scenario.trace_sha256(),
                "events": scenario.trace.events.len(),
                "commands": [source.route_command, source.face_command],
                "milestones": milestones(scenario)
                    .into_iter()
                    .map(|row| json!({ "event": row.event, "kind": row.kind, "text": row.text }))
                    .collect::<Vec<_>>(),
            })
        })
        .collect::<Vec<_>>();
    serde_json::to_string(&json!({
        "schema": MANIFEST_SCHEMA,
        "source": {
            "repository": provenance.repository,
            "revision": provenance.revision,
            "crate": provenance.sim_crate,
        },
        "surface": mer3ly_radio_mirror::SURFACE,
        "page": mer3ly_radio_mirror::LAB_PAGE,
        "default": { "scenario": view.scenario, "step": view.step, "node": view.node },
        "nodes": LAYOUT
            .iter()
            .map(|layout| json!({ "name": layout.name, "label": layout.label }))
            .collect::<Vec<_>>(),
        "scenarios": scenarios,
    }))
    .map_err(|error| format!("serialize the message path manifest: {error}"))
}

/// The manifest as embedded in the radio page.
pub fn manifest_embedded(traces: &TraceSet) -> Result<String, String> {
    Ok(manifest_json(traces)?
        .replace('<', "\\u003c")
        .replace('>', "\\u003e")
        .replace('&', "\\u0026"))
}

/// Where the lab's traces and screen come from, as the page states it.
pub fn source_statement(traces: &TraceSet) -> String {
    format!(
        "Each step is an event in a route trace that retinue-sim generated by running Retinue nodes over this five-radio topology, at retinue revision {}. The traces are committed to this site with their provenance; the site does not run the simulator. The screen is radio-mirror drawing that radio's TRAFFIC page from the trace's state. Radio positions are illustrative, not geography or radio range.",
        traces.provenance.short_revision()
    )
}
