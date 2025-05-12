use html5ever::tokenizer::{Tag, TagKind, Token, TokenSink, TokenSinkResult};
use std::cell::RefCell;

use crate::adf_types::AdfNode;

pub struct ADFBuilder {
    state: RefCell<ADFBuilderState>,
}

struct ADFBuilderState {
    root: Vec<AdfNode>,
    stack: Vec<BlockContext>,
    current_text: String,
}

enum BlockContext {
    Paragraph(Vec<AdfNode>),
    CodeBlock(Vec<String>),
    Blockquote(Vec<AdfNode>),
    BulletList(Vec<AdfNode>),
    ListItem(Vec<AdfNode>),
}

impl ADFBuilder {
    pub fn new() -> Self {
        Self {
            state: RefCell::new(ADFBuilderState {
                root: vec![],
                stack: vec![],
                current_text: String::new(),
            }),
        }
    }

    fn flush_text(state: &mut ADFBuilderState) {
        if !state.current_text.is_empty() {
            let text = std::mem::take(&mut state.current_text);
            if let Some(frame) = state.stack.last_mut() {
                match frame {
                    BlockContext::Blockquote(nodes) => nodes.push(AdfNode::Text{text, marks: None}),
                    BlockContext::Paragraph(nodes) => nodes.push(AdfNode::Text{text, marks: None}),
                    BlockContext::CodeBlock(lines) => lines.push(text),
                    BlockContext::BulletList(_) => {}
                    BlockContext::ListItem(nodes) => {
                        nodes.push(AdfNode::Text{text, marks: None});
                    }
                }
            } // If no block context, ignore text.
        }
    }

    fn close_current_block(state: &mut ADFBuilderState) {
        if let Some(frame) = state.stack.pop() {
            match frame {
                BlockContext::Paragraph(nodes) => state.root.push(AdfNode::Paragraph{content: Some(nodes)}),
                BlockContext::CodeBlock(lines) => {
                    let text = lines.join("");
                    state.root.push(AdfNode::CodeBlock{
                        content: Some(vec![AdfNode::Text{text: text.into(), marks: None}]),
                        attrs: None});
                }
                BlockContext::Blockquote(nodes) => state.root.push(AdfNode::Blockquote{content: nodes}),
                BlockContext::BulletList(nodes) => state.root.push(AdfNode::BulletList{content: nodes}),
                BlockContext::ListItem(_) => {
                    panic!("ListItem closed incorrectly; must use close_current_list_item");
                }
            }
        }
    }

    fn close_current_list_item(state: &mut ADFBuilderState) {
        if let Some(BlockContext::ListItem(nodes)) = state.stack.pop() {
            if let Some(BlockContext::BulletList(list)) = state.stack.last_mut() {
                list.push(AdfNode::ListItem{content: nodes});
            } else {
                panic!("ListItem without BulletList parent");
            }
        }
    }

    pub fn emit(self) -> Vec<AdfNode> {
        let mut state = self.state.into_inner();
        Self::flush_text(&mut state);
        while !state.stack.is_empty() {
            Self::close_current_block(&mut state);
        }
        state.root
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
}

impl TokenSink for ADFBuilder {
    type Handle = ();

    fn process_token(&self, token: Token, _line_number: u64) -> TokenSinkResult<Self::Handle> {
        let mut state = self.state.borrow_mut();
        match token {
            Token::TagToken(Tag {
                kind: TagKind::StartTag,
                name,
                ..
            }) => match name.as_ref() {
                "p" => {
                    Self::flush_text(&mut state);
                    state.stack.push(BlockContext::Paragraph(vec![]));
                }
                "pre" => {
                    Self::flush_text(&mut state);
                    state.stack.push(BlockContext::CodeBlock(vec![]));
                }
                "blockquote" => {
                    Self::flush_text(&mut state);
                    state.stack.push(BlockContext::Blockquote(vec![]));
                }
                "ul" => {
                    Self::flush_text(&mut state);
                    state.stack.push(BlockContext::BulletList(vec![]));
                }
                "li" => {
                    Self::flush_text(&mut state);
                    state.stack.push(BlockContext::ListItem(vec![]));
                }
                "br" => {
                    Self::flush_text_and_push_inline(&mut state, AdfNode::HardBreak);
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
                "ul" => {
                    Self::flush_text(&mut state);
                    Self::close_current_block(&mut state);
                }
                "li" => {
                    Self::flush_text(&mut state);
                    Self::close_current_list_item(&mut state);
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

    fn parse_html(input: &str) -> Vec<AdfNode> {
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

    #[test]
    fn test_blockquote() {
        let adf = parse_html(r#"<blockquote>Quoted text.</blockquote>"#);
        assert_eq!(
            adf,
            vec![AdfNode::Blockquote{content: vec![AdfNode::Text{text: "Quoted text.".into(), marks: None}]}]
        );
    }
    #[test]
    fn test_bullet_list_with_list_items() {
        let adf = parse_html(r#"<ul><li>Item one</li><li>Item two</li></ul>"#);
        assert_eq!(
            adf,
            vec![AdfNode::BulletList{ content: vec![
                AdfNode::ListItem {
                    content: vec![AdfNode::Text{text: "Item one".into(), marks: None}]
                },
                AdfNode::ListItem { 
                    content: vec![AdfNode::Text{text: "Item two".into(), marks: None}]
                },
            ]}]
        );
    }
}
