use std::borrow::Cow;
use std::fmt::Write;

use chrono::{DateTime, Utc};
use urlencoding::encode;

use crate::adf::adf_types::{
    AdfBlockNode, AdfMark, AdfNode, DataSourceView, MediaDataType, MediaMark, MediaNode, Subsup,
    TaskItemState,
};
use crate::html_builder::*;

pub fn adf_to_html(adf: Vec<AdfBlockNode>, buf: &str) -> String {
    let mut buffer = Buffer::new();
    let node = buffer.body();
    inner_block_adf_to_html(node, adf, buf);
    buffer.finish()
}

fn media_adf_to_html(mut node: Node, media_entries: Vec<MediaNode>) {
    for media_node in media_entries {
        let link = media_node
            .marks
            .map(|marks| {
                marks.iter().find_map(|mark| match mark {
                    MediaMark::Link(link) => Some(link.clone()),
                    _ => None,
                })
            })
            .flatten();

        match media_node.attrs.type_ {
            MediaDataType::File => {
                let mut attrs = vec![];
                if let Some(link) = &link {
                    attrs.push(format!("src=\"{}\"", link.href));
                }
                attrs.push(format!(
                    "data-collection=\"{}\"",
                    media_node.attrs.collection
                ));
                attrs.push(format!("data-media-id=\"{}\"", media_node.attrs.id));
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
                node.child(Cow::Borrowed("img")).attr(&attrs_str);
            }
            MediaDataType::Link => {
                if let Some(link) = link {
                    let mut a = node.a().attr(&format!("href=\"{}\"", link.href));
                    if let Some(title) = link.title.as_ref() {
                        write!(a, "{}", title).ok();
                    } else {
                        write!(a, "{}", link.href).ok();
                    }
                } else {
                    tracing::warn!("Media link is missing");
                }
            }
        }
    }
}

fn inner_adf_to_html(mut node: Node, adf: Vec<AdfNode>, buf: &str) {
    for adf_node in adf {
        match adf_node {
            AdfNode::Date { attrs } => {
                let ts_ms = attrs.timestamp.parse::<i64>().unwrap_or_default();
                let dt: DateTime<Utc> = DateTime::from_timestamp_millis(ts_ms).unwrap_or_default();
                let date_str = dt.to_rfc3339();
                let mut date = node.time().attr(&format!("datetime=\"{}\"", date_str));
                write!(date, "{}", date_str).ok();
            }
            AdfNode::Emoji { attrs } => {
                let mut emoji = node
                    .child(Cow::Borrowed("adf-emoji"))
                    .attr(&format!("aria-alt=\"{}\"", attrs.short_name));
                if let Some(text) = &attrs.text {
                    write!(emoji, "{}", text).ok();
                } else {
                    write!(emoji, "{}", attrs.short_name).ok();
                }
            }
            AdfNode::HardBreak => {
                node.br();
            }
            AdfNode::InlineCard { attrs } => {
                if let Some(url) = &attrs.url {
                    let mut a_tag = node
                        .a()
                        .attr(&format!("href={}", url))
                        .attr("data-inline-card=\"true\"")
                        .attr("target=\"_blank\"")
                        .attr("rel=\"noopener noreferrer\"");
                    write!(a_tag, "External Link").ok();
                }
            }
            AdfNode::ListItem { content } => {
                let list_item = node.li();
                inner_block_adf_to_html(list_item, content, buf);
            }
            AdfNode::Mention { attrs } => {
                let mut mention = node
                    .child(Cow::Borrowed("adf-mention"))
                    .attr(&format!("data-mention-id=\"{}\"", attrs.id));
                mention = mention.attr(&format!(
                    "data-user-type={}",
                    serde_json::to_string(&attrs.user_type).expect("Failed to serialize")
                ));
                mention = mention.attr(&format!(
                    "data-access-level={}",
                    serde_json::to_string(&attrs.access_level).expect("Failed to serialize")
                ));
                if let Some(text) = &attrs.text {
                    write!(mention, "{}", text).ok();
                }
            }
            AdfNode::Status { attrs } => {
                let mut status = node.child(Cow::Borrowed("adf-status")).attr(&format!(
                    "style=\"background-color: {}\" aria-label=\"{}\"",
                    attrs.color,
                    attrs.local_id.unwrap_or_default()
                ));
                write!(status, "{}", attrs.text).ok();
            }
            AdfNode::TableCell { content, .. } => {
                let cell = node.td();
                inner_block_adf_to_html(cell, content, buf);
            }
            AdfNode::TableHeader { content, .. } => {
                let header = node.th();
                inner_block_adf_to_html(header, content, buf);
            }
            // TableRow is simplified now, always outputs <tr> (never decides on header itself anymore)
            AdfNode::TableRow { content, .. } => {
                let row = node.tr();
                inner_adf_to_html(row, content, buf);
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
                                node.span().attr("style=text-decoration:underline")
                            }
                            AdfMark::TextColor { color } => {
                                node.span().attr(&format!("style=\"color: {color}\""))
                            }
                            AdfMark::BackgroundColor { color } => node
                                .span()
                                .attr(&format!("style=\"background-color: {color}\"")),
                        };
                        apply_marks(&mut wrapped_node, rest, text)
                    } else {
                        write!(node, "{}", text)
                    }
                }
                apply_marks(&mut node, &marks.unwrap_or_default(), &text).ok();
            }
            AdfNode::TaskItem { attrs, content } => {
                let mut task_item = node.li();
                let checked = if attrs.state == TaskItemState::Done {
                    "checked"
                } else {
                    ""
                };
                let local_id = attrs.local_id;
                task_item
                    .child(Cow::Borrowed("adf-task-item"))
                    .attr(&format!("id=\"{}\" type=checkbox {}", local_id, checked));
                inner_block_adf_to_html(task_item, content, buf);
            }
            AdfNode::DecisionItem { content, attrs } => {
                let mut li = node.li();
                let child = li
                    .child(Cow::Borrowed("adf-decision-item"))
                    .attr(&format!("id=\"{}\"", attrs.local_id));
                inner_block_adf_to_html(child, content, buf);
            }
            AdfNode::Unknown => {
                tracing::warn!("Unknown node type in {}", buf);
            }
        }
    }
}

fn inner_block_adf_to_html(mut node: Node, adf: Vec<AdfBlockNode>, buf: &str) {
    for adf_node in adf {
        match adf_node {
            AdfBlockNode::Blockquote { content } => {
                let blockquote = node.blockquote();
                inner_block_adf_to_html(blockquote, content, buf);
            }
            AdfBlockNode::BlockCard { attrs } => {
                let mut block_card = node
                    .child(Cow::Borrowed("adf-block-card"))
                    .attr(&format!("data-block-card=\"{}\"", attrs.url));
                let jql_attr = encode(&attrs.datasource.parameters.jql);
                let mut datasource = block_card
                    .child(Cow::Borrowed("adf-block-card-data-source"))
                    .attr(&format!("data-source=\"{}\"", attrs.datasource.id))
                    .attr(&format!(
                        "data-cloud-id=\"{}\"",
                        attrs.datasource.parameters.cloud_id
                    ))
                    .attr(&format!("data-jql=\"{}\"", jql_attr));
                for view in attrs.datasource.views {
                    match view {
                        DataSourceView::Table(properties) => {
                            let mut table = datasource
                                .child(Cow::Borrowed("adf-block-card-view"))
                                .attr(&format!("data-type=\"table\""));
                            for (i, column) in properties.columns.into_iter().enumerate() {
                                table = table.attr(&format!("data-key-{}=\"{}\"", i, column.key));
                            }
                        }
                    }
                }
            }
            AdfBlockNode::BulletList { content } => {
                let list = node.ul();
                inner_adf_to_html(list, content, buf);
            }
            AdfBlockNode::CodeBlock { attrs, content } => {
                let mut pre = node.pre();
                let mut code_block = pre.code();
                if let Some(attrs) = &attrs {
                    if let Some(language) = &attrs.language {
                        code_block = code_block.attr(&format!("class=\"language-{}\"", language));
                    }
                }
                if let Some(content) = content {
                    inner_adf_to_html(code_block, content, buf);
                }
            }
            AdfBlockNode::Doc { content, .. } => {
                let doc = node.div();
                inner_block_adf_to_html(doc, content, buf);
            }
            AdfBlockNode::Expand { content, attrs } => {
                let mut expand = node.details();
                if let Some(title) = attrs.title.as_ref() {
                    write!(expand.summary(), "{}", title).ok();
                }
                inner_block_adf_to_html(expand, content, buf);
            }
            AdfBlockNode::Heading { attrs, content } => {
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
                    inner_adf_to_html(heading, content, buf);
                }
            }
            AdfBlockNode::MediaGroup { content } => {
                let media_group = node.child(Cow::Borrowed("adf-media-group"));
                media_adf_to_html(media_group, content);
            }
            AdfBlockNode::MediaSingle { content, attrs } => {
                let mut media_single = node.child(Cow::Borrowed("adf-media-single"));
                media_single = media_single.attr(&format!("data-layout=\"{}\"", attrs.layout));
                media_adf_to_html(media_single, content);
            }
            AdfBlockNode::NestedExpand { content, attrs } => {
                let mut expand = node.details().attr("data-nested=\"true\"");
                write!(expand.summary(), "{}", attrs.title).ok();
                inner_block_adf_to_html(expand, content, buf);
            }
            AdfBlockNode::OrderedList { content, .. } => {
                let list = node.ol();
                inner_adf_to_html(list, content, buf);
            }
            AdfBlockNode::Panel { content, attrs } => {
                let panel_type = attrs.panel_type.as_str();
                let panel = node
                    .figure()
                    .attr(&format!("data-panel-type=\"{panel_type}\""));
                inner_block_adf_to_html(panel, content, buf);
            }
            AdfBlockNode::Paragraph { content } => {
                let para = node.p();
                if let Some(content) = content {
                    inner_adf_to_html(para, content, buf);
                }
            }
            AdfBlockNode::Rule => {
                node.hr();
            }
            AdfBlockNode::Table { content, .. } => {
                let mut table = node.table();

                // Extract header rows and other rows
                let mut header_rows = vec![];
                let mut body_rows = vec![];

                for row in content {
                    if let AdfNode::TableRow {
                        content: row_content,
                        ..
                    } = &row
                    {
                        if row_content
                            .iter()
                            .any(|n| matches!(n, AdfNode::TableHeader { .. }))
                        {
                            header_rows.push(row.clone());
                        } else {
                            body_rows.push(row.clone());
                        }
                    }
                }

                if !header_rows.is_empty() {
                    let mut thead = table.thead();
                    for row in header_rows {
                        inner_adf_to_html(
                            thead.tr(),
                            match row {
                                AdfNode::TableRow { content, .. } => content,
                                _ => unreachable!(),
                            },
                            buf,
                        );
                    }
                }

                if !body_rows.is_empty() {
                    let mut tbody = table.tbody();
                    for row in body_rows {
                        inner_adf_to_html(
                            tbody.tr(),
                            match row {
                                AdfNode::TableRow { content, .. } => content,
                                _ => unreachable!(),
                            },
                            buf,
                        );
                    }
                }
            }
            AdfBlockNode::TaskList { content, attrs } => {
                node.child(Cow::Borrowed("adf-local-data"))
                    .attr(&format!("data-tag=\"task-list\""))
                    .attr(&format!("id=\"{}\"", attrs.local_id));
                let task_list = node.ul();
                inner_adf_to_html(task_list, content, buf);
            }
            AdfBlockNode::DecisionList { content, attrs } => {
                node.child(Cow::Borrowed("adf-local-data"))
                    .attr(&format!("data-tag=\"decision-list\""))
                    .attr(&format!("id=\"{}\"", attrs.local_id));
                let decision_list = node.ul();
                inner_adf_to_html(decision_list, content, buf);
            }
            AdfBlockNode::Unknown => {
                tracing::warn!("Unknown block type encountered in {}", buf);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adf::adf_types::*;
    use crate::html_to_adf::html_to_adf;
    use crate::markdown::{adf_to_markdown, markdown_to_adf};

    fn roundtrip_adf_html_adf(adf: AdfBlockNode) {
        let html = adf_to_html(vec![adf.clone()], "");
        eprintln!("\n\nHTML:\n{}\n\n", html);
        let back = html_to_adf(&html);
        assert_eq!(back, adf, "Failed roundtrip adf -> html -> adf");
    }

    fn roundtrip_adf_html_md_html_adf(adf: AdfBlockNode) {
        let markdown = adf_to_markdown(&[adf.clone()], "");
        eprintln!("\n\nMARKDOWN:\n{}\n\n", markdown);
        let back = markdown_to_adf(&markdown).unwrap();
        assert_eq!(
            back, adf,
            "Failed roundtrip adf -> html -> md -> html -> adf"
        );
    }

    #[test]
    fn test_paragraph_roundtrip() {
        let adf = AdfBlockNode::Doc {
            content: vec![AdfBlockNode::Paragraph {
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
        let adf = AdfBlockNode::Doc {
            content: vec![AdfBlockNode::Heading {
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
        let adf = AdfBlockNode::Doc {
            content: vec![AdfBlockNode::Panel {
                attrs: PanelAttrs {
                    panel_type: "info".into(),
                },
                content: vec![AdfBlockNode::Paragraph {
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
        let adf = AdfBlockNode::Doc {
            content: vec![AdfBlockNode::MediaGroup {
                content: vec![MediaNode {
                    media_type: MediaType::Media,
                    attrs: MediaAttrs {
                        alt: Some("Image description".into()),
                        height: None,
                        width: None,
                        id: "media-id".into(),
                        collection: "collection".into(),
                        type_: MediaDataType::File,
                    },
                    marks: None,
                }],
            }],
            version: 1,
        };
        roundtrip_adf_html_adf(adf.clone());
        roundtrip_adf_html_md_html_adf(adf);
    }

    #[test]
    fn test_media_single_roundtrip() {
        let adf = AdfBlockNode::Doc {
            content: vec![AdfBlockNode::MediaSingle {
                attrs: MediaSingleAttrs {
                    layout: "center".into(),
                },
                content: vec![MediaNode {
                    media_type: MediaType::Media,
                    attrs: MediaAttrs {
                        alt: None,
                        height: Some(300),
                        width: Some(300),
                        id: "media-id".into(),
                        collection: "collection".into(),
                        type_: MediaDataType::File,
                    },
                    marks: None,
                }],
            }],
            version: 1,
        };
        roundtrip_adf_html_adf(adf.clone());
        roundtrip_adf_html_md_html_adf(adf);
    }

    #[test]
    fn test_task_list_roundtrip() {
        let adf = AdfBlockNode::Doc {
            content: vec![AdfBlockNode::TaskList {
                attrs: LocalId {
                    local_id: "task-list-1".into(),
                },
                content: vec![
                    AdfNode::TaskItem {
                        attrs: TaskItemAttrs {
                            local_id: "item-1".into(),
                            state: TaskItemState::Todo,
                        },
                        content: vec![AdfBlockNode::Paragraph {
                            content: Some(vec![AdfNode::Text {
                                text: "Task item".into(),
                                marks: None,
                            }]),
                        }],
                    },
                    AdfNode::TaskItem {
                        attrs: TaskItemAttrs {
                            local_id: "item-2".into(),
                            state: TaskItemState::Done,
                        },
                        content: vec![AdfBlockNode::Paragraph {
                            content: Some(vec![AdfNode::Text {
                                text: "Task item 2".into(),
                                marks: None,
                            }]),
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
        let adf = AdfBlockNode::Doc {
            content: vec![AdfBlockNode::Paragraph {
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

    #[test]
    fn test_expand_roundtrip() {
        let adf = AdfBlockNode::Doc {
            content: vec![AdfBlockNode::Expand {
                attrs: ExpandAttrs {
                    title: Some("Expand Title".into()),
                },
                content: vec![AdfBlockNode::Paragraph {
                    content: Some(vec![AdfNode::Text {
                        text: "Expandable content".into(),
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
    fn test_nested_expand_roundtrip() {
        let adf = AdfBlockNode::Doc {
            content: vec![AdfBlockNode::NestedExpand {
                attrs: NestedAttrs {
                    title: "Nested Title".into(),
                },
                content: vec![AdfBlockNode::Paragraph {
                    content: Some(vec![AdfNode::Text {
                        text: "Nested content".into(),
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
    fn test_date_roundtrip() {
        let adf = AdfBlockNode::Doc {
            content: vec![AdfBlockNode::Paragraph {
                content: Some(vec![AdfNode::Date {
                    attrs: DateAttrs {
                        timestamp: "1700000000".into(),
                    },
                }]),
            }],
            version: 1,
        };
        roundtrip_adf_html_adf(adf.clone());
        roundtrip_adf_html_md_html_adf(adf);
    }

    #[test]
    fn test_mention_roundtrip() {
        let adf = AdfBlockNode::Doc {
            content: vec![AdfBlockNode::Paragraph {
                content: Some(vec![AdfNode::Mention {
                    attrs: MentionAttrs {
                        id: "user-1".into(),
                        text: Some("Mentioned User".into()),
                        access_level: AccessLevel::Site,
                        user_type: UserType::App,
                    },
                }]),
            }],
            version: 1,
        };
        roundtrip_adf_html_adf(adf.clone());
        roundtrip_adf_html_md_html_adf(adf);
    }

    #[test]
    fn test_inline_card_roundtrip() {
        let adf = AdfBlockNode::Doc {
            content: vec![AdfBlockNode::Paragraph {
                content: Some(vec![AdfNode::InlineCard {
                    attrs: InlineCardAttrs {
                        url: Some("https://example.com".into()),
                    },
                }]),
            }],
            version: 1,
        };
        roundtrip_adf_html_adf(adf.clone());
        roundtrip_adf_html_md_html_adf(adf);
    }

    #[test]
    fn test_rule_roundtrip() {
        let adf = AdfBlockNode::Doc {
            content: vec![AdfBlockNode::Rule],
            version: 1,
        };
        roundtrip_adf_html_adf(adf.clone());
        roundtrip_adf_html_md_html_adf(adf);
    }

    #[test]
    fn test_bullet_list_roundtrip() {
        let adf = AdfBlockNode::Doc {
            content: vec![AdfBlockNode::BulletList {
                content: vec![
                    AdfNode::ListItem {
                        content: vec![AdfBlockNode::Paragraph {
                            content: Some(vec![AdfNode::Text {
                                text: "Bullet 1".into(),
                                marks: None,
                            }]),
                        }],
                    },
                    AdfNode::ListItem {
                        content: vec![AdfBlockNode::Paragraph {
                            content: Some(vec![AdfNode::Text {
                                text: "Bullet 2".into(),
                                marks: None,
                            }]),
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
    fn test_ordered_list_roundtrip() {
        let adf = AdfBlockNode::Doc {
            content: vec![AdfBlockNode::OrderedList {
                content: vec![
                    AdfNode::ListItem {
                        content: vec![AdfBlockNode::Paragraph {
                            content: Some(vec![AdfNode::Text {
                                text: "Ordered 1".into(),
                                marks: None,
                            }]),
                        }],
                    },
                    AdfNode::ListItem {
                        content: vec![AdfBlockNode::Paragraph {
                            content: Some(vec![AdfNode::Text {
                                text: "Ordered 2".into(),
                                marks: None,
                            }]),
                        }],
                    },
                ],
                attrs: None,
            }],
            version: 1,
        };
        roundtrip_adf_html_adf(adf.clone());
        roundtrip_adf_html_md_html_adf(adf);
    }

    #[test]
    fn test_blockquote_roundtrip() {
        let adf = AdfBlockNode::Doc {
            content: vec![AdfBlockNode::Blockquote {
                content: vec![AdfBlockNode::Paragraph {
                    content: Some(vec![AdfNode::Text {
                        text: "Blockquoted text".into(),
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
    fn test_codeblock_roundtrip() {
        let adf = AdfBlockNode::Doc {
            content: vec![AdfBlockNode::CodeBlock {
                attrs: None,
                content: Some(vec![AdfNode::Text {
                    text: "let x = 42;\n".into(),
                    marks: None,
                }]),
            }],
            version: 1,
        };
        roundtrip_adf_html_adf(adf.clone());
        roundtrip_adf_html_md_html_adf(adf);
    }

    #[test]
    fn test_hardbreak_roundtrip() {
        let adf = AdfBlockNode::Doc {
            content: vec![AdfBlockNode::Paragraph {
                content: Some(vec![
                    AdfNode::Text {
                        text: "Line one".into(),
                        marks: None,
                    },
                    AdfNode::HardBreak,
                    AdfNode::Text {
                        text: "Line two".into(),
                        marks: None,
                    },
                ]),
            }],
            version: 1,
        };
        roundtrip_adf_html_adf(adf.clone());
        roundtrip_adf_html_md_html_adf(adf);
    }

    #[test]
    fn test_decision_list_roundtrip() {
        let adf = AdfBlockNode::Doc {
            content: vec![AdfBlockNode::DecisionList {
                attrs: LocalId {
                    local_id: "decision-list-1".into(),
                },
                content: vec![AdfNode::DecisionItem {
                    attrs: DecisionItemAttrs {
                        state: "DECIDED".into(),
                        local_id: "item-1".into(),
                    },
                    content: vec![AdfBlockNode::Paragraph {
                        content: Some(vec![AdfNode::Text {
                            text: "Decision content".into(),
                            marks: None,
                        }]),
                    }],
                }],
            }],
            version: 1,
        };
        roundtrip_adf_html_adf(adf.clone());
        roundtrip_adf_html_md_html_adf(adf);
    }

    #[test]
    fn test_table_roundtrip() {
        let adf = AdfBlockNode::Doc {
            content: vec![AdfBlockNode::Table {
                attrs: None,
                content: vec![
                    AdfNode::TableRow {
                        content: vec![AdfNode::TableHeader {
                            attrs: None,
                            content: vec![AdfBlockNode::Paragraph {
                                content: Some(vec![AdfNode::Text {
                                    text: "Header".into(),
                                    marks: None,
                                }]),
                            }],
                        }],
                    },
                    AdfNode::TableRow {
                        content: vec![AdfNode::TableCell {
                            attrs: None,
                            content: vec![AdfBlockNode::Paragraph {
                                content: Some(vec![AdfNode::Text {
                                    text: "Cell".into(),
                                    marks: None,
                                }]),
                            }],
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
    fn test_full_doc_with_header_paragraph_list_table() {
        let adf = AdfBlockNode::Doc {
            content: vec![
                AdfBlockNode::Heading {
                    attrs: HeadingAttrs { level: 1 },
                    content: Some(vec![AdfNode::Text {
                        text: "Document Title".into(),
                        marks: None,
                    }]),
                },
                AdfBlockNode::Paragraph {
                    content: Some(vec![AdfNode::Text {
                        text: "Introductory paragraph.".into(),
                        marks: None,
                    }]),
                },
                AdfBlockNode::BulletList {
                    content: vec![
                        AdfNode::ListItem {
                            content: vec![AdfBlockNode::Paragraph {
                                content: Some(vec![AdfNode::Text {
                                    text: "Item 1".into(),
                                    marks: None,
                                }]),
                            }],
                        },
                        AdfNode::ListItem {
                            content: vec![AdfBlockNode::Paragraph {
                                content: Some(vec![AdfNode::Text {
                                    text: "Item 2".into(),
                                    marks: None,
                                }]),
                            }],
                        },
                    ],
                },
                AdfBlockNode::Table {
                    attrs: None,
                    content: vec![
                        AdfNode::TableRow {
                            content: vec![
                                AdfNode::TableHeader {
                                    attrs: None,
                                    content: vec![AdfBlockNode::Paragraph {
                                        content: Some(vec![AdfNode::Text {
                                            text: "Header 1".into(),
                                            marks: None,
                                        }]),
                                    }],
                                },
                                AdfNode::TableHeader {
                                    attrs: None,
                                    content: vec![AdfBlockNode::Paragraph {
                                        content: Some(vec![AdfNode::Text {
                                            text: "Header 2".into(),
                                            marks: None,
                                        }]),
                                    }],
                                },
                            ],
                        },
                        AdfNode::TableRow {
                            content: vec![
                                AdfNode::TableCell {
                                    attrs: None,
                                    content: vec![AdfBlockNode::Paragraph {
                                        content: Some(vec![AdfNode::Text {
                                            text: "Cell 1".into(),
                                            marks: None,
                                        }]),
                                    }],
                                },
                                AdfNode::TableCell {
                                    attrs: None,
                                    content: vec![AdfBlockNode::Paragraph {
                                        content: Some(vec![AdfNode::Text {
                                            text: "Cell 2".into(),
                                            marks: None,
                                        }]),
                                    }],
                                },
                            ],
                        },
                    ],
                },
            ],
            version: 1,
        };
        roundtrip_adf_html_adf(adf.clone());
        roundtrip_adf_html_md_html_adf(adf);
    }

    #[test]
    fn test_full_doc_with_decision_status_panel() {
        let adf = AdfBlockNode::Doc {
            content: vec![
                AdfBlockNode::DecisionList {
                    attrs: LocalId {
                        local_id: "decision-1".into(),
                    },
                    content: vec![AdfNode::DecisionItem {
                        attrs: DecisionItemAttrs {
                            state: "DECIDED".into(),
                            local_id: "item-1".into(),
                        },
                        content: vec![AdfBlockNode::Paragraph {
                            content: Some(vec![AdfNode::Text {
                                text: "We will proceed.".into(),
                                marks: None,
                            }]),
                        }],
                    }],
                },
                AdfBlockNode::Paragraph {
                    content: Some(vec![AdfNode::Status {
                        attrs: StatusAttrs {
                            text: "Approved".into(),
                            color: "green".into(),
                            local_id: Some("status-1".into()),
                        },
                    }]),
                },
                AdfBlockNode::Panel {
                    attrs: PanelAttrs {
                        panel_type: "warning".into(),
                    },
                    content: vec![AdfBlockNode::Paragraph {
                        content: Some(vec![AdfNode::Text {
                            text: "This is important context.".into(),
                            marks: None,
                        }]),
                    }],
                },
            ],
            version: 1,
        };
        roundtrip_adf_html_adf(adf.clone());
        roundtrip_adf_html_md_html_adf(adf);
    }

    #[test]
    fn test_full_doc_with_media_inline_expand() {
        let adf = AdfBlockNode::Doc {
            content: vec![
                AdfBlockNode::Paragraph {
                    content: Some(vec![AdfNode::InlineCard {
                        attrs: InlineCardAttrs {
                            url: Some("https://example.com".into()),
                        },
                    }]),
                },
                AdfBlockNode::MediaGroup {
                    content: vec![MediaNode {
                        media_type: MediaType::Media,
                        attrs: MediaAttrs {
                            alt: Some("Diagram".into()),
                            height: None,
                            width: None,
                            id: "media-id".into(),
                            collection: "collection".into(),
                            type_: MediaDataType::File,
                        },
                        marks: None,
                    }],
                },
                AdfBlockNode::Expand {
                    attrs: ExpandAttrs {
                        title: Some("See more".into()),
                    },
                    content: vec![AdfBlockNode::Paragraph {
                        content: Some(vec![AdfNode::Text {
                            text: "Hidden details.".into(),
                            marks: None,
                        }]),
                    }],
                },
            ],
            version: 1,
        };
        roundtrip_adf_html_adf(adf.clone());
        roundtrip_adf_html_md_html_adf(adf);
    }

    #[test]
    fn test_paragraph_with_mixed_inline() {
        let adf = AdfBlockNode::Doc {
            content: vec![AdfBlockNode::Paragraph {
                content: Some(vec![
                    AdfNode::Text {
                        text: "Hello ".into(),
                        marks: None,
                    },
                    AdfNode::Emoji {
                        attrs: EmojiAttrs {
                            text: Some("😄".into()),
                            short_name: ":smile:".into(),
                        },
                    },
                    AdfNode::Mention {
                        attrs: MentionAttrs {
                            id: "user-1".into(),
                            text: Some("User".into()),
                            access_level: AccessLevel::None,
                            user_type: UserType::Default,
                        },
                    },
                    AdfNode::Text {
                        text: "link".into(),
                        marks: Some(vec![AdfMark::Link(LinkMark {
                            href: "https://example.com".into(),
                            ..Default::default()
                        })]),
                    },
                    AdfNode::Status {
                        attrs: StatusAttrs {
                            text: "In Progress".into(),
                            color: "blue".into(),
                            local_id: Some("status-1".into()),
                        },
                    },
                    AdfNode::InlineCard {
                        attrs: InlineCardAttrs {
                            url: Some("https://card.com".into()),
                        },
                    },
                ]),
            }],
            version: 1,
        };
        roundtrip_adf_html_adf(adf.clone());
        roundtrip_adf_html_md_html_adf(adf);
    }

    #[test]
    fn test_nested_expand_inside_panel() {
        let adf = AdfBlockNode::Doc {
            content: vec![AdfBlockNode::Panel {
                attrs: PanelAttrs {
                    panel_type: "info".into(),
                },
                content: vec![
                    AdfBlockNode::Paragraph {
                        content: Some(vec![AdfNode::Text {
                            text: "Intro panel".into(),
                            marks: None,
                        }]),
                    },
                    AdfBlockNode::Expand {
                        attrs: ExpandAttrs {
                            title: Some("Expand inside panel".into()),
                        },
                        content: vec![AdfBlockNode::Paragraph {
                            content: Some(vec![AdfNode::Text {
                                text: "More details".into(),
                                marks: None,
                            }]),
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
    fn test_mixed_decision_and_task_lists() {
        let adf = AdfBlockNode::Doc {
            content: vec![
                AdfBlockNode::TaskList {
                    attrs: LocalId {
                        local_id: "task-list".into(),
                    },
                    content: vec![
                        AdfNode::TaskItem {
                            attrs: TaskItemAttrs {
                                local_id: "task-1".into(),
                                state: TaskItemState::Todo,
                            },
                            content: vec![AdfBlockNode::Paragraph {
                                content: Some(vec![AdfNode::Text {
                                    text: "First task".into(),
                                    marks: None,
                                }]),
                            }],
                        },
                        AdfNode::TaskItem {
                            attrs: TaskItemAttrs {
                                local_id: "task-2".into(),
                                state: TaskItemState::Done,
                            },
                            content: vec![AdfBlockNode::Paragraph {
                                content: Some(vec![AdfNode::Text {
                                    text: "Second task".into(),
                                    marks: None,
                                }]),
                            }],
                        },
                    ],
                },
                AdfBlockNode::DecisionList {
                    attrs: LocalId {
                        local_id: "decision-list".into(),
                    },
                    content: vec![
                        AdfNode::DecisionItem {
                            attrs: DecisionItemAttrs {
                                state: "DECIDED".into(),
                                local_id: "decision-1".into(),
                            },
                            content: vec![AdfBlockNode::Paragraph {
                                content: Some(vec![AdfNode::Text {
                                    text: "Agreed decision".into(),
                                    marks: None,
                                }]),
                            }],
                        },
                        AdfNode::DecisionItem {
                            attrs: DecisionItemAttrs {
                                state: "DECIDED".into(),
                                local_id: "decision-2".into(),
                            },
                            content: vec![AdfBlockNode::Paragraph {
                                content: Some(vec![AdfNode::Text {
                                    text: "Pending decision".into(),
                                    marks: None,
                                }]),
                            }],
                        },
                    ],
                },
            ],
            version: 1,
        };
        roundtrip_adf_html_adf(adf.clone());
        roundtrip_adf_html_md_html_adf(adf);
    }

    #[test]
    fn test_table_with_complex_content() {
        let adf = AdfBlockNode::Doc {
            content: vec![AdfBlockNode::Table {
                attrs: None,
                content: vec![
                    AdfNode::TableRow {
                        content: vec![
                            AdfNode::TableHeader {
                                attrs: None,
                                content: vec![AdfBlockNode::Paragraph {
                                    content: Some(vec![
                                        AdfNode::Text {
                                            text: "Bold header".into(),
                                            marks: Some(vec![AdfMark::Strong]),
                                        },
                                        AdfNode::Emoji {
                                            attrs: EmojiAttrs {
                                                text: Some("📊".into()),
                                                short_name: ":bar_chart:".into(),
                                            },
                                        },
                                    ]),
                                }],
                            },
                            AdfNode::TableHeader {
                                attrs: None,
                                content: vec![AdfBlockNode::Paragraph {
                                    content: Some(vec![AdfNode::Text {
                                        text: "Plain header".into(),
                                        marks: None,
                                    }]),
                                }],
                            },
                        ],
                    },
                    AdfNode::TableRow {
                        content: vec![
                            AdfNode::TableCell {
                                attrs: None,
                                content: vec![AdfBlockNode::Paragraph {
                                    content: Some(vec![
                                        AdfNode::Text {
                                            text: "Line 1 ".into(),
                                            marks: None,
                                        },
                                        AdfNode::Text {
                                            text: "Line 2".into(),
                                            marks: Some(vec![AdfMark::Strong]),
                                        },
                                    ]),
                                }],
                            },
                            AdfNode::TableCell {
                                attrs: None,
                                content: vec![AdfBlockNode::Paragraph {
                                    content: Some(vec![AdfNode::InlineCard {
                                        attrs: InlineCardAttrs {
                                            url: Some("https://inline.cell".into()),
                                        },
                                    }]),
                                }],
                            },
                        ],
                    },
                ],
            }],
            version: 1,
        };
        roundtrip_adf_html_adf(adf.clone());
        roundtrip_adf_html_md_html_adf(adf);
    }

    #[test]
    fn test_complex_blockquote() {
        let adf = AdfBlockNode::Doc {
            content: vec![AdfBlockNode::Blockquote {
                content: vec![
                    AdfBlockNode::Paragraph {
                        content: Some(vec![AdfNode::Text {
                            text: "Intro quote".into(),
                            marks: None,
                        }]),
                    },
                    AdfBlockNode::OrderedList {
                        content: vec![
                            AdfNode::ListItem {
                                content: vec![AdfBlockNode::Paragraph {
                                    content: Some(vec![AdfNode::Text {
                                        text: "List item 1".into(),
                                        marks: None,
                                    }]),
                                }],
                            },
                            AdfNode::ListItem {
                                content: vec![AdfBlockNode::Paragraph {
                                    content: Some(vec![AdfNode::Text {
                                        text: "List item 2".into(),
                                        marks: None,
                                    }]),
                                }],
                            },
                        ],
                        attrs: None,
                    },
                    AdfBlockNode::CodeBlock {
                        content: Some(vec![AdfNode::Text {
                            text: "let x = 10;\n".into(),
                            marks: None,
                        }]),
                        attrs: None,
                    },
                ],
            }],
            version: 1,
        };
        roundtrip_adf_html_adf(adf.clone());
        roundtrip_adf_html_md_html_adf(adf);
    }

    #[test]
    fn test_full_document_comprehensive() {
        let adf = AdfBlockNode::Doc {
            content: vec![
                AdfBlockNode::Heading {
                    attrs: HeadingAttrs { level: 1 },
                    content: Some(vec![AdfNode::Text {
                        text: "Comprehensive Doc".into(),
                        marks: None,
                    }]),
                },
                AdfBlockNode::Paragraph {
                    content: Some(vec![
                        AdfNode::Text {
                            text: " Mixed content paragraph ".into(),
                            marks: Some(vec![AdfMark::TextColor {
                                color: "blue".into(),
                            }]),
                        },
                        AdfNode::Emoji {
                            attrs: EmojiAttrs {
                                text: Some("🎉".into()),
                                short_name: ":tada:".into(),
                            },
                        },
                        AdfNode::Status {
                            attrs: StatusAttrs {
                                text: "Done".into(),
                                color: "green".into(),
                                local_id: Some("status-4".into()),
                            },
                        },
                    ]),
                },
                AdfBlockNode::Rule,
                AdfBlockNode::MediaGroup {
                    content: vec![MediaNode {
                        media_type: MediaType::Media,
                        attrs: MediaAttrs {
                            alt: Some("Diagram".into()),
                            collection: "collection".into(),
                            height: Some(200),
                            id: "media-1".into(),
                            type_: MediaDataType::File,
                            width: Some(300),
                        },
                        marks: None,
                    }],
                },
                AdfBlockNode::Expand {
                    attrs: ExpandAttrs {
                        title: Some("Expand Block".into()),
                    },
                    content: vec![AdfBlockNode::Paragraph {
                        content: Some(vec![AdfNode::Text {
                            text: "Expandable content. ".into(),
                            marks: Some(vec![
                                AdfMark::BackgroundColor {
                                    color: "#000".into(),
                                },
                                AdfMark::TextColor {
                                    color: "#fff".into(),
                                },
                            ]),
                        }]),
                    }],
                },
                AdfBlockNode::Table {
                    attrs: None,
                    content: vec![
                        AdfNode::TableRow {
                            content: vec![AdfNode::TableHeader {
                                attrs: None,
                                content: vec![AdfBlockNode::Paragraph {
                                    content: Some(vec![AdfNode::Text {
                                        text: "Header 1".into(),
                                        marks: None,
                                    }]),
                                }],
                            }],
                        },
                        AdfNode::TableRow {
                            content: vec![AdfNode::TableCell {
                                attrs: None,
                                content: vec![AdfBlockNode::Paragraph {
                                    content: Some(vec![AdfNode::Text {
                                        text: "Cell 1".into(),
                                        marks: None,
                                    }]),
                                }],
                            }],
                        },
                    ],
                },
                AdfBlockNode::Blockquote {
                    content: vec![AdfBlockNode::Paragraph {
                        content: Some(vec![AdfNode::Text {
                            text: "Quote in block".into(),
                            marks: None,
                        }]),
                    }],
                },
            ],
            version: 1,
        };
        roundtrip_adf_html_adf(adf.clone());
        roundtrip_adf_html_md_html_adf(adf);
    }

    #[test]
    fn test_header_with_emoji() {
        let adf = AdfBlockNode::Doc {
            content: vec![AdfBlockNode::Heading {
                attrs: HeadingAttrs { level: 1 },
                content: Some(vec![
                    AdfNode::Text {
                        text: "🚀 Let's launch ".into(),
                        marks: None,
                    },
                    AdfNode::Emoji {
                        attrs: EmojiAttrs {
                            text: Some("😄".into()),
                            short_name: ":smile:".into(),
                        },
                    },
                    AdfNode::Text {
                        text: " today".into(),
                        marks: None,
                    },
                ]),
            }],
            version: 1,
        };

        // ADF -> Markdown -> ADF should roundtrip cleanly
        let markdown = adf_to_markdown(&[adf.clone()], "");
        eprintln!("Markdown:\n{}", markdown);

        let parsed = markdown_to_adf(&markdown).unwrap();

        assert_eq!(
            parsed, adf,
            "Failed roundtrip for header containing emoji: {markdown}"
        );
    }
}
