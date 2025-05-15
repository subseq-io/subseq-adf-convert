use std::collections::HashMap;

use super::{ADFBuilderState, BlockContext, CustomBlockType, Element, TableBlockType};
use crate::{
    adf::adf_types::{AdfMark, AdfNode, HeadingAttrs, LinkMark, Subsup},
    html_to_adf::{ADFBuilder, HandlerFn, extract_style},
};

pub(crate) fn hard_break_start_handler() -> HandlerFn {
    Box::new(|state: &mut ADFBuilderState, _element: Element| {
        ADFBuilder::flush_text_and_push_inline(state, AdfNode::HardBreak);
        true
    }) as HandlerFn
}

pub(crate) fn rule_start_handler() -> HandlerFn {
    Box::new(|state: &mut ADFBuilderState, _element: Element| {
        ADFBuilder::flush_text(state);
        // Close any open block contexts that cannot contain Rule
        while matches!(
            state.stack.last(),
            Some(
                BlockContext::Paragraph(_)
                    | BlockContext::ListItem(_)
                    | BlockContext::Blockquote(_)
                    | BlockContext::TableBlock(TableBlockType::Cell, _)
                    | BlockContext::TableBlock(TableBlockType::Header, _)
            )
        ) {
            ADFBuilder::close_current_block(state);
        }
        ADFBuilder::push_block_to_parent(state, AdfNode::Rule);
        true
    }) as HandlerFn
}

pub(crate) fn code_start_handler() -> HandlerFn {
    Box::new(|state: &mut ADFBuilderState, _element: Element| {
        ADFBuilder::flush_text(state);
        let in_pre = state
            .stack
            .iter()
            .any(|ctx| matches!(ctx, BlockContext::CodeBlock(_)));
        if !in_pre {
            state.mark_stack.push(AdfMark::Code);
        }
        // If inside <pre>, do nothing (handled purely as block)
        true
    }) as HandlerFn
}

pub(crate) fn code_end_handler() -> HandlerFn {
    Box::new(|state: &mut ADFBuilderState, _element: Element| {
        ADFBuilder::flush_text(state);
        let in_pre = state
            .stack
            .iter()
            .any(|ctx| matches!(ctx, BlockContext::CodeBlock(_)));
        if !in_pre {
            ADFBuilder::pop_mark(state, |m| matches!(m, AdfMark::Code));
        }
        // Inside <pre>, no mark to pop
        true
    }) as HandlerFn
}

pub(crate) fn span_start_handler() -> HandlerFn {
    Box::new(|state: &mut ADFBuilderState, element: Element| {
        // Check style for color or background-color
        ADFBuilder::flush_text(state);
        if let Some(style_attr) = element
            .attrs
            .iter()
            .find(|attr| attr.name.local.as_ref() == "style")
        {
            let style = style_attr.value.to_ascii_lowercase();
            if let Some(color) = extract_style(&style, "color") {
                state.mark_stack.push(AdfMark::TextColor { color });
            } else if let Some(bg) = extract_style(&style, "background-color") {
                state
                    .mark_stack
                    .push(AdfMark::BackgroundColor { color: bg });
            }
        }
        true
    }) as HandlerFn
}

/// We need to handle the div tag specially because it can be used inside of HTML to represent
/// paragraphs, inside text to represent styles, or as a wrapper around the body (which cannot surpress top-level blocks).
pub(crate) fn div_start_handler() -> HandlerFn {
    Box::new(|state: &mut ADFBuilderState, element: Element| {
        ADFBuilder::flush_text(state);

        let mut node_attrs = HashMap::new();
        for attr in element.attrs {
            node_attrs.insert(attr.name.local.as_ref().to_string(), attr.value.to_string());
        }
        let block = BlockContext::CustomBlock(CustomBlockType::Div, vec![], node_attrs);
        state.stack.push(block);

        true
    }) as HandlerFn
}

pub(crate) fn div_end_handler() -> HandlerFn {
    Box::new(|state: &mut ADFBuilderState, _element: Element| {
        ADFBuilder::flush_text(state);
        if let Some(BlockContext::CustomBlock(CustomBlockType::Div, nodes, attrs)) =
            state.stack.pop()
        {
            let all_inline = nodes
                .iter()
                .all(|n| matches!(n, AdfNode::Text { .. } | AdfNode::HardBreak));
            if all_inline {
                if nodes.is_empty() {
                    return true;
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
                    ADFBuilder::push_block_to_parent(
                        state,
                        AdfNode::Text {
                            text: ADFBuilder::extract_text(&paragraph),
                            marks: Some(marks),
                        },
                    );
                } else {
                    // If no color or background, treat as a normal paragraph
                    ADFBuilder::push_block_to_parent(state, paragraph);
                }
            } else {
                if nodes.is_empty() {
                    return true;
                }
                // treat as transparent container, discard style, forward content
                for node in nodes {
                    ADFBuilder::push_block_to_parent(state, node);
                }
            }
        } else {
            panic!("Mismatched div close tag");
        }
        true
    }) as HandlerFn
}

pub(crate) fn ul_start_handler() -> HandlerFn {
    Box::new(|state, _| {
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
    })
}

pub(crate) fn ol_start_handler() -> HandlerFn {
    Box::new(|state, _| {
        ADFBuilder::flush_text(state);
        state.stack.push(BlockContext::PendingList {
            nodes: vec![],
            ordered: true,
            local_id: None,
            local_tag: None,
        });
        true
    })
}

pub(crate) fn li_start_handler() -> HandlerFn {
    Box::new(|state, _| {
        ADFBuilder::flush_text(state);
        state.stack.push(BlockContext::ListItem(vec![]));
        true
    })
}

pub(crate) fn p_start_handler() -> HandlerFn {
    Box::new(|state, _| {
        ADFBuilder::flush_text(state);
        state.stack.push(BlockContext::Paragraph(vec![]));
        true
    })
}

pub(crate) fn pre_start_handler() -> HandlerFn {
    Box::new(|state, _| {
        ADFBuilder::flush_text(state);
        state.preformatted = true;
        state.stack.push(BlockContext::CodeBlock(vec![]));
        true
    })
}

pub(crate) fn blockquote_start_handler() -> HandlerFn {
    Box::new(|state, _| {
        ADFBuilder::flush_text(state);
        state.stack.push(BlockContext::Blockquote(vec![]));
        true
    })
}

// --- Marks handlers ---
pub(crate) fn em_start_handler() -> HandlerFn {
    Box::new(|state, _| {
        ADFBuilder::flush_text(state);
        state.mark_stack.push(AdfMark::Em);
        true
    })
}

pub(crate) fn strong_start_handler() -> HandlerFn {
    Box::new(|state, _| {
        ADFBuilder::flush_text(state);
        state.mark_stack.push(AdfMark::Strong);
        true
    })
}

pub(crate) fn del_start_handler() -> HandlerFn {
    Box::new(|state, _| {
        ADFBuilder::flush_text(state);
        state.mark_stack.push(AdfMark::Strike);
        true
    })
}

pub(crate) fn a_start_handler() -> HandlerFn {
    Box::new(|state, element| {
        ADFBuilder::flush_text(state);
        if let Some(href) = element
            .attrs
            .iter()
            .find(|attr| attr.name.local.as_ref() == "href")
        {
            state.mark_stack.push(AdfMark::Link(LinkMark {
                href: href.value.to_string(),
                ..Default::default()
            }));
            state.heavy_trim = true;
        }
        true
    })
}

pub(crate) fn u_start_handler() -> HandlerFn {
    Box::new(|state, _| {
        ADFBuilder::flush_text(state);
        state.mark_stack.push(AdfMark::Underline);
        true
    })
}

pub(crate) fn sub_start_handler() -> HandlerFn {
    Box::new(|state, _| {
        ADFBuilder::flush_text(state);
        state
            .mark_stack
            .push(AdfMark::Subsup { type_: Subsup::Sub });
        true
    })
}

pub(crate) fn sup_start_handler() -> HandlerFn {
    Box::new(|state, _| {
        ADFBuilder::flush_text(state);
        state
            .mark_stack
            .push(AdfMark::Subsup { type_: Subsup::Sup });
        true
    })
}

pub(crate) fn header_start_handler(level: u8) -> HandlerFn {
    Box::new(move |state, _| {
        ADFBuilder::flush_text(state);
        state.stack.push(BlockContext::Heading(level, vec![]));
        true
    })
}

pub(crate) fn ul_end_handler() -> HandlerFn {
    Box::new(|state, _| {
        ADFBuilder::flush_text(state);
        ADFBuilder::close_current_block(state);
        true
    })
}

pub(crate) fn ol_end_handler() -> HandlerFn {
    Box::new(|state, _| {
        ADFBuilder::flush_text(state);
        ADFBuilder::close_current_block(state);
        true
    })
}

pub(crate) fn li_end_handler() -> HandlerFn {
    Box::new(|state, _| {
        ADFBuilder::close_current_list_item(state);
        true
    })
}

pub(crate) fn p_end_handler() -> HandlerFn {
    Box::new(|state, _| {
        ADFBuilder::flush_text(state);
        ADFBuilder::close_current_block(state);
        true
    })
}

pub(crate) fn pre_end_handler() -> HandlerFn {
    Box::new(|state, _| {
        ADFBuilder::flush_text(state);
        state.preformatted = false;
        ADFBuilder::close_current_block(state);
        true
    })
}

pub(crate) fn blockquote_end_handler() -> HandlerFn {
    Box::new(|state, _| {
        ADFBuilder::flush_text(state);
        ADFBuilder::close_current_block(state);
        true
    })
}

// --- Marks handlers ---
pub(crate) fn mark_end_handler() -> HandlerFn {
    Box::new(|state, _| {
        ADFBuilder::flush_text(state);
        state.heavy_trim = false;
        state.mark_stack.pop();
        true
    })
}

pub(crate) fn header_end_handler(level: u8) -> HandlerFn {
    Box::new(move |state, _| {
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
    })
}
