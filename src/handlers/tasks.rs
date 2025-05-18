use super::{ADFBuilderState, BlockContext, Element};
use crate::{adf::adf_types::TaskItemState, html_to_adf::HandlerFn};

pub(crate) fn task_item_start_handler() -> HandlerFn {
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

        if let Some(input_type) = element
            .attrs
            .iter()
            .find(|attr| attr.name.local.as_ref() == "type")
        {
            if input_type.value.eq_ignore_ascii_case("checkbox") {
                let checked = element
                    .attrs
                    .iter()
                    .any(|attr| attr.name.local.as_ref() == "checked");
                let item_state = if checked {
                    TaskItemState::Done
                } else {
                    TaskItemState::Todo
                };

                let task_item = BlockContext::TaskItem(
                    inner,
                    item_state,
                    element
                        .attrs
                        .iter()
                        .find(|attr| attr.name.local.as_ref() == "id")
                        .map(|id| id.value.to_string())
                        .unwrap_or_default(),
                );
                state.stack.push(task_item);
            } else {
                panic!(
                    "Unsupported type attribute for task item: {}",
                    input_type.value
                );
            }
        } else {
            panic!("No type attribute found for task item");
        }
        true
    })
}
