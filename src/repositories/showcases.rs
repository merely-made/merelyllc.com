use super::*;

impl ShowcaseManifest {
    pub(super) fn retain_live_repositories(&mut self, authority: &Authority) {
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

            for (index, extra) in showcase.images.iter().enumerate() {
                let position = index + 2;
                check_nonempty(
                    &format!("showcase {} image {position} alt", showcase.repository),
                    &extra.alt,
                    &mut errors,
                );
                check_nonempty(
                    &format!("showcase {} image {position} caption", showcase.repository),
                    &extra.caption,
                    &mut errors,
                );
                if !seen_images.insert(extra.image.as_str()) {
                    errors.push(format!(
                        "showcase {} repeats an image path",
                        showcase.repository
                    ));
                }
                let expected_extra = format!("showcase/{}-{position}.png", showcase.repository);
                if extra.image != expected_extra
                    || extra.image.contains('\\')
                    || extra
                        .image
                        .split('/')
                        .any(|segment| segment.is_empty() || segment == "." || segment == "..")
                {
                    errors.push(format!(
                        "showcase {} image {position} does not use its approved normalized path",
                        showcase.repository
                    ));
                }
                if extra.source_license != repository.license {
                    errors.push(format!(
                        "showcase {} image {position} source license disagrees with repository authority",
                        showcase.repository
                    ));
                }
                if !extra.source_url.starts_with(&expected_source_prefix) {
                    errors.push(format!(
                        "showcase {} image {position} source URL does not belong to its repository",
                        showcase.repository
                    ));
                }
                validate_showcase_png(
                    &root.join("assets").join(&extra.image),
                    &showcase.repository,
                    &mut errors,
                );
            }
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
