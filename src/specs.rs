//! The family spec registry: every external specification a repository in the
//! organization implements, depends on or should follow. Rules and grammar live
//! in docs/2026-10-18_family_spec_registry_plan.md.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::repositories::{Authority, AuthorityError};

pub const SPEC_REGISTRY_PATH: &str = "content/specs.toml";
pub const SPEC_REGISTRY_SCHEMA: &str = "mer3ly.spec-registry/v1";
/// Repository id for a verdict that has no single consumer.
pub const FAMILY: &str = "family";

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SpecRegistry {
    pub schema: String,
    #[serde(default)]
    pub scope: Vec<ScopeRecord>,
    #[serde(default)]
    pub spec: Vec<SpecRecord>,
    #[serde(default)]
    pub claim: Vec<ClaimRecord>,
}

/// A repository whose scope is a whole catalog; its unclaimed catalog specs are
/// reported rather than listed.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ScopeRecord {
    pub repo: String,
    pub catalog: Catalog,
    #[serde(default)]
    pub categories: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SpecRecord {
    pub id: String,
    pub name: String,
    pub body: Body,
    #[serde(default)]
    pub designator: Option<String>,
    pub catalog: Catalog,
    #[serde(default)]
    pub url: Option<String>,
    pub suite: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ClaimRecord {
    pub spec: String,
    pub repo: String,
    #[serde(default, rename = "crate")]
    pub crate_name: Option<String>,
    pub relation: Relation,
    pub depth: Depth,
    pub verdict: Verdict,
    #[serde(default)]
    pub evidence: Option<String>,
    #[serde(default)]
    pub measured: Option<Measured>,
    #[serde(default)]
    pub consumers: Vec<String>,
    pub source: String,
    pub verified_on: String,
}

/// Counts copied from a measurement, never estimated; each block carries its date
/// and scope, and its engine where one ran.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Measured {
    pub on: String,
    pub scope: String,
    #[serde(default)]
    pub engine: Option<String>,
    #[serde(default)]
    pub wpt_pass: Option<u64>,
    #[serde(default)]
    pub wpt_total: Option<u64>,
    #[serde(default)]
    pub files_all_pass: Option<u64>,
    #[serde(default)]
    pub files: Option<u64>,
    #[serde(default)]
    pub idl_named: Option<u64>,
    #[serde(default)]
    pub idl_total: Option<u64>,
    #[serde(default)]
    pub css_props_named: Option<u64>,
    #[serde(default)]
    pub css_props_total: Option<u64>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum Body {
    W3C,
    WHATWG,
    IETF,
    Ecma,
    Khronos,
    Unicode,
    #[serde(rename = "ISO/IEC")]
    IsoIec,
    #[serde(rename = "IAU/IERS")]
    IauIers,
    NIST,
    ITU,
    FIDO,
    #[serde(rename = "freedesktop")]
    Freedesktop,
    #[serde(rename = "vendor")]
    Vendor,
    #[serde(rename = "community")]
    Community,
    /// A published house spec (ruling D5).
    #[serde(rename = "merely")]
    Merely,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Catalog {
    BrowserSpecs,
    W3cTr,
    Rfc,
    None,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Relation {
    Implements,
    ViaDep,
    Should,
    Adjacent,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Depth {
    Full,
    Partial,
    Stub,
    Planned,
    None,
}

/// mere's survey grammar, plus `split` and `open` for rows nobody has ruled.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Verdict {
    ADOPT,
    PULL,
    SKIP,
    WATCH,
    #[serde(rename = "split")]
    Split,
    #[serde(rename = "open")]
    Open,
}

impl Depth {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Full => "full",
            Self::Partial => "partial",
            Self::Stub => "stub",
            Self::Planned => "planned",
            Self::None => "none",
        }
    }
}

impl Verdict {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ADOPT => "ADOPT",
            Self::PULL => "PULL",
            Self::SKIP => "SKIP",
            Self::WATCH => "WATCH",
            Self::Split => "split",
            Self::Open => "open",
        }
    }
}

impl SpecRegistry {
    pub fn load(root: impl AsRef<Path>) -> Result<Self, AuthorityError> {
        let path = root.as_ref().join(SPEC_REGISTRY_PATH);
        let text = fs::read_to_string(&path).map_err(|error| {
            AuthorityError::from_message(format!("read {}: {error}", path.display()))
        })?;
        Self::parse(&text)
    }

    pub fn parse(text: &str) -> Result<Self, AuthorityError> {
        toml::from_str(text).map_err(|error| {
            AuthorityError::from_message(format!("parse {SPEC_REGISTRY_PATH}: {error}"))
        })
    }

    pub fn validate(&self, authority: &Authority) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        if self.schema != SPEC_REGISTRY_SCHEMA {
            errors.push(format!(
                "spec registry schema must be {SPEC_REGISTRY_SCHEMA}, got {}",
                self.schema
            ));
        }
        let repos: BTreeSet<&str> = authority
            .migration
            .migration
            .iter()
            .map(|record| record.id.as_str())
            .chain([FAMILY])
            .collect();

        for scope in &self.scope {
            if !repos.contains(scope.repo.as_str()) {
                errors.push(format!("scope has unknown repo {}", scope.repo));
            }
            if matches!(scope.catalog, Catalog::Rfc | Catalog::None) {
                errors.push(format!("scope {} must name a listing catalog", scope.repo));
            }
        }

        let mut ids = BTreeSet::new();
        for spec in &self.spec {
            let id = &spec.id;
            let valid_id = !id.is_empty()
                && id
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '.' | '_'));
            if !valid_id {
                errors.push(format!("spec id is malformed: {id:?}"));
            }
            if !ids.insert(id.as_str()) {
                errors.push(format!("duplicate spec id {id}"));
            }
            if spec.name.trim().is_empty() {
                errors.push(format!("spec {id} has no name"));
            }
            if spec.catalog == Catalog::Rfc && !is_rfc_id(id) {
                errors.push(format!(
                    "spec {id} is catalog rfc but its id is not rfcNNNN"
                ));
            }
            if let Some(url) = &spec.url
                && !url.starts_with("https://")
            {
                errors.push(format!("spec {id} url is not https: {url}"));
            }
            if !is_suite(&spec.suite) {
                errors.push(format!("spec {id} suite is malformed: {:?}", spec.suite));
            }
        }

        let mut keys = BTreeSet::new();
        let mut claimed = BTreeSet::new();
        for claim in &self.claim {
            let label = format!(
                "claim {} / {}{}",
                claim.spec,
                claim.repo,
                claim
                    .crate_name
                    .as_deref()
                    .map(|c| format!(" / {c}"))
                    .unwrap_or_default()
            );
            if !ids.contains(claim.spec.as_str()) {
                errors.push(format!("{label} names unknown spec"));
            }
            if !repos.contains(claim.repo.as_str()) {
                errors.push(format!("{label} names unknown repo"));
            }
            if claim
                .crate_name
                .as_deref()
                .is_some_and(|c| c.trim().is_empty())
            {
                errors.push(format!("{label} has an empty crate"));
            }
            if !keys.insert((&claim.spec, &claim.repo, &claim.crate_name)) {
                errors.push(format!("duplicate {label}"));
            }
            claimed.insert(claim.spec.as_str());
            if claim.source.trim().is_empty() {
                errors.push(format!("{label} has no source"));
            }
            if claim.consumers.iter().any(|c| c.trim().is_empty()) {
                errors.push(format!("{label} has an empty consumer"));
            }
            check_date(
                &format!("{label} verified_on"),
                &claim.verified_on,
                &mut errors,
            );
            if let Some(measured) = &claim.measured {
                check_measured(&label, measured, &mut errors);
            }
        }

        for spec in &self.spec {
            if !claimed.contains(spec.id.as_str()) {
                errors.push(format!("spec {} has no claim", spec.id));
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    pub fn repositories(&self) -> BTreeSet<&str> {
        self.claim.iter().map(|claim| claim.repo.as_str()).collect()
    }
}

fn check_measured(label: &str, measured: &Measured, errors: &mut Vec<String>) {
    check_date(&format!("{label} measured.on"), &measured.on, errors);
    if measured.scope.trim().is_empty() {
        errors.push(format!("{label} measured block has no scope"));
    }
    let pairs = [
        ("wpt_pass", measured.wpt_pass, measured.wpt_total),
        ("files_all_pass", measured.files_all_pass, measured.files),
        ("idl_named", measured.idl_named, measured.idl_total),
        (
            "css_props_named",
            measured.css_props_named,
            measured.css_props_total,
        ),
    ];
    let mut any = false;
    for (name, part, total) in pairs {
        match (part, total) {
            (Some(part), Some(total)) => {
                any = true;
                if part > total {
                    errors.push(format!("{label} {name} {part} exceeds its total {total}"));
                }
            }
            (None, None) => {}
            _ => errors.push(format!("{label} {name} needs both its count and its total")),
        }
    }
    if !any {
        errors.push(format!("{label} measured block has no counts"));
    }
}

fn is_rfc_id(id: &str) -> bool {
    id.strip_prefix("rfc")
        .is_some_and(|number| !number.is_empty() && number.chars().all(|c| c.is_ascii_digit()))
}

/// `none`, or `<kind>[:<detail>]` with a lowercase kind and no whitespace.
fn is_suite(suite: &str) -> bool {
    let (kind, detail) = match suite.split_once(':') {
        Some((kind, detail)) => (kind, Some(detail)),
        None => (suite, None),
    };
    !kind.is_empty()
        && kind
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '-' | '.'))
        && detail.is_none_or(|d| !d.is_empty() && !d.chars().any(char::is_whitespace))
}

fn check_date(label: &str, value: &str, errors: &mut Vec<String>) {
    let bytes = value.as_bytes();
    let valid = bytes.len() == 10
        && bytes.iter().enumerate().all(|(index, byte)| {
            if matches!(index, 4 | 7) {
                *byte == b'-'
            } else {
                byte.is_ascii_digit()
            }
        });
    if !valid {
        errors.push(format!("{label} is not YYYY-MM-DD: {value}"));
    }
}
