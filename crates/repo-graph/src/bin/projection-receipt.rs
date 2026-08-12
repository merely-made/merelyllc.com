use std::env;
use std::fs;
use std::path::PathBuf;

use mer3ly_repo_graph::consume_portable_projection_json;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .ok_or("usage: projection-receipt PATH_TO_PROJECTION_JSON")?;
    let artifact = fs::read_to_string(&path)?;
    let receipt = consume_portable_projection_json(&artifact)
        .map_err(|error| format!("{}: {error}", path.display()))?;
    println!("{}", serde_json::to_string_pretty(&receipt)?);
    Ok(())
}
