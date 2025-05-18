use std::collections::HashMap;

use super::{ADFBuilderState, BlockContext, CustomBlockType, Element, MediaBlockType};
use crate::{
    adf::adf_types::{
        AdfNode, EmojiAttrs, LinkMark, LocalId, MediaAttrs, MediaMark, MediaNode, MediaSingleAttrs,
        StatusAttrs,
    },
    html_to_adf::{ADFBuilder, HandlerFn, extract_style},
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
                    marks: Some(vec![MediaMark::Link(LinkMark {
                        href,
                        ..Default::default()
                    })]),
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
        state.current_text.clear();
        state.stack.pop();
        let href = attrs.get("href").cloned().unwrap_or_default();
        ADFBuilder::push_block_to_parent(
            state,
            AdfNode::InlineCard {
                attrs: crate::adf::adf_types::InlineCardAttrs { url: Some(href) },
            },
        );

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
            let user_type = serde_json::from_str(
                &attrs
                    .get("data-user-type")
                    .map(|s| format!("\"{}\"", s.as_str()))
                    .unwrap_or("\"DEFAULT\"".to_string()),
            )
            .unwrap_or_default();

            let access_level = serde_json::from_str(
                &attrs
                    .get("data-access-level")
                    .map(|s| format!("\"{}\"", s.as_str()))
                    .unwrap_or("\"NONE\"".to_string()),
            )
            .unwrap_or_default();

            ADFBuilder::push_block_to_parent(
                state,
                AdfNode::Mention {
                    attrs: crate::adf::adf_types::MentionAttrs {
                        id: attrs.get("data-mention-id").cloned().unwrap_or_default(),
                        text: if text.is_empty() { None } else { Some(text) },
                        user_type,
                        access_level,
                    },
                },
            );
            true
        } else {
            false
        }
    })
}

pub(crate) fn local_data_start_handler() -> HandlerFn {
    Box::new(|state: &mut ADFBuilderState, element: Element| {
        let local_id = element
            .attrs
            .iter()
            .find(|attr| attr.name.local.as_ref() == "id")
            .map(|id| id.value.to_string());
        state.custom_block_id = local_id.map(|id| LocalId { local_id: id });

        let tag = element
            .attrs
            .iter()
            .find(|attr| attr.name.local.as_ref() == "data-tag")
            .map(|id| id.value.to_string());
        state.custom_block_tag = tag.map(|tag| tag.to_string());
        true
    }) as HandlerFn
}

pub(crate) fn status_start_handler() -> HandlerFn {
    Box::new(|state: &mut ADFBuilderState, element: Element| {
        ADFBuilder::flush_text(state);

        let mut node_attrs = HashMap::new();
        for attr in element.attrs {
            node_attrs.insert(attr.name.local.as_ref().to_string(), attr.value.to_string());
        }
        state.current_text.clear();
        let block = BlockContext::CustomBlock(CustomBlockType::Status, vec![], node_attrs);
        state.stack.push(block);

        true
    }) as HandlerFn
}

pub(crate) fn status_end_handler() -> HandlerFn {
    Box::new(|state: &mut ADFBuilderState, _element: Element| {
        if let Some(BlockContext::CustomBlock(CustomBlockType::Status, _, attrs)) =
            state.stack.pop()
        {
            let text = state.current_text.trim().to_string();
            state.current_text.clear();
            let color = attrs
                .get("style")
                .and_then(|style| extract_style(style, "background-color"));
            let local_id = attrs.get("aria-label").map(|id| id.to_string());
            ADFBuilder::push_block_to_parent(
                state,
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
        true
    }) as HandlerFn
}

pub(crate) fn emoji_start_handler() -> HandlerFn {
    Box::new(|state: &mut ADFBuilderState, element: Element| {
        ADFBuilder::flush_text(state);

        let mut node_attrs = HashMap::new();
        for attr in element.attrs {
            node_attrs.insert(attr.name.local.as_ref().to_string(), attr.value.to_string());
        }
        state.current_text.clear();
        let block = BlockContext::CustomBlock(CustomBlockType::Emoji, vec![], node_attrs);
        state.stack.push(block);

        true
    }) as HandlerFn
}

pub(crate) fn emoji_end_handler() -> HandlerFn {
    Box::new(|state: &mut ADFBuilderState, _element: Element| {
        if let Some(BlockContext::CustomBlock(CustomBlockType::Emoji, _, attrs)) = state.stack.pop()
        {
            let short_name = if let Some(value) = attrs.get("aria-alt") {
                value.clone()
            } else {
                ":smile:".to_string()
            };

            let text = state.current_text.trim().to_string();
            state.current_text.clear();
            ADFBuilder::push_block_to_parent(
                state,
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
        true
    }) as HandlerFn
}
