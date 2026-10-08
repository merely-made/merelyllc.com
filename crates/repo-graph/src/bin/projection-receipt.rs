use std::env;
use std::fs;
use std::path::PathBuf;

use mer3ly_repo_graph::{
    CAPTURE_FILE, PortableProjection, SHELFMARK_FILE, TRACE_FILE, consume_portable_projection,
};

/// Consume the published projection artifacts natively and print a receipt.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .ok_or("usage: projection-receipt PATH_TO_ARTIFACT_DIRECTORY")?;
    let read = |name: &str| {
        fs::read(root.join(name)).map_err(|error| format!("{}: {error}", root.join(name).display()))
    };
    let artifacts = PortableProjection {
        capture: read(CAPTURE_FILE)?,
        trace: read(TRACE_FILE)?,
        shelfmark: read(SHELFMARK_FILE)?,
    };
    let receipt = consume_portable_projection(&artifacts)
        .map_err(|error| format!("{}: {error}", root.display()))?;
    println!("{}", serde_json::to_string_pretty(&receipt)?);
    Ok(())
}
