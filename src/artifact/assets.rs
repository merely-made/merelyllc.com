use super::*;

pub(super) fn validate_cname(root: &Path, relative: &str, errors: &mut Vec<String>) {
    let contents = read_text(root, relative, errors);
    if contents.trim() != "merelyllc.com" {
        errors.push(format!(
            "{relative} does not name the approved public domain"
        ));
    }
}

pub(super) fn validate_copied_asset(
    artifact_root: &Path,
    source_root: &Path,
    artifact_relative: &str,
    source_relative: &str,
    label: &str,
    errors: &mut Vec<String>,
) {
    match (
        fs::read(artifact_root.join(artifact_relative)),
        fs::read(source_root.join(source_relative)),
    ) {
        (Ok(artifact), Ok(source)) if artifact == source => {}
        (Ok(_), Ok(_)) => errors.push(format!("{label} artifact differs from its approved source")),
        _ => errors.push(format!("{label} artifact or approved source is missing")),
    }
}

pub(super) fn validate_static_assets(
    artifact_root: &Path,
    source_root: &Path,
    errors: &mut Vec<String>,
) {
    validate_cname(artifact_root, "CNAME", errors);
    validate_copied_asset(
        artifact_root,
        source_root,
        "favicon.svg",
        "assets/favicon.svg",
        "favicon",
        errors,
    );
    validate_copied_asset(
        artifact_root,
        source_root,
        "devices.css",
        "assets/devices.css",
        "device stylesheet",
        errors,
    );
    validate_copied_asset(
        artifact_root,
        source_root,
        "message-path-lab.js",
        "assets/message-path-lab.js",
        "message path lab",
        errors,
    );
    for (name, _) in crate::viewer::FILES {
        validate_copied_asset(
            artifact_root,
            source_root,
            &format!("{}/{name}", crate::viewer::prefix()),
            &format!("assets/graphshell/{name}"),
            "qualified viewer",
            errors,
        );
    }
    validate_copied_asset(
        artifact_root,
        source_root,
        "graph-sandbox-mount.js",
        "assets/graph-sandbox-mount.js",
        "graph sandbox mount",
        errors,
    );
    validate_copied_asset(
        artifact_root,
        source_root,
        "projection-proof.js",
        "assets/projection-proof.js",
        "portable projection proof",
        errors,
    );
    validate_copied_asset(
        artifact_root,
        source_root,
        "radio-simulator.js",
        "assets/radio-simulator.js",
        "radio simulator",
        errors,
    );
    validate_copied_asset(
        artifact_root,
        source_root,
        "radio_mirror.js",
        "assets/radio_mirror.js",
        "radio-mirror runtime glue",
        errors,
    );
    validate_copied_asset(
        artifact_root,
        source_root,
        "radio_mirror_bg.wasm",
        "assets/radio_mirror_bg.wasm",
        "radio-mirror runtime",
        errors,
    );
}
