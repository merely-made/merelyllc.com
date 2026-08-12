use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::devices::DeviceCatalog;

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

impl Authority {
    pub fn load(root: impl AsRef<Path>) -> Result<Self, AuthorityError> {
        let root = root.as_ref();
        Ok(Self {
            repositories: read_toml(root.join(REPOSITORIES_PATH))?,
            relations: read_toml(root.join(RELATIONS_PATH))?,
            migration: read_toml(root.join(MIGRATION_PATH))?,
        })
    }

    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        let mut repository_ids = BTreeSet::new();
        let mut repository_slugs = BTreeSet::new();

        if self.repositories.repository.is_empty() {
            errors.push("content/repositories.toml has no repository records".to_owned());
        }

        for repository in &self.repositories.repository {
            check_nonempty("repository id", &repository.id, &mut errors);
            check_nonempty(
                &format!("repository {} name", repository.id),
                &repository.name,
                &mut errors,
            );
            check_nonempty(
                &format!("repository {} summary", repository.id),
                &repository.summary,
                &mut errors,
            );
            check_nonempty(
                &format!("repository {} license", repository.id),
                &repository.license,
                &mut errors,
            );
            check_https_or_github(
                &format!("repository {} homepage", repository.id),
                &repository.homepage,
                &mut errors,
            );
            check_slug(
                &format!("repository {} github_slug", repository.id),
                &repository.github_slug,
                &mut errors,
            );
            if !repository.public {
                errors.push(format!(
                    "repository {} is not public; private repositories do not belong in site authority",
                    repository.id
                ));
            }
            if !repository_ids.insert(repository.id.clone()) {
                errors.push(format!("duplicate repository id {}", repository.id));
            }
            if !repository_slugs.insert(repository.github_slug.clone()) {
                errors.push(format!(
                    "duplicate repository github_slug {}",
                    repository.github_slug
                ));
            }
        }

        let mut relation_ids = BTreeSet::new();
        for relation in &self.relations.relation {
            if !relation_ids.insert(relation.id.clone()) {
                errors.push(format!("duplicate relation id {}", relation.id));
            }
            if !repository_ids.contains(&relation.source) {
                errors.push(format!(
                    "relation {} has unknown source {}",
                    relation.id, relation.source
                ));
            }
            if !repository_ids.contains(&relation.target) {
                errors.push(format!(
                    "relation {} has unknown target {}",
                    relation.id, relation.target
                ));
            }
            if relation.source == relation.target {
                errors.push(format!("relation {} is a self-edge", relation.id));
            }
            check_nonempty(
                &format!("relation {} evidence", relation.id),
                &relation.evidence,
                &mut errors,
            );
            check_date(
                &format!("relation {} verified_on", relation.id),
                &relation.verified_on,
                &mut errors,
            );
        }

        let mut migration_ids = BTreeSet::new();
        let mut current_slugs = BTreeSet::new();
        let mut target_slugs = BTreeSet::new();
        let mut aliases = BTreeSet::new();
        let migrations_by_id: BTreeMap<_, _> = self
            .migration
            .migration
            .iter()
            .map(|migration| (migration.id.as_str(), migration))
            .collect();

        for migration in &self.migration.migration {
            if !migration_ids.insert(migration.id.clone()) {
                errors.push(format!("duplicate migration id {}", migration.id));
            }
            check_slug(
                &format!("migration {} current_slug", migration.id),
                &migration.current_slug,
                &mut errors,
            );
            if !current_slugs.insert(migration.current_slug.clone()) {
                errors.push(format!(
                    "duplicate migration current_slug {}",
                    migration.current_slug
                ));
            }
            if let Some(target_slug) = &migration.target_slug {
                check_slug(
                    &format!("migration {} target_slug", migration.id),
                    target_slug,
                    &mut errors,
                );
                if !target_slugs.insert(target_slug.clone()) {
                    errors.push(format!("duplicate migration target_slug {target_slug}"));
                }
            }
            for alias in &migration.source_aliases {
                check_slug(
                    &format!("migration {} source alias", migration.id),
                    alias,
                    &mut errors,
                );
                if alias == &migration.current_slug {
                    errors.push(format!(
                        "migration {} repeats its current slug as an alias",
                        migration.id
                    ));
                }
                if !aliases.insert(alias.clone()) {
                    errors.push(format!("duplicate migration source alias {alias}"));
                }
            }
            check_branch(&migration.id, &migration.default_branch, &mut errors);
            check_head(&migration.id, &migration.head, &mut errors);
            check_nonempty(
                &format!("migration {} license_status", migration.id),
                &migration.license_status,
                &mut errors,
            );
            check_nonempty(
                &format!("migration {} provenance_status", migration.id),
                &migration.provenance_status,
                &mut errors,
            );
            check_nonempty(
                &format!("migration {} sensitive_information_status", migration.id),
                &migration.sensitive_information_status,
                &mut errors,
            );
            if let Some(public_scope) = &migration.public_scope {
                check_nonempty(
                    &format!("migration {} public_scope", migration.id),
                    public_scope,
                    &mut errors,
                );
            }
            if let Some(history_remediation) = &migration.history_remediation {
                check_nonempty(
                    &format!("migration {} history_remediation", migration.id),
                    history_remediation,
                    &mut errors,
                );
            }
            check_nonempty(
                &format!("migration {} pages_status", migration.id),
                &migration.pages_status,
                &mut errors,
            );
            check_nonempty(
                &format!("migration {} packages_status", migration.id),
                &migration.packages_status,
                &mut errors,
            );
            if let Some(locator) = &migration.local_locator {
                check_safe_locator(&migration.id, locator, &mut errors);
            }

            match migration.disposition {
                MigrationDisposition::AlreadyInOrg => {
                    if !migration.current_slug.starts_with("merely-made/") {
                        errors.push(format!(
                            "migration {} says already-in-org but current slug is {}",
                            migration.id, migration.current_slug
                        ));
                    }
                    if migration.target_slug.as_deref() != Some(&migration.current_slug) {
                        errors.push(format!(
                            "migration {} already-in-org target must equal current slug",
                            migration.id
                        ));
                    }
                }
                MigrationDisposition::Candidate => {
                    let Some(target) = migration.target_slug.as_deref() else {
                        errors.push(format!(
                            "migration {} candidate has no target slug",
                            migration.id
                        ));
                        continue;
                    };
                    if !target.starts_with("merely-made/") {
                        errors.push(format!(
                            "migration {} candidate target is outside merely-made",
                            migration.id
                        ));
                    }
                    if migration.public_scope.as_deref().is_none_or(str::is_empty) {
                        errors.push(format!(
                            "migration {} candidate has no public_scope",
                            migration.id
                        ));
                    }
                    if migration.publication_gate_status.is_none() {
                        errors.push(format!(
                            "migration {} candidate has no publication_gate_status",
                            migration.id
                        ));
                    }
                    if migration
                        .history_remediation
                        .as_deref()
                        .is_none_or(str::is_empty)
                    {
                        errors.push(format!(
                            "migration {} candidate has no history_remediation",
                            migration.id
                        ));
                    }
                }
                MigrationDisposition::Hold => {
                    if migration.target_slug.is_some() {
                        errors.push(format!(
                            "migration {} is on hold but already claims a target slug",
                            migration.id
                        ));
                    }
                }
                MigrationDisposition::KeepPersonal => {
                    if migration.classification != MigrationClass::MaintainedFork {
                        errors.push(format!(
                            "migration {} keeps personal ownership but is not a maintained fork",
                            migration.id
                        ));
                    }
                    if migration.target_slug.is_some() {
                        errors.push(format!(
                            "migration {} keeps personal ownership but claims a target slug",
                            migration.id
                        ));
                    }
                    if !migration.current_slug.starts_with("mark-ik/") {
                        errors.push(format!(
                            "migration {} keeps personal ownership outside mark-ik",
                            migration.id
                        ));
                    }
                }
            }
        }

        for repository in &self.repositories.repository {
            let Some(migration) = migrations_by_id.get(repository.id.as_str()) else {
                errors.push(format!(
                    "site repository {} has no migration ledger entry",
                    repository.id
                ));
                continue;
            };
            if migration.current_slug != repository.github_slug {
                errors.push(format!(
                    "repository {} slug {} disagrees with migration current slug {}",
                    repository.id, repository.github_slug, migration.current_slug
                ));
            }
        }

        check_receipt_path(&self.migration.inventory_receipt, &mut errors);
        if let Some(receipt) = &self.migration.publication_gate_receipt {
            check_receipt_path(receipt, &mut errors);
        }
        if let Some(receipt) = &self.migration.fork_review_receipt {
            check_receipt_path(receipt, &mut errors);
        }

        let mut unresolved_names = BTreeSet::new();
        for unresolved in &self.migration.unresolved_product {
            check_nonempty("unresolved product name", &unresolved.name, &mut errors);
            check_nonempty(
                &format!("unresolved product {} state", unresolved.name),
                &unresolved.state,
                &mut errors,
            );
            check_nonempty(
                &format!("unresolved product {} evidence", unresolved.name),
                &unresolved.evidence,
                &mut errors,
            );
            if !unresolved_names.insert(unresolved.name.to_ascii_lowercase()) {
                errors.push(format!("duplicate unresolved product {}", unresolved.name));
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            errors.sort();
            Err(errors)
        }
    }

    pub fn inventory_targets(&self) -> Vec<InventoryTarget> {
        self.migration
            .migration
            .iter()
            .map(|migration| InventoryTarget {
                id: migration.id.clone(),
                current_slug: migration.current_slug.clone(),
                target_slug: migration.target_slug.clone(),
                classification: migration.classification,
                batch: migration.batch,
                disposition: migration.disposition,
                expected_default_branch: migration.default_branch.clone(),
                expected_head: migration.head.clone(),
                expected_pages_status: migration.pages_status.clone(),
                expected_packages_status: migration.packages_status.clone(),
                expected_actions_workflows: migration.actions_workflows,
                expected_old_owner_files: migration.old_owner_files,
                expected_old_owner_manifests: migration.old_owner_manifests,
                license_status: migration.license_status.clone(),
                provenance_status: migration.provenance_status.clone(),
                sensitive_information_status: migration.sensitive_information_status.clone(),
                public_scope: migration.public_scope.clone(),
                publication_gate_status: migration.publication_gate_status,
                history_remediation: migration.history_remediation.clone(),
                local_locator: migration.local_locator.clone(),
                source_aliases: migration.source_aliases.clone(),
            })
            .collect()
    }

    pub fn inventory_basis(&self) -> InventoryBasis {
        InventoryBasis {
            inventory_receipt: self.migration.inventory_receipt.clone(),
            targets: self.inventory_targets(),
            unresolved_products: self.migration.unresolved_product.clone(),
        }
    }

    fn with_live_github_repositories(mut self, metadata: &PublicMetadataCache) -> Self {
        let (repositories, relations) =
            reconcile_live_github_repositories(&self.repositories, &self.relations, metadata);
        self.repositories = repositories;
        self.relations = relations;
        self
    }
}

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
        Ok(Self {
            authority,
            metadata,
            showcases,
            devices,
        })
    }
}

impl ShowcaseManifest {
    fn retain_live_repositories(&mut self, authority: &Authority) {
        let live_ids = authority
            .repositories
            .repository
            .iter()
            .map(|repository| repository.id.as_str())
            .collect::<BTreeSet<_>>();
        self.showcase
            .retain(|showcase| live_ids.contains(showcase.repository.as_str()));
    }

    pub fn validate(&self, root: &Path, authority: &Authority) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        let repositories = authority
            .repositories
            .repository
            .iter()
            .filter(|repository| repository.public)
            .map(|repository| (repository.id.as_str(), repository))
            .collect::<BTreeMap<_, _>>();
        let mut seen_repositories = BTreeSet::new();
        let mut seen_orders = BTreeSet::new();
        let mut seen_images = BTreeSet::new();

        if self.showcase.is_empty() {
            errors.push("showcase authority has no records".to_owned());
        }

        for showcase in &self.showcase {
            check_nonempty("showcase repository", &showcase.repository, &mut errors);
            check_nonempty(
                &format!("showcase {} headline", showcase.repository),
                &showcase.headline,
                &mut errors,
            );
            check_nonempty(
                &format!("showcase {} copy", showcase.repository),
                &showcase.copy,
                &mut errors,
            );
            check_nonempty(
                &format!("showcase {} alt", showcase.repository),
                &showcase.alt,
                &mut errors,
            );
            check_nonempty(
                &format!("showcase {} caption", showcase.repository),
                &showcase.caption,
                &mut errors,
            );

            if !seen_repositories.insert(showcase.repository.as_str()) {
                errors.push(format!(
                    "duplicate showcase repository {}",
                    showcase.repository
                ));
            }
            if showcase.order == 0 || !seen_orders.insert(showcase.order) {
                errors.push(format!(
                    "showcase {} has a zero or duplicate order",
                    showcase.repository
                ));
            }
            if !seen_images.insert(showcase.image.as_str()) {
                errors.push(format!(
                    "showcase {} repeats an image path",
                    showcase.repository
                ));
            }

            let Some(repository) = repositories.get(showcase.repository.as_str()) else {
                errors.push(format!(
                    "showcase {} does not name a public repository",
                    showcase.repository
                ));
                continue;
            };

            let expected_image = format!("showcase/{}.png", showcase.repository);
            if showcase.image != expected_image
                || showcase.image.contains('\\')
                || showcase
                    .image
                    .split('/')
                    .any(|segment| segment.is_empty() || segment == "." || segment == "..")
            {
                errors.push(format!(
                    "showcase {} image does not use its approved normalized path",
                    showcase.repository
                ));
            }

            if showcase.source_license != repository.license {
                errors.push(format!(
                    "showcase {} source license disagrees with repository authority",
                    showcase.repository
                ));
            }

            let expected_source_prefix =
                format!("https://github.com/{}/blob/", repository.github_slug);
            if !showcase.source_url.starts_with(&expected_source_prefix) {
                errors.push(format!(
                    "showcase {} source URL does not belong to its repository",
                    showcase.repository
                ));
            }

            validate_showcase_png(
                &root.join("assets").join(&showcase.image),
                &showcase.repository,
                &mut errors,
            );
        }

        if errors.is_empty() {
            Ok(())
        } else {
            errors.sort();
            Err(errors)
        }
    }

    pub fn ordered(&self) -> Vec<&ShowcaseRecord> {
        let mut showcases = self.showcase.iter().collect::<Vec<_>>();
        showcases.sort_by_key(|showcase| showcase.order);
        showcases
    }

    pub fn for_repository(&self, repository_id: &str) -> Option<&ShowcaseRecord> {
        self.showcase
            .iter()
            .find(|showcase| showcase.repository == repository_id)
    }
}

impl PublicMetadataCache {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, AuthorityError> {
        read_json(path.as_ref().to_path_buf())
    }

    pub fn validate(&self, authority: &Authority) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        if self.schema != "mer3ly.github-organization/v2" {
            errors.push(format!("unexpected public metadata schema {}", self.schema));
        }
        if self.organization != "merely-made" {
            errors.push(format!(
                "unexpected public metadata organization {}",
                self.organization
            ));
        }
        check_timestamp(
            "public metadata generated_at_utc",
            &self.generated_at_utc,
            &mut errors,
        );
        if self.repository.is_empty() {
            errors.push("public metadata has no repository records".to_owned());
        }

        let editorial_by_slug: BTreeMap<_, _> = authority
            .repositories
            .repository
            .iter()
            .filter(|repository| repository.public)
            .map(|repository| (repository.github_slug.as_str(), repository))
            .collect();
        let mut seen_ids = BTreeSet::new();
        let mut seen_slugs = BTreeSet::new();

        for metadata in &self.repository {
            let valid_id = !metadata.id.is_empty()
                && !metadata.id.starts_with('-')
                && !metadata.id.ends_with('-')
                && metadata.id.chars().all(|character| {
                    character.is_ascii_lowercase() || character.is_ascii_digit() || character == '-'
                });
            if !valid_id {
                errors.push(format!(
                    "public metadata repository id is not lowercase kebab-case: {}",
                    metadata.id
                ));
            }
            if !seen_ids.insert(metadata.id.as_str()) {
                errors.push(format!("duplicate public metadata id {}", metadata.id));
            }
            if !seen_slugs.insert(metadata.github_slug.as_str()) {
                errors.push(format!(
                    "duplicate public metadata github_slug {}",
                    metadata.github_slug
                ));
            }
            if !metadata.github_slug.starts_with("merely-made/") {
                errors.push(format!(
                    "public metadata {} is outside merely-made",
                    metadata.github_slug
                ));
            }
            check_slug(
                &format!("public metadata {} github_slug", metadata.id),
                &metadata.github_slug,
                &mut errors,
            );
            check_nonempty(
                &format!("public metadata {} name", metadata.id),
                &metadata.name,
                &mut errors,
            );
            if let Some(repository) = editorial_by_slug.get(metadata.github_slug.as_str())
                && metadata.id != repository.id
            {
                errors.push(format!(
                    "public metadata {} id disagrees with editorial id {}",
                    metadata.github_slug, repository.id
                ));
            }
            if let Some(homepage) = &metadata.homepage {
                check_https_or_github(
                    &format!("public metadata {} homepage", metadata.id),
                    homepage,
                    &mut errors,
                );
            }
            check_timestamp(
                &format!("public metadata {} updated_at", metadata.id),
                &metadata.updated_at,
                &mut errors,
            );
            check_timestamp(
                &format!("public metadata {} pushed_at", metadata.id),
                &metadata.pushed_at,
                &mut errors,
            );
            let mut topics = BTreeSet::new();
            for topic in &metadata.topics {
                let valid = !topic.is_empty()
                    && !topic.starts_with('-')
                    && !topic.ends_with('-')
                    && topic.chars().all(|character| {
                        character.is_ascii_lowercase()
                            || character.is_ascii_digit()
                            || character == '-'
                    });
                if !valid {
                    errors.push(format!(
                        "public metadata {} has invalid topic {}",
                        metadata.id, topic
                    ));
                }
                if !topics.insert(topic) {
                    errors.push(format!(
                        "public metadata {} repeats topic {}",
                        metadata.id, topic
                    ));
                }
            }
            if metadata.topics.len() > 20 {
                errors.push(format!(
                    "public metadata {} has more than 20 topics",
                    metadata.id
                ));
            }
        }

        let mut event_ids = BTreeSet::new();
        let mut previous_event_time: Option<&str> = None;
        if self.event.len() > 40 {
            errors.push("public metadata has more than 40 organization events".to_owned());
        }
        for event in &self.event {
            check_nonempty("public organization event id", &event.id, &mut errors);
            if !event_ids.insert(event.id.as_str()) {
                errors.push(format!("duplicate public organization event {}", event.id));
            }
            check_nonempty("public organization event kind", &event.kind, &mut errors);
            if !seen_slugs.contains(event.repository.as_str()) {
                errors.push(format!(
                    "public organization event {} names unknown repository {}",
                    event.id, event.repository
                ));
            }
            check_timestamp(
                &format!("public organization event {} created_at", event.id),
                &event.created_at,
                &mut errors,
            );
            if previous_event_time.is_some_and(|previous| event.created_at.as_str() > previous) {
                errors.push("public organization events are not newest-first".to_owned());
            }
            previous_event_time = Some(&event.created_at);
        }

        if errors.is_empty() {
            Ok(())
        } else {
            errors.sort();
            Err(errors)
        }
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

fn read_toml<T>(path: PathBuf) -> Result<T, AuthorityError>
where
    T: for<'de> Deserialize<'de>,
{
    let source = fs::read_to_string(&path)
        .map_err(|error| AuthorityError::new(format!("read {}: {error}", path.display())))?;
    toml::from_str(&source)
        .map_err(|error| AuthorityError::new(format!("parse {}: {error}", path.display())))
}

fn read_json<T>(path: PathBuf) -> Result<T, AuthorityError>
where
    T: for<'de> Deserialize<'de>,
{
    let source = fs::read_to_string(&path)
        .map_err(|error| AuthorityError::new(format!("read {}: {error}", path.display())))?;
    serde_json::from_str(&source)
        .map_err(|error| AuthorityError::new(format!("parse {}: {error}", path.display())))
}

fn validate_showcase_png(path: &Path, repository_id: &str, errors: &mut Vec<String>) {
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

fn check_nonempty(label: &str, value: &str, errors: &mut Vec<String>) {
    if value.trim().is_empty() {
        errors.push(format!("{label} is empty"));
    }
}

fn check_slug(label: &str, value: &str, errors: &mut Vec<String>) {
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

fn check_https_or_github(label: &str, value: &str, errors: &mut Vec<String>) {
    if !value.starts_with("https://") {
        errors.push(format!("{label} must use https: {value}"));
    }
}

fn check_date(label: &str, value: &str, errors: &mut Vec<String>) {
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

fn check_timestamp(label: &str, value: &str, errors: &mut Vec<String>) {
    let date = value.get(..10).unwrap_or_default();
    if value.len() < 20 || !value.ends_with('Z') {
        errors.push(format!("{label} is not a UTC timestamp: {value}"));
        return;
    }
    check_date(label, date, errors);
}

fn check_branch(id: &str, branch: &str, errors: &mut Vec<String>) {
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

fn check_head(id: &str, head: &str, errors: &mut Vec<String>) {
    if head.len() != 40 || !head.chars().all(|ch| ch.is_ascii_hexdigit()) {
        errors.push(format!("migration {id} head is not a full git object id"));
    }
}

fn check_safe_locator(id: &str, locator: &str, errors: &mut Vec<String>) {
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

fn check_receipt_path(path: &str, errors: &mut Vec<String>) {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn workspace_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
    }

    #[test]
    fn committed_authority_is_valid() {
        let authority = Authority::load(workspace_root()).expect("load authority files");
        if let Err(errors) = authority.validate() {
            panic!("authority validation failed:\n{}", errors.join("\n"));
        }
    }

    #[test]
    fn inventory_targets_never_contain_absolute_paths() {
        let authority = Authority::load(workspace_root()).expect("load authority files");
        for target in authority.inventory_targets() {
            if let Some(locator) = target.local_locator {
                assert!(!locator.contains(':'));
                assert!(!locator.contains('\\'));
                assert!(!locator.starts_with('/'));
            }
        }
    }

    #[test]
    fn live_github_roster_adds_fallbacks_and_removes_stale_relations() {
        let root = workspace_root();
        let authority = Authority::load(&root).expect("load authority files");
        let mut metadata = PublicMetadataCache::load(root.join(PUBLIC_METADATA_PATH))
            .expect("load public metadata");
        metadata
            .repository
            .retain(|repository| repository.id != "mere");
        metadata.repository.push(PublicRepositoryMetadata {
            id: "new-public-tool".to_owned(),
            github_slug: "merely-made/new-public-tool".to_owned(),
            name: "new-public-tool".to_owned(),
            description: "A newly discovered public tool.".to_owned(),
            homepage: None,
            license: Some("mit".to_owned()),
            updated_at: "2026-08-11T00:00:00Z".to_owned(),
            pushed_at: "2026-08-11T00:00:00Z".to_owned(),
            primary_language: Some("Rust".to_owned()),
            stargazer_count: 0,
            archived: false,
            fork: false,
            topics: Vec::new(),
        });

        let reconciled = authority.with_live_github_repositories(&metadata);
        let ids = reconciled
            .repositories
            .repository
            .iter()
            .map(|repository| repository.id.as_str())
            .collect::<BTreeSet<_>>();
        assert!(!ids.contains("mere"));
        assert!(ids.contains("new-public-tool"));
        let discovered = reconciled
            .repositories
            .repository
            .iter()
            .find(|repository| repository.id == "new-public-tool")
            .expect("new GitHub repository gets a fallback profile");
        assert_eq!(discovered.class, RepositoryClass::Tool);
        assert_eq!(discovered.summary, "A newly discovered public tool.");
        assert!(reconciled.relations.relation.iter().all(|relation| {
            ids.contains(relation.source.as_str()) && ids.contains(relation.target.as_str())
        }));
    }
}
