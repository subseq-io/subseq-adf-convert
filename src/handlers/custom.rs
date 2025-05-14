use crate::{adf::adf_types::{AdfNode, LinkMark, MediaAttrs, MediaMark, MediaNode, MediaSingleAttrs}, html::{extract_style, ADFBuilder, HandlerFn}};
use super::{ADFBuilderState, BlockContext, CustomBlockType, Element, MediaBlockType};

pub(crate) fn media_single_start_handler() -> HandlerFn {
    Box::new(|state: &mut ADFBuilderState, element: Element| {
        ADFBuilder::flush_text(state);
        state.stack.push(BlockContext::MediaBlock(
            MediaBlockType::MediaSingle,
            vec![],
            element.attrs
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
        if let Some(BlockContext::MediaBlock(
            MediaBlockType::MediaSingle,
            nodes,
            attrs,
        )) = state.stack.pop()
        {
            ADFBuilder::push_block_to_parent(state, AdfNode::MediaSingle {
                attrs: Some(MediaSingleAttrs {
                    layout: attrs.get("data-layout").cloned(),
                }),
                content: nodes
            });
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
            element.attrs
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
        if let Some(BlockContext::MediaBlock(
            MediaBlockType::MediaGroup,
            nodes,
            _,
        )) = state.stack.pop()
        {
            ADFBuilder::push_block_to_parent(state, AdfNode::MediaGroup {
                content: nodes
            });
        }
        true
    }) as HandlerFn
}

impl ADFBuilder {
    pub fn push_media_node_to_parent(
        state: &mut ADFBuilderState,
        node: MediaNode,
    ) {
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

pub(crate) fn media_start_handler() -> HandlerFn {
    Box::new(|state, element| {
        // Only handle if inside MediaGroup or MediaSingle
        if !matches!(
            state.stack.last(),
            Some(BlockContext::MediaBlock { .. })
        ) {
            return false;
        }

        // Extract href for a or src for img
        let (href, type_) = if element.tag == "a" {
            (element.attrs.iter().find(|attr| {
                 attr.name.local.as_ref() == "href"
             }).map(|attr| attr.value.as_ref().to_string()),
             "link")
        } else if element.tag == "img" {
            (element.attrs.iter().find(|attr| {
                 attr.name.local.as_ref() == "src"
             }).map(|attr| attr.value.as_ref().to_string()),
             "file")
        } else {
            (None, "unknown")
        };

        if let Some(href) = href {
            let id = match element.attrs.iter().find(|attr| {
                attr.name.local.as_ref() == "data-media-id"
            }).map(|attr| attr.value.as_ref().to_string()) {
                Some(id) => id,
                None => return false
            };
            let collection = match element.attrs.iter().find(|attr| {
                attr.name.local.as_ref() == "data-collection"
            }).map(|attr| attr.value.as_ref().to_string()) {
                Some(collection) => collection,
                None => return false
            };
            let alt = element.attrs.iter().find(|attr| {
                attr.name.local.as_ref() == "alt"
            }).map(|attr| attr.value.as_ref().to_string());
            let style = element.attrs.iter().find(|attr| {
                attr.name.local.as_ref() == "style"
            });
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

        false
    })
}

pub(crate) fn date_start_handler() -> HandlerFn {
    Box::new(|state: &mut ADFBuilderState, element: Element| {
        ADFBuilder::flush_text(state);
        state.stack.push(BlockContext::CustomBlock(
            CustomBlockType::Date,
            vec![],
            element.attrs
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
        if let Some(BlockContext::CustomBlock(CustomBlockType::Date, _, attrs)) = state.stack.pop() {
            let timestamp = attrs
                .get("datetime")
                .cloned()
                .unwrap_or_default();
            ADFBuilder::push_block_to_parent(
                state,
                AdfNode::Date {
                    attrs: crate::adf::adf_types::DateAttrs { timestamp },
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
        let is_nested = element.attrs.iter().any(|attr| {
            attr.name.local.as_ref() == "data-nested"
        });
        state.stack.push(BlockContext::CustomBlock(
            if is_nested {
                CustomBlockType::NestedExpand
            } else {
                CustomBlockType::Expand
            },
            vec![],
            element.attrs
                .iter()
                .map(|attr| (attr.name.local.to_string(), attr.value.to_string()))
                .collect(),
        ));
        true
    }) as HandlerFn
}

pub(crate) fn details_end_handler() -> HandlerFn {
    Box::new(|state: &mut ADFBuilderState, _element: Element| {
        ADFBuilder::flush_text(state);
        if let Some(BlockContext::CustomBlock(ty, nodes, attrs)) = state.stack.pop() {
            match ty {
                CustomBlockType::Expand => {
                    ADFBuilder::push_block_to_parent(
                        state,
                        AdfNode::Expand {
                            attrs: crate::adf::adf_types::ExpandAttrs {
                                title: attrs.get("title").cloned(),
                            },
                            content: nodes,
                        },
                    );
                    true
                }
                CustomBlockType::NestedExpand => {
                    let title = attrs.get("title").cloned().unwrap_or_default();
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
            element.attrs
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
        if let Some(BlockContext::CustomBlock(CustomBlockType::Panel, nodes, attrs)) = state.stack.pop() {
            let panel_type = attrs.get("data-panel-type").cloned().unwrap_or_else(|| "info".to_string());
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
            element.attrs
                .iter()
                .map(|attr| (attr.name.local.to_string(), attr.value.to_string()))
                .collect(),
        ));
        true
    }) as HandlerFn
}

pub(crate) fn mention_end_handler() -> HandlerFn {
    Box::new(|state: &mut ADFBuilderState, _element: Element| {
        ADFBuilder::flush_text(state);
        if let Some(BlockContext::CustomBlock(CustomBlockType::Mention, _, attrs)) = state.stack.pop() {
            ADFBuilder::push_block_to_parent(
                state,
                AdfNode::Mention {
                    attrs: crate::adf::adf_types::MentionAttrs {
                        id: attrs.get("data-mention-id").cloned().unwrap_or_default(),
                        text: attrs.get("data-mention-text").cloned(),
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
