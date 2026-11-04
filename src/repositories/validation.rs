use super::*;

pub(super) fn read_toml<T>(path: PathBuf) -> Result<T, AuthorityError>
where
    T: for<'de> Deserialize<'de>,
{
    let source = fs::read_to_string(&path)
        .map_err(|error| AuthorityError::new(format!("read {}: {error}", path.display())))?;
    toml::from_str(&source)
        .map_err(|error| AuthorityError::new(format!("parse {}: {error}", path.display())))
}

pub(super) fn read_json<T>(path: PathBuf) -> Result<T, AuthorityError>
where
    T: for<'de> Deserialize<'de>,
{
    let source = fs::read_to_string(&path)
        .map_err(|error| AuthorityError::new(format!("read {}: {error}", path.display())))?;
    serde_json::from_str(&source)
        .map_err(|error| AuthorityError::new(format!("parse {}: {error}", path.display())))
}

pub(super) fn validate_showcase_png(path: &Path, repository_id: &str, errors: &mut Vec<String>) {
    const SIGNATURE: &[u8] = b"\x89PNG\r\n\x1a\n";
    const RETAINED_CHUNKS: &[&str] = &[
        "IHDR", "PLTE", "IDAT", "IEND", "tRNS", "sRGB", "gAMA", "cHRM",
    ];

    let Ok(bytes) = fs::read(path) else {
        errors.push(format!(
            "showcase {repository_id} normalized image is missing"
        ));
        return;
    };
    if !bytes.starts_with(SIGNATURE) {
        errors.push(format!(
            "showcase {repository_id} image is not a normalized PNG"
        ));
        return;
    }

    let mut offset = SIGNATURE.len();
    let mut saw_header = false;
    let mut saw_image_data = false;
    let mut saw_end = false;
    while offset < bytes.len() {
        let Some(header_end) = offset.checked_add(8) else {
            break;
        };
        if header_end > bytes.len() {
            break;
        }
        let length = u32::from_be_bytes(
            bytes[offset..offset + 4]
                .try_into()
                .expect("four-byte PNG length"),
        ) as usize;
        let Some(chunk_end) = offset
            .checked_add(12)
            .and_then(|value| value.checked_add(length))
        else {
            break;
        };
        if chunk_end > bytes.len() {
            break;
        }
        let chunk_type = &bytes[offset + 4..offset + 8];
        let Ok(chunk_type) = std::str::from_utf8(chunk_type) else {
            break;
        };
        if !saw_header && chunk_type != "IHDR" {
            break;
        }
        if !RETAINED_CHUNKS.contains(&chunk_type) {
            errors.push(format!(
                "showcase {repository_id} image contains an unapproved PNG chunk"
            ));
            return;
        }
        match chunk_type {
            "IHDR" => {
                if saw_header || length != 13 {
                    break;
                }
                let width = u32::from_be_bytes(
                    bytes[offset + 8..offset + 12]
                        .try_into()
                        .expect("four-byte PNG width"),
                );
                let height = u32::from_be_bytes(
                    bytes[offset + 12..offset + 16]
                        .try_into()
                        .expect("four-byte PNG height"),
                );
                if width == 0 || height == 0 || width > 4096 || height > 4096 {
                    errors.push(format!(
                        "showcase {repository_id} image dimensions are outside the approved range"
                    ));
                    return;
                }
                saw_header = true;
            }
            "IDAT" => {
                if !saw_header {
                    break;
                }
                saw_image_data = true;
            }
            "IEND" => {
                if length != 0 || !saw_image_data {
                    break;
                }
                saw_end = true;
            }
            _ => {}
        }
        offset = chunk_end;
        if saw_end {
            break;
        }
    }

    if !saw_header || !saw_image_data || !saw_end || offset != bytes.len() {
        errors.push(format!(
            "showcase {repository_id} image is not a complete normalized PNG"
        ));
    }
}

pub(super) fn check_nonempty(label: &str, value: &str, errors: &mut Vec<String>) {
    if value.trim().is_empty() {
        errors.push(format!("{label} is empty"));
    }
}

pub(super) fn check_slug(label: &str, value: &str, errors: &mut Vec<String>) {
    let mut parts = value.split('/');
    let owner = parts.next().unwrap_or_default();
    let repository = parts.next().unwrap_or_default();
    if owner.is_empty()
        || repository.is_empty()
        || parts.next().is_some()
        || !value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | '/'))
    {
        errors.push(format!("{label} is not an owner/repository slug: {value}"));
    }
}

pub(super) fn check_https_or_github(label: &str, value: &str, errors: &mut Vec<String>) {
    if !value.starts_with("https://") {
        errors.push(format!("{label} must use https: {value}"));
    }
}

pub(super) fn check_date(label: &str, value: &str, errors: &mut Vec<String>) {
    let bytes = value.as_bytes();
    let valid = bytes.len() == 10
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes
            .iter()
            .enumerate()
            .all(|(index, byte)| matches!(index, 4 | 7) || byte.is_ascii_digit());
    if !valid {
        errors.push(format!("{label} is not YYYY-MM-DD: {value}"));
    }
}

pub(super) fn check_timestamp(label: &str, value: &str, errors: &mut Vec<String>) {
    let date = value.get(..10).unwrap_or_default();
    if value.len() < 20 || !value.ends_with('Z') {
        errors.push(format!("{label} is not a UTC timestamp: {value}"));
        return;
    }
    check_date(label, date, errors);
}

pub(super) fn check_branch(id: &str, branch: &str, errors: &mut Vec<String>) {
    if branch.is_empty()
        || branch.starts_with('/')
        || branch.ends_with('/')
        || branch.contains("..")
        || branch.contains(char::is_whitespace)
    {
        errors.push(format!(
            "migration {id} has invalid default branch {branch}"
        ));
    }
}

pub(super) fn check_head(id: &str, head: &str, errors: &mut Vec<String>) {
    if head.len() != 40 || !head.chars().all(|ch| ch.is_ascii_hexdigit()) {
        errors.push(format!("migration {id} head is not a full git object id"));
    }
}

pub(super) fn check_safe_locator(id: &str, locator: &str, errors: &mut Vec<String>) {
    let safe = !locator.is_empty()
        && !locator.starts_with('/')
        && !locator.starts_with('\\')
        && !locator.contains(':')
        && !locator.contains('\\')
        && !locator.split('/').any(|part| part == "..");
    if !safe {
        errors.push(format!(
            "migration {id} local_locator must be workspace-relative and slash-separated"
        ));
    }
}

pub(super) fn check_receipt_path(path: &str, errors: &mut Vec<String>) {
    if !path.starts_with("docs/receipts/")
        || path.contains('\\')
        || path.contains(':')
        || path.split('/').any(|part| part == "..")
    {
        errors.push(format!(
            "inventory_receipt must be a safe path below docs/receipts: {path}"
        ));
    }
}
