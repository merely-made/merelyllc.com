//! Coverage report over the spec registry (plan phase R3), computed from the
//! registry and the committed catalog snapshots alone, never the network.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::repositories::AuthorityError;
use crate::specs::{Catalog, Depth, SpecRegistry, Unlock};

pub const BROWSER_SPECS_PATH: &str = "content/spec-catalogs/browser-specs.json";
/// Ruling D4: a claim not verified for this long is reported stale.
pub const STALE_AFTER_DAYS: i64 = 90;
/// A spec with more normative references than this (the catalog's 90th
/// percentile; HTML has 139) is an umbrella, so its references are not read
/// as prerequisites of its features. Hand-written `requires` always count.
pub const BROAD_REFERENCE_LIMIT: usize = 25;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CatalogSnapshot {
    pub source: String,
    pub version: String,
    pub fetched_on: String,
    pub license: String,
    #[serde(default)]
    pub references_source: Option<String>,
    pub spec: Vec<CatalogSpec>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CatalogSpec {
    pub shortname: String,
    pub title: String,
    pub organization: String,
    pub url: String,
    #[serde(default)]
    pub categories: Vec<String>,
    #[serde(default)]
    pub standing: String,
    #[serde(default)]
    pub test_paths: Vec<String>,
    /// Normative references, as registry or catalog ids.
    #[serde(default)]
    pub references: Vec<String>,
}

impl CatalogSnapshot {
    pub fn load(root: impl AsRef<Path>) -> Result<Self, AuthorityError> {
        let path = root.as_ref().join(BROWSER_SPECS_PATH);
        let text = fs::read_to_string(&path).map_err(|error| {
            AuthorityError::from_message(format!("read {}: {error}", path.display()))
        })?;
        serde_json::from_str(&text).map_err(|error| {
            AuthorityError::from_message(format!("parse {BROWSER_SPECS_PATH}: {error}"))
        })
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct Coverage {
    pub on: String,
    pub catalog_version: String,
    pub specs: usize,
    pub claims: usize,
    pub repositories: Vec<RepoCoverage>,
    pub unclaimed: Vec<Unclaimed>,
    /// Registry ids marked `browser-specs` that the snapshot no longer lists.
    pub missing_from_catalog: Vec<String>,
    pub stale: Vec<ClaimRef>,
    /// `full` or `partial` claims on a spec whose suite is `none`.
    pub depth_without_suite: Vec<ClaimRef>,
    /// Specs with a known suite that no claim has measured.
    pub suite_unmeasured: Vec<SuiteGap>,
    /// Claims short of `full` whose gap holds back a named feature, directly or
    /// as a prerequisite of another spec the same repository claims.
    pub cues: Vec<Cue>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Cue {
    pub spec: String,
    pub repo: String,
    pub crate_name: Option<String>,
    pub depth: &'static str,
    pub unlocks: Vec<Unlock>,
    pub prerequisite_for: Vec<Dependent>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Dependent {
    pub spec: String,
    pub unlocks: Vec<Unlock>,
}

#[derive(Clone, Debug, Serialize)]
pub struct RepoCoverage {
    pub repo: String,
    pub claims: usize,
    pub depth: BTreeMap<&'static str, usize>,
    pub verdict: BTreeMap<&'static str, usize>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Unclaimed {
    pub repo: String,
    pub catalog: String,
    pub in_scope: usize,
    pub specs: Vec<CatalogRef>,
}

#[derive(Clone, Debug, Serialize)]
pub struct CatalogRef {
    pub shortname: String,
    pub title: String,
    pub organization: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct ClaimRef {
    pub spec: String,
    pub repo: String,
    pub crate_name: Option<String>,
    pub verified_on: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct SuiteGap {
    pub spec: String,
    pub suite: String,
}

impl Coverage {
    /// `on` is the report date, YYYY-MM-DD; claims verified more than
    /// [`STALE_AFTER_DAYS`] before it are stale.
    pub fn compute(registry: &SpecRegistry, catalog: &CatalogSnapshot, on: &str) -> Self {
        let mut repositories: BTreeMap<&str, RepoCoverage> = BTreeMap::new();
        for claim in &registry.claim {
            let entry = repositories
                .entry(claim.repo.as_str())
                .or_insert_with(|| RepoCoverage {
                    repo: claim.repo.clone(),
                    claims: 0,
                    depth: BTreeMap::new(),
                    verdict: BTreeMap::new(),
                });
            entry.claims += 1;
            *entry.depth.entry(claim.depth.as_str()).or_default() += 1;
            *entry.verdict.entry(claim.verdict.as_str()).or_default() += 1;
        }

        let claimed_by: BTreeSet<(&str, &str)> = registry
            .claim
            .iter()
            .map(|claim| (claim.repo.as_str(), claim.spec.as_str()))
            .collect();
        let unclaimed = registry
            .scope
            .iter()
            .filter(|scope| scope.catalog == Catalog::BrowserSpecs)
            .map(|scope| {
                let in_scope: Vec<&CatalogSpec> = catalog
                    .spec
                    .iter()
                    .filter(|spec| {
                        scope.categories.is_empty()
                            || spec.categories.iter().any(|c| scope.categories.contains(c))
                    })
                    .collect();
                Unclaimed {
                    repo: scope.repo.clone(),
                    catalog: "browser-specs".to_owned(),
                    in_scope: in_scope.len(),
                    specs: in_scope
                        .into_iter()
                        .filter(|spec| {
                            !claimed_by.contains(&(scope.repo.as_str(), spec.shortname.as_str()))
                        })
                        .map(|spec| CatalogRef {
                            shortname: spec.shortname.clone(),
                            title: spec.title.clone(),
                            organization: spec.organization.clone(),
                        })
                        .collect(),
                }
            })
            .collect();

        let listed: BTreeSet<&str> = catalog
            .spec
            .iter()
            .map(|spec| spec.shortname.as_str())
            .collect();
        let missing_from_catalog = registry
            .spec
            .iter()
            .filter(|spec| {
                spec.catalog == Catalog::BrowserSpecs && !listed.contains(spec.id.as_str())
            })
            .map(|spec| spec.id.clone())
            .collect();

        let reference = |claim: &crate::specs::ClaimRecord| ClaimRef {
            spec: claim.spec.clone(),
            repo: claim.repo.clone(),
            crate_name: claim.crate_name.clone(),
            verified_on: claim.verified_on.clone(),
        };
        let cutoff = day_number(on).map(|day| day - STALE_AFTER_DAYS);
        let stale = registry
            .claim
            .iter()
            .filter(|claim| matches!((day_number(&claim.verified_on), cutoff), (Some(day), Some(cut)) if day < cut))
            .map(reference)
            .collect();

        let suite_of: BTreeMap<&str, &str> = registry
            .spec
            .iter()
            .map(|spec| (spec.id.as_str(), spec.suite.as_str()))
            .collect();
        let depth_without_suite = registry
            .claim
            .iter()
            .filter(|claim| matches!(claim.depth, Depth::Full | Depth::Partial))
            .filter(|claim| suite_of.get(claim.spec.as_str()) == Some(&"none"))
            .map(reference)
            .collect();

        let measured: BTreeSet<&str> = registry
            .claim
            .iter()
            .filter(|claim| claim.measured.is_some())
            .map(|claim| claim.spec.as_str())
            .collect();
        let suite_unmeasured = registry
            .spec
            .iter()
            .filter(|spec| spec.suite != "none" && !measured.contains(spec.id.as_str()))
            .map(|spec| SuiteGap {
                spec: spec.id.clone(),
                suite: spec.suite.clone(),
            })
            .collect();

        let requires = requirements(registry, catalog);
        let cues = registry
            .claim
            .iter()
            .filter(|claim| claim.depth != Depth::Full)
            .filter_map(|claim| {
                let mut prerequisite_for: Vec<Dependent> = registry
                    .claim
                    .iter()
                    .filter(|other| other.repo == claim.repo && other.spec != claim.spec)
                    .filter(|other| !other.unlocks.is_empty())
                    .filter(|other| {
                        requires
                            .get(other.spec.as_str())
                            .is_some_and(|set| set.contains(claim.spec.as_str()))
                    })
                    .map(|other| Dependent {
                        spec: other.spec.clone(),
                        unlocks: other.unlocks.clone(),
                    })
                    .collect();
                prerequisite_for.dedup_by(|a, b| a.spec == b.spec);
                (!claim.unlocks.is_empty() || !prerequisite_for.is_empty()).then(|| Cue {
                    spec: claim.spec.clone(),
                    repo: claim.repo.clone(),
                    crate_name: claim.crate_name.clone(),
                    depth: claim.depth.as_str(),
                    unlocks: claim.unlocks.clone(),
                    prerequisite_for,
                })
            })
            .collect();

        Self {
            on: on.to_owned(),
            catalog_version: catalog.version.clone(),
            specs: registry.spec.len(),
            claims: registry.claim.len(),
            repositories: repositories.into_values().collect(),
            unclaimed,
            missing_from_catalog,
            stale,
            depth_without_suite,
            suite_unmeasured,
            cues,
        }
    }
}

/// Each spec's prerequisites: the catalog's normative references, except for
/// umbrella specs (see [`BROAD_REFERENCE_LIMIT`]), plus the registry's own
/// `requires`.
pub fn requirements<'a>(
    registry: &'a SpecRegistry,
    catalog: &'a CatalogSnapshot,
) -> BTreeMap<&'a str, BTreeSet<&'a str>> {
    let mut requires: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for spec in catalog
        .spec
        .iter()
        .filter(|spec| spec.references.len() <= BROAD_REFERENCE_LIMIT)
    {
        requires
            .entry(spec.shortname.as_str())
            .or_default()
            .extend(spec.references.iter().map(String::as_str));
    }
    for spec in &registry.spec {
        requires
            .entry(spec.id.as_str())
            .or_default()
            .extend(spec.requires.iter().map(String::as_str));
    }
    requires
}

impl fmt::Display for Coverage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "spec coverage on {}: {} specs, {} claims, {} repositories (browser-specs {})",
            self.on,
            self.specs,
            self.claims,
            self.repositories.len(),
            self.catalog_version
        )?;
        writeln!(f, "\nclaims by repository")?;
        for repo in &self.repositories {
            let join = |counts: &BTreeMap<&str, usize>| {
                counts
                    .iter()
                    .map(|(k, v)| format!("{k} {v}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            writeln!(
                f,
                "  {:<14} {:>4}  depth: {}  verdict: {}",
                repo.repo,
                repo.claims,
                join(&repo.depth),
                join(&repo.verdict)
            )?;
        }
        for scope in &self.unclaimed {
            writeln!(
                f,
                "\nunclaimed in {} / {}: {} of {}",
                scope.repo,
                scope.catalog,
                scope.specs.len(),
                scope.in_scope
            )?;
            for spec in &scope.specs {
                writeln!(f, "  {:<40} {}", spec.shortname, spec.title)?;
            }
        }
        list(
            f,
            "registry ids missing from the catalog",
            self.missing_from_catalog.iter().map(String::as_str),
        )?;
        claims(
            f,
            &format!(
                "stale (verified more than {STALE_AFTER_DAYS} days before {})",
                self.on
            ),
            &self.stale,
        )?;
        claims(
            f,
            "full or partial with suite none",
            &self.depth_without_suite,
        )?;
        writeln!(
            f,
            "\nsuite known, nothing measured: {}",
            self.suite_unmeasured.len()
        )?;
        for gap in &self.suite_unmeasured {
            writeln!(f, "  {:<40} {}", gap.spec, gap.suite)?;
        }
        writeln!(
            f,
            "\ncues (a gap and what closing it unlocks): {}",
            self.cues.len()
        )?;
        for cue in &self.cues {
            let crate_name = cue
                .crate_name
                .as_deref()
                .map(|c| format!(" / {c}"))
                .unwrap_or_default();
            writeln!(
                f,
                "  {} / {}{crate_name} ({})",
                cue.spec, cue.repo, cue.depth
            )?;
            for unlock in &cue.unlocks {
                writeln!(f, "    unlocks {} ({})", unlock.feature, unlock.consumer)?;
            }
            for dependent in &cue.prerequisite_for {
                for unlock in &dependent.unlocks {
                    writeln!(
                        f,
                        "    via {}: {} ({})",
                        dependent.spec, unlock.feature, unlock.consumer
                    )?;
                }
            }
        }
        Ok(())
    }
}

fn list<'a>(
    f: &mut fmt::Formatter<'_>,
    title: &str,
    items: impl ExactSizeIterator<Item = &'a str>,
) -> fmt::Result {
    writeln!(f, "\n{title}: {}", items.len())?;
    for item in items {
        writeln!(f, "  {item}")?;
    }
    Ok(())
}

fn claims(f: &mut fmt::Formatter<'_>, title: &str, claims: &[ClaimRef]) -> fmt::Result {
    writeln!(f, "\n{title}: {}", claims.len())?;
    for claim in claims {
        let crate_name = claim
            .crate_name
            .as_deref()
            .map(|c| format!(" / {c}"))
            .unwrap_or_default();
        writeln!(
            f,
            "  {} / {}{crate_name}  ({})",
            claim.spec, claim.repo, claim.verified_on
        )?;
    }
    Ok(())
}

/// Today's UTC date as YYYY-MM-DD.
pub fn today() -> String {
    let days = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs() as i64 / 86_400)
        .unwrap_or_default();
    let (year, month, day) = civil_from_days(days);
    format!("{year:04}-{month:02}-{day:02}")
}

/// Days since 1970-01-01 for a YYYY-MM-DD date (Howard Hinnant's algorithm).
fn day_number(date: &str) -> Option<i64> {
    let mut parts = date.splitn(3, '-').map(str::parse::<i64>);
    let (year, month, day) = (
        parts.next()?.ok()?,
        parts.next()?.ok()?,
        parts.next()?.ok()?,
    );
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let year_of_era = year - era * 400;
    let day_of_year = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    Some(era * 146_097 + day_of_era - 719_468)
}

fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let day_of_era = z - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let mp = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

#[cfg(test)]
mod tests {
    use super::{civil_from_days, day_number};

    #[test]
    fn day_numbers_round_trip() {
        assert_eq!(day_number("1970-01-01"), Some(0));
        assert_eq!(
            day_number("2026-10-18").unwrap() - day_number("2026-07-20").unwrap(),
            90
        );
        for date in ["2000-02-29", "2026-10-18", "2024-12-31", "1999-03-01"] {
            let (y, m, d) = civil_from_days(day_number(date).unwrap());
            assert_eq!(format!("{y:04}-{m:02}-{d:02}"), date);
        }
    }
}
