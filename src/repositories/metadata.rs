use super::*;

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
        if self.event.len() > 12 {
            errors.push("public metadata has more than 12 organization events".to_owned());
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
