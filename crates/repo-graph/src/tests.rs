use super::*;
use sceno::Hold;

const SAMPLE: &str = r#"{
      "schema": "mer3ly.repo-graph/v1",
      "nodes": [
        {"id":"mere","name":"Mere","class":"platform","status":"active","pushed_at":"2026-07-30T05:44:23Z"},
        {"id":"genet","name":"Genet","class":"platform","status":"active","pushed_at":"2026-07-30T05:44:24Z"},
        {"id":"turnstone","name":"Turnstone","class":"product","status":"prototype","pushed_at":"2026-07-31T05:07:42Z"}
      ],
      "edges": [
        {"id":"mere-depends-on-genet","source":"mere","target":"genet","kind":"depends_on","provenance":"derived"},
        {"id":"turnstone-hosts-mere","source":"turnstone","target":"mere","kind":"host_for","provenance":"curated"}
      ]
    }"#;

#[test]
fn arrangement_catalog_preserves_graph_identity() {
    let encoded = layout_graph_json(SAMPLE).expect("layout graph");
    let value: serde_json::Value = serde_json::from_str(&encoded).expect("parse layout");
    assert_eq!(value["schema"], "mer3ly.repo-graph-layout/v2");
    assert_eq!(value["engine"], "graph_layout:radial");
    assert_eq!(value["focus"], "mere");
    assert_eq!(value["default_arrangement"], "graph_layout:radial");
    assert_eq!(value["nodes"].as_array().expect("nodes").len(), 3);
    assert_eq!(value["edges"].as_array().expect("edges").len(), 2);
    assert_eq!(
        value["arrangements"]
            .as_array()
            .expect("arrangements")
            .len(),
        8
    );
    assert_eq!(
        value["unavailable_arrangements"]
            .as_array()
            .expect("unavailable arrangements")
            .len(),
        1
    );
    assert_eq!(value["nodes"][0]["id"], "mere");
    assert_eq!(value["nodes"][0]["pushed_at"], "2026-07-30T05:44:23Z");
    assert_eq!(value["edges"][1]["id"], "turnstone-hosts-mere");
    for arrangement in value["arrangements"].as_array().expect("arrangements") {
        assert_eq!(
            arrangement["nodes"].as_array().expect("scene nodes").len(),
            3
        );
        assert_eq!(arrangement["nodes"][0]["id"], "mere");
    }
}

#[test]
fn arrangement_catalog_is_deterministic() {
    let first = layout_graph_json(SAMPLE).expect("first layout");
    let second = layout_graph_json(SAMPLE).expect("second layout");
    assert_eq!(first, second);
}

#[test]
fn unknown_edge_endpoint_is_rejected() {
    let invalid = SAMPLE.replace("\"target\":\"genet\"", "\"target\":\"missing\"");
    let error = layout_graph_json(&invalid).expect_err("unknown endpoint should fail");
    assert!(error.contains("unknown endpoint"));
}

#[test]
fn archived_graph_without_mere_uses_its_first_public_node_as_focus() {
    let archived = r#"{
          "schema":"mer3ly.repo-graph/v1",
          "nodes":[{"id":"graphshell","name":"Graphshell","class":"product","status":"archived","pushed_at":"2026-02-21T13:32:51-05:00"}],
          "edges":[]
        }"#;

    let encoded = layout_graph_json(archived).expect("layout archived graph");
    let value: serde_json::Value = serde_json::from_str(&encoded).expect("parse layout");
    assert_eq!(value["focus"], "graphshell");
    for arrangement in value["arrangements"].as_array().expect("arrangements") {
        assert_eq!(arrangement["nodes"][0]["id"], "graphshell");
    }
}

#[test]
fn every_registered_arrangement_is_selectable_or_explained() {
    let encoded = layout_graph_json(SAMPLE).expect("layout graph catalog");
    let value: serde_json::Value = serde_json::from_str(&encoded).expect("parse layout");
    let mut arrangement_ids = value["arrangements"]
        .as_array()
        .expect("arrangements")
        .iter()
        .map(|arrangement| arrangement["id"].as_str().expect("arrangement id"))
        .collect::<Vec<_>>();
    arrangement_ids.extend(
        value["unavailable_arrangements"]
            .as_array()
            .expect("unavailable arrangements")
            .iter()
            .map(|arrangement| arrangement["id"].as_str().expect("arrangement id")),
    );
    arrangement_ids.sort_unstable();
    assert_eq!(
        arrangement_ids,
        vec![
            "graph_layout:grid",
            "graph_layout:kanban",
            "graph_layout:lsystem",
            "graph_layout:penrose",
            "graph_layout:phyllotaxis",
            "graph_layout:radial",
            "graph_layout:semantic_embedding",
            "graph_layout:stack",
            "graph_layout:timeline",
        ]
    );
}

#[test]
fn timeline_uses_one_proportional_axis_with_collision_free_strips() {
    let placed = place_timeline_strips(vec![
        ("oldest".to_owned(), Point2D::new(0.0, 0.0)),
        ("quarter".to_owned(), Point2D::new(25.0, 0.0)),
        ("newest".to_owned(), Point2D::new(100.0, 0.0)),
    ]);
    let points = placed
        .iter()
        .map(|(_, point)| (point.x.round() as i32, point.y.round() as i32))
        .collect::<HashSet<_>>();

    assert_eq!(points.len(), 3);
    assert_eq!(placed[0].0, "oldest");
    assert_eq!(placed[0].1.x, -310.0);
    assert_eq!(placed[1].1.x, -155.0);
    assert_eq!(placed[2].0, "newest");
    assert_eq!(placed[2].1.x, 310.0);
}

#[test]
fn timestamp_coordinate_preserves_time_of_day_and_midnight_distance() {
    let before = timestamp_coordinate("2026-07-30T23:59:59Z").expect("before midnight");
    let after = timestamp_coordinate("2026-07-31T00:00:01Z").expect("after midnight");
    assert_eq!(after - before, 2.0);
}

fn capture_of(json: &str) -> chirograph::ProjectionCaptureV2 {
    chirograph::ProjectionCaptureV2::decode(json.as_bytes()).expect("a V2 capture")
}

#[test]
fn the_portable_projection_is_a_capture_a_trace_and_a_shelfmark() {
    let artifacts = portable_projection(SAMPLE).expect("portable projection");
    let capture = chirograph::ProjectionCaptureV2::decode(&artifacts.capture)
        .expect("the capture decodes without the compiler");
    assert_eq!(capture.version, chirograph::PROJECTION_CAPTURE_V2);
    let score = capture.score.as_ref().expect("score");
    let authority = capture.authority.as_ref().expect("authority");
    assert_eq!(score.items.len(), 3);
    assert_eq!(capture.scene.active_item_count(), 3);
    assert_eq!(capture.scene.tables.relations.len(), 2);
    assert_eq!(authority.adapter, PROJECTION_ADAPTER);
    assert_eq!(authority.schema, "mer3ly.repo-graph/v1");
    assert!(authority.sha256.to_string().starts_with("ni:///sha-256;"));
    assert_eq!(authority.generation, score.generation);
    assert_eq!(capture.scene.epoch.0, authority.generation);

    let trace: scenotime::SceneTrace =
        serde_json::from_slice(&artifacts.trace).expect("the trace chains");
    assert_eq!(trace.base(), &capture.scene);
    assert_eq!(trace.len(), 7);
    assert_eq!(trace.steps()[1].label, "Move Turnstone");
    assert!(trace.steps()[1].diff.is_some());
    assert_eq!(
        trace.steps()[0].annotation,
        Some(serde_json::json!({"kind": "node", "id": "turnstone"}))
    );

    let shelfmark: ShelfmarkV1 = serde_json::from_slice(&artifacts.shelfmark).expect("shelfmark");
    assert_eq!(
        shelfmark.projection,
        chirograph::ContentHash::of(&artifacts.capture).to_string()
    );
    assert_eq!(
        shelfmark.inputs[SHELFMARK_AUTHORITY_ROLE].expects_generation,
        authority.generation.to_string()
    );

    let receipt = consume_portable_projection(&artifacts).expect("native receipt");
    assert_eq!(receipt.initial_revision, 1);
    assert_eq!(receipt.final_revision, 5);
    assert_eq!(receipt.active_items, 3);
    assert_eq!(receipt.active_relations, 1);
    assert_eq!(receipt.picked_source, "mere");
    assert_eq!(receipt.honored_holds, 0);
    assert_eq!(receipt.trace_steps, 7);
}

#[test]
fn the_shelfmark_is_checked_against_the_capture_bytes() {
    let artifacts = portable_projection(SAMPLE).expect("portable projection");
    let shelfmark: ShelfmarkV1 = serde_json::from_slice(&artifacts.shelfmark).unwrap();
    assert_eq!(check_shelfmark(&shelfmark, &artifacts.capture), Ok(()));

    // Another capture: the same scene one revision later.
    let mut moved = chirograph::ProjectionCaptureV2::decode(&artifacts.capture).unwrap();
    moved.scene.revision.0 += 1;
    let refused = check_shelfmark(&shelfmark, &moved.encode().unwrap()).unwrap_err();
    assert!(refused.contains("cites capture"), "{refused}");

    // The same capture, expecting another generation.
    let mut stale = shelfmark.clone();
    stale
        .inputs
        .get_mut(SHELFMARK_AUTHORITY_ROLE)
        .unwrap()
        .expects_generation = "1".to_owned();
    let refused = check_shelfmark(&stale, &artifacts.capture).unwrap_err();
    assert!(refused.contains("expects generation"), "{refused}");

    // The same capture, citing another authority record.
    let mut foreign = shelfmark.clone();
    foreign
        .inputs
        .get_mut(SHELFMARK_AUTHORITY_ROLE)
        .unwrap()
        .authority
        .record = chirograph::Sha256NamedInformation::of(b"other").to_string();
    let refused = check_shelfmark(&foreign, &artifacts.capture).unwrap_err();
    assert!(refused.contains("different authority"), "{refused}");

    // And consumption refuses a mismatched shelfmark too.
    let mut forged = artifacts.clone();
    forged.shelfmark = serde_json::to_vec(&stale).unwrap();
    assert!(consume_portable_projection(&forged).is_err());
}

#[test]
fn a_trace_off_the_captured_scene_or_past_the_bound_is_refused() {
    let artifacts = portable_projection(SAMPLE).expect("portable projection");
    let trace: scenotime::SceneTrace = serde_json::from_slice(&artifacts.trace).unwrap();

    // A repeated diff is a broken chain in a trace (Ruling 134).
    let mut wire: serde_json::Value = serde_json::from_slice(&artifacts.trace).unwrap();
    let repeated = wire["steps"][1].clone();
    wire["steps"].as_array_mut().unwrap().insert(2, repeated);
    let mut forged = artifacts.clone();
    forged.trace = serde_json::to_vec(&wire).unwrap();
    let refused = consume_portable_projection(&forged).unwrap_err();
    assert!(refused.contains("MissingBase"), "{refused}");

    // A trace whose base is not the captured scene.
    let mut base = trace.base().clone();
    base.revision.0 = 9;
    let elsewhere = scenotime::SceneTrace::new(base).unwrap();
    let mut forged = artifacts.clone();
    forged.trace = serde_json::to_vec(&elsewhere).unwrap();
    let refused = consume_portable_projection(&forged).unwrap_err();
    assert!(refused.contains("not the captured scene"), "{refused}");

    // The step bound is the host's (Ruling 15): a longer trace is a valid
    // trace that this proof refuses to carry.
    let mut long = trace.clone();
    while long.len() <= PROJECTION_STEP_BOUND {
        long = long
            .appended(
                scenotime::TraceStep::mark("Select Mere")
                    .with_annotation(serde_json::json!({"kind": "node", "id": "mere"})),
            )
            .unwrap();
    }
    let mut forged = artifacts.clone();
    forged.trace = serde_json::to_vec(&long).unwrap();
    let refused = consume_portable_projection(&forged).unwrap_err();
    assert!(refused.contains("bound"), "{refused}");
}

/// The sandbox's own shared-scene shape, extra fields and all, so the test
/// proves the seam accepts what the live path really emits.
const SHARED_SCENE: &str = r#"{
      "schema": "mer3ly.graphshell-scene-state/v1",
      "dataset": "public-repos",
      "source": {"source":"mer3ly/specimen","commit":"authored","committed_at":"static"},
      "reading": "neighbors",
      "arrangement": "graph_layout:radial",
      "motion": "free",
      "backdrop": {"kind":"ambient","collidable":false},
      "physics": "settled",
      "selection": "mere",
      "pins": [{"id":"genet","x":-120.5,"y":64.25}]
    }"#;

#[test]
fn the_expected_generation_is_the_score_generation() {
    // The checkability invariant: what a citation carries as
    // expects.generation must be what the solved score stamps, or the
    // check reports drift on every link ever written.
    let expected = authority_generation(SAMPLE).expect("generation");
    let capture = capture_of(&projection_capture_with_placement_json(SAMPLE, "{}").unwrap());
    assert_eq!(expected, capture.score.unwrap().generation.to_string());
    // And it moves when the authority moves, or it checks nothing.
    let altered = SAMPLE.replace("Turnstone", "Ternstone");
    assert_ne!(
        authority_generation(&altered).expect("generation"),
        expected
    );
}

#[test]
fn a_visitor_pin_reaches_the_score_and_the_solver_honors_it() {
    let capture = capture_of(
        &projection_capture_with_placement_json(SAMPLE, SHARED_SCENE)
            .expect("capture with placement"),
    );
    let score = capture.score.as_ref().unwrap();
    assert_eq!(score.holds.len(), 1, "the pin reached the score");
    let held = &score.holds[0];
    assert_eq!(held.source.id, "genet");
    assert_eq!(held.hold, Hold::Pinned);

    // Spiral is the arrangement, and Spiral is exactly the family that used
    // to discard an authored coordinate without saying so.
    let instance = instance_for_source(&capture.scene, "genet").expect("genet is placed");
    let at = capture
        .scene
        .active_item(instance)
        .unwrap()
        .transform
        .translate;
    assert_eq!((at.x, at.y), (-120.5, 64.25));
}

#[test]
fn a_pin_is_pinned_in_either_motion() {
    let anchored = SHARED_SCENE.replace(r#""motion": "free""#, r#""motion": "anchored""#);
    assert_ne!(anchored, SHARED_SCENE, "the fixture must change motion");
    let capture = capture_of(
        &projection_capture_with_placement_json(SAMPLE, &anchored).expect("anchored capture"),
    );
    assert_eq!(capture.score.unwrap().holds[0].hold, Hold::Pinned);
}

#[test]
fn a_legacy_anchored_link_reads_as_pinned() {
    let section = r#"[
          {"source":{"adapter":"mer3ly.repository-graph/v1","id":"genet"},"at":{"x":1.0,"y":2.0},"hold":"Anchored"},
          {"source":{"adapter":"mer3ly.repository-graph/v1","id":"mere"},"at":{"x":3.0,"y":4.0},"hold":"Pinned"}
        ]"#;
    let raw: Vec<HeldPlacement> = serde_json::from_str(section).unwrap();
    assert_eq!(
        raw[0].hold,
        Hold::Anchored,
        "the fixture carries the legacy class"
    );

    let placement = read_placement(section).expect("legacy placement");
    assert!(placement.iter().all(|held| held.hold == Hold::Pinned));
    assert_eq!((placement[0].at.x, placement[0].at.y), (1.0, 2.0));
}

#[test]
fn an_unpinned_share_captures_exactly_as_the_plain_path() {
    let no_pins = SHARED_SCENE.replace(
        r#""pins": [{"id":"genet","x":-120.5,"y":64.25}]"#,
        r#""pins": []"#,
    );
    let held = projection_capture_with_placement_json(SAMPLE, &no_pins).unwrap();
    let plain = portable_projection(SAMPLE).unwrap();
    assert_eq!(
        held.as_bytes(),
        plain.capture,
        "an empty delta must not perturb the capture"
    );
}

#[test]
fn a_pin_naming_an_unknown_node_is_refused() {
    let ghost = SHARED_SCENE.replace(r#""id":"genet""#, r#""id":"no-such-repo""#);
    let error = projection_capture_with_placement_json(SAMPLE, &ghost)
        .expect_err("a pin on a node the authority lacks is a broken citation");
    assert!(error.contains("no-such-repo"), "{error}");
}

#[test]
fn consuming_catches_a_hold_the_scene_did_not_honor() {
    // Forge the failure the seam exists to make impossible: a capture
    // claiming a pin the scene does not actually satisfy.
    let mut capture =
        capture_of(&projection_capture_with_placement_json(SAMPLE, SHARED_SCENE).unwrap());
    capture.score.as_mut().unwrap().holds[0].at = Vec2::new(999.0, 999.0);
    let error = portable::honored_holds_of(&capture)
        .expect_err("an unhonored hold must not pass consumption");
    assert!(error.contains("was not honored"), "{error}");
}

#[test]
fn sandbox_binds_arrangement_slots_to_real_seiche_motion_and_props() {
    let mut physics = graph_physics(SAMPLE).expect("sandbox physics");
    let positions = serde_json::to_string(&vec![
        GraphNodePosition {
            id: "mere".to_owned(),
            x: -120.0,
            y: 0.0,
        },
        GraphNodePosition {
            id: "genet".to_owned(),
            x: 0.0,
            y: 0.0,
        },
        GraphNodePosition {
            id: "turnstone".to_owned(),
            x: 120.0,
            y: 0.0,
        },
    ])
    .expect("positions");

    physics
        .apply_arrangement(&positions, "anchored")
        .expect("anchor slots");
    assert_eq!(physics.simulation.anchor_count(), 3);
    physics
        .apply_backdrop("props", true)
        .expect("collidable props");
    assert_eq!(physics.simulation.scene_body_count(), 6);

    let frame: serde_json::Value =
        serde_json::from_str(&physics.frame_json().expect("physics frame")).expect("JSON");
    assert_eq!(frame["schema"], "mer3ly.graph-physics-frame/v1");
    assert_eq!(frame["nodes"].as_array().expect("nodes").len(), 3);
    assert_eq!(frame["props"].as_array().expect("props").len(), 6);

    physics
        .apply_arrangement(&positions, "free")
        .expect("free motion");
    assert_eq!(physics.simulation.anchor_count(), 0);
    assert_eq!(
        physics
            .apply_arrangement(&positions, "frozen")
            .expect_err("interactive graphs reject frozen motion"),
        "unsupported mobility frozen"
    );
}

#[test]
fn radial_focus_is_host_configurable() {
    let mut value: serde_json::Value = serde_json::from_str(SAMPLE).expect("sample");
    value["focus"] = serde_json::Value::String("turnstone".to_owned());
    let encoded = layout_graph_json(&value.to_string()).expect("focused layout");
    let layout: serde_json::Value = serde_json::from_str(&encoded).expect("layout JSON");
    assert_eq!(layout["focus"], "turnstone");
}

#[test]
fn sandbox_exports_meres_primitive_and_behavior_registry() {
    let encoded = representation_registry().expect("representation registry");
    let registry: serde_json::Value =
        serde_json::from_str(&encoded).expect("representation registry JSON");
    assert_eq!(registry["schema"], "mere.graph-representation-registry/v2");
    assert_eq!(registry["profiles"][0]["primitive"]["body"], "hexagon");
    assert_eq!(
        registry["profiles"][0]["behaviors"][0]["behavior"],
        "inspect"
    );
}

#[test]
fn sandbox_exports_meres_reading_registry() {
    let encoded = reading_registry().expect("reading registry");
    let registry: serde_json::Value =
        serde_json::from_str(&encoded).expect("reading registry JSON");
    assert_eq!(registry["schema"], "mere.graph-reading-registry/v1");
    // Five readings, plus the two-reading matrix S2 added to Mere's
    // catalog (mere fc87c3e81), which the sandbox lists like any other.
    assert_eq!(registry["profiles"].as_array().expect("profiles").len(), 6);
    assert_eq!(registry["profiles"][5]["id"], "two-reading-matrix");
    assert_eq!(registry["profiles"][3]["id"], "neighbors");
    assert_eq!(
        registry["profiles"][3]["actor_scope"],
        "focus_and_neighbors"
    );
}

#[test]
fn neighbors_is_a_real_actor_projection_independent_of_arrangement() {
    let current: serde_json::Value = serde_json::from_str(SAMPLE).expect("sample graph");
    let request = serde_json::json!({
        "reading": "neighbors",
        "current": current,
        "focus": "genet"
    });
    let encoded = project_reading_json(&request.to_string()).expect("neighbors reading");
    let reading: serde_json::Value = serde_json::from_str(&encoded).expect("neighbors graph");
    let ids = reading["nodes"]
        .as_array()
        .expect("nodes")
        .iter()
        .map(|node| node["id"].as_str().expect("node id"))
        .collect::<HashSet<_>>();
    assert_eq!(ids, HashSet::from(["mere", "genet"]));
    assert_eq!(reading["edges"].as_array().expect("edges").len(), 1);
    assert_eq!(reading["focus"], "genet");
}

#[test]
fn changes_is_computed_by_the_native_reading_consumer() {
    let current: serde_json::Value = serde_json::from_str(SAMPLE).expect("sample graph");
    let mut previous = current.clone();
    previous["nodes"]
        .as_array_mut()
        .expect("previous nodes")
        .retain(|node| node["id"] != "turnstone");
    previous["edges"]
        .as_array_mut()
        .expect("previous edges")
        .retain(|edge| edge["source"] != "turnstone");
    previous["nodes"][0]["pushed_at"] = serde_json::json!("2026-07-29T05:44:23Z");
    let request = serde_json::json!({
        "reading": "changes",
        "current": current,
        "previous": previous
    });
    let encoded = project_reading_json(&request.to_string()).expect("changes reading");
    let reading: serde_json::Value = serde_json::from_str(&encoded).expect("changes graph");
    let change_by_id = reading["nodes"]
        .as_array()
        .expect("nodes")
        .iter()
        .map(|node| {
            (
                node["id"].as_str().expect("node id"),
                node["change"].as_str().expect("change"),
            )
        })
        .collect::<HashMap<_, _>>();
    assert_eq!(change_by_id["mere"], "updated");
    assert_eq!(change_by_id["genet"], "stable");
    assert_eq!(change_by_id["turnstone"], "added");
}

fn sample_matrix_request() -> MatrixProjectionRequest {
    let authority: GraphInput = serde_json::from_str(SAMPLE).expect("sample graph");
    MatrixProjectionRequest {
        rows: MatrixAxisRequest {
            dataset: "live".into(),
            record: "rev:1".into(),
            reading: "neighbors".into(),
            current: authority.clone(),
            previous: None,
            focus: Some("mere".into()),
        },
        columns: MatrixAxisRequest {
            dataset: "live".into(),
            record: "rev:1".into(),
            reading: "graph".into(),
            current: authority,
            previous: None,
            focus: None,
        },
    }
}

#[path = "tests/matrix.rs"]
mod matrix;

#[path = "tests/shelfmark.rs"]
mod shelfmark;
