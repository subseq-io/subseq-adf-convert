use std::collections::HashMap;

use super::{ADFBuilderState, BlockContext, Element};
use crate::{
    adf::adf_types::{AdfMark, AdfNode, HeadingAttrs, LinkMark, Subsup},
    html::{ADFBuilder, HandlerFn},
};

pub(crate) fn base_start_handlers() -> HashMap<String, HandlerFn> {
    let mut handlers = HashMap::new();

    handlers.insert(
        "ul".to_string(),
        Box::new(|state: &mut ADFBuilderState, _element: Element| {
            let custom_id = state.custom_block_id.take();
            let custom_tag = state.custom_block_tag.take();
            ADFBuilder::flush_text(state);
            state.stack.push(BlockContext::PendingList {
                nodes: vec![],
                ordered: false,
                local_id: custom_id.map(|id| id.local_id),
                local_tag: custom_tag,
            });
            true
        }) as HandlerFn,
    );

    handlers.insert(
        "ol".to_string(),
        Box::new(|state: &mut ADFBuilderState, _element: Element| {
            ADFBuilder::flush_text(state);
            state.stack.push(BlockContext::PendingList {
                nodes: vec![],
                ordered: true,
                local_id: None,
                local_tag: None,
            });
            true
        }) as HandlerFn,
    );

    handlers.insert(
        "li".to_string(),
        Box::new(|state: &mut ADFBuilderState, _element: Element| {
            ADFBuilder::flush_text(state);
            state.stack.push(BlockContext::ListItem(vec![]));
            true
        }) as HandlerFn,
    );

    handlers.insert(
        "p".to_string(),
        Box::new(|state: &mut ADFBuilderState, _element: Element| {
            ADFBuilder::flush_text(state);
            state.stack.push(BlockContext::Paragraph(vec![]));
            true
        }) as HandlerFn,
    );

    handlers.insert(
        "pre".to_string(),
        Box::new(|state: &mut ADFBuilderState, _element: Element| {
            ADFBuilder::flush_text(state);
            state.stack.push(BlockContext::CodeBlock(vec![]));
            true
        }) as HandlerFn,
    );

    handlers.insert(
        "blockquote".to_string(),
        Box::new(|state: &mut ADFBuilderState, _element: Element| {
            ADFBuilder::flush_text(state);
            state.stack.push(BlockContext::Blockquote(vec![]));
            true
        }) as HandlerFn,
    );

    handlers.insert(
        "em".to_string(),
        Box::new(|state: &mut ADFBuilderState, _element: Element| {
            ADFBuilder::flush_text(state);
            state.mark_stack.push(AdfMark::Em);
            true
        }) as HandlerFn,
    );

    handlers.insert(
        "strong".to_string(),
        Box::new(|state: &mut ADFBuilderState, _element: Element| {
            ADFBuilder::flush_text(state);
            state.mark_stack.push(AdfMark::Strong);
            true
        }) as HandlerFn,
    );

    handlers.insert(
        "del".to_string(),
        Box::new(|state: &mut ADFBuilderState, _element: Element| {
            ADFBuilder::flush_text(state);
            state.mark_stack.push(AdfMark::Strike);
            true
        }) as HandlerFn,
    );

    handlers.insert(
        "a".to_string(),
        Box::new(|state: &mut ADFBuilderState, element: Element| {
            ADFBuilder::flush_text(state);
            if let Some(href) = element
                .attrs
                .iter()
                .find(|attr| attr.name.local.as_ref() == "href")
            {
                let mark = AdfMark::Link(LinkMark {
                    href: href.value.to_string(),
                    ..Default::default()
                });
                state.mark_stack.push(mark);
            }
            true
        }) as HandlerFn,
    );

    handlers.insert(
        "u".to_string(),
        Box::new(|state: &mut ADFBuilderState, _element: Element| {
            ADFBuilder::flush_text(state);
            state.mark_stack.push(AdfMark::Underline);
            true
        }) as HandlerFn,
    );

    handlers.insert(
        "sub".to_string(),
        Box::new(|state: &mut ADFBuilderState, _element: Element| {
            ADFBuilder::flush_text(state);
            state
                .mark_stack
                .push(AdfMark::Subsup { type_: Subsup::Sub });
            true
        }) as HandlerFn,
    );

    handlers.insert(
        "sup".to_string(),
        Box::new(|state: &mut ADFBuilderState, _element: Element| {
            ADFBuilder::flush_text(state);
            state
                .mark_stack
                .push(AdfMark::Subsup { type_: Subsup::Sup });
            true
        }) as HandlerFn,
    );

    for i in 1..=6 {
        let header = format!("h{i}");
        let closure = {
            let level = i;
            Box::new(move |state: &mut ADFBuilderState, _element: Element| {
                ADFBuilder::flush_text(state);
                state.stack.push(BlockContext::Heading(level, vec![]));
                true
            }) as HandlerFn
        };
        handlers.insert(header, closure);
    }
    handlers
}

pub(crate) fn base_end_handlers() -> HashMap<String, HandlerFn> {
    let mut handlers = HashMap::new();

    handlers.insert(
        "ul".to_string(),
        Box::new(|state: &mut ADFBuilderState, _element: Element| {
            ADFBuilder::flush_text(state);
            ADFBuilder::close_current_block(state);
            true
        }) as HandlerFn,
    );

    handlers.insert(
        "ol".to_string(),
        Box::new(|state: &mut ADFBuilderState, _element: Element| {
            ADFBuilder::flush_text(state);
            ADFBuilder::close_current_block(state);
            true
        }) as HandlerFn,
    );

    handlers.insert(
        "li".to_string(),
        Box::new(|state: &mut ADFBuilderState, _element: Element| {
            ADFBuilder::flush_text(state);
            ADFBuilder::close_current_list_item(state);
            true
        }) as HandlerFn,
    );

    handlers.insert(
        "p".to_string(),
        Box::new(|state: &mut ADFBuilderState, _element: Element| {
            ADFBuilder::flush_text(state);
            ADFBuilder::close_current_block(state);
            true
        }) as HandlerFn,
    );

    handlers.insert(
        "pre".to_string(),
        Box::new(|state: &mut ADFBuilderState, _element: Element| {
            ADFBuilder::flush_text(state);
            ADFBuilder::close_current_block(state);
            true
        }) as HandlerFn,
    );

    handlers.insert(
        "blockquote".to_string(),
        Box::new(|state: &mut ADFBuilderState, _element: Element| {
            ADFBuilder::flush_text(state);
            ADFBuilder::close_current_block(state);
            true
        }) as HandlerFn,
    );

    for mark in &["em", "strong", "del", "a", "u", "sub", "sup"] {
        handlers.insert(
            mark.to_string(),
            Box::new(|state: &mut ADFBuilderState, _element: Element| {
                ADFBuilder::flush_text(state);
                state.mark_stack.pop();
                true
            }) as HandlerFn,
        );
    }

    for i in 1..=6 {
        let header = format!("h{i}");
        let closure = {
            let level = i;
            Box::new(move |state: &mut ADFBuilderState, _element: Element| {
                ADFBuilder::flush_text(state);
                if let Some(BlockContext::Heading(lvl, nodes)) = state.stack.pop() {
                    if lvl == level {
                        ADFBuilder::push_block_to_parent(
                            state,
                            AdfNode::Heading {
                                attrs: HeadingAttrs { level },
                                content: Some(nodes),
                            },
                        );
                    } else {
                        panic!("Mismatched heading close level");
                    }
                } else {
                    panic!("Mismatched heading close tag");
                }
                true
            }) as HandlerFn
        };
        handlers.insert(header, closure);
    }
    handlers
}
