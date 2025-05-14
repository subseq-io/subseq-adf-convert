use super::{ADFBuilderState, BlockContext, Element};
use crate::{adf::adf_types::TaskItemState, html_to_adf::HandlerFn};

pub(crate) fn input_start_handler() -> HandlerFn {
    Box::new(|state: &mut ADFBuilderState, element: Element| {
        let stack_back = state.stack.pop();
        match stack_back {
            Some(BlockContext::ListItem(inner)) => {
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
                        state.stack.push(BlockContext::ListItem(inner));
                    }
                }
            }
            Some(item) => {
                state.stack.push(item);
            }
            None => {}
        }
        true
    })
}
