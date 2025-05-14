use std::cell::RefCell;
use std::collections::HashMap;

use html5ever::tendril::Tendril;
use html5ever::tokenizer::{
    BufferQueue, Tag, TagKind, Token, TokenSink, TokenSinkResult, Tokenizer, TokenizerOpts,
};

use crate::adf::adf_types::{
    AdfMark, AdfNode, EmojiAttrs, LocalId, StatusAttrs, TaskItemAttrs, TaskItemState,
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
            start_handlers: base_start_handlers(),
            custom_start_handlers: HashMap::new(),
            end_handlers: base_end_handlers(),
            custom_end_handlers: HashMap::new(),
        };

        this.add_start_handler("adf-media-single", media_single_start_handler());
        this.add_end_handler("adf-media-single", media_single_end_handler());

        this.add_start_handler("adf-media-group", media_group_start_handler());
        this.add_end_handler("adf-media-group", media_group_end_handler());

        this.add_start_handler("a", media_and_inline_card_start_handler());
        this.add_start_handler("img", media_and_inline_card_start_handler());
        this.add_end_handler("a", inline_card_end_handler());

        this.add_start_handler("time", date_start_handler());
        this.add_end_handler("time", date_end_handler());

        this.add_end_handler("summary", summary_end_handler());

        this.add_start_handler("details", details_start_handler());
        this.add_end_handler("details", details_end_handler());

        this.add_start_handler("figure", figure_start_handler());
        this.add_end_handler("figure", figure_end_handler());

        this.add_start_handler("adf-mention", mention_start_handler());
        this.add_end_handler("adf-mention", mention_end_handler());

        this
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
                        | BlockContext::TableCell(_)
                        | BlockContext::TableHeader(_)
                        | BlockContext::Blockquote(_)
                        | BlockContext::ListItem(_)
                )
            );

            if trim_for_blocks {
                if text.trim().is_empty() {
                    return;
                }
                text = clean_surrounding_text(&text).to_string();
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
                    | BlockContext::TableCell(nodes)
                    | BlockContext::TableHeader(nodes) => {
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
                    BlockContext::CodeBlock(lines) => {
                        lines.push(text);
                    }
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

    pub fn close_current_block(state: &mut ADFBuilderState) {
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
                BlockContext::CustomBlock(block_ty, parent_nodes, _) => match block_ty {
                    CustomBlockType::Div => {
                        parent_nodes.push(AdfNode::Paragraph {
                            content: Some(nodes),
                        });
                    }
                    CustomBlockType::Expand
                    | CustomBlockType::NestedExpand
                    | CustomBlockType::Panel => {
                        parent_nodes.push(AdfNode::Paragraph {
                            content: Some(nodes),
                        });
                    }
                    parent => {
                        panic!("Invalid parent for Paragraph: {parent:?}");
                    }
                },
                parent => panic!("Invalid parent for Paragraph: {parent:?}"),
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
            BlockContext::PendingList {
                nodes,
                ordered,
                local_id,
                local_tag,
            } => match parent {
                BlockContext::Document(parent_nodes)
                | BlockContext::CustomBlock(CustomBlockType::Div, parent_nodes, _)
                | BlockContext::Paragraph(parent_nodes)
                | BlockContext::TableCell(parent_nodes)
                | BlockContext::TableHeader(parent_nodes)
                | BlockContext::ListItem(parent_nodes) => {
                    let is_task_list = local_tag
                        .as_ref()
                        .map(|tag| tag == "task-list")
                        .unwrap_or(false);
                    let is_decision_list = local_tag
                        .as_ref()
                        .map(|tag| tag == "decision-list")
                        .unwrap_or(false);
                    eprintln!(
                        "PendingList closed with {nodes:?} (ordered: {ordered:?}) {local_id:?} {local_tag:?}"
                    );
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
                panic!("{block:?} closed incorrectly; must use block-specific close method");
            }
        }
    }

    pub fn close_current_list_item(state: &mut ADFBuilderState) {
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

    fn flush_text_and_push_inline(state: &mut ADFBuilderState, node: AdfNode) {
        Self::flush_text(state);
        Self::push_inline(state, node);
    }

    pub fn push_block_to_parent(state: &mut ADFBuilderState, node: AdfNode) {
        eprintln!("push_block_to_parent: {node:?} {:?}", state.stack);
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
            BlockContext::CustomBlock(block_ty, nodes, _) => {
                if block_ty == &CustomBlockType::Div {
                    nodes.push(node);
                } else {
                    panic!("Invalid block context for custom block: {block_ty:?}");
                }
            }
            node => {
                panic!("Invalid block context for block node: {node:?}");
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

    fn extract_text(paragraph: &AdfNode) -> String {
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
                    return TokenSinkResult::Continue;
                }

                match (name.as_ref(), self_closing) {
                    ("adf-decision-item", _) => {
                        let local_id = attrs
                            .iter()
                            .find(|attr| attr.name.local.as_ref() == "id")
                            .map(|id| id.value.to_string())
                            .unwrap_or_default();
                        let decision_item = BlockContext::DecisionItem(vec![], local_id);
                        state.stack.push(decision_item);
                    }
                    ("input", _) => {
                        // Check if inside TaskItem, and if input type is checkbox
                        let stack_back = state.stack.pop();
                        match stack_back {
                            Some(BlockContext::ListItem(inner)) => {
                                if let Some(input_type) =
                                    attrs.iter().find(|attr| attr.name.local.as_ref() == "type")
                                {
                                    if input_type.value.eq_ignore_ascii_case("checkbox") {
                                        let checked = attrs
                                            .iter()
                                            .any(|attr| attr.name.local.as_ref() == "checked");
                                        let item_state = if checked {
                                            TaskItemState::Done
                                        } else {
                                            TaskItemState::Todo
                                        };

                                        let task_item = BlockContext::TaskItem(
                                            inner,
                                            item_state,
                                            attrs
                                                .iter()
                                                .find(|attr| attr.name.local.as_ref() == "id")
                                                .map(|id| id.value.to_string())
                                                .unwrap_or_default(),
                                        );
                                        state.stack.push(task_item);
                                    } else {
                                        state.stack.push(BlockContext::ListItem(inner));
                                    }
                                }
                            }
                            Some(item) => {
                                state.stack.push(item);
                            }
                            None => {}
                        }
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
                    ("span", false) => {
                        // Check style for color or background-color
                        Self::flush_text(&mut state);
                        if let Some(style_attr) = attrs
                            .iter()
                            .find(|attr| attr.name.local.as_ref() == "style")
                        {
                            let style = style_attr.value.to_ascii_lowercase();
                            if let Some(color) = extract_style(&style, "color") {
                                state.mark_stack.push(AdfMark::TextColor { color });
                            }
                            if let Some(bg) = extract_style(&style, "background-color") {
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
                    ("adf-local-data", _) => {
                        let local_id = attrs
                            .iter()
                            .find(|attr| attr.name.local.as_ref() == "id")
                            .map(|id| id.value.to_string());
                        state.custom_block_id = local_id.map(|id| LocalId { local_id: id });
                        eprintln!(
                            "adf-local-data custom block id: {:?}",
                            state.custom_block_id
                        );

                        let tag = attrs
                            .iter()
                            .find(|attr| attr.name.local.as_ref() == "data-tag")
                            .map(|id| id.value.to_string());
                        state.custom_block_tag = tag.map(|tag| tag.to_string());
                        eprintln!(
                            "adf-local-data custom block tag: {:?}",
                            state.custom_block_tag
                        );
                    }
                    ("adf-status", false) | ("adf-emoji", false) | ("div", false) => {
                        Self::flush_text(&mut state);

                        let mut node_attrs = HashMap::new();
                        for attr in attrs {
                            node_attrs.insert(
                                attr.name.local.as_ref().to_string(),
                                attr.value.to_string(),
                            );
                        }
                        let block = BlockContext::CustomBlock(
                            match name.as_ref() {
                                "adf-status" => {
                                    state.current_text.clear();
                                    CustomBlockType::Status
                                }
                                "adf-emoji" => {
                                    state.current_text.clear();
                                    CustomBlockType::Emoji
                                }
                                "div" => CustomBlockType::Div,
                                _ => unreachable!(),
                            },
                            vec![],
                            node_attrs,
                        );
                        state.stack.push(block);
                    }
                    _ => {}
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
                    return TokenSinkResult::Continue;
                }

                match name.as_ref() {
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
                    "div" => {
                        Self::flush_text(&mut state);
                        if let Some(BlockContext::CustomBlock(CustomBlockType::Div, nodes, attrs)) =
                            state.stack.pop()
                        {
                            let all_inline = nodes
                                .iter()
                                .all(|n| matches!(n, AdfNode::Text { .. } | AdfNode::HardBreak));
                            if all_inline {
                                if nodes.is_empty() {
                                    return TokenSinkResult::Continue;
                                }

                                let paragraph = AdfNode::Paragraph {
                                    content: Some(nodes),
                                };

                                let color = attrs
                                    .get("style")
                                    .and_then(|style| extract_style(style, "color"));
                                let bg = attrs
                                    .get("style")
                                    .and_then(|style| extract_style(style, "background-color"));

                                if color.is_some() || bg.is_some() {
                                    let mut marks = vec![];
                                    if let Some(color) = color {
                                        marks.push(AdfMark::TextColor { color });
                                    }
                                    if let Some(bg) = bg {
                                        marks.push(AdfMark::BackgroundColor { color: bg });
                                    }
                                    Self::push_block_to_parent(
                                        &mut state,
                                        AdfNode::Text {
                                            text: Self::extract_text(&paragraph),
                                            marks: Some(marks),
                                        },
                                    );
                                } else {
                                    // If no color or background, treat as a normal paragraph
                                    Self::push_block_to_parent(&mut state, paragraph);
                                }
                            } else {
                                if nodes.is_empty() {
                                    return TokenSinkResult::Continue;
                                }
                                // treat as transparent container, discard style, forward content
                                for node in nodes {
                                    Self::push_block_to_parent(&mut state, node);
                                }
                            }
                        } else {
                            panic!("Mismatched div close tag");
                        }
                    }
                    "span" => {
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
                    "adf-status" => {
                        if let Some(BlockContext::CustomBlock(CustomBlockType::Status, _, attrs)) =
                            state.stack.pop()
                        {
                            let text = state.current_text.trim().to_string();
                            state.current_text.clear();
                            let color = attrs
                                .get("style")
                                .and_then(|style| extract_style(style, "background-color"));
                            let local_id = attrs.get("aria-label").map(|id| id.to_string());
                            Self::push_block_to_parent(
                                &mut state,
                                AdfNode::Status {
                                    attrs: StatusAttrs {
                                        color: color.unwrap_or_else(|| "neutral".to_string()),
                                        local_id,
                                        text,
                                    },
                                },
                            );
                        } else {
                            panic!("Mismatched status close tag");
                        }
                    }
                    "adf-emoji" => {
                        if let Some(BlockContext::CustomBlock(CustomBlockType::Emoji, _, attrs)) =
                            state.stack.pop()
                        {
                            let short_name = if let Some(value) = attrs.get("aria-alt") {
                                value.clone()
                            } else {
                                ":smile:".to_string()
                            };

                            let text = state.current_text.trim().to_string();
                            state.current_text.clear();
                            Self::push_block_to_parent(
                                &mut state,
                                AdfNode::Emoji {
                                    attrs: EmojiAttrs {
                                        text: Some(text),
                                        short_name,
                                    },
                                },
                            );
                        } else {
                            panic!("Mismatched emoji close tag");
                        }
                    }
                    _ => {}
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

pub fn parse_html(input: &str) -> AdfNode {
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

    #[test]
    fn test_headings_parsing() {
        let adf = parse_html(
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
