use super::*;

/// The projection proof's artifacts, on the Mere profile only.
pub(super) fn proof_for(
    data: &PublicSiteData,
    repository: &RepositoryRecord,
) -> Option<ProjectionProof> {
    (repository.id == "mere").then(|| {
        ProjectionProof::build(data).expect("validated Mere authority projects through Scenograph")
    })
}

pub(super) fn projection_proof_section(proof: &ProjectionProof) -> SiteView {
    let [capture, trace, shelfmark] = proof
        .files()
        .map(|(name, bytes)| ProjectionProof::href(name, bytes));
    let (graph_runtime, graph_wasm) = super::super::repositories::graph_runtime_hrefs();
    let receipt = &proof.reading.receipt;
    element(
        "section",
        &[
            ("class", "content-section projection-proof-section"),
            ("aria-labelledby", "projection-proof-title"),
        ],
        vec![
            section_heading("03", "portable scene"),
            element(
                "div",
                &[("class", "projection-proof-heading")],
                vec![
                    element(
                        "div",
                        &[],
                        vec![
                            element(
                                "h2",
                                &[("id", "projection-proof-title")],
                                vec![txt("One portable scene. Two working projections.")],
                            ),
                            element(
                                "p",
                                &[],
                                vec![txt(
                                    "Move or select a project in either view. Remove a relationship from the scene, fold Mere's dependencies, scrub the revisioned trace, and both projections follow the same serialized state.",
                                )],
                            ),
                        ],
                    ),
                    element(
                        "p",
                        &[("class", "projection-proof-boundary")],
                        vec![txt(
                            "Scenograph supplies the score, solved scene, stable slots, and revisioned diffs · Mer3ly supplies the public-authority adapter",
                        )],
                    ),
                ],
            ),
            element(
                "figure",
                &[
                    ("class", "projection-proof"),
                    ("data-projection-proof", ""),
                    ("data-ready", "false"),
                    ("data-state", "pending"),
                    ("data-cursor", "0"),
                    ("data-capture-src", &capture),
                    ("data-trace-src", &trace),
                    ("data-shelfmark-src", &shelfmark),
                    ("data-dataset-src", &format!("/{HOST_DATASET_FILE}")),
                    // Ruling 140: replay loads on first interaction, from the
                    // graph runtime the repositories page already caches.
                    ("data-replay", "static"),
                    ("data-graph-runtime", &graph_runtime),
                    ("data-graph-wasm", &graph_wasm),
                    ("data-capture-address", &receipt.capture_address),
                    ("data-scene-epoch", &proof.reading.epoch),
                ],
                vec![
                    element(
                        "p",
                        &[
                            ("class", "projection-proof-notice"),
                            ("data-projection-notice", ""),
                            ("role", "status"),
                            ("hidden", ""),
                        ],
                        vec![],
                    ),
                    element(
                        "div",
                        &[
                            ("class", "projection-proof-interface"),
                            ("data-projection-interface", ""),
                            ("hidden", "hidden"),
                        ],
                        vec![
                            projection_proof_controls(),
                            element(
                                "div",
                                &[("class", "projection-proof-views")],
                                vec![
                                    projection_view(
                                        "canvas",
                                        "Canvas projection",
                                        "A full working view of the Mere repository neighborhood.",
                                    ),
                                    projection_view(
                                        "swatch",
                                        "Swatch projection",
                                        "The same scene in a compact, independently operable view.",
                                    ),
                                ],
                            ),
                            element("figcaption", &[], vec![txt(proof.summary())]),
                        ],
                    ),
                    // After the interface, so hiding it once replay is ready
                    // never moves a stage under the visitor's pointer.
                    proof.static_reading(),
                    element(
                        "p",
                        &[
                            ("class", "projection-proof-status sr-only"),
                            ("data-projection-status", ""),
                            ("role", "status"),
                            ("aria-live", "polite"),
                        ],
                        vec![txt("Portable scene not initialized.")],
                    ),
                ],
            ),
        ],
    )
}

pub(super) fn projection_proof_controls() -> SiteView {
    element(
        "div",
        &[
            ("class", "projection-proof-controls"),
            ("aria-label", "Portable scene controls"),
        ],
        vec![
            element(
                "button",
                &[
                    ("class", "button button-primary"),
                    ("type", "button"),
                    ("data-projection-action", "replay"),
                ],
                vec![txt("Replay changes")],
            ),
            element(
                "button",
                &[
                    ("class", "button button-quiet"),
                    ("type", "button"),
                    ("data-projection-action", "fold"),
                ],
                vec![txt("Fold dependencies")],
            ),
            element(
                "button",
                &[
                    ("class", "button button-quiet"),
                    ("type", "button"),
                    ("data-projection-action", "edge"),
                    ("disabled", "disabled"),
                ],
                vec![txt("Select an edge")],
            ),
            element(
                "button",
                &[
                    ("class", "button button-quiet"),
                    ("type", "button"),
                    ("data-projection-action", "reset"),
                ],
                vec![txt("Reset trace")],
            ),
            element(
                "label",
                &[("class", "projection-proof-scrubber")],
                vec![
                    element(
                        "span",
                        &[],
                        vec![
                            txt("Scene history "),
                            element(
                                "output",
                                &[("data-projection-cursor-output", "")],
                                vec![txt("0 of 0")],
                            ),
                        ],
                    ),
                    element(
                        "input",
                        &[
                            ("type", "range"),
                            ("min", "0"),
                            ("max", "0"),
                            ("step", "1"),
                            ("value", "0"),
                            ("data-projection-cursor", ""),
                        ],
                        vec![],
                    ),
                ],
            ),
            element(
                "button",
                &[
                    ("class", "button button-quiet"),
                    ("type", "button"),
                    ("data-projection-action", "share"),
                ],
                vec![txt("Share scene")],
            ),
            element(
                "p",
                &[("class", "projection-proof-readout")],
                vec![
                    element("span", &[], vec![txt("Selected")]),
                    element(
                        "strong",
                        &[("data-projection-readout", "")],
                        vec![txt("Mere")],
                    ),
                ],
            ),
        ],
    )
}

pub(super) fn projection_view(kind: &str, heading: &str, description: &str) -> SiteView {
    element(
        "section",
        &[
            ("class", "projection-proof-view"),
            ("data-projection-view", kind),
            ("aria-label", heading),
        ],
        vec![
            element(
                "header",
                &[("class", "projection-proof-view-heading")],
                vec![
                    element("h3", &[], vec![txt(heading)]),
                    element(
                        "p",
                        &[("data-projection-selection", "")],
                        vec![txt("Mere selected")],
                    ),
                ],
            ),
            element("p", &[("class", "sr-only")], vec![txt(description)]),
            element(
                "div",
                &[
                    ("class", "projection-proof-stage"),
                    ("data-projection-stage", kind),
                ],
                vec![
                    element(
                        "svg",
                        &[
                            ("class", "projection-proof-edges"),
                            ("data-projection-edges", ""),
                            ("aria-hidden", "true"),
                        ],
                        vec![],
                    ),
                    element(
                        "div",
                        &[
                            ("class", "projection-proof-edge-controls"),
                            ("data-projection-edge-controls", ""),
                        ],
                        vec![],
                    ),
                    element(
                        "div",
                        &[
                            ("class", "projection-proof-nodes"),
                            ("data-projection-nodes", ""),
                            ("role", "group"),
                            ("aria-label", "Project nodes"),
                        ],
                        vec![],
                    ),
                ],
            ),
        ],
    )
}

pub(super) fn projection_bootstrap(proof: &ProjectionProof) -> String {
    let mut digest = Sha256::new();
    digest.update(PROJECTION_PROOF);
    for (_, bytes) in proof.files() {
        digest.update(bytes);
    }
    let digest = format!("{:x}", digest.finalize());
    format!(
        "<script type=\"module\" src=\"/projection-proof.js?v={}\"></script>",
        &digest[..12]
    )
}
