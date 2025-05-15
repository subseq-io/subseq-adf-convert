use html5ever::Attribute;
use std::collections::HashMap;

mod base;
mod custom;
mod decisions;
mod table;
mod tasks;

pub(crate) use base::*;
pub(crate) use custom::*;
pub(crate) use decisions::*;
pub(crate) use table::*;
pub(crate) use tasks::*;

use crate::adf::adf_types::{AdfMark, AdfNode, LocalId, MediaNode, TaskItemState};

pub struct Element {
    pub tag: String,
    pub attrs: Vec<Attribute>,
    pub self_closing: bool,
}

pub struct ADFBuilderState {
    pub stack: Vec<BlockContext>,
    pub mark_stack: Vec<AdfMark>,
    pub current_text: String,
    pub preformatted: bool,
    pub heavy_trim: bool,
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

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum TableBlockType {
    Table,
    Section,
    Row,
    Cell,
    Header,
}

#[derive(Debug)]
pub enum BlockContext {
    Document(Vec<AdfNode>),
    Blockquote(Vec<AdfNode>),
    CodeBlock(Vec<String>),
    CustomBlock(CustomBlockType, Vec<AdfNode>, HashMap<String, String>),
    MediaBlock(MediaBlockType, Vec<MediaNode>, HashMap<String, String>),
    TableBlock(TableBlockType, Vec<AdfNode>),
    Heading(u8, Vec<AdfNode>),
    Summary(Vec<AdfNode>),
    Paragraph(Vec<AdfNode>),

    PendingList {
        nodes: Vec<AdfNode>,
        ordered: bool,
        local_id: Option<String>,
        local_tag: Option<String>,
    },
    ListItem(Vec<AdfNode>),
    TaskItem(Vec<AdfNode>, TaskItemState, String),
    DecisionItem(Vec<AdfNode>, String),
}
