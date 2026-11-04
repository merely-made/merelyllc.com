use super::*;

pub(super) fn radio_simulator() -> SiteView {
    let screens = mer3ly_radio_mirror::static_screens()
        .expect("radio-mirror renders every V4 screen through the firmware controller");
    let initial = screens
        .iter()
        .find(|screen| screen.slug == INITIAL_SCREEN)
        .expect("radio-mirror renders the STATUS page");
    let source = radio_mirror_source_statement();
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
                        vec![txt("Retinue firmware UI · radio-mirror")],
                    ),
                    element("h3", &[], vec![txt("Try the V4 radio face.")]),
                    element(
                        "p",
                        &[],
                        vec![
                            txt(source.clone()),
                            txt(" "),
                            external_link(
                                &radio_mirror_source_url(),
                                "Read radio-mirror at that revision.",
                                "radio-source-link",
                            ),
                        ],
                    ),
                ],
            ),
            element(
                "div",
                &[("class", "radio-bench-grid")],
                vec![radio_hardware(initial), radio_controls()],
            ),
            element(
                "p",
                &[
                    ("class", "radio-bench-fallback"),
                    ("data-radio-fallback", ""),
                ],
                vec![txt(format!(
                    "Enable JavaScript to operate the controls. Each screen below was rendered by radio-mirror from Retinue's firmware UI at retinue revision {}.",
                    mer3ly_radio_mirror::short_revision()
                ))],
            ),
            radio_screen_gallery(&screens),
        ],
    )
}

/// Where the screens come from, as the page states it.
pub fn radio_mirror_source_statement() -> String {
    format!(
        "The screen is drawn by radio-mirror, which runs Retinue's own firmware UI (radio-face's renderer and Controller) at retinue revision {}, compiled to WebAssembly. Its status is retinue's fixed receipt fixtures; it is not connected to a radio.",
        mer3ly_radio_mirror::short_revision()
    )
}

pub(super) fn radio_mirror_source_url() -> String {
    format!(
        "{}/tree/{}/crates/radio-mirror",
        mer3ly_radio_mirror::RETINUE_REPOSITORY,
        mer3ly_radio_mirror::RETINUE_REVISION
    )
}

/// A screen's text projection as one `alt` string.
pub fn radio_screen_alt(lines: &[String]) -> String {
    lines.join("; ")
}

pub(super) fn radio_hardware(initial: &mer3ly_radio_mirror::StaticScreen) -> SiteView {
    let src = format!("/{}", initial.path);
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
                            ("data-screen-name", initial.screen.as_str()),
                        ],
                        vec![
                            // The live text below is the reading; the pixels repeat it.
                            element(
                                "img",
                                &[
                                    ("src", src.as_str()),
                                    ("alt", ""),
                                    ("width", "128"),
                                    ("height", "64"),
                                    ("data-radio-static", ""),
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
                                    ("data-radio-canvas", ""),
                                ],
                                vec![],
                            ),
                            radio_handoffs(),
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
                                    ("data-led-state", "off"),
                                    ("aria-hidden", "true"),
                                ],
                                vec![],
                            ),
                            element("span", &[], vec![txt("128 × 64 OLED · status LED")]),
                        ],
                    ),
                ],
            ),
            element(
                "div",
                &[("class", "radio-screen-reading")],
                vec![
                    element(
                        "p",
                        &[
                            ("class", "radio-screen-reading-label"),
                            ("id", "radio-text-label"),
                        ],
                        vec![txt("What the screen says")],
                    ),
                    element(
                        "ol",
                        &[
                            ("class", "radio-screen-text"),
                            ("data-radio-text", ""),
                            ("role", "status"),
                            ("aria-live", "polite"),
                            ("aria-labelledby", "radio-text-label"),
                        ],
                        initial
                            .lines
                            .iter()
                            .map(|line| element("li", &[], vec![txt(line.as_str())]))
                            .collect(),
                    ),
                ],
            ),
        ],
    )
}

pub(super) fn radio_handoffs() -> SiteView {
    element(
        "div",
        &[("class", "radio-handoffs")],
        HANDOFF_FIRMWARE
            .iter()
            .map(|(id, name)| {
                element(
                    "div",
                    &[
                        ("class", "radio-handoff"),
                        ("data-radio-handoff", id),
                        ("hidden", ""),
                    ],
                    vec![
                        element(
                            "p",
                            &[("class", "radio-handoff-kicker")],
                            vec![txt("Site note · not a firmware screen")],
                        ),
                        element(
                            "p",
                            &[],
                            vec![txt(format!(
                                "{name} is the selected image. Its upstream firmware owns the screen and controls, so this bench stops at that boundary and does not counterfeit its interface. radio-mirror draws only Retinue's firmware UI."
                            ))],
                        ),
                    ],
                )
            })
            .collect(),
    )
}

pub(super) fn radio_controls() -> SiteView {
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
                    ("local", "Local radio", false),
                    ("host", "Attached host", true),
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
                &[("class", "radio-control-help"), ("data-radio-help", "one")],
                vec![txt(
                    "Tap the fitted V4 button to step forward. Hold it for the menu; tap to move and hold to select.",
                )],
            ),
            element(
                "p",
                &[
                    ("class", "radio-control-help"),
                    ("data-radio-help", "two"),
                    ("hidden", ""),
                ],
                vec![txt(
                    "A steps forward. B steps back. Hold A+B for the menu; in the menu, A moves, B selects, and holding B leaves.",
                )],
            ),
            element(
                "div",
                &[
                    ("class", "radio-control-pad"),
                    ("role", "group"),
                    ("aria-label", "Radio buttons"),
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
                &[("class", "radio-input-note")],
                vec![txt(
                    "With the screen focused, the A and B keys are the radio's buttons: hold them as long as you would the real ones, and the firmware's press classifier decides tap, hold or chord.",
                )],
            ),
            element(
                "p",
                &[
                    ("class", "radio-truth-boundary"),
                    ("data-radio-boundary", ""),
                ],
                vec![txt(
                    "Pages, menu, buttons and LED follow Retinue's firmware Controller. Local radio detaches the host and leaves the four local pages; Attached host adds IDENTITY, LINKS, PEERS and VERIFY from retinue's host fixture; Radio fault is the V4 firmware's SX1262 init fault, which preempts every page.",
                )],
            ),
            element(
                "p",
                &[("class", "radio-input-note")],
                vec![txt(
                    "The catalog V4 has one fitted button. The two-button face is an enclosure the same firmware supports.",
                )],
            ),
        ],
    )
}

pub(super) fn radio_screen_gallery(screens: &[mer3ly_radio_mirror::StaticScreen]) -> SiteView {
    element(
        "div",
        &[
            ("class", "radio-screen-gallery"),
            ("data-radio-gallery", ""),
        ],
        vec![
            element("h4", &[], vec![txt("Every screen the V4's button reaches")]),
            element(
                "p",
                &[],
                vec![txt(format!(
                    "Rendered at site build time by radio-mirror from Retinue's firmware UI at retinue revision {}, with retinue's receipt fixtures and the host attached; the fault is the V4 firmware's own radio-init fault. Each image's text is radio-face's text projection of the same screen.",
                    mer3ly_radio_mirror::short_revision()
                ))],
            ),
            element(
                "ul",
                &[("class", "radio-screen-grid")],
                screens.iter().map(radio_screen_figure).collect(),
            ),
        ],
    )
}

pub(super) fn radio_screen_figure(screen: &mer3ly_radio_mirror::StaticScreen) -> SiteView {
    let src = format!("/{}", screen.path);
    let alt = radio_screen_alt(&screen.lines);
    element(
        "li",
        &[],
        vec![element(
            "figure",
            &[
                ("class", "radio-screen-figure"),
                ("data-screen-name", screen.screen.as_str()),
            ],
            vec![
                element(
                    "img",
                    &[
                        ("src", src.as_str()),
                        ("alt", alt.as_str()),
                        ("width", "128"),
                        ("height", "64"),
                        ("loading", "lazy"),
                        ("decoding", "async"),
                    ],
                    vec![],
                ),
                element(
                    "figcaption",
                    &[],
                    vec![element("code", &[], vec![txt(screen.screen.as_str())])],
                ),
            ],
        )],
    )
}

pub(super) fn simulator_select(
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

pub(super) fn radio_button(action: &str, label: &str, requires_two: bool) -> SiteView {
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
