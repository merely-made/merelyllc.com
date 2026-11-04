use super::*;

pub(super) fn validate_fixed_metadata(
    document: &str,
    canonical: &str,
    image_url: &str,
    image_type: &str,
    image_alt: &str,
    errors: &mut Vec<String>,
) {
    let expected = ExpectedSocialMetadata {
        canonical,
        image_url,
        image_type,
        image_alt,
    };
    validate_social_head(document, "fixed page", &expected, errors);
    let Some(value) = parse_json_ld(document, "fixed page", errors) else {
        return;
    };
    let Some(graph) = schema_graph(&value, "fixed page", errors) else {
        return;
    };
    validate_base_schema(graph, "fixed page", errors);
}

pub(super) fn validate_project_metadata(
    document: &str,
    repository: &RepositoryRecord,
    metadata: &PublicMetadataCache,
    expected: &ExpectedSocialMetadata<'_>,
    errors: &mut Vec<String>,
) -> (bool, bool) {
    let social_start = errors.len();
    validate_social_head(
        document,
        &format!("project profile {}", repository.id),
        expected,
        errors,
    );
    let social_valid = errors.len() == social_start;

    let structured_start = errors.len();
    let Some(value) = parse_json_ld(
        document,
        &format!("project profile {}", repository.id),
        errors,
    ) else {
        return (social_valid, false);
    };
    let Some(graph) = schema_graph(
        &value,
        &format!("project profile {}", repository.id),
        errors,
    ) else {
        return (social_valid, false);
    };
    validate_base_schema(graph, &format!("project profile {}", repository.id), errors);

    let entity_id = format!("{}#repository", expected.canonical);
    let page = graph
        .iter()
        .find(|node| node.get("@id").and_then(Value::as_str) == Some(expected.canonical));
    if page.is_none_or(|page| {
        page.get("@type").and_then(Value::as_str) != Some("WebPage")
            || page.pointer("/about/@id").and_then(Value::as_str) != Some(entity_id.as_str())
            || page.pointer("/isPartOf/@id").and_then(Value::as_str) != Some(WEBSITE_ID)
    }) {
        errors.push(format!(
            "project profile {} has invalid WebPage structured data",
            repository.id
        ));
    }

    let entity = graph
        .iter()
        .find(|node| node.get("@id").and_then(Value::as_str) == Some(entity_id.as_str()));
    let expected_type = if repository.id == "org-profile" {
        "CreativeWork"
    } else {
        "SoftwareSourceCode"
    };
    let repository_url = format!("https://github.com/{}", repository.github_slug);
    if entity.is_none_or(|entity| {
        entity.get("@type").and_then(Value::as_str) != Some(expected_type)
            || entity.get("url").and_then(Value::as_str) != Some(expected.canonical)
            || entity.pointer("/publisher/@id").and_then(Value::as_str) != Some(ORGANIZATION_ID)
            || entity
                .get("sameAs")
                .and_then(Value::as_array)
                .is_none_or(|same_as| {
                    same_as.len() != 1 || same_as[0].as_str() != Some(repository_url.as_str())
                })
    }) {
        errors.push(format!(
            "project profile {} has invalid repository structured data",
            repository.id
        ));
    }

    if expected_type == "SoftwareSourceCode" {
        if entity
            .and_then(|entity| entity.get("codeRepository"))
            .and_then(Value::as_str)
            != Some(repository_url.as_str())
        {
            errors.push(format!(
                "project profile {} has invalid source repository structured data",
                repository.id
            ));
        }
        let public_metadata = metadata
            .repository
            .iter()
            .find(|record| record.id == repository.id);
        let expected_language =
            public_metadata.and_then(|record| record.primary_language.as_deref());
        let actual_language = entity
            .and_then(|entity| entity.get("programmingLanguage"))
            .and_then(Value::as_str);
        if actual_language != expected_language {
            errors.push(format!(
                "project profile {} has invalid language structured data",
                repository.id
            ));
        }
        let expected_topics = public_metadata
            .map(|record| record.topics.as_slice())
            .unwrap_or_default();
        let actual_topics = entity
            .and_then(|entity| entity.get("keywords"))
            .and_then(Value::as_array);
        if expected_topics.is_empty() {
            if actual_topics.is_some() {
                errors.push(format!(
                    "project profile {} has unexpected topic structured data",
                    repository.id
                ));
            }
        } else if actual_topics.is_none_or(|topics| {
            topics.len() != expected_topics.len()
                || topics
                    .iter()
                    .zip(expected_topics)
                    .any(|(actual, expected)| actual.as_str() != Some(expected.as_str()))
        }) {
            errors.push(format!(
                "project profile {} has invalid topic structured data",
                repository.id
            ));
        }
    }

    (social_valid, errors.len() == structured_start)
}

pub(super) fn validate_device_metadata(
    document: &str,
    device: &DeviceRecord,
    expected: &ExpectedSocialMetadata<'_>,
    errors: &mut Vec<String>,
) -> bool {
    let start = errors.len();
    let label = format!("device profile {}", device.id);
    validate_social_head(document, &label, expected, errors);
    let Some(value) = parse_json_ld(document, &label, errors) else {
        return false;
    };
    let Some(graph) = schema_graph(&value, &label, errors) else {
        return false;
    };
    validate_base_schema(graph, &label, errors);

    let article_id = format!("{}#recipe", expected.canonical);
    let page = graph
        .iter()
        .find(|node| node.get("@id").and_then(Value::as_str) == Some(expected.canonical));
    if page.is_none_or(|page| {
        page.get("@type").and_then(Value::as_str) != Some("WebPage")
            || page.pointer("/about/@id").and_then(Value::as_str) != Some(article_id.as_str())
            || page.pointer("/isPartOf/@id").and_then(Value::as_str) != Some(WEBSITE_ID)
    }) {
        errors.push(format!("{label} has invalid WebPage structured data"));
    }

    let evidence_url = format!(
        "{}/blob/main/{}",
        device.source_repository, device.source_document
    );
    let article = graph
        .iter()
        .find(|node| node.get("@id").and_then(Value::as_str) == Some(article_id.as_str()));
    if article.is_none_or(|article| {
        article.get("@type").and_then(Value::as_str) != Some("TechArticle")
            || article.get("url").and_then(Value::as_str) != Some(expected.canonical)
            || article.pointer("/publisher/@id").and_then(Value::as_str) != Some(ORGANIZATION_ID)
            || article.get("isBasedOn").and_then(Value::as_str) != Some(evidence_url.as_str())
    }) {
        errors.push(format!("{label} has invalid TechArticle structured data"));
    }
    if device.status != DeviceStatus::Sellable
        && graph.iter().any(|node| node.get("offers").is_some())
    {
        errors.push(format!(
            "{label} publishes an Offer for a non-sellable device"
        ));
    }

    errors.len() == start
}

pub(super) fn validate_social_head(
    document: &str,
    label: &str,
    expected: &ExpectedSocialMetadata<'_>,
    errors: &mut Vec<String>,
) {
    for (name, needle) in [
        (
            "canonical URL",
            format!(
                "<link rel=\"canonical\" href=\"{}\">",
                escape_html_attr(expected.canonical)
            ),
        ),
        (
            "Open Graph type",
            "<meta property=\"og:type\" content=\"website\">".to_owned(),
        ),
        (
            "Open Graph site name",
            "<meta property=\"og:site_name\" content=\"Merely\">".to_owned(),
        ),
        (
            "Open Graph URL",
            format!(
                "<meta property=\"og:url\" content=\"{}\">",
                escape_html_attr(expected.canonical)
            ),
        ),
        (
            "Open Graph image",
            format!(
                "<meta property=\"og:image\" content=\"{}\">",
                escape_html_attr(expected.image_url)
            ),
        ),
        (
            "Open Graph image type",
            format!(
                "<meta property=\"og:image:type\" content=\"{}\">",
                escape_html_attr(expected.image_type)
            ),
        ),
        (
            "Open Graph image alt",
            format!(
                "<meta property=\"og:image:alt\" content=\"{}\">",
                escape_html_attr(expected.image_alt)
            ),
        ),
        (
            "Twitter card",
            "<meta name=\"twitter:card\" content=\"summary_large_image\">".to_owned(),
        ),
        (
            "Twitter image",
            format!(
                "<meta name=\"twitter:image\" content=\"{}\">",
                escape_html_attr(expected.image_url)
            ),
        ),
        (
            "Twitter image alt",
            format!(
                "<meta name=\"twitter:image:alt\" content=\"{}\">",
                escape_html_attr(expected.image_alt)
            ),
        ),
        (
            "favicon",
            "<link rel=\"icon\" href=\"/favicon.svg\" type=\"image/svg+xml\">".to_owned(),
        ),
        (
            "sitemap link",
            "<link rel=\"sitemap\" href=\"/sitemap.xml\" type=\"application/xml\" title=\"Sitemap\">"
                .to_owned(),
        ),
    ] {
        if document.matches(&needle).count() != 1 {
            errors.push(format!("{label} has invalid {name} metadata"));
        }
    }

    for (name, first, second) in [
        (
            "title",
            "<meta property=\"og:title\" content=\"",
            "<meta name=\"twitter:title\" content=\"",
        ),
        (
            "description",
            "<meta property=\"og:description\" content=\"",
            "<meta name=\"twitter:description\" content=\"",
        ),
    ] {
        let first_value = quoted_attribute_value(document, first);
        let second_value = quoted_attribute_value(document, second);
        if first_value.is_none()
            || first_value.is_some_and(str::is_empty)
            || first_value != second_value
            || document.matches(first).count() != 1
            || document.matches(second).count() != 1
        {
            errors.push(format!("{label} has invalid social {name} metadata"));
        }
    }
}

pub(super) fn quoted_attribute_value<'a>(document: &'a str, marker: &str) -> Option<&'a str> {
    let start = document.find(marker)? + marker.len();
    let end = document[start..].find('"')? + start;
    Some(&document[start..end])
}

pub(super) fn escape_html_attr(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

pub(super) fn parse_json_ld(
    document: &str,
    label: &str,
    errors: &mut Vec<String>,
) -> Option<Value> {
    let marker = "<script type=\"application/ld+json\">";
    if document.matches(marker).count() != 1 {
        errors.push(format!("{label} does not have exactly one JSON-LD record"));
        return None;
    }
    let start = document.find(marker)? + marker.len();
    let Some(end) = document[start..]
        .find("</script>")
        .map(|offset| start + offset)
    else {
        errors.push(format!("{label} JSON-LD record is not terminated"));
        return None;
    };
    match serde_json::from_str(&document[start..end]) {
        Ok(value) => Some(value),
        Err(_) => {
            errors.push(format!("{label} JSON-LD record is invalid"));
            None
        }
    }
}

pub(super) fn schema_graph<'a>(
    value: &'a Value,
    label: &str,
    errors: &mut Vec<String>,
) -> Option<&'a [Value]> {
    if value.get("@context").and_then(Value::as_str) != Some("https://schema.org") {
        errors.push(format!("{label} has invalid JSON-LD context"));
    }
    let Some(graph) = value.get("@graph").and_then(Value::as_array) else {
        errors.push(format!("{label} has no JSON-LD graph"));
        return None;
    };
    Some(graph)
}

pub(super) fn validate_base_schema(graph: &[Value], label: &str, errors: &mut Vec<String>) {
    let organization = graph
        .iter()
        .find(|node| node.get("@id").and_then(Value::as_str) == Some(ORGANIZATION_ID));
    if organization.is_none_or(|node| {
        node.get("@type").and_then(Value::as_str) != Some("Organization")
            || node.get("name").and_then(Value::as_str) != Some("Merely LLC")
    }) {
        errors.push(format!("{label} has invalid organization structured data"));
    }
    let website = graph
        .iter()
        .find(|node| node.get("@id").and_then(Value::as_str) == Some(WEBSITE_ID));
    if website.is_none_or(|node| {
        node.get("@type").and_then(Value::as_str) != Some("WebSite")
            || node.pointer("/publisher/@id").and_then(Value::as_str) != Some(ORGANIZATION_ID)
    }) {
        errors.push(format!("{label} has invalid website structured data"));
    }
}

pub(super) fn validate_sitemap(
    sitemap: &str,
    expected: &[String],
    errors: &mut Vec<String>,
) -> usize {
    if !sitemap.starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n")
        || !sitemap.contains("<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">")
        || !sitemap.ends_with("</urlset>\n")
    {
        errors.push("sitemap does not use the approved XML envelope".to_owned());
    }
    for unsupported in ["<lastmod>", "<changefreq>", "<priority>"] {
        if sitemap.contains(unsupported) {
            errors.push("sitemap contains unsupported freshness metadata".to_owned());
            break;
        }
    }

    let loc = Regex::new(r"<loc>([^<]+)</loc>").expect("valid sitemap location regex");
    let actual = loc
        .captures_iter(sitemap)
        .filter_map(|capture| capture.get(1).map(|value| value.as_str().to_owned()))
        .collect::<Vec<_>>();
    let unique = actual.iter().collect::<BTreeSet<_>>();
    if actual != expected
        || unique.len() != actual.len()
        || actual
            .iter()
            .any(|url| !url.starts_with("https://merelyllc.com/"))
    {
        errors.push("sitemap URLs differ from canonical public authority".to_owned());
    }
    actual.len()
}
