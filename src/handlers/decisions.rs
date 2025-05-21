use super::{ADFBuilderState, BlockContext, Element};
use crate::{adf::adf_types::AdfBlockNode, html_to_adf::HandlerFn};

pub(crate) fn decision_start_handler() -> HandlerFn {
    Box::new(|state: &mut ADFBuilderState, element: Element| {
        let has_list_item = state
            .stack
            .iter()
            .any(|item| matches!(item, BlockContext::ListItem(_)));

        if !has_list_item {
            return false;
        }

        let inner = loop {
            let item = state.stack.pop();
            match item {
                Some(BlockContext::ListItem(inner)) => {
                    break inner;
                }
                None => {
                    panic!("No list item found in stack");
                }
                _ => {
                    // continue
                }
            }
        };

        let mut nodes = vec![];
        for node in inner {
            match node {
                AdfBlockNode::Paragraph {
                    content: Some(para_nodes),
                } => {
                    nodes.extend(para_nodes);
                }
                _ => {}
            };
        }

        let local_id = element
            .attrs
            .iter()
            .find(|attr| attr.name.local.as_ref() == "id")
            .map(|id| id.value.to_string())
            .unwrap_or_default();
        let decision_item = BlockContext::DecisionItem(nodes, local_id);
        state.stack.push(decision_item);
        true
    })
}
