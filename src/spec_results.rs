//! Counts that arrive by themselves: a claim's `measured_from` names a results
//! file in the repository that runs the suite. `scripts/refresh-spec-results.py`
//! fetches each file at the latest commit that touched it, into
//! `content/spec-results/`, and records that commit in `sources.json`. Every
//! build then fills the claim's `measured` block from the cached file, stamped
//! with the source commit and its date.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::repositories::Authority;
use crate::specs::{ClaimRecord, Measured, ResultFormat, SpecRegistry, check_measured};

pub const RESULTS_DIR: &str = "content/spec-results";
pub const SOURCES_FILE: &str = "content/spec-results/sources.json";
pub const RESULTS_SCHEMA: &str = "merely.spec-results/v1";

/// One file to fetch: `authority spec-sources` prints these as JSON lines.
#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct Source {
    pub repo: String,
    pub slug: String,
    #[serde(rename = "ref")]
    pub git_ref: String,
    pub path: String,
}

/// What the fetch recorded for one source.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Fetched {
    #[serde(flatten)]
    pub source: Source,
    pub commit: String,
    pub committed_on: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SourcesManifest {
    pub fetched_on: String,
    pub source: Vec<Fetched>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ResultsFile {
    schema: String,
    #[serde(default)]
    result: Vec<ResultRecord>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ResultRecord {
    spec: String,
    measured: Measured,
}

#[derive(Deserialize)]
struct Test262Metrics {
    results: Test262Results,
    total: u64,
}

#[derive(Deserialize)]
struct Test262Results {
    pass: u64,
}

fn repo_of(claim: &ClaimRecord) -> &str {
    claim
        .measured_from
        .as_ref()
        .and_then(|from| from.repo.as_deref())
        .unwrap_or(&claim.repo)
}

/// Every distinct file the registry's claims read, resolved through the
/// migration ledger's slug and default branch.
pub fn sources(registry: &SpecRegistry, authority: &Authority) -> Result<Vec<Source>, Vec<String>> {
    let ledger: BTreeMap<&str, _> = authority
        .migration
        .migration
        .iter()
        .map(|record| (record.id.as_str(), record))
        .collect();
    let mut errors = Vec::new();
    let mut found = Vec::new();
    for claim in &registry.claim {
        let Some(from) = &claim.measured_from else {
            continue;
        };
        let repo = repo_of(claim);
        let Some(record) = ledger.get(repo) else {
            errors.push(format!(
                "claim {} / {} reads from unknown repo {repo}",
                claim.spec, claim.repo
            ));
            continue;
        };
        found.push(Source {
            repo: repo.to_owned(),
            slug: record.current_slug.clone(),
            git_ref: from
                .git_ref
                .clone()
                .unwrap_or_else(|| record.default_branch.clone()),
            path: from.path.clone(),
        });
    }
    found.sort();
    found.dedup();
    if errors.is_empty() {
        Ok(found)
    } else {
        Err(errors)
    }
}

/// Fill each `measured_from` claim's `measured` block from the cached files,
/// returning what was fetched for each such claim, by claim index.
pub fn apply(
    registry: &mut SpecRegistry,
    authority: &Authority,
    root: impl AsRef<Path>,
) -> Result<BTreeMap<usize, Fetched>, Vec<String>> {
    let root = root.as_ref();
    let wanted = sources(registry, authority)?;
    let manifest: SourcesManifest = match fs::read_to_string(root.join(SOURCES_FILE)) {
        Ok(text) => serde_json::from_str(&text)
            .map_err(|error| vec![format!("parse {SOURCES_FILE}: {error}")])?,
        Err(_) if wanted.is_empty() => return Ok(BTreeMap::new()),
        Err(error) => {
            return Err(vec![format!(
                "read {SOURCES_FILE}: {error}; run scripts/refresh-spec-results.py"
            )]);
        }
    };
    let fetched: BTreeMap<&Source, &Fetched> = manifest
        .source
        .iter()
        .map(|entry| (&entry.source, entry))
        .collect();
    let mut errors = Vec::new();
    for source in &wanted {
        if !fetched.contains_key(source) {
            errors.push(format!(
                "{} {}@{} is not in {SOURCES_FILE}; run scripts/refresh-spec-results.py",
                source.slug, source.path, source.git_ref
            ));
        }
    }
    if !errors.is_empty() {
        return Err(errors);
    }

    let ledger: BTreeMap<&str, _> = authority
        .migration
        .migration
        .iter()
        .map(|record| (record.id.as_str(), record))
        .collect();
    let mut provenance = BTreeMap::new();
    for (index, claim) in registry.claim.iter_mut().enumerate() {
        let Some(from) = claim.measured_from.clone() else {
            continue;
        };
        let repo = repo_of(claim).to_owned();
        let record = ledger[repo.as_str()];
        let source = Source {
            repo: repo.clone(),
            slug: record.current_slug.clone(),
            git_ref: from
                .git_ref
                .clone()
                .unwrap_or_else(|| record.default_branch.clone()),
            path: from.path.clone(),
        };
        let entry = fetched[&source];
        provenance.insert(index, entry.clone());
        let label = format!("claim {} / {}", claim.spec, claim.repo);
        let file = root.join(RESULTS_DIR).join(&repo).join(&from.path);
        let text = match fs::read_to_string(&file) {
            Ok(text) => text,
            Err(error) => {
                errors.push(format!("{label}: read {}: {error}", file.display()));
                continue;
            }
        };
        let stamp = format!(
            "{} {} at {}",
            entry.source.slug,
            from.path,
            &entry.commit[..entry.commit.len().min(10)]
        );
        let measured = match from.format {
            ResultFormat::SpecResults => read_spec_results(&text, &claim.spec, &stamp),
            ResultFormat::Test262Metrics => read_test262(
                &text,
                from.key.as_deref(),
                &repo,
                &entry.committed_on,
                &stamp,
            ),
        };
        match measured {
            Ok(measured) => {
                check_measured(&label, &measured, &mut errors);
                claim.measured = Some(measured);
            }
            Err(error) => errors.push(format!("{label}: {error}")),
        }
    }
    if errors.is_empty() {
        Ok(provenance)
    } else {
        Err(errors)
    }
}

fn read_spec_results(text: &str, spec: &str, stamp: &str) -> Result<Measured, String> {
    let file: ResultsFile =
        toml::from_str(text).map_err(|error| format!("parse results file: {error}"))?;
    if file.schema != RESULTS_SCHEMA {
        return Err(format!(
            "results file schema must be {RESULTS_SCHEMA}, got {}",
            file.schema
        ));
    }
    let mut record = file
        .result
        .into_iter()
        .find(|record| record.spec == spec)
        .ok_or_else(|| format!("results file has no entry for {spec}"))?;
    record.measured.scope = format!("{} ({stamp})", record.measured.scope);
    Ok(record.measured)
}

fn read_test262(
    text: &str,
    key: Option<&str>,
    repo: &str,
    committed_on: &str,
    stamp: &str,
) -> Result<Measured, String> {
    let value: serde_json::Value =
        serde_json::from_str(text).map_err(|error| format!("parse metrics: {error}"))?;
    let section = match key {
        Some(key) => value
            .get(key)
            .ok_or_else(|| format!("metrics have no {key} section"))?
            .clone(),
        None => value,
    };
    let metrics: Test262Metrics =
        serde_json::from_value(section).map_err(|error| format!("read metrics: {error}"))?;
    Ok(Measured {
        on: committed_on.to_owned(),
        scope: format!(
            "test262 metrics{} ({stamp})",
            key.map(|k| format!(", {k}")).unwrap_or_default()
        ),
        engine: Some(repo.to_owned()),
        wpt_pass: Some(metrics.results.pass),
        wpt_total: Some(metrics.total),
        files_all_pass: None,
        files: None,
        idl_named: None,
        idl_total: None,
        css_props_named: None,
        css_props_total: None,
    })
}
