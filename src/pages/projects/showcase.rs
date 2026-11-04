use super::*;

pub(super) fn showcase_section(showcase: Option<&ShowcaseRecord>) -> SiteView {
    let Some(showcase) = showcase else {
        return element(
            "section",
            &[("class", "content-section project-no-image")],
            vec![
                section_heading("01", "current public description"),
                element(
                    "p",
                    &[("class", "project-no-image-copy")],
                    vec![txt(
                        "This profile is intentionally text-first. Merely has not selected a current visual that would clarify the project without overstating its state.",
                    )],
                ),
            ],
        );
    };

    let visual = if showcase.images.is_empty() {
        showcase_figure(
            &showcase.image,
            &showcase.alt,
            &showcase.caption,
            &showcase.source_url,
            &showcase.source_license,
            "eager",
            None,
        )
    } else {
        showcase_carousel(showcase)
    };
    element(
        "section",
        &[("class", "content-section project-showcase-section")],
        vec![
            section_heading("01", "current view"),
            element(
                "div",
                &[("class", "project-showcase-layout")],
                vec![
                    element(
                        "div",
                        &[("class", "project-showcase-copy")],
                        vec![
                            element("h2", &[], vec![txt(&showcase.headline)]),
                            element("p", &[], vec![txt(&showcase.copy)]),
                        ],
                    ),
                    visual,
                ],
            ),
        ],
    )
}

/// One approved capture as a `<figure>`: image, caption, source attribution.
/// `slide_id` is present only inside a rotation, where the dot anchors need a
/// jump target.
#[allow(clippy::too_many_arguments)]
pub(super) fn showcase_figure(
    image: &str,
    alt: &str,
    caption: &str,
    source_url: &str,
    source_license: &str,
    loading: &str,
    slide_id: Option<&str>,
) -> SiteView {
    let image_src = format!("/{image}");
    let mut attrs = vec![("class", "project-showcase-figure")];
    if let Some(id) = slide_id {
        attrs.push(("id", id));
    }
    element(
        "figure",
        &attrs,
        vec![
            element(
                "img",
                &[
                    ("src", image_src.as_str()),
                    ("alt", alt),
                    ("loading", loading),
                    ("decoding", "async"),
                ],
                vec![],
            ),
            element(
                "figcaption",
                &[],
                vec![
                    txt(format!("{caption} Source image: ")),
                    external_link(source_url, "repository ↗", "text-link"),
                    txt(format!(". License: {source_license}.")),
                ],
            ),
        ],
    )
}

/// The multi-image rotation: every approved capture is in the initial HTML in
/// order, side by side in a scroll-snap strip, with anchor dots to jump
/// between them. No script is involved, so the no-script reading order is the
/// same figures top to bottom.
pub(super) fn showcase_carousel(showcase: &ShowcaseRecord) -> SiteView {
    let count = showcase.images.len() + 1;
    let slide_ids = (1..=count)
        .map(|position| format!("showcase-{}-{position}", showcase.repository))
        .collect::<Vec<_>>();
    let mut slides = vec![showcase_figure(
        &showcase.image,
        &showcase.alt,
        &showcase.caption,
        &showcase.source_url,
        &showcase.source_license,
        "eager",
        Some(slide_ids[0].as_str()),
    )];
    for (index, extra) in showcase.images.iter().enumerate() {
        slides.push(showcase_figure(
            &extra.image,
            &extra.alt,
            &extra.caption,
            &extra.source_url,
            &extra.source_license,
            "lazy",
            Some(slide_ids[index + 1].as_str()),
        ));
    }
    let dots = slide_ids
        .iter()
        .enumerate()
        .map(|(index, id)| {
            let href = format!("#{id}");
            let label = format!("Image {} of {count}", index + 1);
            element(
                "a",
                &[
                    ("class", "project-showcase-dot"),
                    ("href", href.as_str()),
                    ("aria-label", label.as_str()),
                ],
                vec![txt((index + 1).to_string())],
            )
        })
        .collect::<Vec<_>>();
    element(
        "div",
        &[("class", "project-showcase-rotation")],
        vec![
            element("div", &[("class", "project-showcase-strip")], slides),
            element(
                "nav",
                &[
                    ("class", "project-showcase-dots"),
                    ("aria-label", "Showcase images"),
                ],
                dots,
            ),
        ],
    )
}
