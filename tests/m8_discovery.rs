use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use mer3ly_site::discovery::{ROBOTS_TXT, canonical_urls, sitemap};
use mer3ly_site::pages::{devices, home, projects, radio, repositories};
use mer3ly_site::repositories::PublicSiteData;
use mer3ly_site::site::{
    DEFAULT_SOCIAL_IMAGE_ALT, DEFAULT_SOCIAL_IMAGE_URL, ORGANIZATION_ID, WEBSITE_ID,
};
use serde_json::Value;

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

fn json_ld(document: &str) -> Value {
    let marker = "<script type=\"application/ld+json\">";
    let start = document.find(marker).expect("JSON-LD script") + marker.len();
    let end = document[start..]
        .find("</script>")
        .expect("closed JSON-LD script")
        + start;
    serde_json::from_str(&document[start..end]).expect("valid JSON-LD")
}

fn graph(value: &Value) -> &[Value] {
    value
        .get("@graph")
        .and_then(Value::as_array)
        .expect("JSON-LD graph")
}

#[test]
fn sitemap_projects_exact_canonical_authority_without_fake_freshness() {
    let data = PublicSiteData::load(workspace_root()).expect("load validated public site data");
    let expected = canonical_urls(&data);
    let document = sitemap(&data);
    let actual = document
        .lines()
        .filter_map(|line| line.trim().strip_prefix("<url><loc>"))
        .filter_map(|line| line.strip_suffix("</loc></url>"))
        .collect::<Vec<_>>();

    assert_eq!(
        expected.len(),
        4 + data.authority.repositories.repository.len() + data.devices.ordered().len()
    );
    assert_eq!(actual, expected);
    assert_eq!(actual.iter().collect::<BTreeSet<_>>().len(), actual.len());
    assert!(
        actual
            .iter()
            .all(|url| url.starts_with("https://mer3ly.net/"))
    );
    for unsupported in ["lastmod", "changefreq", "priority"] {
        assert!(!document.contains(unsupported));
    }
}

#[test]
fn robots_policy_names_the_canonical_sitemap() {
    assert_eq!(
        ROBOTS_TXT,
        "User-agent: *\nAllow: /\nSitemap: https://mer3ly.net/sitemap.xml\n"
    );
}

#[test]
fn every_html_document_links_the_discovery_identity() {
    let data = PublicSiteData::load(workspace_root()).expect("load validated public site data");
    let mut documents = vec![
        home::document_for(&data),
        repositories::document_for(&data),
        radio::document(),
        devices::index_document_for(&data.devices),
    ];
    documents.extend(
        projects::documents(&data)
            .into_iter()
            .map(|(_, document)| document),
    );
    documents.extend(
        devices::documents(&data)
            .into_iter()
            .map(|(_, document)| document),
    );

    assert_eq!(
        documents.len(),
        4 + data.authority.repositories.repository.len() + data.devices.ordered().len()
    );
    for document in documents {
        assert_eq!(
            document
                .matches("<link rel=\"icon\" href=\"/favicon.svg\" type=\"image/svg+xml\">")
                .count(),
            1
        );
        assert_eq!(
            document
                .matches("<link rel=\"sitemap\" href=\"/sitemap.xml\" type=\"application/xml\" title=\"Sitemap\">")
                .count(),
            1
        );
        assert_eq!(
            document
                .matches("<script type=\"application/ld+json\">")
                .count(),
            1
        );
    }
}

#[test]
fn project_social_images_follow_showcase_evidence() {
    let data = PublicSiteData::load(workspace_root()).expect("load validated public site data");
    for repository in data
        .authority
        .repositories
        .repository
        .iter()
        .filter(|repository| repository.public)
    {
        let document = projects::document(&workspace_root(), &repository.id)
            .expect("render public project profile");
        let showcase = data.showcases.for_repository(&repository.id);
        let image_url = showcase.map_or(DEFAULT_SOCIAL_IMAGE_URL.to_owned(), |showcase| {
            format!("https://mer3ly.net/{}", showcase.image)
        });
        let image_type = if showcase.is_some() {
            "image/png"
        } else {
            "image/jpeg"
        };
        let image_alt = showcase.map_or(DEFAULT_SOCIAL_IMAGE_ALT, |showcase| showcase.alt.as_str());

        for needle in [
            format!("<meta property=\"og:image\" content=\"{image_url}\">"),
            format!("<meta property=\"og:image:type\" content=\"{image_type}\">"),
            format!("<meta property=\"og:image:alt\" content=\"{image_alt}\">"),
            format!("<meta name=\"twitter:image\" content=\"{image_url}\">"),
            format!("<meta name=\"twitter:image:alt\" content=\"{image_alt}\">"),
        ] {
            assert!(
                document.contains(&needle),
                "{} is missing {needle}",
                repository.id
            );
        }
    }
}

#[test]
fn project_json_ld_names_the_public_source_and_work_type() {
    let data = PublicSiteData::load(workspace_root()).expect("load validated public site data");
    for repository in data
        .authority
        .repositories
        .repository
        .iter()
        .filter(|repository| repository.public)
    {
        let document = projects::document(&workspace_root(), &repository.id)
            .expect("render public project profile");
        let value = json_ld(&document);
        let graph = graph(&value);
        let canonical = format!("https://mer3ly.net/projects/{}/", repository.id);
        let entity_id = format!("{canonical}#repository");
        let entity = graph
            .iter()
            .find(|node| node.get("@id").and_then(Value::as_str) == Some(entity_id.as_str()))
            .expect("repository entity");
        let page = graph
            .iter()
            .find(|node| node.get("@id").and_then(Value::as_str) == Some(canonical.as_str()))
            .expect("project WebPage");

        assert_eq!(
            page.pointer("/about/@id").and_then(Value::as_str),
            Some(entity_id.as_str())
        );
        assert_eq!(
            page.pointer("/isPartOf/@id").and_then(Value::as_str),
            Some(WEBSITE_ID)
        );
        assert!(
            graph
                .iter()
                .any(|node| { node.get("@id").and_then(Value::as_str) == Some(ORGANIZATION_ID) })
        );

        if repository.id == "org-profile" {
            assert_eq!(
                entity.get("@type").and_then(Value::as_str),
                Some("CreativeWork")
            );
            assert!(entity.get("codeRepository").is_none());
        } else {
            assert_eq!(
                entity.get("@type").and_then(Value::as_str),
                Some("SoftwareSourceCode")
            );
            assert_eq!(
                entity.get("codeRepository").and_then(Value::as_str),
                Some(format!("https://github.com/{}", repository.github_slug).as_str())
            );
        }
    }
}
