use html5ever::tokenizer::{Tag, TagKind, Token, TokenSink, TokenSinkResult};
use std::cell::RefCell;

use crate::adf_types::{AdfMark, AdfNode, Subsup};

pub struct ADFBuilder {
    state: RefCell<ADFBuilderState>,
}

struct ADFBuilderState {
    stack: Vec<BlockContext>,
    mark_stack: Vec<AdfMark>,
    current_text: String,
}

#[derive(Debug)]
enum BlockContext {
    Document(Vec<AdfNode>),
    Paragraph(Vec<AdfNode>),
    CodeBlock(Vec<String>),
    Blockquote(Vec<AdfNode>),
    OrderedList(Vec<AdfNode>),
    BulletList(Vec<AdfNode>),
    ListItem(Vec<AdfNode>),
    Table(Vec<AdfNode>),
    TableRow(Vec<AdfNode>),
    TableCell(Vec<AdfNode>),
    TableHeader(Vec<AdfNode>),
}

impl ADFBuilder {
    pub fn new() -> Self {
        Self {
            state: RefCell::new(ADFBuilderState {
                stack: vec![BlockContext::Document(vec![])],
                mark_stack: vec![],
                current_text: String::new(),
            }),
        }
    }

    fn flush_text(state: &mut ADFBuilderState) {
        if !state.current_text.is_empty() {
            let mut text = std::mem::take(&mut state.current_text);

            // Determine if we are in a block context that shouldn't retain whitespace-only text
            let trim_for_blocks = matches!(
                state.stack.last(),
                Some(BlockContext::TableCell(_) | BlockContext::TableHeader(_))
            );

            if trim_for_blocks {
                if text.trim().is_empty() {
                    return; // Ignore pure whitespace in table cell/header context
                }
                text = text.trim().to_string();
            }

            let marks = if state.mark_stack.is_empty() {
                None
            } else {
                Some(state.mark_stack.clone())
            };
            if let Some(frame) = state.stack.last_mut() {
                let node = AdfNode::Text { text, marks };
                match frame {
                    BlockContext::Blockquote(nodes)
                    | BlockContext::Paragraph(nodes)
                    | BlockContext::ListItem(nodes)
                    | BlockContext::TableCell(nodes)
                    | BlockContext::TableHeader(nodes) => nodes.push(node),
                    BlockContext::CodeBlock(lines) => match node {
                        AdfNode::Text { text, .. } => lines.push(text),
                        _ => panic!("Invalid node type for CodeBlock"),
                    },
                    _ => {}
                }
            }
        }
    }

    fn pop_mark(state: &mut ADFBuilderState, pred: impl Fn(&AdfMark) -> bool) {
        if let Some(pos) = state.mark_stack.iter().rposition(pred) {
            state.mark_stack.remove(pos);
        }
    }

    fn close_current_block(state: &mut ADFBuilderState) {
        let frame = state.stack.pop().expect("Expected a block context");
        let parent = state
            .stack
            .last_mut()
            .expect("Document should always be present");
        match frame {
            BlockContext::Paragraph(nodes) => match parent {
                BlockContext::Document(parent_nodes)
                | BlockContext::TableCell(parent_nodes)
                | BlockContext::TableHeader(parent_nodes)
                | BlockContext::ListItem(parent_nodes) => parent_nodes.push(AdfNode::Paragraph {
                    content: Some(nodes),
                }),
                _ => panic!("Invalid parent for Paragraph"),
            },
            BlockContext::CodeBlock(lines) => match parent {
                BlockContext::Document(parent_nodes)
                | BlockContext::TableCell(parent_nodes)
                | BlockContext::TableHeader(parent_nodes)
                | BlockContext::ListItem(parent_nodes) => {
                    let text = lines.join("");
                    parent_nodes.push(AdfNode::CodeBlock {
                        content: Some(vec![AdfNode::Text {
                            text: text.into(),
                            marks: None,
                        }]),
                        attrs: None,
                    });
                }
                _ => panic!("Invalid parent for CodeBlock"),
            },
            BlockContext::Blockquote(nodes) => match parent {
                BlockContext::Document(parent_nodes)
                | BlockContext::TableCell(parent_nodes)
                | BlockContext::TableHeader(parent_nodes)
                | BlockContext::ListItem(parent_nodes) => {
                    parent_nodes.push(AdfNode::Blockquote { content: nodes })
                }
                _ => panic!("Invalid parent for Blockquote"),
            },
            BlockContext::BulletList(nodes) => match parent {
                BlockContext::Document(parent_nodes)
                | BlockContext::TableCell(parent_nodes)
                | BlockContext::TableHeader(parent_nodes)
                | BlockContext::ListItem(parent_nodes) => {
                    parent_nodes.push(AdfNode::BulletList { content: nodes })
                }
                _ => panic!("Invalid parent for BulletList"),
            },
            BlockContext::OrderedList(nodes) => match parent {
                BlockContext::Document(parent_nodes)
                | BlockContext::TableCell(parent_nodes)
                | BlockContext::TableHeader(parent_nodes)
                | BlockContext::ListItem(parent_nodes) => parent_nodes.push(AdfNode::OrderedList {
                    content: nodes,
                    attrs: None,
                }),
                _ => panic!("Invalid parent for OrderedList"),
            },
            block => {
                panic!("{block:?} closed incorrectly; must use block-specific close method");
            }
        }
    }

    fn close_current_list_item(state: &mut ADFBuilderState) {
        if let Some(BlockContext::ListItem(nodes)) = state.stack.pop() {
            match state.stack.last_mut() {
                Some(BlockContext::BulletList(list)) => {
                    list.push(AdfNode::ListItem { content: nodes });
                }
                Some(BlockContext::OrderedList(list)) => {
                    list.push(AdfNode::ListItem { content: nodes });
                }
                _ => {
                    panic!("ListItem closed without BulletList or OrderedList parent");
                }
            }
        }
    }

    pub fn emit(self) -> AdfNode {
        let mut state = self.state.into_inner();
        Self::flush_text(&mut state);
        while state.stack.len() > 1 {
            Self::close_current_block(&mut state);
        }
        if let BlockContext::Document(content) = state.stack.pop().unwrap() {
            AdfNode::Doc {
                content,
                version: 1,
            }
        } else {
            panic!("Expected Document at the base of stack");
        }
    }

    fn push_inline(state: &mut ADFBuilderState, node: AdfNode) {
        if let Some(frame) = state.stack.last_mut() {
            match frame {
                BlockContext::CodeBlock(lines) => lines.push("\n".into()),
                BlockContext::Paragraph(nodes)
                | BlockContext::Blockquote(nodes)
                | BlockContext::ListItem(nodes) => nodes.push(node),
                _ => {
                    // Invalid context for LineBreak
                }
            }
        }
    }

    fn flush_text_and_push_inline(state: &mut ADFBuilderState, node: AdfNode) {
        Self::flush_text(state);
        Self::push_inline(state, node);
    }

    fn push_block_to_parent(state: &mut ADFBuilderState, node: AdfNode) {
        let frame = state
            .stack
            .last_mut()
            .expect("There should always be at least the Document node");
        match frame {
            BlockContext::TableRow(nodes)
            | BlockContext::Table(nodes)
            | BlockContext::Paragraph(nodes)
            | BlockContext::Blockquote(nodes)
            | BlockContext::ListItem(nodes)
            | BlockContext::Document(nodes) => nodes.push(node),
            _ => {
                panic!("Invalid block context for block nodes");
            }
        }
    }

    fn close_current_table_row(state: &mut ADFBuilderState) {
        if let Some(BlockContext::TableRow(cells)) = state.stack.pop() {
            Self::push_block_to_parent(state, AdfNode::TableRow { content: cells });
        }
    }

    fn close_current_table_cell(state: &mut ADFBuilderState) {
        if let Some(BlockContext::TableCell(content)) = state.stack.pop() {
            Self::push_block_to_parent(
                state,
                AdfNode::TableCell {
                    attrs: None,
                    content,
                },
            );
        }
    }

    fn close_current_table_header(state: &mut ADFBuilderState) {
        if let Some(BlockContext::TableHeader(content)) = state.stack.pop() {
            Self::push_block_to_parent(
                state,
                AdfNode::TableHeader {
                    attrs: None,
                    content,
                },
            );
        }
    }
}

fn extract_css_color(style: &str, property: &str) -> Option<String> {
    style
        .split(';')
        .filter_map(|rule| {
            let mut parts = rule.splitn(2, ':');
            let name = parts.next()?.trim();
            let value = parts.next()?.trim();
            if name.eq_ignore_ascii_case(property) {
                Some(value.to_string())
            } else {
                None
            }
        })
        .next()
}

impl TokenSink for ADFBuilder {
    type Handle = ();

    fn process_token(&self, token: Token, _line_number: u64) -> TokenSinkResult<Self::Handle> {
        let mut state = self.state.borrow_mut();
        match token {
            Token::TagToken(Tag {
                kind: TagKind::StartTag,
                name,
                attrs,
                self_closing,
            }) => match (name.as_ref(), self_closing) {
                ("p", false) => {
                    Self::flush_text(&mut state);
                    state.stack.push(BlockContext::Paragraph(vec![]));
                }
                ("pre", false) => {
                    Self::flush_text(&mut state);
                    state.stack.push(BlockContext::CodeBlock(vec![]));
                }
                ("blockquote", false) => {
                    Self::flush_text(&mut state);
                    state.stack.push(BlockContext::Blockquote(vec![]));
                }
                ("ul", false) => {
                    Self::flush_text(&mut state);
                    state.stack.push(BlockContext::BulletList(vec![]));
                }
                ("ol", false) => {
                    Self::flush_text(&mut state);
                    state.stack.push(BlockContext::OrderedList(vec![]));
                }
                ("li", false) => {
                    Self::flush_text(&mut state);
                    state.stack.push(BlockContext::ListItem(vec![]));
                }
                ("br", true) => {
                    Self::flush_text_and_push_inline(&mut state, AdfNode::HardBreak);
                }
                ("hr", true) => {
                    Self::flush_text(&mut state);
                    // Close any open block contexts that cannot contain Rule
                    while matches!(
                        state.stack.last(),
                        Some(
                            BlockContext::Paragraph(_)
                                | BlockContext::ListItem(_)
                                | BlockContext::Blockquote(_)
                                | BlockContext::TableCell(_)
                                | BlockContext::TableHeader(_)
                        )
                    ) {
                        Self::close_current_block(&mut state);
                    }
                    Self::push_block_to_parent(&mut state, AdfNode::Rule);
                }
                ("em", false) => {
                    Self::flush_text(&mut state);
                    state.mark_stack.push(AdfMark::Em);
                }
                ("strong", false) => {
                    Self::flush_text(&mut state);
                    state.mark_stack.push(AdfMark::Strong);
                }
                ("code", false) => {
                    Self::flush_text(&mut state);
                    let in_pre = state
                        .stack
                        .iter()
                        .any(|ctx| matches!(ctx, BlockContext::CodeBlock(_)));
                    if !in_pre {
                        state.mark_stack.push(AdfMark::Code);
                    }
                    // If inside <pre>, do nothing (handled purely as block)
                }
                ("del", false) => {
                    Self::flush_text(&mut state);
                    state.mark_stack.push(AdfMark::Strike);
                }
                ("a", false) => {
                    Self::flush_text(&mut state);
                    if let Some(href) = attrs.iter().find(|attr| attr.name.local.as_ref() == "href")
                    {
                        let mark = AdfMark::Link {
                            collection: None,
                            href: href.value.to_string(),
                            id: None,
                            occurrence_key: None,
                            title: None,
                        };
                        state.mark_stack.push(mark);
                    }
                }
                ("sub", false) => {
                    Self::flush_text(&mut state);
                    state
                        .mark_stack
                        .push(AdfMark::Subsup { type_: Subsup::Sub });
                }
                ("sup", false) => {
                    Self::flush_text(&mut state);
                    state
                        .mark_stack
                        .push(AdfMark::Subsup { type_: Subsup::Sup });
                }
                ("u", false) => {
                    Self::flush_text(&mut state);
                    state.mark_stack.push(AdfMark::Underline);
                }
                ("span", false) | ("div", false) => {
                    // Check style for color or background-color
                    Self::flush_text(&mut state);
                    if let Some(style_attr) = attrs
                        .iter()
                        .find(|attr| attr.name.local.as_ref() == "style")
                    {
                        let style = style_attr.value.to_ascii_lowercase();
                        if let Some(color) = extract_css_color(&style, "color") {
                            state.mark_stack.push(AdfMark::TextColor { color });
                        }
                        if let Some(bg) = extract_css_color(&style, "background-color") {
                            state
                                .mark_stack
                                .push(AdfMark::BackgroundColor { color: bg });
                        }
                    } else {
                        Self::flush_text(&mut state);
                        state.stack.push(BlockContext::Paragraph(vec![]));
                    }
                }
                ("table", false) => {
                    Self::flush_text(&mut state);
                    state.stack.push(BlockContext::Table(vec![]));
                }
                ("tr", false) => {
                    Self::flush_text(&mut state);
                    state.stack.push(BlockContext::TableRow(vec![]));
                }
                ("td", false) => {
                    Self::flush_text(&mut state);
                    state.stack.push(BlockContext::TableCell(vec![]));
                }
                ("th", false) => {
                    Self::flush_text(&mut state);
                    state.stack.push(BlockContext::TableHeader(vec![]));
                }
                _ => {}
            },
            Token::TagToken(Tag {
                kind: TagKind::EndTag,
                name,
                ..
            }) => match name.as_ref() {
                "p" | "pre" | "blockquote" => {
                    Self::flush_text(&mut state);
                    Self::close_current_block(&mut state);
                }
                "ul" | "ol" => {
                    Self::flush_text(&mut state);
                    Self::close_current_block(&mut state);
                }
                "li" => {
                    Self::flush_text(&mut state);
                    Self::close_current_list_item(&mut state);
                }
                "tr" => {
                    Self::flush_text(&mut state);
                    Self::close_current_table_row(&mut state);
                }
                "td" => {
                    Self::flush_text(&mut state);
                    Self::close_current_table_cell(&mut state);
                }
                "th" => {
                    Self::flush_text(&mut state);
                    Self::close_current_table_header(&mut state);
                }
                "table" => {
                    Self::flush_text(&mut state);
                    if let Some(BlockContext::Table(rows)) = state.stack.pop() {
                        Self::push_block_to_parent(
                            &mut state,
                            AdfNode::Table {
                                attrs: None,
                                content: rows,
                            },
                        );
                    }
                }
                "code" => {
                    Self::flush_text(&mut state);
                    let in_pre = state
                        .stack
                        .iter()
                        .any(|ctx| matches!(ctx, BlockContext::CodeBlock(_)));
                    if !in_pre {
                        Self::pop_mark(&mut state, |m| matches!(m, AdfMark::Code));
                    }
                    // Inside <pre>, no mark to pop
                }
                "span" | "div" => {
                    Self::flush_text(&mut state);
                    // Pop all marks that might have been added by style (need robust tracking, for now pop both if present)
                    if let Some(last) = state.mark_stack.last() {
                        match last {
                            AdfMark::TextColor { .. } | AdfMark::BackgroundColor { .. } => {
                                state.mark_stack.pop();
                                // Attempt to pop again if both present on same tag
                                if let Some(second) = state.mark_stack.last() {
                                    match second {
                                        AdfMark::TextColor { .. }
                                        | AdfMark::BackgroundColor { .. } => {
                                            state.mark_stack.pop();
                                        }
                                        _ => {}
                                    }
                                }
                            }
                            _ => {}
                        }
                    } else {
                        Self::flush_text(&mut state);
                        Self::close_current_block(&mut state);
                    }
                }
                "em" | "strong" | "del" | "a" | "u" | "sub" | "sup" => {
                    Self::flush_text(&mut state);
                    state.mark_stack.pop();
                }
                _ => {}
            },
            Token::CharacterTokens(t) => {
                state.current_text.push_str(&t);
            }
            _ => {}
        }
        TokenSinkResult::Continue
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use html5ever::tendril::Tendril;
    use html5ever::tokenizer::{BufferQueue, Tokenizer, TokenizerOpts};

    use crate::adf_types::AdfNode;

    fn parse_html(input: &str) -> AdfNode {
        let mut queue: BufferQueue = Default::default();
        queue.push_back(Tendril::from_slice(input));

        let builder = ADFBuilder::new();
        let tok = Tokenizer::new(builder, TokenizerOpts::default());

        while !queue.is_empty() {
            let _ = tok.feed(&mut queue);
        }
        tok.end();
        tok.sink.emit()
    }

    fn assert_content_eq(adf: AdfNode, expected: Vec<AdfNode>) {
        assert_eq!(
            adf,
            AdfNode::Doc {
                content: expected,
                version: 1
            }
        );
    }

    #[test]
    fn test_blockquote() {
        let adf = parse_html(r#"<blockquote>Quoted text.</blockquote>"#);
        assert_content_eq(
            adf,
            vec![AdfNode::Blockquote {
                content: vec![AdfNode::Text {
                    text: "Quoted text.".into(),
                    marks: None,
                }],
            }],
        );
    }

    #[test]
    fn test_bullet_list_with_list_items() {
        let adf = parse_html(r#"<ul><li>Item one</li><li>Item two</li></ul>"#);
        assert_content_eq(
            adf,
            vec![AdfNode::BulletList {
                content: vec![
                    AdfNode::ListItem {
                        content: vec![AdfNode::Text {
                            text: "Item one".into(),
                            marks: None,
                        }],
                    },
                    AdfNode::ListItem {
                        content: vec![AdfNode::Text {
                            text: "Item two".into(),
                            marks: None,
                        }],
                    },
                ],
            }],
        );
    }

    #[test]
    fn test_ordered_list_with_list_items() {
        let adf = parse_html(r#"<ol><li>Item one</li><li>Item two</li></ol>"#);
        assert_content_eq(
            adf,
            vec![AdfNode::OrderedList {
                attrs: None,
                content: vec![
                    AdfNode::ListItem {
                        content: vec![AdfNode::Text {
                            text: "Item one".into(),
                            marks: None,
                        }],
                    },
                    AdfNode::ListItem {
                        content: vec![AdfNode::Text {
                            text: "Item two".into(),
                            marks: None,
                        }],
                    },
                ],
            }],
        );
    }

    #[test]
    fn test_combined_marks_splitting() {
        let adf = parse_html(
            r#"<div>Some text <a href="https://www.example.com">examples</a> are <strong><em>complicated</em></strong></div>"#,
        );
        assert_content_eq(
            adf,
            vec![AdfNode::Paragraph {
                content: Some(vec![
                    AdfNode::Text {
                        text: "Some text ".into(),
                        marks: None,
                    },
                    AdfNode::Text {
                        text: "examples".into(),
                        marks: Some(vec![AdfMark::Link {
                            collection: None,
                            href: "https://www.example.com".into(),
                            id: None,
                            occurrence_key: None,
                            title: None,
                        }]),
                    },
                    AdfNode::Text {
                        text: " are ".into(),
                        marks: None,
                    },
                    AdfNode::Text {
                        text: "complicated".into(),
                        marks: Some(vec![AdfMark::Strong, AdfMark::Em]),
                    },
                ]),
            }],
        );
    }

    #[test]
    fn test_subsup_underline() {
        let adf = parse_html(
            r#"<p>This is <u>underlined</u> and <sub>subscript</sub> and <sup>superscript</sup>.</p>"#,
        );
        assert_content_eq(
            adf,
            vec![AdfNode::Paragraph {
                content: Some(vec![
                    AdfNode::Text {
                        text: "This is ".into(),
                        marks: None,
                    },
                    AdfNode::Text {
                        text: "underlined".into(),
                        marks: Some(vec![AdfMark::Underline]),
                    },
                    AdfNode::Text {
                        text: " and ".into(),
                        marks: None,
                    },
                    AdfNode::Text {
                        text: "subscript".into(),
                        marks: Some(vec![AdfMark::Subsup { type_: Subsup::Sub }]),
                    },
                    AdfNode::Text {
                        text: " and ".into(),
                        marks: None,
                    },
                    AdfNode::Text {
                        text: "superscript".into(),
                        marks: Some(vec![AdfMark::Subsup { type_: Subsup::Sup }]),
                    },
                    AdfNode::Text {
                        text: ".".into(),
                        marks: None,
                    },
                ]),
            }],
        );
    }

    #[test]
    fn test_span_styles() {
        let adf = parse_html(
            r#"<p><span style="color: red">red text</span> and <span style="background-color: yellow">yellow background</span>.</p>"#,
        );
        assert_content_eq(
            adf,
            vec![AdfNode::Paragraph {
                content: Some(vec![
                    AdfNode::Text {
                        text: "red text".into(),
                        marks: Some(vec![AdfMark::TextColor {
                            color: "red".into(),
                        }]),
                    },
                    AdfNode::Text {
                        text: " and ".into(),
                        marks: None,
                    },
                    AdfNode::Text {
                        text: "yellow background".into(),
                        marks: Some(vec![AdfMark::BackgroundColor {
                            color: "yellow".into(),
                        }]),
                    },
                    AdfNode::Text {
                        text: ".".into(),
                        marks: None,
                    },
                ]),
            }],
        );
    }

    #[test]
    fn test_span_combined_styles() {
        let adf = parse_html(
            r#"<p><span style="color: green; background-color: black">styled text</span></p>"#,
        );
        assert_content_eq(
            adf,
            vec![AdfNode::Paragraph {
                content: Some(vec![AdfNode::Text {
                    text: "styled text".into(),
                    marks: Some(vec![
                        AdfMark::TextColor {
                            color: "green".into(),
                        },
                        AdfMark::BackgroundColor {
                            color: "black".into(),
                        },
                    ]),
                }]),
            }],
        );
    }

    #[test]
    fn test_code_inside_pre_and_outside_pre() {
        let adf = parse_html(
            r#"<pre><code>let x = 42;</code></pre><p>This is <code>inline code</code>.</p>"#,
        );
        assert_content_eq(
            adf,
            vec![
                AdfNode::CodeBlock {
                    content: Some(vec![AdfNode::Text {
                        text: "let x = 42;".into(),
                        marks: None,
                    }]),
                    attrs: None,
                },
                AdfNode::Paragraph {
                    content: Some(vec![
                        AdfNode::Text {
                            text: "This is ".into(),
                            marks: None,
                        },
                        AdfNode::Text {
                            text: "inline code".into(),
                            marks: Some(vec![AdfMark::Code]),
                        },
                        AdfNode::Text {
                            text: ".".into(),
                            marks: None,
                        },
                    ]),
                },
            ],
        );
    }

    #[test]
    fn test_html_table_parsing() {
        let adf = parse_html(
            r#"
            <table>
                <tr>
                    <th>Header 1</th>
                    <th>Header 2</th>
                </tr>
                <tr>
                    <td>Cell 1</td>
                    <td></td>
                </tr>
                <tr>
                    <td>
                        <p>Nested paragraph</p>
                        <blockquote>Blockquote inside cell</blockquote>
                    </td>
                    <td>Simple text</td>
                </tr>
            </table>
        "#,
        );

        assert_content_eq(
            adf,
            vec![AdfNode::Table {
                attrs: None,
                content: vec![
                    AdfNode::TableRow {
                        content: vec![
                            AdfNode::TableHeader {
                                attrs: None,
                                content: vec![AdfNode::Text {
                                    text: "Header 1".into(),
                                    marks: None,
                                }],
                            },
                            AdfNode::TableHeader {
                                attrs: None,
                                content: vec![AdfNode::Text {
                                    text: "Header 2".into(),
                                    marks: None,
                                }],
                            },
                        ],
                    },
                    AdfNode::TableRow {
                        content: vec![
                            AdfNode::TableCell {
                                attrs: None,
                                content: vec![AdfNode::Text {
                                    text: "Cell 1".into(),
                                    marks: None,
                                }],
                            },
                            AdfNode::TableCell {
                                attrs: None,
                                content: vec![], // empty cell
                            },
                        ],
                    },
                    AdfNode::TableRow {
                        content: vec![
                            AdfNode::TableCell {
                                attrs: None,
                                content: vec![
                                    AdfNode::Paragraph {
                                        content: Some(vec![AdfNode::Text {
                                            text: "Nested paragraph".into(),
                                            marks: None,
                                        }]),
                                    },
                                    AdfNode::Blockquote {
                                        content: vec![AdfNode::Text {
                                            text: "Blockquote inside cell".into(),
                                            marks: None,
                                        }],
                                    },
                                ],
                            },
                            AdfNode::TableCell {
                                attrs: None,
                                content: vec![AdfNode::Text {
                                    text: "Simple text".into(),
                                    marks: None,
                                }],
                            },
                        ],
                    },
                ],
            }],
        );
    }

    #[test]
    fn test_br_inside_paragraph() {
        let adf = parse_html(r#"<p>First line<br/>Second line</p>"#);
        assert_content_eq(
            adf,
            vec![AdfNode::Paragraph {
                content: Some(vec![
                    AdfNode::Text {
                        text: "First line".into(),
                        marks: None,
                    },
                    AdfNode::HardBreak,
                    AdfNode::Text {
                        text: "Second line".into(),
                        marks: None,
                    },
                ]),
            }],
        );
    }

    #[test]
    fn test_hr_between_paragraphs() {
        let adf = parse_html(r#"<p>Before rule</p><hr/><p>After rule</p>"#);
        assert_content_eq(
            adf,
            vec![
                AdfNode::Paragraph {
                    content: Some(vec![AdfNode::Text {
                        text: "Before rule".into(),
                        marks: None,
                    }]),
                },
                AdfNode::Rule,
                AdfNode::Paragraph {
                    content: Some(vec![AdfNode::Text {
                        text: "After rule".into(),
                        marks: None,
                    }]),
                },
            ],
        );
    }
}
