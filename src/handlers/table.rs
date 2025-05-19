use crate::{
    adf::adf_types::{AdfBlockNode, AdfNode},
    html_to_adf::{ADFBuilder, HandlerFn},
};

use super::{ADFBuilderState, BlockContext};

pub(crate) fn table_start_handler() -> HandlerFn {
    Box::new(|state, _element| {
        ADFBuilder::flush_text(state);
        state.stack.push(BlockContext::TableBlock(vec![]));
        true
    })
}

pub(crate) fn table_section_start_handler() -> HandlerFn {
    Box::new(|state, _element| {
        ADFBuilder::flush_text(state);
        state.stack.push(BlockContext::TableSectionBlock(vec![]));
        true
    })
}

pub(crate) fn table_section_end_handler() -> HandlerFn {
    Box::new(|state, _element| {
        ADFBuilder::flush_text(state);
        if let Some(BlockContext::TableSectionBlock(section_rows)) = state.stack.pop() {
            for row in section_rows {
                ADFBuilder::push_node_to_parent(state, row);
            }
            true
        } else {
            false
        }
    })
}

pub(crate) fn table_row_start_handler() -> HandlerFn {
    Box::new(|state, _element| {
        ADFBuilder::flush_text(state);
        state.stack.push(BlockContext::TableRowBlock(vec![]));
        true
    })
}

pub(crate) fn table_cell_start_handler() -> HandlerFn {
    Box::new(|state, _element| {
        ADFBuilder::flush_text(state);
        state.stack.push(BlockContext::TableBlockCell(vec![]));
        true
    })
}

pub(crate) fn table_header_start_handler() -> HandlerFn {
    Box::new(|state, _element| {
        ADFBuilder::flush_text(state);
        state.stack.push(BlockContext::TableBlockHeader(vec![]));
        true
    })
}

pub(crate) fn table_end_handler() -> HandlerFn {
    Box::new(|state, _element| {
        ADFBuilder::flush_text(state);
        if let Some(BlockContext::TableBlock(rows)) = state.stack.pop() {
            ADFBuilder::push_node_block_to_parent(
                state,
                AdfBlockNode::Table {
                    attrs: None,
                    content: rows,
                },
            );
            true
        } else {
            false
        }
    })
}

pub(crate) fn table_row_end_handler() -> HandlerFn {
    Box::new(|state, _element| {
        ADFBuilder::flush_text(state);
        ADFBuilder::close_current_table_row(state);
        true
    })
}

pub(crate) fn table_cell_end_handler() -> HandlerFn {
    Box::new(|state, _element| {
        ADFBuilder::flush_text(state);
        ADFBuilder::close_current_table_cell(state);
        true
    })
}

pub(crate) fn table_header_end_handler() -> HandlerFn {
    Box::new(|state, _element| {
        ADFBuilder::flush_text(state);
        ADFBuilder::close_current_table_header(state);
        true
    })
}

impl ADFBuilder {
    fn close_current_table_row(state: &mut ADFBuilderState) {
        if let Some(BlockContext::TableRowBlock(cells)) = state.stack.pop() {
            Self::push_node_to_parent(state, AdfNode::TableRow { content: cells });
        }
    }

    fn close_current_table_cell(state: &mut ADFBuilderState) {
        if let Some(BlockContext::TableBlockCell(nodes)) = state.stack.pop() {
            Self::push_node_to_parent(
                state,
                AdfNode::TableCell {
                    attrs: None,
                    content: nodes,
                },
            );
        }
    }

    fn close_current_table_header(state: &mut ADFBuilderState) {
        if let Some(BlockContext::TableBlockHeader(nodes)) = state.stack.pop() {
            Self::push_node_to_parent(
                state,
                AdfNode::TableHeader {
                    attrs: None,
                    content: nodes,
                },
            );
        }
    }
}
