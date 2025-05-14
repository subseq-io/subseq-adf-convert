use htmd::{Element, HtmlToMarkdown};
use markdown::{CompileOptions, Options, ParseOptions, to_html_with_options as markdown_to_html};
use markup5ever_rcdom::{Handle, NodeData};

use crate::{
    adf::adf_types::AdfNode,
    adf_to_html::adf_to_html,
    html_to_adf::html_to_adf,
};

pub(crate) fn table_handler(element: Element) -> Option<String> {
    let mut headers = vec![];
    let mut rows = vec![];

    for child in element.node.children.borrow().iter() {
        if let NodeData::Element { ref name, .. } = child.data {
            match name.local.as_ref() {
                "thead" => {
                    if let Some(row) = extract_table_body(child).first() {
                        headers.extend(row.clone());
                    }
                }
                "tbody" => {
                    rows.extend(extract_table_body(child));
                }
                "tr" => {
                    // Fallback if <tr> directly inside <table>
                    if headers.is_empty() {
                        headers.extend(extract_table_row(child));
                    } else {
                        rows.push(extract_table_row(child));
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
    md.push_str(&headers.join(" | "));
    md.push_str(" |\n|");
    for _ in &headers {
        md.push_str(" --- |");
    }
    md.push('\n');

    for row in rows {
        md.push_str("| ");
        md.push_str(&row.join(" | "));
        md.push_str(" |\n");
    }

    md.push('\n');
    Some(md)
}

fn extract_table_body(node: &Handle) -> Vec<Vec<String>> {
    node.children
        .borrow()
        .iter()
        .filter_map(|child| {
            if let NodeData::Element { ref name, .. } = child.data {
                if name.local.as_ref() == "tr" {
                    Some(extract_table_row(child))
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect()
}

fn extract_table_row(node: &Handle) -> Vec<String> {
    node.children
        .borrow()
        .iter()
        .filter_map(|child| {
            if let NodeData::Element { ref name, .. } = child.data {
                let tag = name.local.as_ref();
                if tag == "th" || tag == "td" {
                    Some(get_node_content(child).trim().to_string())
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect()
}

fn get_node_content(node: &Handle) -> String {
    let mut content = String::new();
    for child in node.children.borrow().iter() {
        match &child.data {
            NodeData::Text { contents } => {
                content.push_str(&contents.borrow());
            }
            NodeData::Element { .. } => {
                content.push_str(&get_node_content(child));
            }
            _ => {}
        }
    }
    content
}

pub fn html_to_markdown(html: String) -> String {
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
    Some(html_to_adf(&html))
}
