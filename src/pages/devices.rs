use std::path::Path;

use serde_json::json;
use sha2::{Digest, Sha256};

use crate::devices::{DeviceCatalog, DeviceRecord, DeviceStatus};
use crate::firmware_catalog::{FirmwareCatalog, FirmwarePackage, FirmwareRecipeState};
use crate::repositories::{AuthorityError, PublicSiteData};
use crate::site::{
    ActivePage, DEFAULT_SOCIAL_IMAGE_ALT, DEFAULT_SOCIAL_IMAGE_URL, DEVICE_CSS, DocumentMetadata,
    ORGANIZATION_ID, PageMetadata, SiteView, SocialImage, WEBSITE_ID, base_schema_graph, element,
    external_link, json_ld_for_script, link, render_with_dynamic_stylesheet_and_body_end,
    render_with_stylesheet, section_heading, shell, txt,
};

const RADIO_SIMULATOR: &[u8] = include_bytes!("../../assets/radio-simulator.js");

pub const INDEX_METADATA: PageMetadata = PageMetadata {
    title: "Radio hardware catalog | Merely",
    description: "Open radio hardware recipes ordered by product role, with demonstrated network support, installation paths, authorization records, missing work, and sale state kept separate.",
    canonical_url: "https://mer3ly.net/devices/",
};

pub fn documents(data: &PublicSiteData) -> Vec<(String, String)> {
    data.devices
        .ordered()
        .into_iter()
        .map(|device| (device.id.clone(), document_for(device, &data.firmware)))
        .collect()
}

pub fn index_document(root: &Path) -> Result<String, AuthorityError> {
    let data = PublicSiteData::load(root)?;
    Ok(index_document_for(&data.devices))
}

pub fn index_document_for(catalog: &DeviceCatalog) -> String {
    render_with_stylesheet(
        &INDEX_METADATA,
        || index_view(catalog),
        "/devices.css",
        DEVICE_CSS,
    )
}

pub fn document(root: &Path, device_id: &str) -> Result<String, AuthorityError> {
    let data = PublicSiteData::load(root)?;
    let device = data.devices.by_id(device_id).ok_or_else(|| {
        AuthorityError::from_message(format!("unknown public device {device_id}"))
    })?;
    Ok(document_for(device, &data.firmware))
}

pub fn document_for(device: &DeviceRecord, firmware: &FirmwareCatalog) -> String {
    let title = format!("{} | Merely", device.name);
    let canonical = format!("https://mer3ly.net/devices/{}/", device.id);
    let json_ld = device_json_ld(device, &canonical);
    let metadata = DocumentMetadata {
        title: &title,
        description: &device.summary,
        canonical_url: &canonical,
        social_image: SocialImage {
            url: DEFAULT_SOCIAL_IMAGE_URL,
            mime_type: "image/jpeg",
            alt: DEFAULT_SOCIAL_IMAGE_ALT,
        },
        json_ld: &json_ld,
    };
    let body_end = if device.id == "v4-desktop-radio" {
        radio_simulator_bootstrap()
    } else {
        String::new()
    };
    render_with_dynamic_stylesheet_and_body_end(
        &metadata,
        || device_view(device, firmware),
        "/devices.css",
        DEVICE_CSS,
        &body_end,
    )
}

fn radio_simulator_bootstrap() -> String {
    let digest = format!("{:x}", Sha256::digest(RADIO_SIMULATOR));
    format!(
        "<script type=\"module\" src=\"/radio-simulator.js?v={}\"></script>",
        &digest[..12]
    )
}

fn device_json_ld(device: &DeviceRecord, canonical: &str) -> String {
    let article_id = format!("{canonical}#recipe");
    let evidence_url = evidence_url(device);
    let mut graph = base_schema_graph();
    graph.push(json!({
        "@type": "WebPage",
        "@id": canonical,
        "url": canonical,
        "name": device.name,
        "description": device.summary,
        "isPartOf": { "@id": WEBSITE_ID },
        "about": { "@id": article_id }
    }));
    graph.push(json!({
        "@type": "TechArticle",
        "@id": article_id,
        "name": device.name,
        "headline": device.role,
        "description": device.summary,
        "url": canonical,
        "publisher": { "@id": ORGANIZATION_ID },
        "isBasedOn": evidence_url,
        "articleSection": ["Exact recipe state", "Build it", "Verify it", "Network support", "Install firmware", "Radio authorization", "Purchase"]
    }));
    json_ld_for_script(&json!({
        "@context": "https://schema.org",
        "@graph": graph
    }))
}

fn index_view(catalog: &DeviceCatalog) -> SiteView {
    shell(
        ActivePage::Devices,
        element(
            "main",
            &[("id", "main"), ("class", "device-catalog-main")],
            vec![
                element(
                    "section",
                    &[
                        ("class", "hero device-catalog-hero"),
                        ("aria-labelledby", "device-catalog-title"),
                    ],
                    vec![
                        element(
                            "p",
                            &[("class", "eyebrow")],
                            vec![txt("open hardware catalog")],
                        ),
                        element(
                            "h1",
                            &[("id", "device-catalog-title")],
                            vec![txt("Start with the job. Keep the recipe open.")],
                        ),
                        element(
                            "p",
                            &[("class", "hero-copy")],
                            vec![txt(concat!(
                                "Each device begins as a role and becomes an exact build recipe. ",
                                "Demonstrated networks, installable images, radio authorization, ",
                                "and sale readiness stay separate. Any purchase link comes after ",
                                "the complete DIY path.",
                            ))],
                        ),
                        catalog_flow(),
                    ],
                ),
                element(
                    "section",
                    &[("class", "content-section")],
                    vec![
                        section_heading("01", "development specimens"),
                        element(
                            "p",
                            &[("class", "section-intro")],
                            vec![txt(concat!(
                                "These are working board-level specimens. Their Reticulum, ",
                                "Meshtastic, and MeshCore receipts are stated as demonstrated ",
                                "support; their public flashing flows, authorization dossiers, ",
                                "enclosures, power assemblies, and sale readiness remain separate work.",
                            ))],
                        ),
                        element(
                            "div",
                            &[("class", "device-card-grid")],
                            catalog.ordered().into_iter().map(device_card).collect(),
                        ),
                    ],
                ),
                catalog_principle(),
            ],
        ),
    )
}

fn catalog_flow() -> SiteView {
    element(
        "ol",
        &[("class", "catalog-flow"), ("aria-label", "Catalog path")],
        [
            "choose a role",
            "build it",
            "verify it",
            "choose a network",
            "install firmware",
            "check authorization",
            "buy assembled",
        ]
        .into_iter()
        .enumerate()
        .map(|(index, label)| {
            element(
                "li",
                &[],
                vec![
                    element(
                        "span",
                        &[("aria-hidden", "true")],
                        vec![txt(format!("{:02}", index + 1))],
                    ),
                    txt(label),
                ],
            )
        })
        .collect(),
    )
}

fn device_card(device: &DeviceRecord) -> SiteView {
    let href = format!("/devices/{}/", device.id);
    let silhouette_class = if device.id.starts_with("v4-") {
        "device-silhouette device-silhouette-v4"
    } else {
        "device-silhouette device-silhouette-t114"
    };
    element(
        "article",
        &[
            ("class", "device-card"),
            ("data-device-id", device.id.as_str()),
        ],
        vec![
            element(
                "div",
                &[("class", "device-card-figure"), ("aria-hidden", "true")],
                vec![element(
                    "div",
                    &[("class", silhouette_class)],
                    vec![
                        element("span", &[("class", "device-screen")], vec![]),
                        element("span", &[("class", "device-control")], vec![]),
                        element("span", &[("class", "device-antenna")], vec![]),
                    ],
                )],
            ),
            element(
                "div",
                &[("class", "device-card-copy")],
                vec![
                    element(
                        "p",
                        &[("class", "card-kicker")],
                        vec![txt(device.status.label())],
                    ),
                    element("h2", &[], vec![txt(&device.name)]),
                    element("p", &[("class", "device-role")], vec![txt(&device.role)]),
                    element("p", &[], vec![txt(&device.summary)]),
                    element(
                        "ul",
                        &[("class", "device-card-facts")],
                        vec![
                            element("li", &[], vec![txt(&device.processor)]),
                            element("li", &[], vec![txt(&device.radio)]),
                            element("li", &[], vec![txt(&device.form)]),
                        ],
                    ),
                    link(&href, "Open the recipe and evidence", "text-link"),
                ],
            ),
        ],
    )
}

fn catalog_principle() -> SiteView {
    element(
        "section",
        &[
            ("class", "closing-note"),
            ("aria-label", "Catalog principle"),
        ],
        vec![
            element("p", &[("class", "eyebrow")], vec![txt("catalog principle")]),
            element(
                "p",
                &[("class", "closing-copy")],
                vec![txt(
                    "The assembled object is a convenience. The recipe, evidence, and right to replace its software are the product's foundation.",
                )],
            ),
        ],
    )
}

fn device_view(device: &DeviceRecord, firmware: &FirmwareCatalog) -> SiteView {
    shell(
        ActivePage::Devices,
        element(
            "main",
            &[
                ("id", "main"),
                ("class", "device-profile-main"),
                ("data-device-id", device.id.as_str()),
                ("data-device-status", device.status.label()),
            ],
            vec![
                device_hero(device),
                recipe_section(device),
                build_section(device),
                verify_section(device),
                network_section(device),
                flash_section(device, firmware),
                authorization_section(device),
                purchase_section(device),
            ],
        ),
    )
}

fn device_hero(device: &DeviceRecord) -> SiteView {
    element(
        "section",
        &[
            ("class", "hero device-profile-hero"),
            ("aria-labelledby", "device-title"),
        ],
        vec![
            link("/devices/", "All devices", "back-link"),
            element(
                "p",
                &[("class", "eyebrow")],
                vec![txt(device.status.label())],
            ),
            element("h1", &[("id", "device-title")], vec![txt(&device.name)]),
            element(
                "p",
                &[("class", "device-profile-role")],
                vec![txt(&device.role)],
            ),
            element("p", &[("class", "hero-copy")], vec![txt(&device.summary)]),
            element(
                "div",
                &[("class", "device-status-notice")],
                vec![
                    element("strong", &[], vec![txt("Current boundary")]),
                    txt(" This is a development specimen, not a finished kit or offered product."),
                ],
            ),
        ],
    )
}

fn recipe_section(device: &DeviceRecord) -> SiteView {
    let mut contents = vec![
        section_heading("01", "exact recipe state"),
        element(
            "dl",
            &[("class", "device-spec-grid")],
            vec![
                spec("Board", &device.board),
                spec("Processor", &device.processor),
                spec("Radio", &device.radio),
                spec("Controls", &device.interaction),
                spec("Power", &device.power),
                spec("Antenna", &device.antenna),
                spec("Enclosure", &device.enclosure),
                spec("Form", &device.form),
            ],
        ),
    ];
    if device.id == "v4-desktop-radio" {
        contents.push(radio_simulator());
    }
    element(
        "section",
        &[("class", "content-section device-profile-section")],
        contents,
    )
}

fn radio_simulator() -> SiteView {
    element(
        "div",
        &[
            ("class", "radio-bench"),
            ("data-radio-simulator", ""),
            ("data-ready", "false"),
            ("data-input-face", "one"),
            ("data-firmware-owner", "retinue"),
        ],
        vec![
            element(
                "div",
                &[("class", "radio-bench-heading")],
                vec![
                    element(
                        "p",
                        &[("class", "eyebrow")],
                        vec![txt("deterministic controller model")],
                    ),
                    element("h3", &[], vec![txt("Try the V4 radio face.")]),
                    element(
                        "p",
                        &[],
                        vec![txt(concat!(
                            "This is a simulation of Retinue's current 128 × 64 PANEL × LEDGER contract, ",
                            "using fixed example state. It is not connected to a radio."
                        ))],
                    ),
                ],
            ),
            element(
                "div",
                &[("class", "radio-bench-grid")],
                vec![radio_hardware(), radio_controls()],
            ),
            element(
                "p",
                &[
                    ("class", "radio-bench-fallback"),
                    ("data-radio-fallback", ""),
                ],
                vec![txt(
                    "Enable JavaScript to operate the controls. The displayed STATUS page remains an accurate static example.",
                )],
            ),
        ],
    )
}

fn radio_hardware() -> SiteView {
    element(
        "div",
        &[("class", "radio-hardware")],
        vec![
            element("span", &[("class", "radio-hardware-antenna")], vec![]),
            element(
                "div",
                &[("class", "radio-hardware-shell")],
                vec![
                    element(
                        "div",
                        &[
                            ("class", "radio-oled"),
                            ("data-radio-screen", ""),
                            ("data-screen-mode", "page"),
                            ("role", "status"),
                            ("aria-live", "polite"),
                            (
                                "aria-label",
                                "PHY OK. Board Heltec V4. Firmware Retinue. Host unavailable. Radio SX1262 ready. Local modem ready.",
                            ),
                        ],
                        vec![
                            element(
                                "div",
                                &[("class", "radio-oled-header")],
                                vec![
                                    element(
                                        "strong",
                                        &[("data-screen-header", "")],
                                        vec![txt("PHY · OK")],
                                    ),
                                    element(
                                        "span",
                                        &[("data-screen-counter", "")],
                                        vec![txt("1/4")],
                                    ),
                                ],
                            ),
                            element(
                                "div",
                                &[("class", "radio-oled-body")],
                                vec![
                                    screen_row("BOARD  HELTEC V4"),
                                    screen_row("FW     RETINUE"),
                                    screen_row("HOST   —"),
                                    screen_row("RADIO  SX1262 READY"),
                                ],
                            ),
                            element(
                                "div",
                                &[("class", "radio-oled-ticker"), ("data-screen-ticker", "")],
                                vec![txt("LOCAL · MODEM READY")],
                            ),
                        ],
                    ),
                    element(
                        "div",
                        &[("class", "radio-hardware-status")],
                        vec![
                            element(
                                "span",
                                &[
                                    ("class", "radio-led"),
                                    ("data-radio-led", ""),
                                    ("data-led-state", "idle"),
                                    ("aria-hidden", "true"),
                                ],
                                vec![],
                            ),
                            element("span", &[], vec![txt("128 × 64 OLED · status LED")]),
                        ],
                    ),
                ],
            ),
        ],
    )
}

fn screen_row(label: &str) -> SiteView {
    element(
        "div",
        &[("class", "radio-oled-row"), ("data-screen-row", "")],
        vec![txt(label)],
    )
}

fn radio_controls() -> SiteView {
    element(
        "div",
        &[("class", "radio-bench-controls")],
        vec![
            simulator_select(
                "Installed image",
                "radio-firmware",
                "data-radio-firmware",
                &[
                    ("retinue", "Retinue", true),
                    ("rnode", "RNode", false),
                    ("meshtastic", "Meshtastic", false),
                    ("meshcore", "MeshCore", false),
                ],
            ),
            simulator_select(
                "Scenario",
                "radio-scenario",
                "data-radio-scenario",
                &[
                    ("local", "Local radio", true),
                    ("host", "Attached host", false),
                    ("fault", "Radio fault", false),
                ],
            ),
            simulator_select(
                "Input face",
                "radio-input",
                "data-radio-input",
                &[
                    ("one", "V4 fitted button", true),
                    ("two", "Two-button enclosure", false),
                ],
            ),
            element(
                "p",
                &[("class", "radio-control-help"), ("data-radio-help", "")],
                vec![txt(
                    "Tap the fitted V4 button to step forward. Hold it for the menu; tap to move and hold to select.",
                )],
            ),
            element(
                "div",
                &[
                    ("class", "radio-control-pad"),
                    ("aria-label", "Simulated radio controls"),
                ],
                vec![
                    radio_button("a-short", "A tap", false),
                    radio_button("a-long", "A hold", false),
                    radio_button("b-short", "B tap", true),
                    radio_button("b-long", "B hold", true),
                    radio_button("chord", "A+B hold", true),
                ],
            ),
            element(
                "p",
                &[
                    ("class", "radio-truth-boundary"),
                    ("data-radio-boundary", ""),
                ],
                vec![txt(
                    "Local-radio mode exposes only board, power, radio, traffic, and fault facts the firmware owns.",
                )],
            ),
            element(
                "p",
                &[("class", "radio-input-note")],
                vec![txt(
                    "The catalog V4 has one fitted button. The two-button face is a simulated enclosure using the same controller grammar.",
                )],
            ),
        ],
    )
}

fn simulator_select(
    label: &str,
    id: &str,
    data_attribute: &str,
    options: &[(&str, &str, bool)],
) -> SiteView {
    element(
        "label",
        &[("class", "radio-control-field"), ("for", id)],
        vec![
            element("span", &[], vec![txt(label)]),
            element(
                "select",
                &[("id", id), (data_attribute, "")],
                options
                    .iter()
                    .map(|(value, label, selected)| {
                        if *selected {
                            element(
                                "option",
                                &[("value", *value), ("selected", "selected")],
                                vec![txt(*label)],
                            )
                        } else {
                            element("option", &[("value", *value)], vec![txt(*label)])
                        }
                    })
                    .collect(),
            ),
        ],
    )
}

fn radio_button(action: &str, label: &str, requires_two: bool) -> SiteView {
    let requires_two = if requires_two { "true" } else { "false" };
    element(
        "button",
        &[
            ("type", "button"),
            ("class", "radio-control-button"),
            ("data-radio-action", action),
            ("data-requires-two", requires_two),
        ],
        vec![txt(label)],
    )
}

fn spec(term: &str, detail: &str) -> SiteView {
    element(
        "div",
        &[("class", "device-spec")],
        vec![
            element("dt", &[], vec![txt(term)]),
            element("dd", &[], vec![txt(detail)]),
        ],
    )
}

fn build_section(device: &DeviceRecord) -> SiteView {
    let source_url = evidence_url(device);
    element(
        "section",
        &[("class", "content-section device-profile-section")],
        vec![
            section_heading("02", "build it"),
            element(
                "p",
                &[("class", "section-lead")],
                vec![txt(&device.recipe_state)],
            ),
            element("h3", &[], vec![txt("What the complete recipe still needs")]),
            element(
                "div",
                &[("class", "requirement-list")],
                device
                    .open_requirement
                    .iter()
                    .map(|requirement| {
                        element(
                            "article",
                            &[("class", "requirement-item")],
                            vec![
                                element("h4", &[], vec![txt(&requirement.label)]),
                                element("p", &[], vec![txt(&requirement.note)]),
                            ],
                        )
                    })
                    .collect(),
            ),
            external_link(
                &source_url,
                "Read the checked hardware receipt on GitHub ↗",
                "button button-quiet",
            ),
        ],
    )
}

fn verify_section(device: &DeviceRecord) -> SiteView {
    element(
        "section",
        &[("class", "content-section device-profile-section")],
        vec![
            section_heading("03", "verify it"),
            element(
                "p",
                &[("class", "section-intro")],
                vec![txt(
                    "A proof says exactly what happened. It does not silently stand in for range, routing, loss, battery runtime, or product qualification.",
                )],
            ),
            element(
                "div",
                &[("class", "evidence-ledger")],
                device
                    .evidence
                    .iter()
                    .map(|evidence| {
                        element(
                            "article",
                            &[("class", "evidence-item")],
                            vec![
                                element(
                                    "p",
                                    &[("class", "evidence-state")],
                                    vec![txt(evidence.state.label())],
                                ),
                                element("h3", &[], vec![txt(&evidence.label)]),
                                element("p", &[], vec![txt(&evidence.note)]),
                            ],
                        )
                    })
                    .collect(),
            ),
        ],
    )
}

fn network_section(device: &DeviceRecord) -> SiteView {
    element(
        "section",
        &[("class", "content-section device-profile-section")],
        vec![
            section_heading("04", "network support"),
            element(
                "p",
                &[("class", "section-intro")],
                vec![txt(concat!(
                    "This ledger records demonstrated network behavior. It is independent of ",
                    "whether a one-click installation recipe or assembled product is ready.",
                ))],
            ),
            element(
                "div",
                &[("class", "catalog-choice-grid")],
                device
                    .network_support
                    .iter()
                    .map(|network| {
                        element(
                            "article",
                            &[("class", "catalog-choice")],
                            vec![
                                element(
                                    "p",
                                    &[("class", "evidence-state")],
                                    vec![txt(network.state.label())],
                                ),
                                element("h3", &[], vec![txt(&network.name)]),
                                element("p", &[], vec![txt(&network.note)]),
                            ],
                        )
                    })
                    .collect(),
            ),
        ],
    )
}

fn flash_section(device: &DeviceRecord, firmware: &FirmwareCatalog) -> SiteView {
    element(
        "section",
        &[("class", "content-section device-profile-section")],
        vec![
            section_heading("05", "install firmware"),
            element(
                "p",
                &[("class", "section-intro")],
                vec![txt(concat!(
                    "Firmware belongs to the owner. Installation recipes are tracked separately ",
                    "from network support, and one SX1262 radio runs one selected personality at a time.",
                ))],
            ),
            external_link(
                firmware.source_url(),
                "Read the published Retinue package index ↗",
                "button button-quiet",
            ),
            element(
                "div",
                &[("class", "catalog-choice-grid")],
                device
                    .flash_recipe
                    .iter()
                    .map(|recipe| {
                        let package = firmware.package(&recipe.package_id).expect(
                            "PublicSiteData validates every device firmware package reference",
                        );
                        firmware_recipe_card(&recipe.name, package)
                    })
                    .collect(),
            ),
        ],
    )
}

fn firmware_recipe_card(name: &str, package: &FirmwarePackage) -> SiteView {
    let mut contents = vec![
        element(
            "p",
            &[("class", "evidence-state")],
            vec![txt(package.state.label())],
        ),
        element("h3", &[], vec![txt(name)]),
        element(
            "p",
            &[],
            vec![txt(format!(
                "Firmware publisher: {}.",
                package.firmware_publisher
            ))],
        ),
    ];

    match package.state {
        FirmwareRecipeState::ProvenRecipe | FirmwareRecipeState::Sellable => {
            let hosts = package
                .receipt_hosts
                .iter()
                .map(|host| receipt_host_label(host))
                .collect::<Vec<_>>()
                .join(", ");
            contents.push(element(
                "p",
                &[],
                vec![txt(format!(
                    "Installer and recovery receipts are retained for: {hosts}."
                ))],
            ));
            contents.push(external_link(
                &package.instructions_url,
                "Read installation instructions ↗",
                "button button-quiet",
            ));
            contents.push(external_link(
                &package.recovery_url,
                "Read recovery instructions ↗",
                "button button-quiet",
            ));
            contents.push(external_link(
                &package.installer_receipts[0],
                "Read installer receipt ↗",
                "button button-quiet",
            ));
            contents.push(external_link(
                &package.recovery_receipts[0],
                "Read recovery receipt ↗",
                "button button-quiet",
            ));
        }
        FirmwareRecipeState::Partial => contents.push(element(
            "p",
            &[],
            vec![txt(
                "This package is retained in the public index, but it is not yet a proven public recipe. Its required external interface check is still open.",
            )],
        )),
    }

    element("article", &[("class", "catalog-choice")], contents)
}

fn receipt_host_label(host: &str) -> &'static str {
    match host {
        "windows-x86_64" => "Windows x86-64",
        "macos-x86_64" => "Intel macOS",
        "macos-aarch64" => "Apple-silicon macOS",
        "linux-x86_64" => "Linux x86-64",
        "linux-aarch64" => "Linux Arm64",
        _ => "unrecognized host",
    }
}

fn authorization_section(device: &DeviceRecord) -> SiteView {
    element(
        "section",
        &[("class", "content-section device-profile-section")],
        vec![
            section_heading("06", "radio authorization"),
            element(
                "div",
                &[("class", "device-status-notice")],
                vec![
                    element("strong", &[], vec![txt(device.authorization.state.label())]),
                    txt(format!(" {}", device.authorization.note)),
                ],
            ),
            element(
                "dl",
                &[("class", "device-spec-grid authorization-grid")],
                vec![
                    spec("Exact device", &device.authorization.device),
                    spec("Antenna conditions", &device.authorization.antenna),
                    spec(
                        "Operating envelope",
                        &device.authorization.operating_envelope,
                    ),
                ],
            ),
        ],
    )
}

fn purchase_section(device: &DeviceRecord) -> SiteView {
    let mut contents = vec![
        section_heading("07", "buy assembled hardware"),
        element(
            "p",
            &[("class", "section-lead")],
            vec![txt(
                "The purchase control comes last, after the recipe, evidence, network support, installation paths, and radio authorization.",
            )],
        ),
    ];
    match (&device.status, &device.sale.purchase_url) {
        (DeviceStatus::Sellable, Some(url)) => contents.push(external_link(
            url,
            "Buy this assembled device ↗",
            "button button-primary purchase-link",
        )),
        _ => contents.push(element(
            "div",
            &[
                ("class", "purchase-unavailable"),
                ("data-purchase-status", "unavailable"),
            ],
            vec![
                element("strong", &[], vec![txt(device.sale.state.label())]),
                element("p", &[], vec![txt(&device.sale.note)]),
            ],
        )),
    }
    element(
        "section",
        &[(
            "class",
            "content-section device-profile-section device-purchase-section",
        )],
        contents,
    )
}

fn evidence_url(device: &DeviceRecord) -> String {
    format!(
        "{}/blob/main/{}",
        device.source_repository, device.source_document
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    #[test]
    fn tech_article_does_not_emit_an_offer() {
        let data = PublicSiteData::load(env!("CARGO_MANIFEST_DIR")).expect("public site data");
        let device = data.devices.ordered()[0];
        let canonical = format!("https://mer3ly.net/devices/{}/", device.id);
        let encoded = device_json_ld(device, &canonical);
        let value: Value = serde_json::from_str(&encoded).expect("valid JSON-LD");
        assert_eq!(value["@context"], "https://schema.org");
        assert!(!encoded.contains("\"offers\""));
    }
}
