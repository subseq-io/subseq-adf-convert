use std::cell::RefCell;
use std::collections::HashMap;

use html5ever::tendril::Tendril;
use html5ever::tokenizer::{
    BufferQueue, Tag, TagKind, Token, TokenSink, TokenSinkResult, Tokenizer, TokenizerOpts,
};

use crate::adf::adf_types::{
    AdfBlockNode, AdfMark, AdfNode, DecisionItem, DecisionItemAttrs, DecisionItemState,
    ExpandAttrs, ListItem, LocalId, TaskItem, TaskItemAttrs,
};
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

        // Noops for handling the section tags
        this.insert_start_handler("thead", table_section_start_handler());
        this.insert_end_handler("thead", table_section_end_handler());
        this.insert_start_handler("tbody", table_section_start_handler());
        this.insert_end_handler("tbody", table_section_end_handler());

        this.insert_start_handler("tr", table_row_start_handler());
        this.insert_end_handler("tr", table_row_end_handler());

        this.insert_start_handler("th", table_header_start_handler());
        this.insert_end_handler("th", table_header_end_handler());
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
        this.insert_start_handler("adf-decision-item", decision_start_handler());
        this.insert_start_handler("adf-local-data", local_data_start_handler());

        this.insert_start_handler("adf-status", status_start_handler());
        this.insert_end_handler("adf-status", status_end_handler());

        this.insert_start_handler("adf-emoji", emoji_start_handler());
        this.insert_end_handler("adf-emoji", emoji_end_handler());

        this.insert_start_handler("adf-mention", mention_start_handler());
        this.insert_end_handler("adf-mention", mention_end_handler());

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

    pub fn push_into_last_paragraph(nodes: &mut Vec<AdfBlockNode>, adf_node: AdfNode) {
        match nodes.last_mut() {
            Some(AdfBlockNode::Paragraph { content }) => {
                if let Some(content) = content {
                    content.push(adf_node);
                } else {
                    *content = Some(vec![adf_node]);
                }
            }
            _ => {
                let paragraph = AdfBlockNode::Paragraph {
                    content: Some(vec![adf_node]),
                };
                nodes.push(paragraph);
            }
        }
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
                        | BlockContext::TableBlockCell(_)
                        | BlockContext::TableBlockHeader(_)
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
                    BlockContext::Paragraph(nodes) | BlockContext::Heading(_, nodes) => {
                        let node = AdfNode::Text {
                            text: text.clone(),
                            marks,
                        };
                        nodes.push(node);
                    }
                    BlockContext::ListItem(nodes)
                    | BlockContext::Blockquote(nodes)
                    | BlockContext::TableBlockHeader(nodes)
                    | BlockContext::TableBlockCell(nodes) => {
                        let node = AdfNode::Text {
                            text: text.clone(),
                            marks,
                        };
                        Self::push_into_last_paragraph(nodes, node);
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

    pub fn close_current_block(state: &mut ADFBuilderState) {
        let frame = state.stack.pop().expect("Expected a block context");
        let mut parent = state
            .stack
            .last_mut()
            .expect("Document should always be present");
        match frame {
            BlockContext::Paragraph(nodes) => match &mut parent {
                BlockContext::Document(parent_nodes)
                | BlockContext::TableBlockCell(parent_nodes)
                | BlockContext::TableBlockHeader(parent_nodes)
                | BlockContext::Blockquote(parent_nodes)
                | BlockContext::ListItem(parent_nodes) => {
                    if nodes.is_empty() {
                        return;
                    }
                    parent_nodes.push(AdfBlockNode::Paragraph {
                        content: Some(nodes),
                    });
                }
                BlockContext::CustomBlock(block_ty, parent_nodes, _) => match block_ty {
                    CustomBlockType::Div
                    | CustomBlockType::Expand
                    | CustomBlockType::NestedExpand
                    | CustomBlockType::Panel => {
                        parent_nodes.push(AdfBlockNode::Paragraph {
                            content: Some(nodes),
                        });
                    }
                    parent => {
                        panic!("Invalid parent for Paragraph: {parent:?}");
                    }
                },
                parent => panic!("Invalid parent for Paragraph: {parent:?}"),
            },
            BlockContext::CustomBlock(CustomBlockType::Expand, nodes, attrs) => match parent {
                BlockContext::Document(parent_nodes)
                | BlockContext::TableBlockCell(parent_nodes)
                | BlockContext::TableBlockHeader(parent_nodes)
                | BlockContext::ListItem(parent_nodes)
                | BlockContext::Blockquote(parent_nodes) => {
                    let title = attrs.get("title").cloned();
                    let expand_attrs = ExpandAttrs { title };

                    parent_nodes.push(AdfBlockNode::Expand {
                        content: nodes,
                        attrs: expand_attrs,
                    });
                }
                BlockContext::CustomBlock(block_ty, parent_nodes, _) => match block_ty {
                    CustomBlockType::Div
                    | CustomBlockType::Expand
                    | CustomBlockType::NestedExpand
                    | CustomBlockType::Panel => {
                        let title = attrs.get("title").cloned();
                        let expand_attrs = ExpandAttrs { title };
                        parent_nodes.push(AdfBlockNode::Expand {
                            content: nodes,
                            attrs: expand_attrs,
                        });
                    }
                    parent => {
                        panic!("Invalid parent for Paragraph: {parent:?}");
                    }
                },
                _ => panic!("Invalid parent for CustomBlock"),
            },
            BlockContext::CodeBlock(lines) => match parent {
                BlockContext::Document(parent_nodes)
                | BlockContext::TableBlockCell(parent_nodes)
                | BlockContext::TableBlockHeader(parent_nodes)
                | BlockContext::ListItem(parent_nodes)
                | BlockContext::Blockquote(parent_nodes)
                | BlockContext::CustomBlock(CustomBlockType::Div, parent_nodes, _) => {
                    let text = lines.join("");
                    parent_nodes.push(AdfBlockNode::CodeBlock {
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
                | BlockContext::TableBlockCell(parent_nodes)
                | BlockContext::TableBlockHeader(parent_nodes)
                | BlockContext::ListItem(parent_nodes)
                | BlockContext::CustomBlock(CustomBlockType::Div, parent_nodes, _) => {
                    parent_nodes.push(AdfBlockNode::Blockquote { content: nodes })
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
                | BlockContext::Blockquote(parent_nodes)
                | BlockContext::TableBlockCell(parent_nodes)
                | BlockContext::TableBlockHeader(parent_nodes)
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
                        let task_list_items = nodes
                            .into_iter()
                            .filter_map(|item| {
                                if let ListItemType::TaskItem(task_item) = item {
                                    Some(task_item)
                                } else {
                                    None
                                }
                            })
                            .collect::<Vec<_>>();

                        parent_nodes.push(AdfBlockNode::TaskList {
                            attrs: LocalId {
                                local_id: local_id.unwrap_or_default(),
                            },
                            content: task_list_items,
                        });
                    } else if is_decision_list {
                        let decision_list_items = nodes
                            .into_iter()
                            .filter_map(|item| {
                                if let ListItemType::DecisionItem(decision_item) = item {
                                    Some(decision_item)
                                } else {
                                    None
                                }
                            })
                            .collect::<Vec<_>>();

                        parent_nodes.push(AdfBlockNode::DecisionList {
                            content: decision_list_items,
                            attrs: LocalId {
                                local_id: local_id.unwrap_or_default(),
                            },
                        });
                    } else if ordered {
                        let ordered_list_items = nodes
                            .into_iter()
                            .filter_map(|item| {
                                if let ListItemType::ListItem(list_item) = item {
                                    Some(list_item)
                                } else {
                                    None
                                }
                            })
                            .collect::<Vec<_>>();
                        parent_nodes.push(AdfBlockNode::OrderedList {
                            content: ordered_list_items,
                            attrs: None,
                        });
                    } else {
                        let bullet_list_items = nodes
                            .into_iter()
                            .filter_map(|item| {
                                if let ListItemType::ListItem(list_item) = item {
                                    Some(list_item)
                                } else {
                                    None
                                }
                            })
                            .collect::<Vec<_>>();
                        parent_nodes.push(AdfBlockNode::BulletList {
                            content: bullet_list_items,
                        });
                    }
                }
                parent => panic!("Invalid parent for PendingList: {parent:?}"),
            },
            block => {
                let stack = &state.stack;
                panic!(
                    "{block:?} closed incorrectly; must use block-specific close method: {stack:?}"
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
                    list.push(ListItemType::ListItem(ListItem::new(nodes)));
                }
                _ => {
                    panic!("ListItem closed without PendingList parent");
                }
            }
        } else if let Some(BlockContext::TaskItem(nodes, item_state, local_id)) = stack_item {
            if let Some(BlockContext::PendingList { nodes: list, .. }) = state.stack.last_mut() {
                let task_item = TaskItem::new(
                    nodes,
                    TaskItemAttrs {
                        local_id,
                        state: item_state,
                    },
                );
                list.push(ListItemType::TaskItem(task_item));
            } else {
                panic!("TaskItem closed without PendingList parent");
            }
        } else if let Some(BlockContext::DecisionItem(nodes, local_id)) = stack_item {
            if let Some(BlockContext::PendingList { nodes: list, .. }) = state.stack.last_mut() {
                let decision_item = DecisionItem::new(
                    nodes,
                    DecisionItemAttrs {
                        local_id,
                        state: DecisionItemState,
                    },
                );
                list.push(ListItemType::DecisionItem(decision_item));
            } else {
                panic!("DecisionItem closed without PendingList parent");
            }
        } else {
            panic!("Invalid context for ListItem close {:?}", stack_item);
        }
    }

    pub fn emit(self) -> AdfBlockNode {
        let mut state = self.state.into_inner();
        Self::flush_text(&mut state);
        while state.stack.len() > 1 {
            Self::close_current_block(&mut state);
        }
        if let BlockContext::Document(content) = state.stack.pop().unwrap() {
            AdfBlockNode::Doc {
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
                | BlockContext::Heading(_, nodes)
                | BlockContext::DecisionItem(nodes, _)
                | BlockContext::TaskItem(nodes, _, _) => nodes.push(node),
                BlockContext::Blockquote(nodes) | BlockContext::ListItem(nodes) => {
                    Self::push_into_last_paragraph(nodes, node);
                }
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

    pub fn trim_empty_paragraphs(nodes: Vec<AdfBlockNode>) -> Vec<AdfBlockNode> {
        nodes
            .into_iter()
            .filter(|node| match node {
                AdfBlockNode::Paragraph { content } => {
                    if let Some(content) = content {
                        !content.is_empty()
                    } else {
                        false
                    }
                }
                _ => true,
            })
            .collect()
    }

    pub fn push_node_block_to_parent(state: &mut ADFBuilderState, node: AdfBlockNode) {
        let frame = state
            .stack
            .last_mut()
            .expect("There should always be at least the Document node");
        match frame {
            BlockContext::Document(nodes)
            | BlockContext::Blockquote(nodes)
            | BlockContext::CustomBlock(CustomBlockType::Panel, nodes, _)
            | BlockContext::CustomBlock(CustomBlockType::Expand, nodes, _)
            | BlockContext::CustomBlock(CustomBlockType::NestedExpand, nodes, _)
            | BlockContext::CustomBlock(CustomBlockType::Div, nodes, _)
            | BlockContext::ListItem(nodes)
            | BlockContext::TableBlockCell(nodes)
            | BlockContext::TableBlockHeader(nodes) => {
                match &node {
                    AdfBlockNode::Paragraph { content } => match content {
                        Some(content) => {
                            if content.is_empty() {
                                return;
                            }
                        }
                        None => {
                            return;
                        }
                    },
                    _ => {}
                }
                nodes.push(node);
                return;
            }
            BlockContext::Paragraph(nodes) => {
                // Invalid paragraph context for block node
                // We need to drop the paragraph context
                // and push the block node to the grandparent
                if !nodes.is_empty() {
                    panic!("Invalid paragraph context for block node: {frame:?} <-- {node:?}");
                }
            }
            _ => {
                panic!("Invalid block context for block node: {frame:?} <-- {node:?}");
            }
        }

        state.stack.pop();
        Self::push_node_block_to_parent(state, node);
    }

    pub fn push_node_to_parent(state: &mut ADFBuilderState, node: AdfNode) {
        let frame = state
            .stack
            .last_mut()
            .expect("There should always be at least the Document node");
        match frame {
            BlockContext::Paragraph(nodes) | BlockContext::Heading(_, nodes) => nodes.push(node),
            BlockContext::Blockquote(nodes)
            | BlockContext::ListItem(nodes)
            | BlockContext::Document(nodes)
            | BlockContext::TableBlockCell(nodes)
            | BlockContext::TableBlockHeader(nodes) => {
                Self::push_into_last_paragraph(nodes, node);
            }
            BlockContext::CustomBlock(block_ty, nodes, _) => match block_ty {
                CustomBlockType::Div | CustomBlockType::Expand | CustomBlockType::Panel => {
                    Self::push_into_last_paragraph(nodes, node);
                }
                _ => panic!("Invalid block context for custom block: {block_ty:?} {node:?}"),
            },
            frame => {
                panic!("Invalid block context for node: {frame:?} <-- {node:?}");
            }
        }
    }

    pub fn extract_text(paragraph: &AdfBlockNode) -> String {
        match paragraph {
            AdfBlockNode::Paragraph {
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

pub fn html_to_adf(input: &str) -> AdfBlockNode {
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

    use crate::adf::adf_types::{
        AdfNode, DecisionItem, DecisionItemAttrs, HeadingAttrs, LinkMark, ListItem, MediaAttrs,
        MediaDataType, MediaNode, MediaSingleAttrs, MediaType, Subsup, TableRow, TableRowEntry,
    };

    fn assert_content_eq(adf: AdfBlockNode, expected: Vec<AdfBlockNode>) {
        assert_eq!(
            adf,
            AdfBlockNode::Doc {
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
            vec![AdfBlockNode::Blockquote {
                content: vec![AdfBlockNode::Paragraph {
                    content: Some(vec![AdfNode::Text {
                        text: "Quoted text.".into(),
                        marks: None,
                    }]),
                }],
            }],
        );
    }

    #[test]
    fn test_bullet_list_with_list_items() {
        let adf = html_to_adf(r#"<ul><li>Item one</li><li>Item two</li></ul>"#);
        assert_content_eq(
            adf,
            vec![AdfBlockNode::BulletList {
                content: vec![
                    ListItem::new(vec![AdfBlockNode::Paragraph {
                        content: Some(vec![AdfNode::Text {
                            text: "Item one".into(),
                            marks: None,
                        }]),
                    }]),
                    ListItem::new(vec![AdfBlockNode::Paragraph {
                        content: Some(vec![AdfNode::Text {
                            text: "Item two".into(),
                            marks: None,
                        }]),
                    }]),
                ],
            }],
        );
    }

    #[test]
    fn test_ordered_list_with_list_items() {
        let adf = html_to_adf(r#"<ol><li>Item one</li><li>Item two</li></ol>"#);
        assert_content_eq(
            adf,
            vec![AdfBlockNode::OrderedList {
                attrs: None,
                content: vec![
                    ListItem::new(vec![AdfBlockNode::Paragraph {
                        content: Some(vec![AdfNode::Text {
                            text: "Item one".into(),
                            marks: None,
                        }]),
                    }]),
                    ListItem::new(vec![AdfBlockNode::Paragraph {
                        content: Some(vec![AdfNode::Text {
                            text: "Item two".into(),
                            marks: None,
                        }]),
                    }]),
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
            vec![AdfBlockNode::Paragraph {
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
            vec![AdfBlockNode::Paragraph {
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
            vec![AdfBlockNode::Paragraph {
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
                AdfBlockNode::CodeBlock {
                    content: Some(vec![AdfNode::Text {
                        text: "let x = 42;".into(),
                        marks: None,
                    }]),
                    attrs: None,
                },
                AdfBlockNode::Paragraph {
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
            vec![AdfBlockNode::Table {
                attrs: None,
                content: vec![
                    TableRow::new(vec![
                        TableRowEntry::new_table_header(
                            vec![AdfBlockNode::Paragraph {
                                content: Some(vec![AdfNode::Text {
                                    text: "Header 1".into(),
                                    marks: None,
                                }]),
                            }],
                            None,
                        ),
                        TableRowEntry::new_table_header(
                            vec![AdfBlockNode::Paragraph {
                                content: Some(vec![AdfNode::Text {
                                    text: "Header 2".into(),
                                    marks: None,
                                }]),
                            }],
                            None,
                        ),
                    ]),
                    TableRow::new(vec![
                        TableRowEntry::new_table_cell(
                            vec![AdfBlockNode::Paragraph {
                                content: Some(vec![AdfNode::Text {
                                    text: "Cell 1".into(),
                                    marks: None,
                                }]),
                            }],
                            None,
                        ),
                        TableRowEntry::new_table_cell(
                            vec![], // empty cell
                            None,
                        ),
                    ]),
                    TableRow::new(vec![
                        TableRowEntry::new_table_cell(
                            vec![
                                AdfBlockNode::Paragraph {
                                    content: Some(vec![AdfNode::Text {
                                        text: "Nested paragraph".into(),
                                        marks: None,
                                    }]),
                                },
                                AdfBlockNode::Blockquote {
                                    content: vec![AdfBlockNode::Paragraph {
                                        content: Some(vec![AdfNode::Text {
                                            text: "Blockquote inside cell".into(),
                                            marks: None,
                                        }]),
                                    }],
                                },
                            ],
                            None,
                        ),
                        TableRowEntry::new_table_cell(
                            vec![AdfBlockNode::Paragraph {
                                content: Some(vec![AdfNode::Text {
                                    text: "Simple text".into(),
                                    marks: None,
                                }]),
                            }],
                            None,
                        ),
                    ]),
                ],
            }],
        );
    }

    #[test]
    fn test_media_parsing() {
        let adf = html_to_adf(
            r#"
                <adf-media-single data-layout="align-start">
                    <img
                        data-collection=""
                        data-media-id="76add7bf-0485-4fe8-88c2-30dcad78e7b5"
                        alt="pants.png"
                        style="width: 659px; height: 291px">
                    </img>
                </adf-media-single>
            "#,
        );
        assert_content_eq(
            adf,
            vec![AdfBlockNode::MediaSingle {
                content: vec![MediaNode {
                    media_type: MediaType::Media,
                    attrs: MediaAttrs {
                        alt: Some("pants.png".to_string()),
                        collection: "".to_string(),
                        height: Some(291),
                        id: "76add7bf-0485-4fe8-88c2-30dcad78e7b5".to_string(),
                        type_: MediaDataType::File,
                        width: Some(659),
                    },
                    marks: None,
                }],
                attrs: MediaSingleAttrs {
                    layout: "align-start".to_string(),
                },
            }],
        );
    }

    #[test]
    fn test_decision_item_parsing() {
        let adf = html_to_adf(
            r#"
            <p><adf-local-data data-tag="decision-list" id="6e80893d-7501-409d-9cd9-d2f5366ba665"></adf-local-data></p>
            <ul>
                <li>
                    <p><adf-decision-item id="f041c6cd-eb80-47ec-8cba-2e6d13d726de">Decision?</adf-decision-item></p>
                </li>
                <li>
                    <p><adf-decision-item id="d34c6e8f-fc4b-4368-bb3c-794b29b6190b">Do it</adf-decision-item></p>
                </li>
            </ul>
            "#,
        );
        assert_content_eq(
            adf,
            vec![AdfBlockNode::DecisionList {
                content: vec![
                    DecisionItem::new(
                        vec![AdfNode::Text {
                            text: "Decision?".into(),
                            marks: None,
                        }],
                        DecisionItemAttrs {
                            local_id: "f041c6cd-eb80-47ec-8cba-2e6d13d726de".to_string(),
                            state: DecisionItemState,
                        },
                    ),
                    DecisionItem::new(
                        vec![AdfNode::Text {
                            text: "Do it".into(),
                            marks: None,
                        }],
                        DecisionItemAttrs {
                            local_id: "d34c6e8f-fc4b-4368-bb3c-794b29b6190b".to_string(),
                            state: DecisionItemState,
                        },
                    ),
                ],
                attrs: LocalId {
                    local_id: "6e80893d-7501-409d-9cd9-d2f5366ba665".to_string(),
                },
            }],
        );
    }

    #[test]
    fn test_br_inside_paragraph() {
        let adf = html_to_adf(r#"<p>First line<br/>Second line</p>"#);
        assert_content_eq(
            adf,
            vec![AdfBlockNode::Paragraph {
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
                AdfBlockNode::Paragraph {
                    content: Some(vec![AdfNode::Text {
                        text: "Before rule".into(),
                        marks: None,
                    }]),
                },
                AdfBlockNode::Rule,
                AdfBlockNode::Paragraph {
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
                AdfBlockNode::Heading {
                    attrs: HeadingAttrs { level: 1 },
                    content: Some(vec![AdfNode::Text {
                        text: "Main Heading".into(),
                        marks: None,
                    }]),
                },
                AdfBlockNode::Heading {
                    attrs: HeadingAttrs { level: 2 },
                    content: Some(vec![AdfNode::Text {
                        text: "Sub Heading".into(),
                        marks: None,
                    }]),
                },
                AdfBlockNode::Heading {
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
