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
