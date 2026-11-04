use super::*;

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

    pub(super) fn with_live_github_repositories(mut self, metadata: &PublicMetadataCache) -> Self {
        let (repositories, relations) =
            reconcile_live_github_repositories(&self.repositories, &self.relations, metadata);
        self.repositories = repositories;
        self.relations = relations;
        self
    }
}
