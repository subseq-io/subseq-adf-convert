use super::{ADFBuilderState, BlockContext, CustomBlockType, Element, MediaBlockType};
use crate::{
    adf::adf_types::{AdfNode, LinkMark, MediaAttrs, MediaMark, MediaNode, MediaSingleAttrs},
    html::{ADFBuilder, HandlerFn, extract_style},
};

pub(crate) fn media_single_start_handler() -> HandlerFn {
    Box::new(|state: &mut ADFBuilderState, element: Element| {
        ADFBuilder::flush_text(state);
        state.stack.push(BlockContext::MediaBlock(
            MediaBlockType::MediaSingle,
            vec![],
            element
                .attrs
                .iter()
                .map(|attr| (attr.name.local.to_string(), attr.value.to_string()))
                .collect(),
        ));
        true
    }) as HandlerFn
}

pub(crate) fn media_single_end_handler() -> HandlerFn {
    Box::new(|state: &mut ADFBuilderState, _element: Element| {
        ADFBuilder::flush_text(state);
        if let Some(BlockContext::MediaBlock(MediaBlockType::MediaSingle, nodes, attrs)) =
            state.stack.pop()
        {
            ADFBuilder::push_block_to_parent(
                state,
                AdfNode::MediaSingle {
                    attrs: Some(MediaSingleAttrs {
                        layout: attrs.get("data-layout").cloned(),
                    }),
                    content: nodes,
                },
            );
        }
        true
    }) as HandlerFn
}

pub(crate) fn media_group_start_handler() -> HandlerFn {
    Box::new(|state: &mut ADFBuilderState, element: Element| {
        ADFBuilder::flush_text(state);
        state.stack.push(BlockContext::MediaBlock(
            MediaBlockType::MediaGroup,
            vec![],
            element
                .attrs
                .iter()
                .map(|attr| (attr.name.local.to_string(), attr.value.to_string()))
                .collect(),
        ));
        true
    }) as HandlerFn
}

pub(crate) fn media_group_end_handler() -> HandlerFn {
    Box::new(|state: &mut ADFBuilderState, _element: Element| {
        ADFBuilder::flush_text(state);
        if let Some(BlockContext::MediaBlock(MediaBlockType::MediaGroup, nodes, _)) =
            state.stack.pop()
        {
            ADFBuilder::push_block_to_parent(state, AdfNode::MediaGroup { content: nodes });
        }
        true
    }) as HandlerFn
}

impl ADFBuilder {
    pub fn push_media_node_to_parent(state: &mut ADFBuilderState, node: MediaNode) {
        let frame = state
            .stack
            .last_mut()
            .expect("There should always be at least the Document node");

        match frame {
            BlockContext::MediaBlock(_, nodes, _) => {
                nodes.push(node);
            }
            _ => {
                panic!("Expected MediaBlock on stack");
            }
        }
    }
}

pub(crate) fn media_and_inline_card_start_handler() -> HandlerFn {
    Box::new(|state, element| {
        // --- MEDIA HANDLING ---
        if matches!(state.stack.last(), Some(BlockContext::MediaBlock { .. })) {
            let (href, type_) = if element.tag == "a" {
                (
                    element
                        .attrs
                        .iter()
                        .find(|attr| attr.name.local.as_ref() == "href")
                        .map(|attr| attr.value.as_ref().to_string()),
                    "link",
                )
            } else if element.tag == "img" {
                (
                    element
                        .attrs
                        .iter()
                        .find(|attr| attr.name.local.as_ref() == "src")
                        .map(|attr| attr.value.as_ref().to_string()),
                    "file",
                )
            } else {
                (None, "unknown")
            };

            if let Some(href) = href {
                let id = element
                    .attrs
                    .iter()
                    .find(|attr| attr.name.local.as_ref() == "data-media-id")
                    .map(|attr| attr.value.as_ref().to_string())
                    .unwrap_or_default();

                let collection = element
                    .attrs
                    .iter()
                    .find(|attr| attr.name.local.as_ref() == "data-collection")
                    .map(|attr| attr.value.as_ref().to_string())
                    .unwrap_or_default();

                let alt = element
                    .attrs
                    .iter()
                    .find(|attr| attr.name.local.as_ref() == "alt")
                    .map(|attr| attr.value.as_ref().to_string());

                let style = element
                    .attrs
                    .iter()
                    .find(|attr| attr.name.local.as_ref() == "style");

                let width = style
                    .and_then(|style| extract_style(&style.value, "width"))
                    .and_then(|v| v.trim().trim_end_matches("px").parse::<u32>().ok());

                let height = style
                    .and_then(|style| extract_style(&style.value, "height"))
                    .and_then(|v| v.trim().trim_end_matches("px").parse::<u32>().ok());

                let media_node = MediaNode {
                    media_type: "image".into(),
                    attrs: MediaAttrs {
                        alt,
                        collection,
                        id,
                        type_: type_.to_string(),
                        width,
                        height,
                    },
                    marks: vec![MediaMark::Link(LinkMark {
                        href,
                        ..Default::default()
                    })],
                };

                ADFBuilder::push_media_node_to_parent(state, media_node);
                return true;
            }
            return false;
        }

        // --- INLINE CARD HANDLING ---
        if element.tag == "a" {
            let has_inline_card = element
                .attrs
                .iter()
                .any(|attr| attr.name.local.as_ref() == "data-inline-card");

            if has_inline_card {
                if element
                    .attrs
                    .iter()
                    .find(|attr| attr.name.local.as_ref() == "href")
                    .map(|attr| attr.value.as_ref().to_string())
                    .is_some()
                {
                    ADFBuilder::flush_text(state);
                    state.stack.push(BlockContext::CustomBlock(
                        CustomBlockType::InlineCard,
                        vec![],
                        element
                            .attrs
                            .iter()
                            .map(|attr| (attr.name.local.to_string(), attr.value.to_string()))
                            .collect(),
                    ));
                    return true;
                } else {
                    panic!("Inline card without href");
                }
            }
        }
        false
    })
}

pub(crate) fn inline_card_end_handler() -> HandlerFn {
    Box::new(|state: &mut ADFBuilderState, element: Element| {
        if element.tag != "a" {
            return false;
        }

        let attrs = match state.stack.last() {
            Some(BlockContext::CustomBlock(CustomBlockType::InlineCard, _, attrs)) => attrs.clone(),
            _ => {
                return false;
            }
        };
        state.stack.pop();
        let href = attrs.get("href").cloned().unwrap_or_default();
        ADFBuilder::push_block_to_parent(
            state,
            AdfNode::InlineCard {
                attrs: crate::adf::adf_types::InlineCardAttrs { url: Some(href) },
            },
        );
        state.current_text.clear();

        true
    })
}

pub(crate) fn date_start_handler() -> HandlerFn {
    Box::new(|state: &mut ADFBuilderState, element: Element| {
        ADFBuilder::flush_text(state);
        state.stack.push(BlockContext::CustomBlock(
            CustomBlockType::Date,
            vec![],
            element
                .attrs
                .iter()
                .map(|attr| (attr.name.local.to_string(), attr.value.to_string()))
                .collect(),
        ));
        true
    }) as HandlerFn
}

pub(crate) fn date_end_handler() -> HandlerFn {
    Box::new(|state: &mut ADFBuilderState, _element: Element| {
        ADFBuilder::flush_text(state);
        if let Some(BlockContext::CustomBlock(CustomBlockType::Date, _, attrs)) = state.stack.pop()
        {
            let timestamp_str = attrs.get("datetime").cloned().unwrap_or_default();
            let timestamp = chrono::DateTime::parse_from_rfc3339(&timestamp_str)
                .map(|dt| dt.timestamp())
                .unwrap_or(0);
            ADFBuilder::push_block_to_parent(
                state,
                AdfNode::Date {
                    attrs: crate::adf::adf_types::DateAttrs {
                        timestamp: timestamp.to_string(),
                    },
                },
            );
            true
        } else {
            false
        }
    })
}

pub(crate) fn details_start_handler() -> HandlerFn {
    Box::new(|state: &mut ADFBuilderState, element: Element| {
        ADFBuilder::flush_text(state);
        let is_nested = element
            .attrs
            .iter()
            .any(|attr| attr.name.local.as_ref() == "data-nested");
        state.stack.push(BlockContext::CustomBlock(
            if is_nested {
                CustomBlockType::NestedExpand
            } else {
                CustomBlockType::Expand
            },
            vec![],
            element
                .attrs
                .iter()
                .map(|attr| (attr.name.local.to_string(), attr.value.to_string()))
                .collect(),
        ));
        true
    }) as HandlerFn
}

pub(crate) fn summary_end_handler() -> HandlerFn {
    Box::new(|state: &mut ADFBuilderState, _element: Element| {
        // Capture and clean the text collected in current_text
        let summary_text = state.current_text.trim().to_string();
        eprintln!("Summary text: {}", summary_text);
        state.current_text.clear();

        // Find the nearest CustomBlock of type Expand or NestedExpand and store the title
        if let Some(BlockContext::CustomBlock(_, _, attrs)) =
            state.stack.iter_mut().rev().find(|ctx| {
                matches!(
                    ctx,
                    BlockContext::CustomBlock(
                        CustomBlockType::Expand | CustomBlockType::NestedExpand,
                        _,
                        _
                    )
                )
            })
        {
            eprintln!("Stack push");
            attrs.insert("data-summary".to_string(), summary_text);
            true
        } else {
            // No matching parent, ignore
            false
        }
    })
}

pub(crate) fn details_end_handler() -> HandlerFn {
    Box::new(|state: &mut ADFBuilderState, _element: Element| {
        ADFBuilder::flush_text(state);

        if let Some(BlockContext::CustomBlock(ty, nodes, attrs)) = state.stack.pop() {
            let title = attrs.get("data-summary").cloned().unwrap_or_default();
            eprintln!("Details end: {:?} {:?}", title, attrs);

            match ty {
                CustomBlockType::Expand => {
                    ADFBuilder::push_block_to_parent(
                        state,
                        AdfNode::Expand {
                            attrs: crate::adf::adf_types::ExpandAttrs {
                                title: if title.is_empty() { None } else { Some(title) },
                            },
                            content: nodes,
                        },
                    );
                    true
                }
                CustomBlockType::NestedExpand => {
                    ADFBuilder::push_block_to_parent(
                        state,
                        AdfNode::NestedExpand {
                            attrs: crate::adf::adf_types::NestedAttrs { title },
                            content: nodes,
                        },
                    );
                    true
                }
                _ => false,
            }
        } else {
            false
        }
    })
}

pub(crate) fn figure_start_handler() -> HandlerFn {
    Box::new(|state: &mut ADFBuilderState, element: Element| {
        ADFBuilder::flush_text(state);
        state.stack.push(BlockContext::CustomBlock(
            CustomBlockType::Panel,
            vec![],
            element
                .attrs
                .iter()
                .map(|attr| (attr.name.local.to_string(), attr.value.to_string()))
                .collect(),
        ));
        true
    }) as HandlerFn
}

pub(crate) fn figure_end_handler() -> HandlerFn {
    Box::new(|state: &mut ADFBuilderState, _element: Element| {
        ADFBuilder::flush_text(state);
        if let Some(BlockContext::CustomBlock(CustomBlockType::Panel, nodes, attrs)) =
            state.stack.pop()
        {
            let panel_type = attrs
                .get("data-panel-type")
                .cloned()
                .unwrap_or_else(|| "info".to_string());
            ADFBuilder::push_block_to_parent(
                state,
                AdfNode::Panel {
                    attrs: crate::adf::adf_types::PanelAttrs { panel_type },
                    content: nodes,
                },
            );
            true
        } else {
            false
        }
    })
}

pub(crate) fn mention_start_handler() -> HandlerFn {
    Box::new(|state: &mut ADFBuilderState, element: Element| {
        ADFBuilder::flush_text(state);
        state.stack.push(BlockContext::CustomBlock(
            CustomBlockType::Mention,
            vec![],
            element
                .attrs
                .iter()
                .map(|attr| (attr.name.local.to_string(), attr.value.to_string()))
                .collect(),
        ));
        true
    }) as HandlerFn
}

pub(crate) fn mention_end_handler() -> HandlerFn {
    Box::new(|state: &mut ADFBuilderState, _element: Element| {
        if let Some(BlockContext::CustomBlock(CustomBlockType::Mention, _, attrs)) =
            state.stack.pop()
        {
            let text = state.current_text.trim().to_string();
            state.current_text.clear();

            ADFBuilder::push_block_to_parent(
                state,
                AdfNode::Mention {
                    attrs: crate::adf::adf_types::MentionAttrs {
                        id: attrs.get("data-mention-id").cloned().unwrap_or_default(),
                        text: if text.is_empty() { None } else { Some(text) },
                        user_type: attrs.get("data-user-type").cloned(),
                        access_level: attrs.get("data-access-level").cloned(),
                    },
                },
            );
            true
        } else {
            false
        }
    })
}

impl ADFBuilder {
    pub fn close_current_decision_item(state: &mut ADFBuilderState) {
        Self::flush_text(state);
        let stack_item = state.stack.pop();
        if let Some(BlockContext::DecisionItem(nodes, local_id)) = stack_item {
            match state.stack.last_mut() {
                Some(BlockContext::ListItem(_)) => {
                    state.stack.pop(); // Replace the ListItem with the DecisionItem
                    state
                        .stack
                        .push(BlockContext::DecisionItem(nodes, local_id));
                }
                _ => {
                    // We are closing a decision item outside of a list item
                    panic!("DecisionItem closed incorrectly; must use block-specific close method");
                }
            }
        } else {
            panic!("DecisionItem closed incorrectly; must use block-specific close method");
        }
    }
}
