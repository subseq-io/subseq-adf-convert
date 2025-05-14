use crate::{
    adf::adf_types::AdfNode,
    html_to_adf::{ADFBuilder, HandlerFn},
};

use super::{ADFBuilderState, BlockContext, TableBlockType};

pub(crate) fn table_start_handler() -> HandlerFn {
    Box::new(|state, _element| {
        ADFBuilder::flush_text(state);
        state
            .stack
            .push(BlockContext::TableBlock(TableBlockType::Table, vec![]));
        true
    })
}

pub(crate) fn table_section_start_handler() -> HandlerFn {
    Box::new(|state, _element| {
        ADFBuilder::flush_text(state);
        state
            .stack
            .push(BlockContext::TableBlock(TableBlockType::Section, vec![]));
        true
    })
}

pub(crate) fn table_section_end_handler() -> HandlerFn {
    Box::new(|state, _element| {
        ADFBuilder::flush_text(state);
        if let Some(BlockContext::TableBlock(TableBlockType::Section, section_rows)) =
            state.stack.pop()
        {
            for row in section_rows {
                ADFBuilder::push_block_to_parent(state, row);
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
        state
            .stack
            .push(BlockContext::TableBlock(TableBlockType::Row, vec![]));
        true
    })
}

pub(crate) fn table_cell_start_handler() -> HandlerFn {
    Box::new(|state, _element| {
        ADFBuilder::flush_text(state);
        state
            .stack
            .push(BlockContext::TableBlock(TableBlockType::Cell, vec![]));
        true
    })
}

pub(crate) fn table_header_start_handler() -> HandlerFn {
    Box::new(|state, _element| {
        ADFBuilder::flush_text(state);
        state
            .stack
            .push(BlockContext::TableBlock(TableBlockType::Header, vec![]));
        true
    })
}

pub(crate) fn table_end_handler() -> HandlerFn {
    Box::new(|state, _element| {
        ADFBuilder::flush_text(state);
        if let Some(BlockContext::TableBlock(TableBlockType::Table, rows)) = state.stack.pop() {
            ADFBuilder::push_block_to_parent(
                state,
                AdfNode::Table {
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
        if let Some(BlockContext::TableBlock(TableBlockType::Row, cells)) = state.stack.pop() {
            Self::push_block_to_parent(state, AdfNode::TableRow { content: cells });
        }
    }

    fn close_current_table_cell(state: &mut ADFBuilderState) {
        if let Some(BlockContext::TableBlock(TableBlockType::Cell, content)) = state.stack.pop() {
            Self::push_block_to_parent(
                state,
                AdfNode::TableCell {
                    attrs: None,
                    content,
                },
            );
        }
    }

    fn close_current_table_header(state: &mut ADFBuilderState) {
        if let Some(BlockContext::TableBlock(TableBlockType::Header, content)) = state.stack.pop() {
            Self::push_block_to_parent(
                state,
                AdfNode::TableHeader {
                    attrs: None,
                    content,
                },
            );
        }
    }
}
