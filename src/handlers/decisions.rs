use crate::html_to_adf::HandlerFn;

use super::{ADFBuilderState, BlockContext, Element};

pub(crate) fn decision_start_handler() -> HandlerFn {
    Box::new(|state: &mut ADFBuilderState, element: Element| {
        let has_list_item = state
            .stack
            .iter()
            .any(|item| matches!(item, BlockContext::ListItem(_)));

        if !has_list_item {
            return false;
        }

        let inner = loop{
            let item = state.stack.pop();
            match item {
                Some(BlockContext::ListItem(inner)) => {
                    break inner;
                },
                None => {
                    panic!("No list item found in stack");
                },
                _ => {
                    // continue
                }
            }
        };
        let local_id = element
            .attrs
            .iter()
            .find(|attr| attr.name.local.as_ref() == "id")
            .map(|id| id.value.to_string())
            .unwrap_or_default();
        let decision_item = BlockContext::DecisionItem(inner, local_id);
        state.stack.push(decision_item);
        true
    })
}
