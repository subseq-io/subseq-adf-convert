use htmd::{Element, HtmlToMarkdown};
use html5ever::serialize::{SerializeOpts, serialize};
use markdown::{CompileOptions, Options, ParseOptions, to_html_with_options as markdown_to_html};
use markup5ever_rcdom::{Handle, NodeData, SerializableHandle};

use crate::{
    adf::adf_types::AdfNode, adf_to_html::adf_to_html, html_sanitize::normalize_html,
    html_to_adf::html_to_adf,
};

pub(crate) fn table_handler(element: Element) -> Option<String> {
    let mut headers = vec![];
    let mut rows = vec![];
    let internal_converter = create_converter();

    for child in element.node.children.borrow().iter() {
        if let NodeData::Element { ref name, .. } = child.data {
            match name.local.as_ref() {
                "thead" => {
                    if let Some(row) = extract_table_body(child, &internal_converter).first() {
                        headers.extend(row.clone());
                    }
                }
                "tbody" => {
                    rows.extend(extract_table_body(child, &internal_converter));
                }
                "tr" => {
                    // Fallback if <tr> directly inside <table>
                    if headers.is_empty() {
                        headers.extend(extract_table_row(child, &internal_converter));
                    } else {
                        rows.push(extract_table_row(child, &internal_converter));
                    }
                }
                _ => {}
            }
        }
    }

    if headers.is_empty() && rows.is_empty() {
        return None;
    }

    let mut md = String::from("\n\n| ");
    md.push_str(
        &headers
            .iter()
            .map(|r| trim_newlines(r))
            .collect::<Vec<_>>()
            .join(" | "),
    );
    md.push_str(" |\n|");

    if headers.is_empty() {
        // If no headers, still put the header bar above the table
        for _ in &rows {
            md.push_str(" --- |");
        }
    } else {
        for _ in &headers {
            md.push_str(" --- |");
        }
    }
    md.push('\n');

    for row in rows {
        md.push_str("| ");
        md.push_str(
            &row.iter()
                .map(|r| trim_newlines(r))
                .collect::<Vec<_>>()
                .join(" | "),
        );
        md.push_str(" |\n");
    }

    md.push('\n');
    Some(md)
}

fn extract_table_body(node: &Handle, converter: &HtmlToMarkdown) -> Vec<Vec<String>> {
    node.children
        .borrow()
        .iter()
        .filter_map(|child| {
            if let NodeData::Element { ref name, .. } = child.data {
                if name.local.as_ref() == "tr" {
                    Some(extract_table_row(child, converter))
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect()
}

fn extract_table_row(node: &Handle, converter: &HtmlToMarkdown) -> Vec<String> {
    node.children
        .borrow()
        .iter()
        .filter_map(|child| {
            if let NodeData::Element { ref name, .. } = child.data {
                let tag = name.local.as_ref();
                if tag == "th" || tag == "td" {
                    let mut buf = Vec::new();
                    serialize(
                        &mut buf,
                        &SerializableHandle::from(child.clone()),
                        SerializeOpts::default(),
                    )
                    .ok()?;
                    let html_string = String::from_utf8(buf).ok()?;
                    Some(trim_newlines(
                        &converter
                            .convert(&html_string)
                            .unwrap_or_default()
                            .trim()
                            .to_string(),
                    ))
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect()
}

fn trim_newlines(text: &str) -> String {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

fn create_converter() -> HtmlToMarkdown {
    let converter = HtmlToMarkdown::builder()
        .add_handler(vec!["table"], table_handler)
        .add_handler(
            vec![
                "a",
                "img",
                "time",
                "input",
                "figure",
                "details",
                "summary",
                "adf-emoji",
                "adf-mention",
                "adf-status",
                "adf-media-single",
                "adf-media-group",
                "adf-decision-item",
                "adf-local-data",
            ],
            |element: Element| {
                let attrs = element
                    .attrs
                    .iter()
                    .map(|attr| format!("{}=\"{}\"", attr.name.local.as_ref(), attr.value))
                    .collect::<Vec<_>>()
                    .join(" ");
                Some(format!(
                    "<{0} {1}>{2}</{0}>",
                    element.tag, attrs, element.content
                ))
            },
        )
        .build();
    converter
}

pub fn html_to_markdown(html: String) -> String {
    let converter = create_converter();
    converter.convert(&html).unwrap_or_default()
}

pub fn adf_to_markdown(adf: &[AdfNode]) -> String {
    html_to_markdown(adf_to_html(adf.to_vec()))
}

pub fn markdown_to_adf(markdown: &str) -> Option<AdfNode> {
    let parse_options = ParseOptions::gfm();
    let options = Options {
        parse: parse_options,
        compile: CompileOptions {
            allow_any_img_src: true, // We're going round trip to ADF so we can allow this
            allow_dangerous_html: true,
            allow_dangerous_protocol: true,
            ..Default::default()
        },
    };
    let html = markdown_to_html(markdown, &options)
        .map_err(|err| {
            tracing::warn!("Failed to convert markdown to HTML: {}", err);
        })
        .unwrap_or_default();
    let sanitized = normalize_html(&html);
    eprintln!("HTML: {}", sanitized);
    Some(html_to_adf(&sanitized))
}
