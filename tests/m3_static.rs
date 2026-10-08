use genet_scripted_dom::ScriptedDom;
use layout_dom_api::LayoutDom;
use mer3ly_site::message_path;
use mer3ly_site::pages::{devices, home, radio};
use mer3ly_site::repositories::PublicSiteData;
use mer3ly_site::retinue_traces::TraceSet;
use mer3ly_site::site::SITE_CSS;
use std::path::{Path, PathBuf};

const FORBIDDEN_PUBLIC_MARKERS: &[&str] = &[
    "tel:",
    "outlook.com",
    "C:\\Users\\",
    "support.js",
    "<script src",
    "x-dc",
    "__next",
    "webpack",
];

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

#[test]
fn pages_are_static_genet_documents() {
    let root = workspace_root();
    let data = PublicSiteData::load(&root).expect("load public site data");
    for (name, document) in [
        (
            "home",
            home::document(&root).expect("render authority-backed home page"),
        ),
        ("radio", radio::document()),
        ("devices", devices::index_document_for(&data.devices)),
    ] {
        assert!(
            document.starts_with("<!doctype html>"),
            "{name} has an HTML doctype"
        );
        assert_eq!(
            document.matches("<h1").count(),
            1,
            "{name} has one primary heading"
        );
        assert!(
            document.contains("<main id=\"main\""),
            "{name} exposes the skip-link target"
        );
        assert!(
            document.contains("href=\"mailto:markik@mer3ly.net\""),
            "{name} exposes the approved public contact"
        );
        assert!(
            document.contains("<link rel=\"canonical\""),
            "{name} has a canonical URL"
        );
        assert!(
            document.contains("application/ld+json"),
            "{name} has structured data"
        );
        assert!(
            document.contains("https://merelyllc.com/og.jpg"),
            "{name} names the generated social preview"
        );
        assert!(
            document.contains("href=\"/site.css?v="),
            "{name} cache-busts the shared stylesheet"
        );
        assert!(
            document.contains("<body class=\"site-body\">\n  <a href=\"#main\""),
            "{name} emits a readable, indented body"
        );
        assert!(
            document.lines().count() > 40,
            "{name} does not collapse its body into one source line"
        );

        for marker in FORBIDDEN_PUBLIC_MARKERS {
            assert!(
                !document.contains(marker),
                "{name} contains forbidden public marker {marker:?}"
            );
        }

        let dom = ScriptedDom::from_serialized_document(&document);
        let serialized = dom.inner_html(dom.document());
        assert!(
            serialized.contains("<html lang=\"en\">"),
            "Genet parses and serializes the {name} document"
        );
        assert!(
            serialized.contains("<footer class=\"site-footer\">"),
            "Genet preserves the {name} landmarks"
        );
    }
}

#[test]
fn static_baseline_stays_below_the_m3_budget() {
    let root = workspace_root();
    let data = PublicSiteData::load(&root).expect("load public site data");
    let bytes = home::document(&root)
        .expect("render authority-backed home page")
        .len()
        + radio::document().len()
        + devices::index_document_for(&data.devices).len()
        + SITE_CSS.len();
    assert!(
        bytes < 200 * 1024,
        "base HTML and CSS use {bytes} bytes, over the 200 KiB M3 budget"
    );
}

#[test]
fn stylesheet_has_responsive_and_accessibility_contracts() {
    for contract in [
        "@media (max-width: 760px)",
        "@media (max-width: 440px)",
        "@media (prefers-reduced-motion: reduce)",
        ".skip-link",
        ":focus-visible",
    ] {
        assert!(
            SITE_CSS.contains(contract),
            "site CSS is missing {contract}"
        );
    }
}

#[test]
fn radio_lab_preserves_the_mesh_and_pilot_topology() {
    let document = radio::document();
    let traces = TraceSet::embedded().expect("committed traces");
    let cold = traces.scenario("cold").expect("cold trace");

    assert_eq!(
        document.matches("data-lab-node=").count(),
        cold.trace.nodes.len()
    );
    assert_eq!(
        document.matches("data-lab-edge=").count(),
        cold.trace.edges.len()
    );
    assert_eq!(
        document.matches("data-lab-event=").count(),
        message_path::milestones(cold).len()
    );
    assert_eq!(document.matches("class=\"county-shape").count(), 5);
    assert_eq!(document.matches("class=\"county-site\"").count(), 10);
    assert!(document.contains("data-message-path-lab"));
    assert!(document.contains("message-path-edge is-cut"));
    assert!(document.contains("Positions are illustrative, not geography or radio range."));
    assert!(document.contains("not a live traffic or radio-range receipt"));
    assert!(document.contains(&format!(
        "<script type=\"module\" src=\"{}\"></script>",
        radio::message_path_lab_href()
    )));
    assert!(document.contains("ten proposed sites · stylized, not to scale"));
    assert!(document.contains("aria-labelledby=\"message-path-title message-path-description\""));
    assert!(document.contains("aria-labelledby=\"county-map-title county-map-description\""));
}

#[test]
fn radio_lab_static_reading_is_the_cold_trace() {
    let document = radio::document();
    let traces = TraceSet::embedded().expect("committed traces");
    let cold = traces.scenario("cold").expect("cold trace");
    let delivered = cold.trace.messages[0]
        .delivered
        .as_ref()
        .expect("the cold trace delivers its message");
    // The route sentence is the trace's delivered path, not authored text.
    let path = delivered
        .path
        .iter()
        .map(|name| {
            message_path::layout(name)
                .expect("layout")
                .label
                .to_lowercase()
        })
        .collect::<Vec<_>>()
        .join(" → ");
    assert_eq!(
        path,
        "fire station → church steeple → water tower → county garage"
    );
    assert!(document.contains(&format!(
        "<p data-path-route=\"\">Message 1 delivered at 10.9 s: {path}, 2 relays.</p>"
    )));
    for row in message_path::milestones(cold) {
        assert!(
            document.contains(&row.text),
            "the ledger lacks {}",
            row.text
        );
    }
    // The screen is radio-mirror's TRAFFIC page for the destination.
    let screen = message_path::static_screen(&traces).expect("lab screen");
    assert_eq!(screen.path, "radio-mirror/message-path-cold-garage.png");
    assert!(document.contains(&format!("src=\"/{}\"", screen.path)));
    for line in &screen.lines {
        assert!(document.contains(&format!("<li>{line}</li>")), "{line}");
    }
    assert!(document.contains("data-screen-name=\"traffic\""));
    // The retired authored story and its counterfeit firmware screens.
    for retired in [
        "three relays",
        "RET · ROUTE",
        "RET · DELIVERED",
        "TX QUEUED",
        "RX FRAME",
        "Static route:",
        "data-path-blocked",
    ] {
        assert!(
            !document.contains(retired),
            "radio page still says {retired}"
        );
    }
    assert!(document.contains(&format!(
        "retinue revision {}",
        traces.provenance.short_revision()
    )));
}

#[test]
fn radio_lab_manifest_points_at_the_published_traces() {
    let traces = TraceSet::embedded().expect("committed traces");
    let manifest: serde_json::Value =
        serde_json::from_str(&message_path::manifest_json(&traces).expect("manifest"))
            .expect("manifest JSON");
    assert_eq!(manifest["schema"], message_path::MANIFEST_SCHEMA);
    assert_eq!(manifest["page"], "traffic");
    assert_eq!(
        manifest["default"],
        serde_json::json!({ "scenario": "cold", "step": 78, "node": "garage" })
    );
    let scenarios = manifest["scenarios"].as_array().expect("scenarios");
    assert_eq!(scenarios.len(), 2);
    for (entry, scenario) in scenarios.iter().zip(&traces.scenarios) {
        assert_eq!(entry["trace_sha256"], scenario.faces.trace_sha256.as_str());
        assert_eq!(entry["events"], scenario.trace.events.len());
        for key in ["route", "face"] {
            assert!(
                entry[key].as_str().is_some_and(
                    |href| href.starts_with("/retinue-traces/") && href.contains("?v=")
                ),
                "{key}"
            );
        }
    }
    let warm = &scenarios[1]["milestones"];
    let kinds = warm
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["kind"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(kinds.iter().filter(|kind| **kind == "expired").count(), 3);
    assert_eq!(kinds.iter().filter(|kind| **kind == "delivered").count(), 2);
}

#[test]
fn message_path_lab_reads_traces_and_keeps_no_authored_script() {
    let script = std::fs::read_to_string(workspace_root().join("assets/message-path-lab.js"))
        .expect("lab source");
    for retired in [
        "scenarios = {",
        "defaultPositions",
        "three relays",
        "RET · ",
        "TX QUEUED",
        "RX FRAME",
        "pointerdown",
    ] {
        assert!(
            !script.contains(retired),
            "message-path-lab.js still carries {retired}"
        );
    }
    for read in [
        "retinue-sim.route-trace/v1",
        "retinue-sim.face-track/v1",
        "RadioMirror",
        "./radio_mirror.js",
        "set_local_json",
        "set_host_json",
        ".text()",
        "message-path",
        "\"v2\"",
    ] {
        assert!(script.contains(read), "message-path-lab.js lacks {read}");
    }
}
