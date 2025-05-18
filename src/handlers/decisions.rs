use crate::html_to_adf::HandlerFn;

use super::{ADFBuilderState, BlockContext, Element};

pub(crate) fn decision_start_handler() -> HandlerFn {
    Box::new(|state: &mut ADFBuilderState, element: Element| {
        let stack_back = state.stack.pop();
        match stack_back {
            Some(BlockContext::ListItem(inner)) => {
                let local_id = element
                    .attrs
                    .iter()
                    .find(|attr| attr.name.local.as_ref() == "id")
                    .map(|id| id.value.to_string())
                    .unwrap_or_default();
                let decision_item = BlockContext::DecisionItem(inner, local_id);
                state.stack.push(decision_item);
            }
            Some(item) => {
                state.stack.push(item);
            }
            None => {}
        }
        true
    })
}
