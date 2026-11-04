pub(super) fn format_html_fragment(markup: &str) -> String {
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

pub(super) fn write_text(output: &mut String, text: &str, depth: usize) {
    if text.is_empty() {
        return;
    }
    if output.ends_with('\n') {
        write_indent(output, depth);
    }
    output.push_str(text);
}

pub(super) fn write_indent(output: &mut String, depth: usize) {
    for _ in 0..depth {
        output.push_str("  ");
    }
}

pub(super) fn indent_lines(value: &str, depth: usize) -> String {
    let indent = "  ".repeat(depth);
    value
        .lines()
        .map(|line| format!("{indent}{line}"))
        .collect::<Vec<_>>()
        .join("\n")
}

pub(super) fn html_tag_name(token: &str) -> &str {
    token
        .trim_start_matches('<')
        .trim_start_matches('/')
        .split(|character: char| character.is_ascii_whitespace() || matches!(character, '/' | '>'))
        .next()
        .unwrap_or_default()
}

pub(super) fn is_block_tag(tag: &str) -> bool {
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

pub(super) fn is_container_tag(tag: &str) -> bool {
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

pub(super) fn is_void_tag(tag: &str) -> bool {
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
