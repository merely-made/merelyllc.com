use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

use mer3ly_site::artifact::validate_public_artifact;
use mer3ly_site::repositories::{Authority, PublicMetadataCache, PublicSiteData};
use mer3ly_site::retinue_traces::TraceSet;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let mut arguments = env::args().skip(1);
    let command = arguments.next().unwrap_or_else(|| "validate".to_owned());
    let root = arguments
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")));
    let path_argument = arguments.next().map(PathBuf::from);
    if arguments.next().is_some() {
        return Err(
            "usage: authority [validate|summary|inventory-targets|public-repositories|validate-metadata|validate-artifact] [root] [metadata-or-artifact-path]"
                .to_owned(),
        );
    }
    if command != "validate-metadata" && command != "validate-artifact" && path_argument.is_some() {
        return Err(format!("{command} does not accept a metadata path"));
    }

    let authority = Authority::load(&root).map_err(|error| error.to_string())?;
    authority
        .validate()
        .map_err(|errors| format!("authority validation failed:\n{}", errors.join("\n")))?;
    // The committed route traces are authority input too: their provenance
    // records the retinue revision, the commands and every file's hash.
    let traces = TraceSet::load(&root)
        .map_err(|errors| format!("retinue trace validation failed:\n{}", errors.join("\n")))?;

    match command.as_str() {
        "validate" => {
            println!(
                "authority valid: {} repositories, {} relations, {} migration records, {} unresolved products",
                authority.repositories.repository.len(),
                authority.relations.relation.len(),
                authority.migration.migration.len(),
                authority.migration.unresolved_product.len()
            );
            println!(
                "retinue traces valid: {} at retinue {}",
                traces
                    .scenarios
                    .iter()
                    .map(|scenario| format!(
                        "{} ({} events, {} face entries)",
                        scenario.trace.scenario,
                        scenario.trace.events.len(),
                        scenario.faces.entries.len()
                    ))
                    .collect::<Vec<_>>()
                    .join(", "),
                traces.provenance.short_revision()
            );
        }
        "summary" => {
            let candidates = authority
                .migration
                .migration
                .iter()
                .filter(|record| {
                    record.disposition == mer3ly_site::repositories::MigrationDisposition::Candidate
                })
                .count();
            let holds = authority
                .migration
                .migration
                .iter()
                .filter(|record| {
                    record.disposition == mer3ly_site::repositories::MigrationDisposition::Hold
                })
                .count();
            let personal_forks = authority
                .migration
                .migration
                .iter()
                .filter(|record| {
                    record.disposition
                        == mer3ly_site::repositories::MigrationDisposition::KeepPersonal
                })
                .count();
            println!(
                "{} public site repositories; {candidates} transfer candidates; {personal_forks} maintained forks kept personal; {holds} unresolved fork-review holds",
                authority.repositories.repository.len()
            );
        }
        "inventory-targets" => {
            println!(
                "{}",
                serde_json::to_string_pretty(&authority.inventory_basis())
                    .map_err(|error| format!("serialize inventory targets: {error}"))?
            );
        }
        "public-repositories" => {
            println!(
                "{}",
                serde_json::to_string_pretty(&authority.repositories)
                    .map_err(|error| format!("serialize public repositories: {error}"))?
            );
        }
        "validate-metadata" => {
            let path = path_argument.unwrap_or_else(|| root.join("content/github-metadata.json"));
            let metadata = PublicMetadataCache::load(&path).map_err(|error| error.to_string())?;
            metadata.validate(&authority).map_err(|errors| {
                format!("public metadata validation failed:\n{}", errors.join("\n"))
            })?;
            println!(
                "public metadata valid: {} repositories refreshed {}",
                metadata.repository.len(),
                metadata.generated_at_utc
            );
        }
        "validate-artifact" => {
            let artifact_path = path_argument.unwrap_or_else(|| root.join(".tmp/pages-artifact"));
            let metadata_path = root.join("content/github-metadata.json");
            let data = PublicSiteData::load(&root).map_err(|error| error.to_string())?;
            let receipt = validate_public_artifact(
                &artifact_path,
                &root,
                &data.authority,
                &data.metadata,
                &data.showcases,
                &data.devices,
                &metadata_path,
            )
            .map_err(|errors| {
                format!("public artifact validation failed:\n{}", errors.join("\n"))
            })?;
            println!(
                "{}",
                serde_json::to_string_pretty(&receipt)
                    .map_err(|error| format!("serialize public artifact receipt: {error}"))?
            );
        }
        _ => {
            return Err(format!(
                "unknown command {command}; expected validate, summary, inventory-targets, public-repositories, validate-metadata, or validate-artifact"
            ));
        }
    }

    Ok(())
}
