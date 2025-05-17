use std::cell::RefCell;
use std::collections::HashMap;

use html5ever::tendril::Tendril;
use html5ever::tokenizer::{
    BufferQueue, Tag, TagKind, Token, TokenSink, TokenSinkResult, Tokenizer, TokenizerOpts,
};

use crate::adf::adf_types::{AdfMark, AdfNode, DecisionItemAttrs, LocalId, TaskItemAttrs};
use crate::handlers::*;

/// Cleans surrounding text by removing leading and trailing whitespace before and after newlines
fn clean_surrounding_text(text: &str) -> &str {
    let chars: Vec<_> = text.char_indices().collect();
    let len = text.len();

    // From left
    let mut start = 0;
    for (i, c) in &chars {
        if *c == '\n' {
            start = i + c.len_utf8();
            break;
        } else if !c.is_whitespace() {
            start = 0;
            break;
        }
    }

    // From right
    let mut end = len;
    for (i, c) in chars.iter().rev() {
        if *c == '\n' {
            end = *i;
            break;
        } else if !c.is_whitespace() {
            end = len;
            break;
        }
    }

    if start > end { "" } else { &text[start..end] }
}

pub type HandlerFn = Box<dyn Fn(&mut ADFBuilderState, Element) -> bool>;

pub struct ADFBuilder {
    state: RefCell<ADFBuilderState>,
    custom_start_handlers: HashMap<String, HandlerFn>,
    start_handlers: HashMap<String, HandlerFn>,
    custom_end_handlers: HashMap<String, HandlerFn>,
    end_handlers: HashMap<String, HandlerFn>,
}

impl ADFBuilder {
    pub fn new() -> Self {
        let mut this = Self {
            state: RefCell::new(ADFBuilderState {
                stack: vec![BlockContext::Document(vec![])],
                mark_stack: vec![],
                current_text: String::new(),
                custom_block_id: None,
                custom_block_tag: None,
            }),
            start_handlers: HashMap::new(),
            custom_start_handlers: HashMap::new(),
            end_handlers: HashMap::new(),
            custom_end_handlers: HashMap::new(),
        };

        this.insert_start_handler("ul", ul_start_handler());
        this.insert_end_handler("ul", ul_end_handler());

        this.insert_start_handler("ol", ol_start_handler());
        this.insert_end_handler("ol", ol_end_handler());

        this.insert_start_handler("li", li_start_handler());
        this.insert_end_handler("li", li_end_handler());

        this.insert_start_handler("p", p_start_handler());
        this.insert_end_handler("p", p_end_handler());

        this.insert_start_handler("pre", pre_start_handler());
        this.insert_end_handler("pre", pre_end_handler());

        this.insert_start_handler("blockquote", blockquote_start_handler());
        this.insert_end_handler("blockquote", blockquote_end_handler());

        this.insert_start_handler("em", em_start_handler());
        this.insert_start_handler("strong", strong_start_handler());
        this.insert_start_handler("del", del_start_handler());
        this.insert_start_handler("a", a_start_handler());
        this.insert_start_handler("u", u_start_handler());
        this.insert_start_handler("sub", sub_start_handler());
        this.insert_start_handler("sup", sup_start_handler());

        // For all mark tags use same generic mark handler
        for tag in &["em", "strong", "del", "a", "u", "sub", "sup"] {
            this.insert_end_handler(tag, mark_end_handler());
        }

        for i in 1..=6 {
            let tag = format!("h{}", i);
            this.insert_start_handler(&tag, header_start_handler(i));
            this.insert_end_handler(&tag, header_end_handler(i));
        }

        this.insert_start_handler("table", table_start_handler());
        this.insert_end_handler("table", table_end_handler());
        this.insert_start_handler("th", table_header_start_handler());
        this.insert_end_handler("th", table_header_end_handler());
        this.insert_start_handler("tr", table_row_start_handler());
        this.insert_end_handler("tr", table_row_end_handler());
        this.insert_start_handler("thead", table_section_start_handler());
        this.insert_end_handler("thead", table_section_end_handler());
        this.insert_start_handler("tbody", table_section_start_handler());
        this.insert_end_handler("tbody", table_section_end_handler());
        this.insert_start_handler("td", table_cell_start_handler());
        this.insert_end_handler("td", table_cell_end_handler());

        this.insert_start_handler("br", hard_break_start_handler());
        this.insert_start_handler("hr", rule_start_handler());

        this.insert_start_handler("code", code_start_handler());
        this.insert_end_handler("code", code_end_handler());
        this.insert_start_handler("div", div_start_handler());
        this.insert_end_handler("div", div_end_handler());

        this.insert_start_handler("span", span_start_handler());
        this.insert_end_handler("span", mark_end_handler());

        this.insert_start_handler("time", date_start_handler());
        this.insert_end_handler("time", date_end_handler());

        this.insert_start_handler("details", details_start_handler());
        this.insert_end_handler("details", details_end_handler());

        this.insert_start_handler("figure", figure_start_handler());
        this.insert_end_handler("figure", figure_end_handler());

        this.insert_end_handler("summary", summary_end_handler());

        this.insert_start_handler("adf-task-item", task_item_start_handler());
        this.insert_start_handler("adf-local-data", local_data_start_handler());

        this.insert_start_handler("adf-status", status_start_handler());
        this.insert_end_handler("adf-status", status_end_handler());

        this.insert_start_handler("adf-emoji", emoji_start_handler());
        this.insert_end_handler("adf-emoji", emoji_end_handler());

        this.insert_start_handler("adf-mention", mention_start_handler());
        this.insert_end_handler("adf-mention", mention_end_handler());

        this.insert_start_handler("adf-decision-item", decision_start_handler());
        this.insert_end_handler("adf-decision-item", decision_end_handler());

        this.insert_start_handler("adf-media-single", media_single_start_handler());
        this.insert_end_handler("adf-media-single", media_single_end_handler());

        this.insert_start_handler("adf-media-group", media_group_start_handler());
        this.insert_end_handler("adf-media-group", media_group_end_handler());

        // Custom handlers
        this.add_start_handler("a", media_and_inline_card_start_handler());
        this.add_start_handler("img", media_and_inline_card_start_handler());
        this.add_end_handler("a", inline_card_end_handler());

        this
    }

    fn insert_start_handler(
        &mut self,
        tag: &str,
        handler: impl Fn(&mut ADFBuilderState, Element) -> bool + 'static,
    ) {
        self.start_handlers
            .insert(tag.to_string(), Box::new(handler));
    }

    fn insert_end_handler(
        &mut self,
        tag: &str,
        handler: impl Fn(&mut ADFBuilderState, Element) -> bool + 'static,
    ) {
        self.end_handlers.insert(tag.to_string(), Box::new(handler));
    }

    pub fn add_start_handler(
        &mut self,
        tag: &str,
        handler: impl Fn(&mut ADFBuilderState, Element) -> bool + 'static,
    ) {
        self.custom_start_handlers
            .insert(tag.to_string(), Box::new(handler));
    }

    pub fn add_end_handler(
        &mut self,
        tag: &str,
        handler: impl Fn(&mut ADFBuilderState, Element) -> bool + 'static,
    ) {
        self.custom_end_handlers
            .insert(tag.to_string(), Box::new(handler));
    }

    pub fn flush_text(state: &mut ADFBuilderState) {
        if !state.current_text.is_empty() {
            let mut text = std::mem::take(&mut state.current_text);

            // Always trim block contexts (safe for all known block types)
            let trim_for_blocks = matches!(
                state.stack.last(),
                Some(
                    BlockContext::Heading(_, _)
                        | BlockContext::Paragraph(_)
                        | BlockContext::TableBlock(TableBlockType::Cell, _)
                        | BlockContext::TableBlock(TableBlockType::Header, _)
                        | BlockContext::Blockquote(_)
                        | BlockContext::ListItem(_)
                )
            );

            if trim_for_blocks {
                text = clean_surrounding_text(&text).to_string();
            }

            if text.trim().is_empty() {
                return;
            }

            let marks = if state.mark_stack.is_empty() {
                None
            } else {
                Some(state.mark_stack.clone())
            };

            if let Some(frame) = state.stack.last_mut() {
                match frame {
                    BlockContext::Paragraph(nodes)
                    | BlockContext::Heading(_, nodes)
                    | BlockContext::Blockquote(nodes)
                    | BlockContext::ListItem(nodes)
                    | BlockContext::TableBlock(TableBlockType::Cell, nodes)
                    | BlockContext::TableBlock(TableBlockType::Header, nodes) => {
                        let node = AdfNode::Text {
                            text: text.clone(),
                            marks,
                        };
                        nodes.push(node);
                    }
                    BlockContext::TaskItem(nodes, _, _) => {
                        let node = AdfNode::Text {
                            text: text.trim().to_string(),
                            marks,
                        };
                        nodes.push(node);
                    }
                    BlockContext::DecisionItem(nodes, _) => {
                        let node = AdfNode::Text {
                            text: text.trim().to_string(),
                            marks,
                        };
                        nodes.push(node);
                    }
                    BlockContext::CodeBlock(lines) => {
                        if let Some(stripped) = text.strip_suffix('\n') {
                            text = stripped.to_string();
                        }
                        lines.push(text);
                    }
                    _ => {}
                }
            }
        }
    }

    pub fn pop_mark(state: &mut ADFBuilderState, pred: impl Fn(&AdfMark) -> bool) {
        if let Some(pos) = state.mark_stack.iter().rposition(pred) {
            state.mark_stack.remove(pos);
        }
    }

    fn flatten_top_level_paragraph(nodes: Vec<AdfNode>, parent_nodes: &mut Vec<AdfNode>) {
        if !nodes.is_empty() {
            if nodes.iter().all(|n| n.is_top_level_block()) {
                parent_nodes.extend(nodes);
            } else {
                parent_nodes.push(AdfNode::Paragraph {
                    content: Some(nodes),
                });
            }
        }
    }

    pub fn close_current_block(state: &mut ADFBuilderState) {
        let frame = state.stack.pop().expect("Expected a block context");
        let mut parent = state
            .stack
            .last_mut()
            .expect("Document should always be present");
        match frame {
            BlockContext::Paragraph(nodes) => match &mut parent {
                BlockContext::Document(parent_nodes)
                | BlockContext::TableBlock(TableBlockType::Cell, parent_nodes)
                | BlockContext::TableBlock(TableBlockType::Header, parent_nodes)
                | BlockContext::Blockquote(parent_nodes)
                | BlockContext::ListItem(parent_nodes) => {
                    Self::flatten_top_level_paragraph(nodes, parent_nodes);
                }
                BlockContext::CustomBlock(block_ty, parent_nodes, _) => match block_ty {
                    CustomBlockType::Div
                    | CustomBlockType::Expand
                    | CustomBlockType::NestedExpand
                    | CustomBlockType::Panel => {
                        Self::flatten_top_level_paragraph(nodes, parent_nodes);
                    }
                    parent => {
                        panic!("Invalid parent for Paragraph: {parent:?}");
                    }
                },
                parent => panic!("Invalid parent for Paragraph: {parent:?}"),
            },
            BlockContext::CustomBlock(CustomBlockType::Expand, nodes, _) => match parent {
                BlockContext::Document(parent_nodes)
                | BlockContext::TableBlock(TableBlockType::Cell, parent_nodes)
                | BlockContext::TableBlock(TableBlockType::Header, parent_nodes)
                | BlockContext::ListItem(parent_nodes)
                | BlockContext::Blockquote(parent_nodes) => {
                    Self::flatten_top_level_paragraph(nodes, parent_nodes);
                }
                BlockContext::CustomBlock(block_ty, parent_nodes, _) => match block_ty {
                    CustomBlockType::Div
                    | CustomBlockType::Expand
                    | CustomBlockType::NestedExpand
                    | CustomBlockType::Panel => {
                        Self::flatten_top_level_paragraph(nodes, parent_nodes);
                    }
                    parent => {
                        panic!("Invalid parent for Paragraph: {parent:?}");
                    }
                },
                _ => panic!("Invalid parent for CustomBlock"),
            },
            BlockContext::CodeBlock(lines) => match parent {
                BlockContext::Document(parent_nodes)
                | BlockContext::TableBlock(TableBlockType::Cell, parent_nodes)
                | BlockContext::TableBlock(TableBlockType::Header, parent_nodes)
                | BlockContext::ListItem(parent_nodes)
                | BlockContext::Blockquote(parent_nodes)
                | BlockContext::CustomBlock(CustomBlockType::Div, parent_nodes, _) => {
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
                | BlockContext::TableBlock(TableBlockType::Cell, parent_nodes)
                | BlockContext::TableBlock(TableBlockType::Header, parent_nodes)
                | BlockContext::ListItem(parent_nodes)
                | BlockContext::CustomBlock(CustomBlockType::Div, parent_nodes, _) => {
                    parent_nodes.push(AdfNode::Blockquote { content: nodes })
                }
                _ => panic!("Invalid parent for Blockquote"),
            },
            BlockContext::PendingList {
                nodes,
                ordered,
                local_id,
                local_tag,
            } => match parent {
                BlockContext::Document(parent_nodes)
                | BlockContext::CustomBlock(CustomBlockType::Div, parent_nodes, _)
                | BlockContext::Paragraph(parent_nodes)
                | BlockContext::Blockquote(parent_nodes)
                | BlockContext::TableBlock(TableBlockType::Cell, parent_nodes)
                | BlockContext::TableBlock(TableBlockType::Header, parent_nodes)
                | BlockContext::ListItem(parent_nodes) => {
                    let is_task_list = local_tag
                        .as_ref()
                        .map(|tag| tag == "task-list")
                        .unwrap_or(false);
                    let is_decision_list = local_tag
                        .as_ref()
                        .map(|tag| tag == "decision-list")
                        .unwrap_or(false);
                    if is_task_list {
                        parent_nodes.push(AdfNode::TaskList {
                            attrs: LocalId {
                                local_id: local_id.unwrap_or_default(),
                            },
                            content: nodes,
                        });
                    } else if is_decision_list {
                        parent_nodes.push(AdfNode::DecisionList {
                            content: nodes,
                            attrs: LocalId {
                                local_id: local_id.unwrap_or_default(),
                            },
                        });
                    } else if ordered {
                        parent_nodes.push(AdfNode::OrderedList {
                            content: nodes,
                            attrs: None,
                        });
                    } else {
                        parent_nodes.push(AdfNode::BulletList { content: nodes });
                    }
                }
                parent => panic!("Invalid parent for PendingList: {parent:?}"),
            },
            block => {
                panic!(
                    "{block:?} closed incorrectly; must use block-specific close method: {parent:?}"
                );
            }
        }
    }

    pub fn close_current_list_item(state: &mut ADFBuilderState) {
        ADFBuilder::flush_text(state);
        let stack_item = state.stack.pop();
        if let Some(BlockContext::ListItem(nodes)) = stack_item {
            match state.stack.last_mut() {
                Some(BlockContext::PendingList { nodes: list, .. }) => {
                    list.push(AdfNode::ListItem { content: nodes });
                }
                _ => {
                    panic!("ListItem closed without PendingList parent");
                }
            }
        } else if let Some(BlockContext::TaskItem(nodes, item_state, local_id)) = stack_item {
            if let Some(BlockContext::PendingList { nodes: list, .. }) = state.stack.last_mut() {
                list.push(AdfNode::TaskItem {
                    content: nodes,
                    attrs: TaskItemAttrs {
                        local_id,
                        state: item_state,
                    },
                });
            } else {
                panic!("TaskItem closed without PendingList parent");
            }
        } else if let Some(BlockContext::DecisionItem(nodes, local_id)) = stack_item {
            if let Some(BlockContext::PendingList { nodes: list, .. }) = state.stack.last_mut() {
                list.push(AdfNode::DecisionItem {
                    content: nodes,
                    attrs: DecisionItemAttrs {
                        local_id,
                        state: "DECIDED".to_string(),
                    },
                });
            }
        } else {
            panic!("Invalid context for ListItem close");
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

    pub fn flush_text_and_push_inline(state: &mut ADFBuilderState, node: AdfNode) {
        Self::flush_text(state);
        Self::push_inline(state, node);
    }

    pub fn push_block_to_parent(state: &mut ADFBuilderState, node: AdfNode) {
        let frame = state
            .stack
            .last_mut()
            .expect("There should always be at least the Document node");
        match frame {
            BlockContext::TableBlock(_, nodes)
            | BlockContext::Paragraph(nodes)
            | BlockContext::Blockquote(nodes)
            | BlockContext::Heading(_, nodes)
            | BlockContext::ListItem(nodes)
            | BlockContext::Document(nodes) => nodes.push(node),
            BlockContext::CustomBlock(block_ty, nodes, _) => match block_ty {
                CustomBlockType::Div | CustomBlockType::Expand | CustomBlockType::Panel => {
                    nodes.push(node);
                }
                _ => panic!("Invalid block context for custom block: {block_ty:?} {node:?}"),
            },
            frame => {
                panic!("Invalid block context for block node: {frame:?} {node:?}");
            }
        }
    }

    pub fn extract_text(paragraph: &AdfNode) -> String {
        match paragraph {
            AdfNode::Paragraph {
                content: Some(nodes),
            } => nodes
                .iter()
                .filter_map(|n| match n {
                    AdfNode::Text { text, .. } => Some(text.clone()),
                    _ => None,
                })
                .collect::<String>(),
            _ => String::new(),
        }
    }
}

pub fn extract_style(style: &str, property: &str) -> Option<String> {
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
            }) => {
                if let Some(handler) = self.custom_start_handlers.get(name.as_ref()) {
                    let element = Element {
                        tag: name.to_string(),
                        attrs: attrs.clone(),
                        self_closing,
                    };
                    if handler(&mut state, element) {
                        return TokenSinkResult::Continue;
                    }
                }

                if let Some(handler) = self.start_handlers.get(name.as_ref()) {
                    let element = Element {
                        tag: name.to_string(),
                        attrs: attrs.clone(),
                        self_closing,
                    };
                    handler(&mut state, element);
                }
            }
            Token::TagToken(Tag {
                kind: TagKind::EndTag,
                name,
                attrs,
                self_closing,
            }) => {
                if let Some(handler) = self.custom_end_handlers.get(name.as_ref()) {
                    let element = Element {
                        tag: name.to_string(),
                        attrs: attrs.clone(),
                        self_closing,
                    };
                    if handler(&mut state, element) {
                        return TokenSinkResult::Continue;
                    }
                }

                if let Some(handler) = self.end_handlers.get(name.as_ref()) {
                    handler(
                        &mut state,
                        Element {
                            tag: name.to_string(),
                            attrs,
                            self_closing,
                        },
                    );
                }
            }
            Token::CharacterTokens(t) => {
                state.current_text.push_str(&t);
            }
            _ => {}
        }
        TokenSinkResult::Continue
    }
}

pub fn html_to_adf(input: &str) -> AdfNode {
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

#[cfg(test)]
mod tests {
    use super::*;

    use crate::adf::adf_types::{AdfNode, HeadingAttrs, LinkMark, Subsup};

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
    fn test_clean_surrounding_text() {
        assert_eq!(
            clean_surrounding_text("\n  Heading 1  \n  "),
            "  Heading 1  "
        );
        assert_eq!(clean_surrounding_text("\nHeading 1"), "Heading 1");
        assert_eq!(clean_surrounding_text("Heading 1\n  "), "Heading 1");
        assert_eq!(clean_surrounding_text("Heading 1"), "Heading 1");
        assert_eq!(clean_surrounding_text("   Heading 1   "), "   Heading 1   ");
        assert_eq!(
            clean_surrounding_text("\n   Heading 1   "),
            "   Heading 1   "
        );
        assert_eq!(
            clean_surrounding_text("   Heading 1   \n"),
            "   Heading 1   "
        );
    }

    #[test]
    fn test_blockquote() {
        let adf = html_to_adf(r#"<blockquote>Quoted text.</blockquote>"#);
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
        let adf = html_to_adf(r#"<ul><li>Item one</li><li>Item two</li></ul>"#);
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
        let adf = html_to_adf(r#"<ol><li>Item one</li><li>Item two</li></ol>"#);
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
        let adf = html_to_adf(
            r#"<p>Some text <a href="https://www.example.com">examples</a> are <strong><em>complicated</em></strong></p>"#,
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
                        marks: Some(vec![AdfMark::Link(LinkMark {
                            href: "https://www.example.com".into(),
                            ..Default::default()
                        })]),
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
        let adf = html_to_adf(
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
        let adf = html_to_adf(
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
    fn test_code_inside_pre_and_outside_pre() {
        let adf = html_to_adf(
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
        let adf = html_to_adf(
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
        let adf = html_to_adf(r#"<p>First line<br/>Second line</p>"#);
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
        let adf = html_to_adf(r#"<p>Before rule</p><hr/><p>After rule</p>"#);
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

    #[test]
    fn test_headings_parsing() {
        let adf = html_to_adf(
            r#"
            <h1>Main Heading</h1>
            <h2>Sub Heading</h2>
            <h3><em>Marked</em> heading</h3>
        "#,
        );

        assert_content_eq(
            adf,
            vec![
                AdfNode::Heading {
                    attrs: HeadingAttrs { level: 1 },
                    content: Some(vec![AdfNode::Text {
                        text: "Main Heading".into(),
                        marks: None,
                    }]),
                },
                AdfNode::Heading {
                    attrs: HeadingAttrs { level: 2 },
                    content: Some(vec![AdfNode::Text {
                        text: "Sub Heading".into(),
                        marks: None,
                    }]),
                },
                AdfNode::Heading {
                    attrs: HeadingAttrs { level: 3 },
                    content: Some(vec![
                        AdfNode::Text {
                            text: "Marked".into(),
                            marks: Some(vec![AdfMark::Em]),
                        },
                        AdfNode::Text {
                            text: " heading".into(),
                            marks: None,
                        },
                    ]),
                },
            ],
        );
    }
}
