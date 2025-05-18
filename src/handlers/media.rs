use super::{ADFBuilderState, BlockContext, CustomBlockType, Element, MediaBlockType};
use crate::{
    adf::adf_types::{
        AdfNode, LinkMark, MediaAttrs, MediaDataType, MediaMark, MediaNode, MediaSingleAttrs,
        MediaType,
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
                    attrs: MediaSingleAttrs {
                        layout: attrs
                            .get("data-layout")
                            .expect("Required attribute data-layout")
                            .to_string(),
                    },
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
        if matches!(state.stack.last(), Some(BlockContext::MediaBlock { .. })) {
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

            if element.tag == "a" {
                let type_ = MediaDataType::Link;
                let href = element
                    .attrs
                    .iter()
                    .find(|attr| attr.name.local.as_ref() == "href")
                    .map(|attr| attr.value.as_ref().to_string())
                    .expect("a tag should have href");

                let media_node = MediaNode {
                    media_type: MediaType::Media,
                    attrs: MediaAttrs {
                        alt,
                        collection,
                        id,
                        type_,
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
            } else if element.tag == "img" {
                let type_ = MediaDataType::File;

                let media_node = MediaNode {
                    media_type: MediaType::Media,
                    attrs: MediaAttrs {
                        alt,
                        collection,
                        id,
                        type_,
                        width,
                        height,
                    },
                    marks: None,
                };

                ADFBuilder::push_media_node_to_parent(state, media_node);
                return true;
            } else {
                panic!("Unknown media type {}", element.tag);
            };
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
