use std::borrow::Cow;
use std::fmt::Write;

use htmd::{Element, HtmlToMarkdown};
use html_builder::*;
use markdown::{CompileOptions, Options, ParseOptions, to_html_with_options as markdown_to_html};

use crate::{
    adf::adf_types::{AdfMark, AdfNode, MediaMark, MediaNode, Subsup, TaskItemState},
    html::parse_html,
};

pub fn close(node: Void) {
    node.attr("/");
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
    eprintln!("\n\nHTML:\n{}\n\n", html);
    Some(html_to_adf(html))
}

pub fn html_to_adf(html: String) -> AdfNode {
    parse_html(&html)
}

pub fn adf_to_html(adf: Vec<AdfNode>) -> String {
    let mut buffer = Buffer::new();
    let node = buffer.body();
    inner_adf_to_html(node, adf);
    buffer.finish()
}

fn media_adf_to_html(mut node: Node, media: Vec<MediaNode>) {
    for media_node in media {
        let link = media_node.marks.iter().find_map(|mark| match mark {
            MediaMark::Link(link) => Some(link),
            _ => None,
        });

        if let Some(link) = link {
            match media_node.attrs.type_.as_str() {
                "file" => {
                    let mut attrs = vec![];
                    attrs.push(format!("src=\"{}\"", link.href));

                    if let Some(alt) = &media_node.attrs.alt {
                        attrs.push(format!("alt=\"{}\"", alt));
                    }

                    let mut styles = vec![];
                    if let Some(width) = media_node.attrs.width {
                        styles.push(format!("width: {}px", width));
                    }
                    if let Some(height) = media_node.attrs.height {
                        styles.push(format!("height: {}px", height));
                    }
                    if !styles.is_empty() {
                        attrs.push(format!("style=\"{}\"", &styles.join("; ")));
                    }
                    let attrs_str = attrs
                        .iter()
                        .map(|a| a.as_str())
                        .collect::<Vec<_>>()
                        .join(" ");
                    close(node.img().attr(&attrs_str));
                }
                "link" => {
                    let mut a = node.a().attr(&format!("href=\"{}\"", link.href));
                    if let Some(title) = link.title.as_ref() {
                        writeln!(a, "{}", title).ok();
                    } else {
                        writeln!(a, "{}", link.href).ok();
                    }
                }
                other => {
                    tracing::warn!("Unknown media type: {}", other);
                }
            }
        } else {
            tracing::warn!(
                "Media node of type {} missing Link mark, skipping",
                media_node.attrs.type_
            );
        }
    }
}

fn inner_adf_to_html(mut node: Node, adf: Vec<AdfNode>) {
    for adf_node in adf {
        match adf_node {
            AdfNode::Blockquote { content } => {
                let blockquote = node.blockquote();
                inner_adf_to_html(blockquote, content);
            }
            AdfNode::BulletList { content } => {
                let list = node.ul();
                inner_adf_to_html(list, content);
            }
            AdfNode::CodeBlock { attrs, content } => {
                let mut pre = node.pre();
                let mut code_block = pre.code();
                if let Some(attrs) = &attrs {
                    if let Some(language) = &attrs.language {
                        code_block = code_block.attr(&format!("class=\"language-{}\"", language));
                    }
                }
                if let Some(content) = content {
                    inner_adf_to_html(code_block, content);
                }
            }
            AdfNode::Date { attrs } => {
                let mut date = node.child(Cow::Borrowed("adf-date"));
                let ts = attrs.timestamp.parse::<i64>().unwrap_or_default();
                let date_str = chrono::DateTime::from_timestamp(ts, 0)
                    .unwrap_or_default()
                    .to_rfc3339();
                writeln!(date, "{}", date_str).ok();
            }
            AdfNode::Doc { content, .. } => {
                let doc = node.div();
                inner_adf_to_html(doc, content);
            }
            AdfNode::Emoji { attrs } => {
                let mut emoji = node
                    .child(Cow::Borrowed("adf-emoji"))
                    .attr(&format!("aria-alt=\"{}\"", attrs.short_name));
                if let Some(text) = &attrs.text {
                    writeln!(emoji, "{}", text).ok();
                } else {
                    writeln!(emoji, "{}", attrs.short_name).ok();
                }
            }
            AdfNode::Expand { content, .. } => {
                let expand = node.details();
                inner_adf_to_html(expand, content);
            }
            AdfNode::HardBreak => close(node.br()),
            AdfNode::Heading { attrs, content } => {
                let heading = match attrs.level {
                    1 => node.h1(),
                    2 => node.h2(),
                    3 => node.h3(),
                    4 => node.h4(),
                    5 => node.h5(),
                    6 => node.h6(),
                    _ => node.h6(),
                };
                if let Some(content) = content {
                    inner_adf_to_html(heading, content);
                }
            }
            AdfNode::InlineCard { attrs } => {
                if let Some(url) = &attrs.url {
                    let mut a_tag = node.a().attr(&format!("href={}", url));
                    writeln!(a_tag, "{}", url).ok();
                }
            }
            AdfNode::ListItem { content } => {
                let list_item = node.li();
                inner_adf_to_html(list_item, content);
            }
            AdfNode::MediaGroup { content } => {
                let media_group = node.child(Cow::Borrowed("adf-media-group"));
                media_adf_to_html(media_group, content);
            }
            AdfNode::MediaSingle { content, attrs } => {
                let mut media_single = node.child(Cow::Borrowed("adf-media-single"));
                if let Some(layout) = attrs.as_ref().map(|a| a.layout.as_ref()).flatten() {
                    media_single = media_single.attr(&format!("data-layout=\"{}\"", layout));
                }
                media_adf_to_html(media_single, content);
            }
            AdfNode::Mention { attrs } => {
                let mut mention = node.span();
                if let Some(text) = &attrs.text {
                    writeln!(mention, "@{}", text).ok();
                } else {
                    writeln!(mention, "@{}", attrs.id).ok();
                }
            }
            AdfNode::NestedExpand { content, .. } => {
                let expand = node.details();
                inner_adf_to_html(expand, content);
            }
            AdfNode::OrderedList { content, .. } => {
                let list = node.ol();
                inner_adf_to_html(list, content);
            }
            AdfNode::Panel { content, .. } => {
                let panel = node.div();
                inner_adf_to_html(panel, content);
            }
            AdfNode::Paragraph { content } => {
                let para = node.p();
                if let Some(content) = content {
                    inner_adf_to_html(para, content);
                }
            }
            AdfNode::Rule => {
                node.hr();
            }
            AdfNode::Status { attrs } => {
                let mut status = node.child(Cow::Borrowed("adf-status")).attr(&format!(
                    "style=\"background-color: {}\" aria-label=\"{}\"",
                    attrs.color,
                    attrs.local_id.unwrap_or_default()
                ));
                writeln!(status, "{}", attrs.text).ok();
            }
            AdfNode::Table { content, .. } => {
                let table = node.table();
                inner_adf_to_html(table, content);
            }
            AdfNode::TableCell { content, .. } => {
                let cell = node.td();
                inner_adf_to_html(cell, content);
            }
            AdfNode::TableHeader { content, .. } => {
                let header = node.tr();
                inner_adf_to_html(header, content);
            }
            AdfNode::TableRow { content, .. } => {
                let row = node.tr();
                inner_adf_to_html(row, content);
            }
            AdfNode::Text { text, marks } => {
                fn apply_marks(node: &mut Node, marks: &[AdfMark], text: &str) -> std::fmt::Result {
                    if let Some((first, rest)) = marks.split_first() {
                        let mut wrapped_node = match first {
                            AdfMark::Strong => node.strong(),
                            AdfMark::Em => node.em(),
                            AdfMark::Code => node.code(),
                            AdfMark::Link(mark) => node.a().attr(&format!("href={}", mark.href)),
                            AdfMark::Strike => node.del(),
                            AdfMark::Subsup { type_ } => match type_ {
                                Subsup::Sup => node.sup(),
                                Subsup::Sub => node.sub(),
                            },
                            AdfMark::Underline => {
                                node.div().attr("style=text-decoration:underline")
                            }
                            AdfMark::TextColor { .. } => node.div(), // ignored
                            AdfMark::BackgroundColor { .. } => node.div(), // ignored
                        };
                        apply_marks(&mut wrapped_node, rest, text)
                    } else {
                        writeln!(node, "{}", text)
                    }
                }
                apply_marks(&mut node, &marks.unwrap_or_default(), &text).ok();
            }
            AdfNode::TaskList { content, attrs } => {
                let task_list = node.ul().attr(&format!("id=\"{}\"", attrs.local_id));
                inner_adf_to_html(task_list, content);
            }
            AdfNode::TaskItem { attrs, content } => {
                let mut task_item = node.li();
                let checked = if attrs.state == TaskItemState::Done {
                    "checked"
                } else {
                    ""
                };
                let local_id = attrs.local_id;
                close(
                    task_item
                        .input()
                        .attr(&format!("id=\"{}\" type=checkbox {}", local_id, checked)),
                );
                inner_adf_to_html(task_item, content);
            }
            AdfNode::DecisionList { content, attrs } => {
                let decision = node
                    .child(Cow::Borrowed("adf-decision-list"))
                    .attr(&format!("id=\"{}\"", attrs.local_id));
                inner_adf_to_html(decision, content);
            }
            AdfNode::DecisionItem { content, attrs } => {
                let attrs = vec![
                    format!("id=\"{}\"", attrs.local_id),
                    format!("data-state=\"{}\"", attrs.state),
                ];
                let attrs_str = attrs.join(" ");
                let child = node
                    .child(Cow::Borrowed("adf-decision-item"))
                    .attr(&attrs_str);
                inner_adf_to_html(child, content);
            }
            AdfNode::Unknown => {
                // Ignore unknown nodes
                panic!("Unknown node type");
            }
        }
    }
}

pub fn html_to_markdown(html: String) -> String {
    let converter = HtmlToMarkdown::builder()
        //.add_handler(vec!["input"], |element: Element| {
        //    let checked = element
        //        .attrs
        //        .iter()
        //        .find(|attr| attr.name.local.as_ref() == "checked")
        //        .is_some();
        //    if checked {
        //        Some("[x]".to_string())
        //    } else {
        //        Some("[ ]".to_string())
        //    }
        //})
        .add_handler(
            vec![
                "input",
                "adf-emoji",
                "adf-status",
                "adf-media-single",
                "adf-media-group",
                "adf-decision-item",
                "adf-decision-list",
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adf::adf_types::*;

    fn roundtrip_adf_html_adf(adf: AdfNode) {
        let html = adf_to_html(vec![adf.clone()]);
        eprintln!("\n\nHTML:\n{}\n\n", html);
        let back = html_to_adf(html);
        assert_eq!(back, adf, "Failed roundtrip adf -> html -> adf");
    }

    fn roundtrip_adf_html_md_html_adf(adf: AdfNode) {
        let markdown = adf_to_markdown(&[adf.clone()]);
        eprintln!("\n\nMARKDOWN:\n{}\n\n", markdown);
        let back = markdown_to_adf(&markdown).unwrap();
        assert_eq!(
            back, adf,
            "Failed roundtrip adf -> html -> md -> html -> adf"
        );
    }

    #[test]
    fn test_paragraph_roundtrip() {
        let adf = AdfNode::Doc {
            content: vec![AdfNode::Paragraph {
                content: Some(vec![AdfNode::Text {
                    text: "Simple text".into(),
                    marks: None,
                }]),
            }],
            version: 1,
        };
        roundtrip_adf_html_adf(adf.clone());
        roundtrip_adf_html_md_html_adf(adf);
    }

    #[test]
    fn test_heading_roundtrip() {
        let adf = AdfNode::Doc {
            content: vec![AdfNode::Heading {
                attrs: HeadingAttrs { level: 2 },
                content: Some(vec![AdfNode::Text {
                    text: "Heading level 2".into(),
                    marks: None,
                }]),
            }],
            version: 1,
        };
        roundtrip_adf_html_adf(adf.clone());
        roundtrip_adf_html_md_html_adf(adf);
    }

    #[test]
    fn test_panel_roundtrip() {
        let adf = AdfNode::Doc {
            content: vec![AdfNode::Panel {
                attrs: PanelAttrs {
                    panel_type: "info".into(),
                },
                content: vec![AdfNode::Paragraph {
                    content: Some(vec![AdfNode::Text {
                        text: "Inside panel".into(),
                        marks: None,
                    }]),
                }],
            }],
            version: 1,
        };
        roundtrip_adf_html_adf(adf.clone());
        roundtrip_adf_html_md_html_adf(adf);
    }

    #[test]
    fn test_media_group_roundtrip() {
        let adf = AdfNode::Doc {
            content: vec![AdfNode::MediaGroup {
                content: vec![MediaNode {
                    media_type: "image".into(),
                    attrs: MediaAttrs {
                        alt: None,
                        height: None,
                        width: None,
                        id: "media-id".into(),
                        collection: "collection".into(),
                        type_: "file".into(),
                    },
                    marks: vec![MediaMark::Link(LinkMark {
                        href: "https://example.com".into(),
                        ..Default::default()
                    })],
                }],
            }],
            version: 1,
        };
        roundtrip_adf_html_adf(adf.clone());
        roundtrip_adf_html_md_html_adf(adf);
    }

    #[test]
    fn test_task_list_roundtrip() {
        let adf = AdfNode::Doc {
            content: vec![AdfNode::TaskList {
                attrs: LocalId {
                    local_id: "list-1".into(),
                },
                content: vec![
                    AdfNode::TaskItem {
                        attrs: TaskItemAttrs {
                            local_id: "item-1".into(),
                            state: TaskItemState::Todo,
                        },
                        content: vec![AdfNode::Text {
                            text: "Task item".into(),
                            marks: None,
                        }],
                    },
                    AdfNode::TaskItem {
                        attrs: TaskItemAttrs {
                            local_id: "item-2".into(),
                            state: TaskItemState::Done,
                        },
                        content: vec![AdfNode::Text {
                            text: "Task item 2".into(),
                            marks: None,
                        }],
                    },
                ],
            }],
            version: 1,
        };
        roundtrip_adf_html_adf(adf.clone());
        roundtrip_adf_html_md_html_adf(adf);
    }

    #[test]
    fn test_status_emoji_roundtrip() {
        let adf = AdfNode::Doc {
            content: vec![AdfNode::Paragraph {
                content: Some(vec![
                    AdfNode::Status {
                        attrs: StatusAttrs {
                            text: "Done".into(),
                            color: "green".into(),
                            local_id: Some("status-1".into()),
                        },
                    },
                    AdfNode::Emoji {
                        attrs: EmojiAttrs {
                            text: Some("😄".into()),
                            short_name: ":smile:".into(),
                        },
                    },
                ]),
            }],
            version: 1,
        };
        roundtrip_adf_html_adf(adf.clone());
        roundtrip_adf_html_md_html_adf(adf);
    }
}
