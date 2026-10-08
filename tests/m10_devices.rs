use std::path::{Path, PathBuf};

use mer3ly_site::devices::{
    AuthorizationState, DeviceCatalog, DeviceStatus, EvidenceState, NetworkSupportState, SaleState,
};
use mer3ly_site::firmware_catalog::{FirmwareCatalog, FirmwareRecipeState};
use mer3ly_site::pages::devices;
use mer3ly_site::repositories::PublicSiteData;
use mer3ly_site::site::DEVICE_CSS;

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

#[test]
fn catalog_exposes_two_honest_development_specimens() {
    let catalog = DeviceCatalog::load(workspace_root()).expect("validated device catalog");
    let devices = catalog.ordered();

    assert_eq!(devices.len(), 2);
    assert_eq!(devices[0].id, "v4-desktop-radio");
    assert_eq!(devices[1].id, "t114-field-radio");
    for device in devices {
        assert_eq!(device.status, DeviceStatus::DevelopmentSpecimen);
        assert_eq!(device.sale.state, SaleState::NotOffered);
        assert!(device.sale.purchase_url.is_none());
        assert!(device.open_requirement.len() >= 4);
        assert_eq!(device.authorization.state, AuthorizationState::Open);
        assert!(
            device
                .network_support
                .iter()
                .all(|network| network.state == NetworkSupportState::Demonstrated)
        );
        assert_eq!(device.flash_recipe[0].name, "Retinue radio image");
        assert!(
            device
                .evidence
                .iter()
                .any(|evidence| evidence.state == EvidenceState::Proven)
        );
    }
}

#[test]
fn firmware_recipe_state_is_derived_from_the_retained_retinue_index() {
    let catalog = DeviceCatalog::load(workspace_root()).expect("validated device catalog");
    let firmware = FirmwareCatalog::load(workspace_root()).expect("retained firmware index");
    firmware
        .validate_device_recipes(&catalog)
        .expect("catalog package references");

    assert_eq!(
        firmware.package("retinue.heltec-v4").unwrap().state,
        FirmwareRecipeState::ProvenRecipe
    );
    assert_eq!(
        firmware.package("retinue.t114").unwrap().state,
        FirmwareRecipeState::ProvenRecipe
    );
    assert_eq!(
        firmware
            .package("meshtastic.heltec-mesh-node-t114")
            .unwrap()
            .state,
        FirmwareRecipeState::Partial
    );
}

#[test]
fn index_starts_with_roles_and_distinguishes_the_forms() {
    let data = PublicSiteData::load(workspace_root()).expect("public site data");
    let document = devices::index_document_for(&data.devices);

    assert!(document.contains("Start with the job. Keep the recipe open."));
    assert!(document.contains("choose a role"));
    assert!(document.contains("device-silhouette-v4"));
    assert!(document.contains("device-silhouette-t114"));
    assert!(document.contains("href=\"/devices.css?v="));
    assert_eq!(document.matches("data-device-id=").count(), 2);
}

#[test]
fn profile_order_puts_purchase_after_the_open_recipe() {
    let data = PublicSiteData::load(workspace_root()).expect("public site data");
    for device in data.devices.ordered() {
        let document = devices::document_for(device, &data.firmware);
        let recipe = document.find("exact recipe state").expect("recipe section");
        let build = document.find("build it").expect("build section");
        let verify = document.find("verify it").expect("verify section");
        let networks = document.find("network support").expect("network section");
        let flash = document.find("install firmware").expect("flash section");
        let authorization = document
            .find("radio authorization")
            .expect("authorization section");
        let purchase = document
            .find("buy assembled hardware")
            .expect("purchase section");

        assert!(
            recipe < build
                && build < verify
                && verify < networks
                && networks < flash
                && flash < authorization
                && authorization < purchase
        );
        assert!(document.contains("data-purchase-status=\"unavailable\""));
        assert!(!document.contains("class=\"button button-primary purchase-link\""));
        assert!(
            document.contains("One selected personality at a time")
                || document.contains("one selected personality at a time")
        );
        assert!(document.contains("TechArticle"));
        assert!(document.contains(&format!("data-device-id=\"{}\"", device.id)));
    }
}

#[test]
fn catalog_layout_has_phone_specific_single_column_ledgers() {
    for contract in [
        ".device-card",
        ".device-silhouette-v4",
        ".device-silhouette-t114",
        ".device-spec-grid",
        ".catalog-choice-grid",
        ".authorization-grid",
        ".purchase-unavailable",
        ".radio-bench-grid",
        ".radio-oled",
        ".radio-screen-text",
        ".radio-screen-gallery",
        ".radio-control-button",
        "@media (max-width: 440px)",
        "@media (prefers-reduced-motion: reduce)",
    ] {
        assert!(
            DEVICE_CSS.contains(contract),
            "site CSS is missing {contract}"
        );
    }
}

#[test]
fn v4_profile_embeds_the_radio_mirror_bench() {
    let data = PublicSiteData::load(workspace_root()).expect("public site data");
    let v4 = data
        .devices
        .by_id("v4-desktop-radio")
        .expect("V4 catalog record");
    let document = devices::document_for(v4, &data.firmware);

    for contract in [
        "data-radio-simulator",
        "Try the V4 radio face.",
        "Retinue firmware UI · radio-mirror",
        "V4 fitted button",
        "Two-button enclosure",
        "A+B hold",
        "Local radio",
        "Attached host",
        "Radio fault",
        "Retinue",
        "RNode",
        "Meshtastic",
        "MeshCore",
        "data-radio-canvas",
        "aria-live=\"polite\"",
        "id=\"radio-mirror-scenarios\" type=\"application/json\"",
    ] {
        assert!(
            document.contains(contract),
            "V4 bench is missing {contract}"
        );
    }
    assert!(document.contains(&format!(
        "<script type=\"module\" src=\"{}\"></script>",
        devices::radio_simulator_href()
    )));

    let t114 = data
        .devices
        .by_id("t114-field-radio")
        .expect("T114 catalog record");
    let t114_document = devices::document_for(t114, &data.firmware);
    assert!(!t114_document.contains("data-radio-simulator"));
    assert!(!t114_document.contains("radio-simulator.js"));
    assert!(!t114_document.contains("radio-mirror/"));
}

#[test]
fn v4_bench_states_where_its_screens_come_from() {
    let data = PublicSiteData::load(workspace_root()).expect("public site data");
    let v4 = data.devices.by_id("v4-desktop-radio").expect("V4 record");
    let document = devices::document_for(v4, &data.firmware);

    assert!(!document.contains("accurate static example"));
    assert!(!document.contains("deterministic controller model"));
    assert!(document.contains(&devices::radio_mirror_source_statement()));
    assert!(document.contains("at retinue revision 6aa78fc"));
    assert_eq!(mer3ly_radio_mirror::short_revision(), "6aa78fc");
    assert!(document.contains(&format!(
        "https://github.com/merely-made/retinue/tree/{}/crates/radio-mirror",
        mer3ly_radio_mirror::RETINUE_REVISION
    )));
    for firmware in ["RNode", "Meshtastic", "MeshCore"] {
        assert!(document.contains(&format!(
            "{firmware} is the selected image. Its upstream firmware owns the screen and controls"
        )));
    }
    assert_eq!(
        document
            .matches("Site note · not a firmware screen")
            .count(),
        3
    );
}

#[test]
fn v4_bench_shows_every_controller_screen_with_its_text_projection() {
    let data = PublicSiteData::load(workspace_root()).expect("public site data");
    let v4 = data.devices.by_id("v4-desktop-radio").expect("V4 record");
    let document = devices::document_for(v4, &data.firmware);
    let screens = mer3ly_radio_mirror::static_screens().expect("radio-mirror screens");

    let names = screens
        .iter()
        .map(|screen| screen.screen.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        names,
        [
            "status",
            "power",
            "radio",
            "traffic",
            "identity",
            "links",
            "peers",
            "menu:brightness:0",
            "verify",
            "display-off",
            "fault",
        ]
    );
    for screen in &screens {
        assert!(
            document.contains(&format!("data-screen-name=\"{}\"", screen.screen)),
            "{} is not named in the page",
            screen.screen
        );
        let alt = devices::radio_screen_alt(&screen.lines);
        assert!(
            document.contains(&format!("src=\"/{}\" alt=\"{alt}\"", screen.path)),
            "{} lacks its text projection",
            screen.screen
        );
    }

    // radio-face's text projection, not a site-written mock.
    let status = &screens[0];
    assert_eq!(status.lines[0], "STATUS, RAD OK");
    assert!(status.lines.contains(&"BOARD: HELTEC V4".to_owned()));
    for line in &status.lines {
        assert!(document.contains(&format!("<li>{line}</li>")));
    }
    let fault = screens.last().expect("fault screen");
    assert_eq!(fault.lines[0], "FAULT, E01");
    assert!(fault.lines.contains(&"SX1262 INIT".to_owned()));
    let peers = screens
        .iter()
        .find(|screen| screen.slug == "peers")
        .unwrap();
    assert_eq!(peers.lines[0], "PEERS, HOST");
}

#[test]
fn v4_bench_scenarios_are_the_pinned_fixtures() {
    let scenarios: serde_json::Value =
        serde_json::from_str(&mer3ly_radio_mirror::scenarios_json()).expect("scenario JSON");
    assert_eq!(scenarios["schema"], "mer3ly.radio-bench-scenarios/v1");
    assert_eq!(scenarios["surface"], "oled-128x64");
    assert_eq!(
        scenarios["source"]["revision"],
        mer3ly_radio_mirror::RETINUE_REVISION
    );
    let local: serde_json::Value =
        serde_json::from_str(mer3ly_radio_mirror::LOCAL_FIXTURE).expect("local fixture");
    let host: serde_json::Value =
        serde_json::from_str(mer3ly_radio_mirror::HOST_FIXTURE).expect("host fixture");
    assert_eq!(scenarios["scenarios"]["host"]["local"], local);
    assert_eq!(scenarios["scenarios"]["host"]["host"], host);
    assert!(scenarios["scenarios"]["local"]["host"].is_null());
    assert_eq!(scenarios["scenarios"]["local"]["local"]["host"], "detached");
    assert_eq!(
        scenarios["scenarios"]["fault"]["local"]["fault"],
        serde_json::json!({ "code": 1, "message": "SX1262 INIT" })
    );
}

#[test]
fn radio_simulator_drives_radio_mirror_and_keeps_no_page_table() {
    let script = std::fs::read_to_string(workspace_root().join("assets/radio-simulator.js"))
        .expect("radio simulator source");
    for retired in [
        "PAGE_CONTENT",
        "LOCAL_PAGES",
        "HOST_PAGES",
        "SX1262 READY",
        "menuItems",
        "PHY · ",
        "RET · ",
    ] {
        assert!(
            !script.contains(retired),
            "radio-simulator.js still carries {retired}"
        );
    }
    for driven in [
        "RadioMirror",
        "./radio_mirror.js",
        "./radio_mirror_bg.wasm",
        ".press(",
        ".edge(",
        ".rgba()",
        ".text()",
        ".screen()",
        ".led(",
        "set_local_json",
        "set_host_json",
    ] {
        assert!(script.contains(driven), "radio-simulator.js lacks {driven}");
    }
}

#[test]
fn the_site_resolve_holds_no_reticulum_licensed_crate() {
    for lock in ["Cargo.lock", "crates/radio-mirror/Cargo.lock"] {
        let text = std::fs::read_to_string(workspace_root().join(lock)).expect("lockfile");
        for forbidden in ["retinue", "retinue-sim"] {
            assert!(
                !text.contains(&format!("name = \"{forbidden}\"\n")),
                "{lock} resolves {forbidden}"
            );
        }
        assert!(
            text.contains("name = \"radio-mirror\"\n"),
            "{lock} lacks radio-mirror"
        );
    }
}

#[test]
fn profiles_render_the_retained_installer_ledger_before_sale() {
    let data = PublicSiteData::load(workspace_root()).expect("public site data");
    let v4 = data
        .devices
        .by_id("v4-desktop-radio")
        .expect("V4 catalog record");
    let v4_document = devices::document_for(v4, &data.firmware);
    assert!(v4_document.contains("proven recipe"));
    assert!(v4_document.contains("Read installation instructions"));
    assert!(v4_document.contains("Read recovery instructions"));
    assert!(v4_document.contains("Windows x86-64, Intel macOS, Apple-silicon macOS, Linux x86-64"));

    let t114 = data
        .devices
        .by_id("t114-field-radio")
        .expect("T114 catalog record");
    let t114_document = devices::document_for(t114, &data.firmware);
    assert!(t114_document.contains("partial recipe"));
    assert!(t114_document.contains("required external interface check is still open"));
    let install = t114_document
        .find("install firmware")
        .expect("firmware section");
    let purchase = t114_document
        .find("buy assembled hardware")
        .expect("purchase section");
    assert!(install < purchase);
}
