use htmd::{Element, HtmlToMarkdown};
use html5ever::serialize::{SerializeOpts, serialize};
use markdown::{CompileOptions, Options, ParseOptions, to_html_with_options as markdown_to_html};
use markup5ever_rcdom::{Handle, NodeData, SerializableHandle};

use crate::{
    adf::adf_types::AdfBlockNode, adf_to_html::adf_to_html, html_sanitize::normalize_html,
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
                "span",
                "del",
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
                "adf-task-item",
                "adf-local-data",
                "adf-block-card",
                "adf-block-card-data-source",
                "adf-block-card-view",
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

pub fn adf_to_markdown(adf: &[AdfBlockNode], buf: &str) -> String {
    html_to_markdown(adf_to_html(adf.to_vec(), buf))
}

pub fn markdown_to_adf(markdown: &str) -> Option<AdfBlockNode> {
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
    Some(html_to_adf(&sanitized))
}

#[cfg(feature = "fuzzing")]
#[cfg(test)]
mod tests {
    use crate::adf::adf_types::*;
    use crate::markdown::{adf_to_markdown, markdown_to_adf};
    use rand::Rng;
    use rand::prelude::IndexedRandom;

    fn insert_into_markdown(markdown: &mut String, position: usize, insert: &str) {
        let pos = position.min(markdown.len());
        markdown.insert_str(pos, insert);
    }

    fn remove_from_markdown(markdown: &mut String, position: usize, amount: usize) {
        if position >= markdown.len() {
            return;
        }
        let end = (position + amount).min(markdown.len());
        markdown.replace_range(position..end, "");
    }

    fn sample_adf_doc() -> AdfNode {
        AdfNode::Doc {
            content: vec![
                AdfNode::Heading {
                    attrs: HeadingAttrs { level: 2 },
                    content: Some(vec![AdfNode::Text {
                        text: "User document".into(),
                        marks: None,
                    }]),
                },
                AdfNode::Paragraph {
                    content: Some(vec![
                        AdfNode::Text {
                            text: "Hello ".into(),
                            marks: None,
                        },
                        AdfNode::Emoji {
                            attrs: EmojiAttrs {
                                short_name: ":smile:".into(),
                                text: Some("😄".into()),
                            },
                        },
                        AdfNode::Mention {
                            attrs: MentionAttrs {
                                id: "user-1".into(),
                                text: Some("User".into()),
                                access_level: None,
                                user_type: None,
                            },
                        },
                        AdfNode::Text {
                            text: " and welcome!".into(),
                            marks: None,
                        },
                    ]),
                },
                AdfNode::Status {
                    attrs: StatusAttrs {
                        text: "In Progress".into(),
                        color: "blue".into(),
                        local_id: Some("status-1".into()),
                    },
                },
            ],
            version: 1,
        }
    }

    #[test]
    fn fuzz_user_editing_markdown_pipeline() {
        let adf = sample_adf_doc();
        let mut markdown = adf_to_markdown(&[adf], "");

        let inserts = [
            " Hello world! ",
            "\n## Inserted heading\n",
            "<adf-emoji aria-alt=\":tada:\">🎉</adf-emoji>",
            "**bold text**",
            "`inline code`",
            "\n- new bullet\n",
        ];

        let mut rng = rand::rng();

        for _ in 0..100 {
            let choice = rng.random_range(0..3);
            match choice {
                0 => {
                    // Insert at random position
                    let insert = inserts.choose(&mut rng).unwrap();
                    let pos = rng.random_range(0..=markdown.len());
                    insert_into_markdown(&mut markdown, pos, insert);
                }
                1 => {
                    // Remove at random position
                    let pos = rng.random_range(0..markdown.len().saturating_sub(1));
                    let len = rng.random_range(1..=10).min(markdown.len() - pos);
                    remove_from_markdown(&mut markdown, pos, len);
                }
                _ => {
                    // Do nothing (simulate pause)
                }
            }

            // Ensure it does not panic and produces some ADF
            let adf = markdown_to_adf(&markdown);
            assert!(
                matches!(adf, Some(AdfNode::Doc { .. })),
                "ADF not a document after edit cycle"
            );
        }
    }
}
