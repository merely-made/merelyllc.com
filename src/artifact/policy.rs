use super::*;

pub(super) fn is_scannable(path: &str) -> bool {
    path.ends_with(".html")
        || path.ends_with(".css")
        || path.ends_with(".js")
        || path.ends_with(".json")
        || path.ends_with(".svg")
        || path.ends_with(".txt")
        || path.ends_with(".xml")
        || path.ends_with(".wasm")
        || path.ends_with("CNAME")
}

pub(super) fn scan_public_bytes(
    relative_path: &str,
    bytes: &[u8],
    allowed_github_slugs: &BTreeSet<String>,
    errors: &mut Vec<String>,
) {
    // Only the exact pinned application binary admits parser constants and
    // upstream shader references. A changed binary or a lookalike filename
    // does not inherit this allowance. Machine paths remain forbidden even
    // in these verified bytes.
    let qualified_viewer = relative_path
        == crate::viewer::href("pkg/graphshell_web_bg.wasm").trim_start_matches('/')
        && base16ct::lower::encode_string(&Sha256::digest(bytes)) == crate::viewer::WASM_SHA256;
    scan_public_text_policy(
        relative_path,
        &String::from_utf8_lossy(bytes),
        allowed_github_slugs,
        qualified_viewer,
        errors,
    );
}

#[cfg(test)]
pub(super) fn scan_public_text(
    relative_path: &str,
    text: &str,
    allowed_github_slugs: &BTreeSet<String>,
    errors: &mut Vec<String>,
) {
    scan_public_bytes(relative_path, text.as_bytes(), allowed_github_slugs, errors);
}

pub(super) fn scan_public_text_policy(
    relative_path: &str,
    text: &str,
    allowed_github_slugs: &BTreeSet<String>,
    qualified_viewer: bool,
    errors: &mut Vec<String>,
) {
    let lower = text.to_ascii_lowercase();
    // The bare username marker must sit at a word start: product vocabulary
    // like SHELFMARK_SCHEMA lowercases to shelfmark_schema, which contains
    // "mark_" mid-identifier without leaking anything.
    let username_leak = lower
        .match_indices("mark_")
        .any(|(index, _)| index == 0 || !lower.as_bytes()[index - 1].is_ascii_alphanumeric());
    if username_leak {
        errors.push(format!(
            "{relative_path} contains a forbidden public-data marker"
        ));
    }
    for marker in [
        "c:\\users\\",
        "\\users\\",
        "/users/",
        "/home/",
        "file://",
        "viewerpermission",
        "\"viewer_permission\"",
        "\"ssh_url\"",
        "\"sshurl\"",
        "authorization: bearer",
        "github_pat_",
        "begin rsa private key",
        "begin ec private key",
        "begin openssh private key",
    ] {
        if lower.contains(marker) && !(qualified_viewer && marker == "file://") {
            errors.push(format!(
                "{relative_path} contains a forbidden public-data marker"
            ));
            break;
        }
    }

    let drive_path = Regex::new(
        r#"(?i)(?:^|[^a-z0-9])(?:[a-z]:[\\/](?:users|home|documents and settings|workspaces?|code)[\\/]|\\\\[a-z0-9._-]+[\\/])"#,
    )
    .expect("valid path regex");
    if drive_path.is_match(text) {
        errors.push(format!(
            "{relative_path} contains an absolute or network filesystem path"
        ));
    }

    let email =
        Regex::new(r"(?i)\b[a-z0-9._%+-]+@[a-z0-9.-]+\.[a-z]{2,}\b").expect("valid email regex");
    if email.find_iter(text).any(|candidate| {
        !candidate
            .as_str()
            .eq_ignore_ascii_case(APPROVED_CONTACT_EMAIL)
    }) {
        errors.push(format!(
            "{relative_path} contains an unapproved contact address"
        ));
    }

    let private_host = Regex::new(r"(?i)(?:https?|wss?)://[a-z0-9.-]+\.(?:internal|local)\b")
        .expect("valid host regex");
    if private_host.is_match(text) {
        errors.push(format!("{relative_path} contains a private hostname"));
    }

    let ipv4 = Regex::new(r"\b(?:[0-9]{1,3}\.){3}[0-9]{1,3}\b").expect("valid IPv4 regex");
    if ipv4.find_iter(text).any(|candidate| {
        candidate
            .as_str()
            .parse::<IpAddr>()
            .is_ok_and(|address| match address {
                IpAddr::V4(address) => {
                    address.is_private()
                        || address.is_loopback()
                        || address.is_link_local()
                        || address.is_unspecified()
                }
                IpAddr::V6(_) => false,
            })
    }) {
        errors.push(format!(
            "{relative_path} contains a private or local network address"
        ));
    }

    let github = Regex::new(r"(?i)https://github\.com/([a-z0-9_.-]+)(?:/([a-z0-9_.-]+))?")
        .expect("valid GitHub URL regex");
    // The radio trace manifest records its original repository locator. The
    // rename must not rewrite that historical evidence or admit old links in
    // ordinary page content. Only the exact, independently validated manifest
    // payload may retain its recorded URL.
    let historical_trace_manifest = (relative_path == "radio.html")
        .then(|| crate::retinue_traces::TraceSet::embedded().ok())
        .flatten()
        .and_then(|traces| message_path::manifest_embedded(&traces).ok())
        .filter(|manifest| inline_json(text, message_path::MANIFEST_ID) == Some(manifest.as_str()));
    let historical_span = historical_trace_manifest.as_ref().and_then(|manifest| {
        let script = format!(
            "<script id=\"{}\" type=\"application/json\">{}",
            message_path::MANIFEST_ID,
            manifest
        );
        text.find(&script).map(|start| start..start + script.len())
    });
    for captures in github.captures_iter(text) {
        let owner = captures
            .get(1)
            .map(|value| value.as_str().to_ascii_lowercase())
            .unwrap_or_default();
        let repository = captures
            .get(2)
            .map(|value| value.as_str().trim_end_matches(".git").to_ascii_lowercase());
        let upstream_shader_reference = qualified_viewer
            && matches!(
                (owner.as_str(), repository.as_deref()),
                ("gfx-rs", Some("naga")) | ("linebender", Some("vello")) | ("google", Some("skia"))
            );
        let recorded_trace_locator = captures.get(0).is_some_and(|capture| {
            capture.as_str() == "https://github.com/merely-made/retinue"
                && historical_span.as_ref().is_some_and(|span| {
                    span.contains(&capture.start()) && capture.end() <= span.end
                })
        });
        let approved = recorded_trace_locator
            || upstream_shader_reference
            || owner == "merely-made"
                && repository.as_ref().is_none_or(|repository| {
                    allowed_github_slugs.contains(&format!("{owner}/{repository}"))
                });
        if !approved {
            errors.push(format!(
                "{relative_path} contains an unapproved GitHub repository link"
            ));
            break;
        }
    }
}
