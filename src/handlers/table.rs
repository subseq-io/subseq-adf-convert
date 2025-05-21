use crate::{
    adf::adf_types::{AdfBlockNode, TableRow, TableRowEntry},
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
    Box::new(|_state, _element| true)
}

pub(crate) fn table_section_end_handler() -> HandlerFn {
    Box::new(|_state, _element| true)
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
    fn push_row_to_table(state: &mut ADFBuilderState, row: TableRow) {
        if let Some(BlockContext::TableBlock(rows)) = state.stack.last_mut() {
            rows.push(row);
        } else {
            panic!("No table block found in stack");
        }
    }

    fn push_cell_to_row(state: &mut ADFBuilderState, cell_nodes: Vec<AdfBlockNode>) {
        if let Some(BlockContext::TableRowBlock(cells)) = state.stack.last_mut() {
            cells.push(TableRowEntry::new_table_cell(cell_nodes, None));
        } else {
            panic!("No table row block found in stack");
        }
    }

    fn push_header_to_row(state: &mut ADFBuilderState, cell_nodes: Vec<AdfBlockNode>) {
        if let Some(BlockContext::TableRowBlock(cells)) = state.stack.last_mut() {
            cells.push(TableRowEntry::new_table_header(cell_nodes, None));
        } else {
            panic!("No table row block found in stack");
        }
    }

    fn close_current_table_row(state: &mut ADFBuilderState) {
        if let Some(BlockContext::TableRowBlock(cells)) = state.stack.pop() {
            Self::push_row_to_table(state, TableRow::new(cells));
        } else {
            panic!("No table row block found in stack");
        }
    }

    fn close_current_table_cell(state: &mut ADFBuilderState) {
        if let Some(BlockContext::TableBlockCell(nodes)) = state.stack.pop() {
            Self::push_cell_to_row(state, nodes);
        } else {
            panic!("No table cell block found in stack");
        }
    }

    fn close_current_table_header(state: &mut ADFBuilderState) {
        if let Some(BlockContext::TableBlockHeader(nodes)) = state.stack.pop() {
            Self::push_header_to_row(state, nodes);
        } else {
            panic!("No table header block found in stack");
        }
    }
}
