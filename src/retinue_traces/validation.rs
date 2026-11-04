use super::*;

pub(super) fn is_hex(value: &str, len: usize) -> bool {
    value.len() == len
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

pub(super) fn check_provenance(provenance: &Provenance, errors: &mut Vec<String>) {
    if provenance.schema != PROVENANCE_SCHEMA {
        errors.push(format!(
            "trace provenance schema {:?} is not {PROVENANCE_SCHEMA:?}",
            provenance.schema
        ));
    }
    if provenance.repository != RETINUE_REPOSITORY {
        errors.push("trace provenance does not name the retinue repository".to_owned());
    }
    if !is_hex(&provenance.revision, 40) {
        errors.push("trace provenance revision is not a full commit hash".to_owned());
    }
    if provenance.sim_crate != SIM_CRATE || provenance.crate_license != "MPL-2.0" {
        errors.push("trace provenance does not name retinue-sim under MPL-2.0".to_owned());
    }
    let ids = provenance
        .trace
        .iter()
        .map(|trace| trace.id.as_str())
        .collect::<Vec<_>>();
    if ids != SCENARIO_IDS {
        errors.push(format!(
            "trace provenance lists {ids:?}, expected {SCENARIO_IDS:?}"
        ));
    }
}

pub(super) fn check_scenario(
    entry: &TraceProvenance,
    read: &impl Fn(&str) -> Option<Vec<u8>>,
    errors: &mut Vec<String>,
) -> Option<TraceScenario> {
    let id = &entry.id;
    let before = errors.len();
    let expected = [
        ("route", &entry.route, format!("{id}.route-trace.json")),
        ("face", &entry.face, format!("{id}.face-track.json")),
        (
            "route command",
            &entry.route_command,
            format!("cargo run -q -p retinue-sim --example lab -- {id}"),
        ),
        (
            "face command",
            &entry.face_command,
            format!("cargo run -q -p retinue-sim --example lab --features face -- {id} --faces"),
        ),
        ("route schema", &entry.route_schema, ROUTE_SCHEMA.to_owned()),
        ("face schema", &entry.face_schema, FACE_SCHEMA.to_owned()),
    ];
    for (field, found, wanted) in expected {
        if *found != wanted {
            errors.push(format!(
                "trace {id}: {field} is {found:?}, expected {wanted:?}"
            ));
        }
    }
    let route_bytes = read(&entry.route);
    let face_bytes = read(&entry.face);
    let (Some(route_bytes), Some(face_bytes)) = (route_bytes, face_bytes) else {
        errors.push(format!(
            "trace {id}: a file its provenance names is missing"
        ));
        return None;
    };
    for (name, bytes, sha) in [
        (&entry.route, &route_bytes, &entry.route_sha256),
        (&entry.face, &face_bytes, &entry.face_sha256),
    ] {
        if sha256_hex(bytes) != *sha {
            errors.push(format!(
                "trace {id}: {name} differs from the sha256 its provenance records"
            ));
        }
        // The generator prints one compact JSON document and a newline.
        if !bytes.ends_with(b"}\n") || bytes[..bytes.len() - 1].contains(&b'\n') {
            errors.push(format!(
                "trace {id}: {name} is not one compact JSON line ending in a newline"
            ));
        }
    }
    let trace: RouteTrace = match serde_json::from_slice(&route_bytes) {
        Ok(trace) => trace,
        Err(error) => {
            errors.push(format!(
                "trace {id}: {} is unreadable: {error}",
                entry.route
            ));
            return None;
        }
    };
    let faces: FaceTrack = match serde_json::from_slice(&face_bytes) {
        Ok(faces) => faces,
        Err(error) => {
            errors.push(format!("trace {id}: {} is unreadable: {error}", entry.face));
            return None;
        }
    };
    check_route_trace(id, &trace, errors);
    check_face_track(id, &trace, &route_bytes, &faces, errors);
    (errors.len() == before).then(|| TraceScenario {
        id: id.clone(),
        route_file: entry.route.clone(),
        face_file: entry.face.clone(),
        route_bytes,
        face_bytes,
        trace,
        faces,
    })
}

pub(super) fn check_route_trace(id: &str, trace: &RouteTrace, errors: &mut Vec<String>) {
    if trace.schema != ROUTE_SCHEMA {
        errors.push(format!("trace {id}: route schema is {:?}", trace.schema));
    }
    let names = trace
        .nodes
        .iter()
        .map(|node| node.name.as_str())
        .collect::<BTreeSet<_>>();
    if names.len() != trace.nodes.len() {
        errors.push(format!("trace {id}: node names repeat"));
    }
    let known = |name: &str| names.contains(name);
    for edge in &trace.edges {
        if !known(&edge.a) || !known(&edge.b) {
            errors.push(format!("trace {id}: an edge names an unknown node"));
        }
    }
    for cut in &trace.cuts {
        if !trace
            .edges
            .iter()
            .any(|edge| same_edge(edge, &cut.a, &cut.b))
        {
            errors.push(format!("trace {id}: a cut names no edge"));
        }
    }
    let mut last = 0;
    for (index, event) in trace.events.iter().enumerate() {
        if event.t() < last {
            errors.push(format!("trace {id}: event {index} runs backward in time"));
        }
        last = event.t();
        if let Some(name) = event.named_nodes().into_iter().find(|name| !known(name)) {
            errors.push(format!(
                "trace {id}: event {index} names unknown node {name:?}"
            ));
        }
    }
    for (index, message) in trace.messages.iter().enumerate() {
        if message.id as usize != index {
            errors.push(format!("trace {id}: message ids are not their positions"));
        }
        if let Some(delivered) = &message.delivered
            && (delivered.path.first() != Some(&message.from)
                || delivered.path.last() != Some(&message.to))
        {
            errors.push(format!(
                "trace {id}: message {} was delivered on a path that does not join its ends",
                message.id
            ));
        }
    }
}

pub(super) fn check_face_track(
    id: &str,
    trace: &RouteTrace,
    route_bytes: &[u8],
    faces: &FaceTrack,
    errors: &mut Vec<String>,
) {
    if faces.schema != FACE_SCHEMA {
        errors.push(format!("trace {id}: face schema is {:?}", faces.schema));
    }
    if faces.route_trace != trace.schema || faces.scenario != trace.scenario {
        errors.push(format!(
            "trace {id}: the face track names another route trace or scenario"
        ));
    }
    // The digest covers the route trace's canonical JSON: the file without
    // the newline the generator prints after it.
    let canonical = route_bytes.strip_suffix(b"\n").unwrap_or(route_bytes);
    if faces.trace_sha256 != sha256_hex(canonical) {
        errors.push(format!(
            "trace {id}: the face track's trace_sha256 is not its route trace's"
        ));
    }
    let carrying = trace
        .events
        .iter()
        .enumerate()
        .filter(|(_, event)| event.state_node().is_some())
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    let indices = faces
        .entries
        .iter()
        .map(|entry| entry.event)
        .collect::<Vec<_>>();
    if indices != carrying {
        errors.push(format!(
            "trace {id}: the face track does not hold one entry per state-carrying event"
        ));
        return;
    }
    for entry in &faces.entries {
        let event = &trace.events[entry.event];
        if event.state_node() != Some(entry.node.as_str()) || event.t() != entry.t {
            errors.push(format!(
                "trace {id}: face entry for event {} names another node or time",
                entry.event
            ));
        }
        if entry.local["schema"] != LOCAL_DOCUMENT_SCHEMA
            || entry.host["schema"] != HOST_DOCUMENT_SCHEMA
        {
            errors.push(format!(
                "trace {id}: face entry for event {} is not in radio-mirror's document schemas",
                entry.event
            ));
        }
    }
}

/// The lab draws one topology, so both scenarios must run over the same one.
pub(super) fn check_shared_topology(scenarios: &[TraceScenario], errors: &mut Vec<String>) {
    let Some((first, rest)) = scenarios.split_first() else {
        return;
    };
    let names = |scenario: &TraceScenario| {
        scenario
            .trace
            .nodes
            .iter()
            .map(|node| (node.name.clone(), node.transit))
            .collect::<Vec<_>>()
    };
    for scenario in rest {
        if names(scenario) != names(first) || scenario.trace.edges != first.trace.edges {
            errors.push(format!(
                "trace {} runs over another topology than trace {}",
                scenario.id, first.id
            ));
        }
    }
}

pub fn same_edge(edge: &TraceEdge, a: &str, b: &str) -> bool {
    (edge.a == a && edge.b == b) || (edge.a == b && edge.b == a)
}
