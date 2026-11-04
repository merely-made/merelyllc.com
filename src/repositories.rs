use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::devices::DeviceCatalog;
use crate::firmware_catalog::FirmwareCatalog;

const REPOSITORIES_PATH: &str = "content/repositories.toml";
const RELATIONS_PATH: &str = "content/relations.toml";
const SHOWCASES_PATH: &str = "content/showcases.toml";
const MIGRATION_PATH: &str = "ops/org-migration.toml";
const PUBLIC_METADATA_PATH: &str = "content/github-metadata.json";

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RepositoryManifest {
    #[serde(default)]
    pub repository: Vec<RepositoryRecord>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RepositoryRecord {
    pub id: String,
    pub github_slug: String,
    pub name: String,
    pub summary: String,
    pub class: RepositoryClass,
    pub status: RepositoryStatus,
    pub license: String,
    pub homepage: String,
    pub public: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ShowcaseManifest {
    #[serde(default)]
    pub showcase: Vec<ShowcaseRecord>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ShowcaseRecord {
    pub repository: String,
    pub order: u32,
    pub headline: String,
    pub copy: String,
    pub image: String,
    pub alt: String,
    pub caption: String,
    pub source_url: String,
    pub source_license: String,
    /// Additional approved captures after the primary `image`. The primary
    /// stays the social/fallback image; extras join it in the project
    /// profile's showcase rotation, in order.
    #[serde(default)]
    pub images: Vec<ShowcaseImageRecord>,
}

/// One additional approved capture in a showcase rotation. Carries the same
/// editorial obligations as the primary image: an exact description of the
/// visible state, a caption, and source attribution.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ShowcaseImageRecord {
    pub image: String,
    pub alt: String,
    pub caption: String,
    pub source_url: String,
    pub source_license: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PublicMetadataCache {
    pub schema: String,
    #[serde(default)]
    pub organization: String,
    pub generated_at_utc: String,
    #[serde(default)]
    pub repository: Vec<PublicRepositoryMetadata>,
    #[serde(default)]
    pub event: Vec<PublicOrganizationEvent>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PublicRepositoryMetadata {
    pub id: String,
    pub github_slug: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub homepage: Option<String>,
    #[serde(default)]
    pub license: Option<String>,
    pub updated_at: String,
    pub pushed_at: String,
    #[serde(default)]
    pub primary_language: Option<String>,
    pub stargazer_count: u64,
    pub archived: bool,
    pub fork: bool,
    #[serde(default)]
    pub topics: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PublicOrganizationEvent {
    pub id: String,
    pub kind: String,
    pub repository: String,
    pub created_at: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RepositoryClass {
    Foundation,
    Platform,
    Product,
    Tool,
    MaintainedFork,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RepositoryStatus {
    Active,
    Prototype,
    Reference,
    Research,
    Archived,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RelationManifest {
    #[serde(default)]
    pub relation: Vec<RelationRecord>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RelationRecord {
    pub id: String,
    pub source: String,
    pub target: String,
    pub kind: RelationKind,
    pub provenance: RelationProvenance,
    pub evidence: String,
    pub verified_on: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RelationKind {
    DependsOn,
    Contains,
    ReferenceAppFor,
    HostFor,
    UsesUiFrom,
    RendersWith,
    ForkOf,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum RelationProvenance {
    Derived,
    Curated,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MigrationManifest {
    pub inventory_receipt: String,
    #[serde(default)]
    pub publication_gate_receipt: Option<String>,
    #[serde(default)]
    pub fork_review_receipt: Option<String>,
    #[serde(default)]
    pub migration: Vec<MigrationRecord>,
    #[serde(default)]
    pub unresolved_product: Vec<UnresolvedProduct>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MigrationRecord {
    pub id: String,
    pub current_slug: String,
    #[serde(default)]
    pub target_slug: Option<String>,
    pub classification: MigrationClass,
    pub batch: MigrationBatch,
    pub disposition: MigrationDisposition,
    pub visibility: Visibility,
    pub default_branch: String,
    pub head: String,
    pub license_status: String,
    pub provenance_status: String,
    pub sensitive_information_status: String,
    #[serde(default)]
    pub public_scope: Option<String>,
    #[serde(default)]
    pub publication_gate_status: Option<PublicationGateStatus>,
    #[serde(default)]
    pub history_remediation: Option<String>,
    pub pages_status: String,
    pub packages_status: String,
    pub actions_workflows: u32,
    #[serde(default)]
    pub old_owner_files: Option<u32>,
    #[serde(default)]
    pub old_owner_manifests: Option<u32>,
    #[serde(default)]
    pub local_locator: Option<String>,
    #[serde(default)]
    pub source_aliases: Vec<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum MigrationClass {
    Foundation,
    Platform,
    Product,
    Tool,
    MaintainedFork,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum MigrationBatch {
    Infrastructure,
    Foundation,
    Platform,
    Product,
    ForkReview,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum MigrationDisposition {
    AlreadyInOrg,
    Candidate,
    Hold,
    KeepPersonal,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum PublicationGateStatus {
    Ready,
    Blocked,
    NotApplicable,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Visibility {
    Public,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct UnresolvedProduct {
    pub name: String,
    pub state: String,
    pub evidence: String,
}

#[derive(Clone, Debug)]
pub struct Authority {
    pub repositories: RepositoryManifest,
    pub relations: RelationManifest,
    pub migration: MigrationManifest,
}

#[derive(Clone, Debug)]
pub struct PublicSiteData {
    pub authority: Authority,
    pub metadata: PublicMetadataCache,
    pub showcases: ShowcaseManifest,
    pub devices: DeviceCatalog,
    pub firmware: FirmwareCatalog,
}

#[derive(Clone, Debug, Serialize)]
pub struct InventoryTarget {
    pub id: String,
    pub current_slug: String,
    pub target_slug: Option<String>,
    pub classification: MigrationClass,
    pub batch: MigrationBatch,
    pub disposition: MigrationDisposition,
    pub expected_default_branch: String,
    pub expected_head: String,
    pub expected_pages_status: String,
    pub expected_packages_status: String,
    pub expected_actions_workflows: u32,
    pub expected_old_owner_files: Option<u32>,
    pub expected_old_owner_manifests: Option<u32>,
    pub license_status: String,
    pub provenance_status: String,
    pub sensitive_information_status: String,
    pub public_scope: Option<String>,
    pub publication_gate_status: Option<PublicationGateStatus>,
    pub history_remediation: Option<String>,
    pub local_locator: Option<String>,
    pub source_aliases: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct InventoryBasis {
    pub inventory_receipt: String,
    pub targets: Vec<InventoryTarget>,
    pub unresolved_products: Vec<UnresolvedProduct>,
}

#[derive(Debug)]
pub struct AuthorityError {
    context: String,
}

impl AuthorityError {
    fn new(context: impl Into<String>) -> Self {
        Self {
            context: context.into(),
        }
    }

    pub fn from_message(context: impl Into<String>) -> Self {
        Self::new(context)
    }
}

impl fmt::Display for AuthorityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.context)
    }
}

impl std::error::Error for AuthorityError {}

pub(crate) fn reconcile_live_github_repositories(
    editorial: &RepositoryManifest,
    relations: &RelationManifest,
    metadata: &PublicMetadataCache,
) -> (RepositoryManifest, RelationManifest) {
    if metadata.schema != "mer3ly.github-organization/v2" {
        return (editorial.clone(), relations.clone());
    }

    let live_by_slug = metadata
        .repository
        .iter()
        .map(|repository| (repository.github_slug.as_str(), repository))
        .collect::<BTreeMap<_, _>>();
    let editorial_slugs = editorial
        .repository
        .iter()
        .map(|repository| repository.github_slug.as_str())
        .collect::<BTreeSet<_>>();
    let mut repositories = editorial
        .repository
        .iter()
        .filter(|repository| live_by_slug.contains_key(repository.github_slug.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    repositories.extend(
        metadata
            .repository
            .iter()
            .filter(|live| !editorial_slugs.contains(live.github_slug.as_str()))
            .map(repository_fallback_from_github),
    );
    let live_ids = repositories
        .iter()
        .map(|repository| repository.id.as_str())
        .collect::<BTreeSet<_>>();
    let relations = relations
        .relation
        .iter()
        .filter(|relation| {
            live_ids.contains(relation.source.as_str())
                && live_ids.contains(relation.target.as_str())
        })
        .cloned()
        .collect();

    (
        RepositoryManifest {
            repository: repositories,
        },
        RelationManifest {
            relation: relations,
        },
    )
}

fn repository_fallback_from_github(live: &PublicRepositoryMetadata) -> RepositoryRecord {
    RepositoryRecord {
        id: live.id.clone(),
        github_slug: live.github_slug.clone(),
        name: live.name.clone(),
        summary: if live.description.is_empty() {
            "A public repository in the Merely GitHub organization.".to_owned()
        } else {
            live.description.clone()
        },
        class: if live.fork {
            RepositoryClass::MaintainedFork
        } else {
            RepositoryClass::Tool
        },
        status: if live.archived {
            RepositoryStatus::Archived
        } else {
            RepositoryStatus::Active
        },
        license: live
            .license
            .clone()
            .unwrap_or_else(|| "NOASSERTION".to_owned()),
        homepage: live
            .homepage
            .clone()
            .unwrap_or_else(|| format!("https://github.com/{}", live.github_slug)),
        public: true,
    }
}

impl PublicSiteData {
    pub fn load(root: impl AsRef<Path>) -> Result<Self, AuthorityError> {
        let root = root.as_ref();
        let authority = Authority::load(root)?;
        authority.validate().map_err(|errors| {
            AuthorityError::new(format!(
                "authority validation failed:\n{}",
                errors.join("\n")
            ))
        })?;
        let metadata: PublicMetadataCache = read_json(root.join(PUBLIC_METADATA_PATH))?;
        metadata.validate(&authority).map_err(|errors| {
            AuthorityError::new(format!(
                "public metadata validation failed:\n{}",
                errors.join("\n")
            ))
        })?;
        let authority = authority.with_live_github_repositories(&metadata);
        let mut showcases: ShowcaseManifest = read_toml(root.join(SHOWCASES_PATH))?;
        showcases.retain_live_repositories(&authority);
        showcases.validate(root, &authority).map_err(|errors| {
            AuthorityError::new(format!(
                "showcase validation failed:\n{}",
                errors.join("\n")
            ))
        })?;
        let devices = DeviceCatalog::load(root)?;
        let firmware = FirmwareCatalog::load(root)?;
        firmware.validate_device_recipes(&devices)?;
        Ok(Self {
            authority,
            metadata,
            showcases,
            devices,
            firmware,
        })
    }
}

impl RepositoryClass {
    pub const fn slug(self) -> &'static str {
        match self {
            Self::Foundation => "foundation",
            Self::Platform => "platform",
            Self::Product => "product",
            Self::Tool => "tool",
            Self::MaintainedFork => "maintained-fork",
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Foundation => "foundation",
            Self::Platform => "platform",
            Self::Product => "product",
            Self::Tool => "tool",
            Self::MaintainedFork => "maintained fork",
        }
    }
}

impl RepositoryStatus {
    pub const fn slug(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Prototype => "prototype",
            Self::Reference => "reference",
            Self::Research => "research",
            Self::Archived => "archived",
        }
    }

    pub const fn label(self) -> &'static str {
        self.slug()
    }
}

impl RelationKind {
    pub const fn slug(self) -> &'static str {
        match self {
            Self::DependsOn => "depends-on",
            Self::Contains => "contains",
            Self::ReferenceAppFor => "reference-app-for",
            Self::HostFor => "host-for",
            Self::UsesUiFrom => "uses-ui-from",
            Self::RendersWith => "renders-with",
            Self::ForkOf => "fork-of",
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::DependsOn => "depends on",
            Self::Contains => "contains",
            Self::ReferenceAppFor => "is a reference app for",
            Self::HostFor => "hosts",
            Self::UsesUiFrom => "uses UI from",
            Self::RendersWith => "renders with",
            Self::ForkOf => "is a fork of",
        }
    }

    pub const fn incoming_label(self) -> &'static str {
        match self {
            Self::DependsOn => "depends on this repository",
            Self::Contains => "contains this repository",
            Self::ReferenceAppFor => "is a reference app for this repository",
            Self::HostFor => "hosts this repository",
            Self::UsesUiFrom => "uses UI from this repository",
            Self::RendersWith => "renders with this repository",
            Self::ForkOf => "is a fork of this repository",
        }
    }
}

impl RelationProvenance {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Derived => "derived",
            Self::Curated => "curated",
        }
    }
}

#[cfg(test)]
mod tests;

mod authority;

mod showcases;

mod metadata;

mod validation;

use validation::{
    check_branch, check_date, check_head, check_https_or_github, check_nonempty,
    check_receipt_path, check_safe_locator, check_slug, check_timestamp, read_json, read_toml,
    validate_showcase_png,
};
