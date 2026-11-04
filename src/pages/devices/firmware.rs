use super::*;

pub(super) fn build_section(device: &DeviceRecord) -> SiteView {
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

pub(super) fn verify_section(device: &DeviceRecord) -> SiteView {
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

pub(super) fn network_section(device: &DeviceRecord) -> SiteView {
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

pub(super) fn flash_section(device: &DeviceRecord, firmware: &FirmwareCatalog) -> SiteView {
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

pub(super) fn firmware_recipe_card(name: &str, package: &FirmwarePackage) -> SiteView {
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
