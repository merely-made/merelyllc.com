use super::*;

#[test]
fn historical_trace_locator_is_scoped_to_the_exact_manifest() {
    let traces = crate::retinue_traces::TraceSet::embedded().unwrap();
    let manifest = message_path::manifest_embedded(&traces).unwrap();
    let document = format!(
        "<script id=\"{}\" type=\"application/json\">{manifest}</script>",
        message_path::MANIFEST_ID
    );
    let allowed = BTreeSet::new();
    let mut errors = Vec::new();
    scan_public_text("radio.html", &document, &allowed, &mut errors);
    assert!(errors.is_empty(), "{errors:?}");
    for (path, text) in [
        ("index.html", document.clone()),
        (
            "radio.html",
            format!("{document}<a href=\"https://github.com/merely-made/retinue\">old link</a>"),
        ),
        (
            "radio.html",
            document.replace(
                "fa0d98b1562357e617062283be1de483f3dcd4d4",
                "0000000000000000000000000000000000000000",
            ),
        ),
    ] {
        errors.clear();
        scan_public_text(path, &text, &allowed, &mut errors);
        assert!(
            errors
                .iter()
                .any(|error| error.contains("unapproved GitHub")),
            "{errors:?}"
        );
    }
}

#[test]
fn viewer_binary_scanning_retains_private_boundaries() {
    let path = crate::viewer::href("pkg/graphshell_web_bg.wasm");
    let path = path.trim_start_matches('/');
    let allowed = BTreeSet::new();
    let mut errors = Vec::new();
    scan_public_text(
        path,
        "file:// https://github.com/gfx-rs/naga/issues/1930",
        &allowed,
        &mut errors,
    );
    assert!(
        !errors.is_empty(),
        "a filename alone does not admit binary parser literals"
    );
    for forbidden in [
        "file:///tmp/private-file",
        "https://service.internal",
        "person@example.com",
        "/Users/person/source.rs",
        "C:\\Users\\person\\source.rs",
        "https://github.com/unapproved/repository",
        "github_pat_example",
    ] {
        errors.clear();
        scan_public_text(path, forbidden, &allowed, &mut errors);
        assert!(
            !errors.is_empty(),
            "viewer scanner must reject private or unapproved content"
        );
    }
    errors.clear();
    scan_public_text(
        "index.html",
        "file:// https://github.com/gfx-rs/naga",
        &allowed,
        &mut errors,
    );
    assert!(
        !errors.is_empty(),
        "binary allowances must not extend to published content"
    );
}

#[test]
fn public_text_scan_accepts_approved_github_links() {
    let allowed = BTreeSet::from(["merely-made/mere".to_owned()]);
    let mut errors = Vec::new();
    scan_public_text(
        "index.html",
        "mailto:markik@mer3ly.net https://github.com/merely-made https://github.com/merely-made/mere",
        &allowed,
        &mut errors,
    );
    assert!(errors.is_empty(), "{errors:?}");
}

#[test]
fn public_text_scan_rejects_private_boundaries_without_echoing_values() {
    let allowed = BTreeSet::from(["merely-made/mere".to_owned()]);
    let mut errors = Vec::new();
    scan_public_text(
        "index.html",
        "C:\\Users\\person\\secret 192.168.1.4 person@example.com https://github.com/private-owner/private-repo",
        &allowed,
        &mut errors,
    );
    assert!(errors.len() >= 4);
    let joined = errors.join("\n");
    assert!(!joined.contains("person"));
    assert!(!joined.contains("192.168.1.4"));
    assert!(!joined.contains("private-owner"));
}
