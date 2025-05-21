use html5ever::Attribute;
use std::collections::HashMap;

mod base;
mod custom;
mod decisions;
mod media;
mod table;
mod tasks;

pub(crate) use base::*;
pub(crate) use custom::*;
pub(crate) use decisions::*;
pub(crate) use media::*;
pub(crate) use table::*;
pub(crate) use tasks::*;

use crate::adf::adf_types::{
    AdfBlockNode, AdfMark, AdfNode, DecisionItem, ListItem, LocalId, MediaNode, TableRow,
    TableRowEntry, TaskItem, TaskItemState,
};

#[derive(Debug)]
pub struct Element {
    pub tag: String,
    pub attrs: Vec<Attribute>,
    pub self_closing: bool,
}

pub struct ADFBuilderState {
    pub stack: Vec<BlockContext>,
    pub mark_stack: Vec<AdfMark>,
    pub current_text: String,
    pub custom_block_id: Option<LocalId>,
    pub custom_block_tag: Option<String>,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum CustomBlockType {
    Div,
    Emoji,
    Status,
    DecisionItem,
    Expand,
    NestedExpand,
    Panel,
    Mention,
    InlineCard,
    Date,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum MediaBlockType {
    MediaGroup,
    MediaSingle,
}

#[derive(Debug)]
pub enum ListItemType {
    DecisionItem(DecisionItem),
    ListItem(ListItem),
    TaskItem(TaskItem),
}

#[derive(Debug)]
pub enum BlockContext {
    Document(Vec<AdfBlockNode>),
    Blockquote(Vec<AdfBlockNode>),
    CodeBlock(Vec<String>),
    CustomBlock(CustomBlockType, Vec<AdfBlockNode>, HashMap<String, String>),
    MediaBlock(MediaBlockType, Vec<MediaNode>, HashMap<String, String>),
    TableBlock(Vec<TableRow>),
    TableRowBlock(Vec<TableRowEntry>),
    TableBlockCell(Vec<AdfBlockNode>),
    TableBlockHeader(Vec<AdfBlockNode>),
    Heading(u8, Vec<AdfNode>),
    Summary(Vec<AdfNode>),
    Paragraph(Vec<AdfNode>),
    PendingList {
        nodes: Vec<ListItemType>,
        ordered: bool,
        local_id: Option<String>,
        local_tag: Option<String>,
    },
    ListItem(Vec<AdfBlockNode>),
    TaskItem(Vec<AdfNode>, TaskItemState, String),
    DecisionItem(Vec<AdfNode>, String),
}
