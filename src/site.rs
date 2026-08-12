use std::cell::RefCell;
use std::rc::Rc;

use cambium::{AnyView, GenetAppRunner, GenetCtx, GenetElement, el, text};
use genet_scripted_dom::ScriptedDom;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

pub type SiteView = Box<dyn AnyView<(), (), GenetCtx, GenetElement>>;

pub const SITE_CSS: &str = include_str!("../assets/site.css");
pub const DEVICE_CSS: &str = include_str!("../assets/devices.css");

pub const ORGANIZATION_ID: &str = "https://mer3ly.net/#organization";
pub const WEBSITE_ID: &str = "https://mer3ly.net/#website";
pub const DEFAULT_SOCIAL_IMAGE_URL: &str = "https://mer3ly.net/og.jpg";
pub const DEFAULT_SOCIAL_IMAGE_ALT: &str =
    "Merely, software and hardware for people who are their own infrastructure.";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActivePage {
    Home,
    Devices,
    Radio,
    Repositories,
}

pub struct PageMetadata {
    pub title: &'static str,
    pub description: &'static str,
    pub canonical_url: &'static str,
}

pub struct SocialImage<'a> {
    pub url: &'a str,
    pub mime_type: &'a str,
    pub alt: &'a str,
}

pub struct DocumentMetadata<'a> {
    pub title: &'a str,
    pub description: &'a str,
    pub canonical_url: &'a str,
    pub social_image: SocialImage<'a>,
    pub json_ld: &'a str,
}

pub fn base_schema_graph() -> Vec<Value> {
    vec![
        json!({
            "@type": "Organization",
            "@id": ORGANIZATION_ID,
            "name": "Merely LLC",
            "url": "https://mer3ly.net/",
            "email": "markik@mer3ly.net",
            "sameAs": ["https://github.com/merely-made"],
            "address": {
                "@type": "PostalAddress",
                "addressLocality": "Ashland",
                "addressRegion": "KY",
                "addressCountry": "US"
            }
        }),
        json!({
            "@type": "WebSite",
            "@id": WEBSITE_ID,
            "name": "Merely",
            "url": "https://mer3ly.net/",
            "publisher": { "@id": ORGANIZATION_ID }
        }),
    ]
}

pub fn site_json_ld() -> String {
    json_ld_for_script(&json!({
        "@context": "https://schema.org",
        "@graph": base_schema_graph()
    }))
}

pub fn json_ld_for_script(value: &Value) -> String {
    serde_json::to_string_pretty(value)
        .expect("site structured data is serializable")
        .replace('&', "\\u0026")
        .replace('<', "\\u003c")
        .replace('>', "\\u003e")
}

pub fn txt(value: impl Into<String>) -> SiteView {
    Box::new(text(value))
}

pub fn element(tag: &str, attrs: &[(&str, &str)], children: Vec<SiteView>) -> SiteView {
    let mut node = el::<_, (), ()>(tag, children);
    for (name, value) in attrs {
        node = node.attr(*name, *value);
    }
    Box::new(node)
}

pub fn link(href: &str, label: impl Into<String>, class: &str) -> SiteView {
    element("a", &[("href", href), ("class", class)], vec![txt(label)])
}

pub fn external_link(href: &str, label: impl Into<String>, class: &str) -> SiteView {
    element(
        "a",
        &[("href", href), ("class", class), ("rel", "noreferrer")],
        vec![txt(label)],
    )
}

pub fn section_heading(number: &str, title: &str) -> SiteView {
    element(
        "div",
        &[("class", "section-heading")],
        vec![
            element(
                "span",
                &[("class", "section-number"), ("aria-hidden", "true")],
                vec![txt(number)],
            ),
            element("h2", &[], vec![txt(title)]),
        ],
    )
}

pub fn shell(active: ActivePage, main: SiteView) -> SiteView {
    let home_attrs = if active == ActivePage::Home {
        vec![
            ("href", "/"),
            ("aria-current", "page"),
            ("class", "nav-link is-current"),
        ]
    } else {
        vec![("href", "/"), ("class", "nav-link")]
    };
    let radio_attrs = if active == ActivePage::Radio {
        vec![
            ("href", "/radio.html"),
            ("aria-current", "page"),
            ("class", "nav-link is-current"),
        ]
    } else {
        vec![("href", "/radio.html"), ("class", "nav-link")]
    };
    let devices_attrs = if active == ActivePage::Devices {
        vec![
            ("href", "/devices/"),
            ("aria-current", "page"),
            ("class", "nav-link is-current"),
        ]
    } else {
        vec![("href", "/devices/"), ("class", "nav-link")]
    };
    let repositories_attrs = if active == ActivePage::Repositories {
        vec![
            ("href", "/repos/"),
            ("aria-current", "page"),
            ("class", "nav-link is-current"),
        ]
    } else {
        vec![("href", "/repos/"), ("class", "nav-link")]
    };

    let header = element(
        "header",
        &[("class", "site-header")],
        vec![
            element(
                "a",
                &[("href", "/"), ("class", "brand")],
                vec![
                    element("span", &[("class", "brand-name")], vec![txt("merely")]),
                    element(
                        "span",
                        &[("class", "brand-kind")],
                        vec![txt("software + hardware")],
                    ),
                ],
            ),
            element(
                "nav",
                &[("aria-label", "Main navigation")],
                vec![element(
                    "ul",
                    &[("class", "nav-list")],
                    vec![
                        element(
                            "li",
                            &[],
                            vec![element("a", &home_attrs, vec![txt("home")])],
                        ),
                        element(
                            "li",
                            &[],
                            vec![element("a", &repositories_attrs, vec![txt("repositories")])],
                        ),
                        element(
                            "li",
                            &[],
                            vec![element("a", &devices_attrs, vec![txt("devices")])],
                        ),
                        element(
                            "li",
                            &[],
                            vec![element("a", &radio_attrs, vec![txt("community radio")])],
                        ),
                        element(
                            "li",
                            &[],
                            vec![external_link(
                                "https://github.com/merely-made",
                                "github ↗",
                                "nav-link",
                            )],
                        ),
                    ],
                )],
            ),
        ],
    );

    let footer = element(
        "footer",
        &[("class", "site-footer")],
        vec![
            element(
                "p",
                &[],
                vec![txt("Merely LLC · Ashland, Kentucky · mer3ly.net")],
            ),
            element(
                "p",
                &[],
                vec![
                    link(
                        "mailto:markik@mer3ly.net",
                        "markik@mer3ly.net",
                        "footer-link",
                    ),
                    txt(" · "),
                    external_link(
                        "https://github.com/merely-made",
                        "public work on GitHub ↗",
                        "footer-link",
                    ),
                ],
            ),
            element(
                "p",
                &[("class", "footer-licenses")],
                vec![
                    external_link(
                        "https://www.mozilla.org/MPL/2.0/",
                        "source MPL-2.0 ↗",
                        "footer-link",
                    ),
                    txt(" · "),
                    external_link(
                        "https://creativecommons.org/licenses/by/4.0/",
                        "original content CC BY 4.0 ↗",
                        "footer-link",
                    ),
                ],
            ),
        ],
    );

    element(
        "body",
        &[("class", "site-body")],
        vec![
            link("#main", "Skip to content", "skip-link"),
            element(
                "div",
                &[("class", "page-shell")],
                vec![header, main, footer],
            ),
        ],
    )
}

pub fn render_with(metadata: &PageMetadata, view: impl Fn() -> SiteView) -> String {
    render_with_body_end(metadata, view, "")
}

pub fn render_with_stylesheet(
    metadata: &PageMetadata,
    view: impl Fn() -> SiteView,
    href: &str,
    stylesheet: &str,
) -> String {
    let json_ld = site_json_ld();
    let metadata = DocumentMetadata {
        title: metadata.title,
        description: metadata.description,
        canonical_url: metadata.canonical_url,
        social_image: SocialImage {
            url: DEFAULT_SOCIAL_IMAGE_URL,
            mime_type: "image/jpeg",
            alt: DEFAULT_SOCIAL_IMAGE_ALT,
        },
        json_ld: &json_ld,
    };
    render_body(&metadata, view, "", Some((href, stylesheet)))
}

pub fn render_with_dynamic(metadata: &DocumentMetadata<'_>, view: impl Fn() -> SiteView) -> String {
    render_body(metadata, view, "", None)
}

pub fn render_with_dynamic_and_body_end(
    metadata: &DocumentMetadata<'_>,
    view: impl Fn() -> SiteView,
    body_end: &str,
) -> String {
    render_body(metadata, view, body_end, None)
}

pub fn render_with_dynamic_stylesheet(
    metadata: &DocumentMetadata<'_>,
    view: impl Fn() -> SiteView,
    href: &str,
    stylesheet: &str,
) -> String {
    render_body(metadata, view, "", Some((href, stylesheet)))
}

pub fn render_with_dynamic_stylesheet_and_body_end(
    metadata: &DocumentMetadata<'_>,
    view: impl Fn() -> SiteView,
    href: &str,
    stylesheet: &str,
    body_end: &str,
) -> String {
    render_body(metadata, view, body_end, Some((href, stylesheet)))
}

pub fn render_with_body_end(
    metadata: &PageMetadata,
    view: impl Fn() -> SiteView,
    body_end: &str,
) -> String {
    let json_ld = site_json_ld();
    let metadata = DocumentMetadata {
        title: metadata.title,
        description: metadata.description,
        canonical_url: metadata.canonical_url,
        social_image: SocialImage {
            url: DEFAULT_SOCIAL_IMAGE_URL,
            mime_type: "image/jpeg",
            alt: DEFAULT_SOCIAL_IMAGE_ALT,
        },
        json_ld: &json_ld,
    };
    render_body(&metadata, view, body_end, None)
}

fn render_body(
    metadata: &DocumentMetadata<'_>,
    view: impl Fn() -> SiteView,
    body_end: &str,
    page_stylesheet: Option<(&str, &str)>,
) -> String {
    let dom = Rc::new(RefCell::new(ScriptedDom::new()));
    let runner = GenetAppRunner::<_, _, _, ()>::new(dom, move |_: &()| view(), ());
    let body_markup = format_html_fragment(&runner.dom().borrow().outer_html(runner.root()));
    let body_markup = if body_end.is_empty() {
        body_markup
    } else {
        let body_end = indent_lines(body_end, 1);
        body_markup.replacen("</body>\n", &format!("{body_end}\n</body>\n"), 1)
    };
    render_shell(metadata, &body_markup, page_stylesheet)
}

fn format_html_fragment(markup: &str) -> String {
    let mut output = String::with_capacity(markup.len() + markup.len() / 8);
    let mut depth = 0usize;
    let mut cursor = 0usize;

    while cursor < markup.len() {
        let Some(relative_tag_start) = markup[cursor..].find('<') else {
            write_text(&mut output, &markup[cursor..], depth);
            break;
        };
        let tag_start = cursor + relative_tag_start;
        write_text(&mut output, &markup[cursor..tag_start], depth);

        let Some(relative_tag_end) = markup[tag_start..].find('>') else {
            write_text(&mut output, &markup[tag_start..], depth);
            break;
        };
        let tag_end = tag_start + relative_tag_end + 1;
        let token = &markup[tag_start..tag_end];
        let tag = html_tag_name(token);
        let closing = token.starts_with("</");
        let block = is_block_tag(tag);
        let container = is_container_tag(tag);
        let void = is_void_tag(tag) || token.ends_with("/>");

        if block && closing {
            depth = depth.saturating_sub(1);
            if output.ends_with('\n') {
                write_indent(&mut output, depth);
            }
            output.push_str(token);
            output.push('\n');
        } else if block {
            if !output.is_empty() && !output.ends_with('\n') {
                output.push('\n');
            }
            write_indent(&mut output, depth);
            output.push_str(token);
            if !void {
                depth += 1;
            }
            if container || void {
                output.push('\n');
            }
        } else {
            if output.ends_with('\n') {
                write_indent(&mut output, depth);
            }
            output.push_str(token);
        }

        cursor = tag_end;
    }

    output
}

fn write_text(output: &mut String, text: &str, depth: usize) {
    if text.is_empty() {
        return;
    }
    if output.ends_with('\n') {
        write_indent(output, depth);
    }
    output.push_str(text);
}

fn write_indent(output: &mut String, depth: usize) {
    for _ in 0..depth {
        output.push_str("  ");
    }
}

fn indent_lines(value: &str, depth: usize) -> String {
    let indent = "  ".repeat(depth);
    value
        .lines()
        .map(|line| format!("{indent}{line}"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn html_tag_name(token: &str) -> &str {
    token
        .trim_start_matches('<')
        .trim_start_matches('/')
        .split(|character: char| character.is_ascii_whitespace() || matches!(character, '/' | '>'))
        .next()
        .unwrap_or_default()
}

fn is_block_tag(tag: &str) -> bool {
    matches!(
        tag,
        "address"
            | "article"
            | "aside"
            | "blockquote"
            | "body"
            | "canvas"
            | "caption"
            | "circle"
            | "dd"
            | "desc"
            | "div"
            | "dl"
            | "dt"
            | "fieldset"
            | "figcaption"
            | "figure"
            | "footer"
            | "g"
            | "h1"
            | "h2"
            | "h3"
            | "h4"
            | "h5"
            | "h6"
            | "header"
            | "hr"
            | "img"
            | "input"
            | "li"
            | "line"
            | "main"
            | "nav"
            | "ol"
            | "p"
            | "path"
            | "polygon"
            | "polyline"
            | "rect"
            | "section"
            | "svg"
            | "table"
            | "tbody"
            | "td"
            | "text"
            | "tfoot"
            | "th"
            | "thead"
            | "title"
            | "tr"
            | "ul"
    )
}

fn is_container_tag(tag: &str) -> bool {
    matches!(
        tag,
        "article"
            | "aside"
            | "body"
            | "div"
            | "dl"
            | "fieldset"
            | "figure"
            | "footer"
            | "g"
            | "header"
            | "main"
            | "nav"
            | "ol"
            | "section"
            | "svg"
            | "table"
            | "tbody"
            | "tfoot"
            | "thead"
            | "tr"
            | "ul"
    )
}

fn is_void_tag(tag: &str) -> bool {
    matches!(
        tag,
        "area"
            | "base"
            | "br"
            | "col"
            | "embed"
            | "hr"
            | "img"
            | "input"
            | "link"
            | "meta"
            | "param"
            | "source"
            | "track"
            | "wbr"
    )
}

fn render_shell(
    metadata: &DocumentMetadata<'_>,
    body_markup: &str,
    page_stylesheet: Option<(&str, &str)>,
) -> String {
    let stylesheet_href = stylesheet_href();
    let page_stylesheet_link = page_stylesheet.map_or_else(String::new, |(href, stylesheet)| {
        format!(
            "  <link rel=\"stylesheet\" href=\"{}\">\n",
            escape_attr(&content_addressed_href(href, stylesheet))
        )
    });
    format!(
        "<!doctype html>\n\
<html lang=\"en\">\n\
<head>\n\
  <meta charset=\"utf-8\">\n\
  <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n\
  <title>{title}</title>\n\
  <meta name=\"description\" content=\"{description}\">\n\
  <link rel=\"canonical\" href=\"{canonical}\">\n\
  <meta property=\"og:type\" content=\"website\">\n\
  <meta property=\"og:site_name\" content=\"Merely\">\n\
  <meta property=\"og:title\" content=\"{title}\">\n\
  <meta property=\"og:description\" content=\"{description}\">\n\
  <meta property=\"og:url\" content=\"{canonical}\">\n\
  <meta property=\"og:image\" content=\"{image_url}\">\n\
  <meta property=\"og:image:type\" content=\"{image_type}\">\n\
  <meta property=\"og:image:alt\" content=\"{image_alt}\">\n\
  <meta name=\"twitter:card\" content=\"summary_large_image\">\n\
  <meta name=\"twitter:title\" content=\"{title}\">\n\
  <meta name=\"twitter:description\" content=\"{description}\">\n\
  <meta name=\"twitter:image\" content=\"{image_url}\">\n\
  <meta name=\"twitter:image:alt\" content=\"{image_alt}\">\n\
  <meta name=\"theme-color\" content=\"#f0ebdd\">\n\
  <link rel=\"icon\" href=\"/favicon.svg\" type=\"image/svg+xml\">\n\
  <link rel=\"sitemap\" href=\"/sitemap.xml\" type=\"application/xml\" title=\"Sitemap\">\n\
  <link rel=\"preconnect\" href=\"https://fonts.googleapis.com\">\n\
  <link rel=\"preconnect\" href=\"https://fonts.gstatic.com\" crossorigin>\n\
  <link rel=\"stylesheet\" href=\"https://fonts.googleapis.com/css2?family=Young+Serif&family=Newsreader:ital,opsz,wght@0,6..72,400;0,6..72,500;0,6..72,600;1,6..72,400&family=IBM+Plex+Mono:wght@400;500;600&display=swap\">\n\
  <link rel=\"stylesheet\" href=\"{stylesheet_href}\">\n\
{page_stylesheet_link}\
  <script type=\"application/ld+json\">{json_ld}</script>\n\
</head>\n\
{body}\n\
</html>\n",
        title = escape_text(metadata.title),
        description = escape_attr(metadata.description),
        canonical = escape_attr(metadata.canonical_url),
        image_url = escape_attr(metadata.social_image.url),
        image_type = escape_attr(metadata.social_image.mime_type),
        image_alt = escape_attr(metadata.social_image.alt),
        stylesheet_href = escape_attr(&stylesheet_href),
        page_stylesheet_link = page_stylesheet_link,
        json_ld = metadata.json_ld,
        body = body_markup,
    )
}

fn stylesheet_href() -> String {
    content_addressed_href("/site.css", SITE_CSS)
}

fn content_addressed_href(path: &str, content: &str) -> String {
    let digest = format!("{:x}", Sha256::digest(content.as_bytes()));
    format!("{path}?v={}", &digest[..12])
}

fn escape_text(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn escape_attr(value: &str) -> String {
    escape_text(value).replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metadata_escaping_is_safe() {
        assert_eq!(escape_text("A & B < C"), "A &amp; B &lt; C");
        assert_eq!(escape_attr("\"quoted\""), "&quot;quoted&quot;");
    }

    #[test]
    fn structured_data_escaping_closes_no_script_element() {
        let value = json!({ "description": "</script><script>alert(1)</script>" });
        let encoded = json_ld_for_script(&value);
        assert!(!encoded.contains("</script>"));
        assert!(encoded.contains("\\u003c/script\\u003e"));
    }

    #[test]
    fn stylesheet_href_is_content_addressed() {
        let href = stylesheet_href();
        assert!(href.starts_with("/site.css?v="));
        assert_eq!(href.len(), "/site.css?v=".len() + 12);
    }

    #[test]
    fn html_formatter_puts_structural_elements_on_indented_lines() {
        let source = concat!(
            "<body class=\"site-body\"><div class=\"page-shell\">",
            "<p>Hello <a href=\"/\">reader</a>.</p>",
            "<svg><line x1=\"0\" x2=\"1\"></line><text>label</text></svg>",
            "</div></body>",
        );
        let formatted = format_html_fragment(source);

        assert_eq!(
            formatted,
            concat!(
                "<body class=\"site-body\">\n",
                "  <div class=\"page-shell\">\n",
                "    <p>Hello <a href=\"/\">reader</a>.</p>\n",
                "    <svg>\n",
                "      <line x1=\"0\" x2=\"1\"></line>\n",
                "      <text>label</text>\n",
                "    </svg>\n",
                "  </div>\n",
                "</body>\n",
            )
        );
    }
}
