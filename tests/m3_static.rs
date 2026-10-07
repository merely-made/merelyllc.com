use genet_scripted_dom::ScriptedDom;
use layout_dom_api::LayoutDom;
use mer3ly_site::pages::{devices, home, radio};
use mer3ly_site::repositories::PublicSiteData;
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

    assert_eq!(document.matches("data-lab-node=").count(), 5);
    assert_eq!(document.matches("data-lab-edge=").count(), 5);
    assert_eq!(document.matches("data-lab-event=").count(), 6);
    assert_eq!(document.matches("class=\"county-shape").count(), 5);
    assert_eq!(document.matches("class=\"county-site\"").count(), 10);
    assert!(document.contains("data-message-path-lab"));
    assert!(document.contains("message-path-edge is-blocked"));
    assert!(document.contains("Static route: fire station"));
    assert!(document.contains("not a live traffic or radio-range receipt"));
    assert!(document.contains("<script type=\"module\" src=\"/message-path-lab.js?v="));
    assert!(document.contains("ten proposed sites · stylized, not to scale"));
    assert!(document.contains("aria-labelledby=\"message-path-title message-path-description\""));
    assert!(document.contains("aria-labelledby=\"county-map-title county-map-description\""));
}
