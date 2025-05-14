use crate::html_to_adf::{ADFBuilder, HandlerFn};

use super::{ADFBuilderState, BlockContext, Element};

pub(crate) fn decision_start_handler() -> HandlerFn {
    Box::new(|state: &mut ADFBuilderState, element: Element| {
        let local_id = element
            .attrs
            .iter()
            .find(|attr| attr.name.local.as_ref() == "id")
            .map(|id| id.value.to_string())
            .unwrap_or_default();
        let decision_item = BlockContext::DecisionItem(vec![], local_id);
        state.stack.push(decision_item);
        true
    }) as HandlerFn
}

pub(crate) fn decision_end_handler() -> HandlerFn {
    Box::new(|state: &mut ADFBuilderState, _element: Element| {
        ADFBuilder::close_current_decision_item(state);
        true
    }) as HandlerFn
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
