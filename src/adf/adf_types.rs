use serde::{Deserialize, Serialize};
use strum_macros::{AsRefStr, Display, EnumIter, EnumString};

#[derive(Clone, Deserialize, Serialize, Eq, PartialEq, Debug, EnumString, Display)]
#[strum(serialize_all = "camelCase")]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum AdfNode {
    Doc {
        content: Vec<AdfNode>,
        version: i32,
    },
    Blockquote {
        content: Vec<AdfNode>,
    },
    BulletList {
        content: Vec<AdfNode>,
    },
    CodeBlock {
        #[serde(skip_serializing_if = "Option::is_none")]
        attrs: Option<CodeBlockAttrs>,
        #[serde(skip_serializing_if = "Option::is_none")]
        content: Option<Vec<AdfNode>>,
    },
    HardBreak,
    Heading {
        attrs: HeadingAttrs,
        #[serde(skip_serializing_if = "Option::is_none")]
        content: Option<Vec<AdfNode>>,
    },
    ListItem {
        content: Vec<AdfNode>,
    },
    OrderedList {
        #[serde(skip_serializing_if = "Option::is_none")]
        attrs: Option<OrderedListAttrs>,
        content: Vec<AdfNode>,
    },
    Paragraph {
        #[serde(skip_serializing_if = "Option::is_none")]
        content: Option<Vec<AdfNode>>,
    },
    Rule,
    Table {
        #[serde(skip_serializing_if = "Option::is_none")]
        attrs: Option<TableAttrs>,
        content: Vec<AdfNode>,
    },
    TableCell {
        attrs: Option<TableCellAttrs>,
        content: Vec<AdfNode>,
    },
    TableHeader {
        attrs: Option<TableCellAttrs>,
        content: Vec<AdfNode>,
    },
    TableRow {
        content: Vec<AdfNode>,
    },
    Text {
        text: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        marks: Option<Vec<AdfMark>>,
    },
    // Nodes which do not directly correspond to an HTML element
    Date {
        attrs: DateAttrs,
    },
    InlineCard {
        attrs: InlineCardAttrs,
    },
    Emoji {
        attrs: EmojiAttrs,
    },
    Expand {
        content: Vec<AdfNode>,
        attrs: ExpandAttrs,
    },
    Panel {
        attrs: PanelAttrs,
        content: Vec<AdfNode>,
    },
    MediaGroup {
        content: Vec<MediaNode>,
    },
    MediaSingle {
        #[serde(skip_serializing_if = "Option::is_none")]
        attrs: Option<MediaSingleAttrs>,
        content: Vec<MediaNode>,
    },
    Mention {
        attrs: MentionAttrs,
    },
    NestedExpand {
        attrs: NestedAttrs,
        content: Vec<AdfNode>,
    },
    Status {
        attrs: StatusAttrs,
    },
    // The following nodes exist but are not documented in the ADF spec:
    // https://developer.atlassian.com/cloud/jira/platform/apis/document/structure/
    TaskList {
        attrs: LocalId,
        content: Vec<AdfNode>,
    },
    TaskItem {
        content: Vec<AdfNode>,
        attrs: TaskItemAttrs,
    },
    DecisionList {
        content: Vec<AdfNode>,
        attrs: LocalId,
    },
    DecisionItem {
        content: Vec<AdfNode>,
        attrs: DecisionItemAttrs,
    },
    #[serde(other)]
    Unknown,
}

impl AdfNode {
    pub fn unwrap_doc(&mut self) -> Vec<AdfNode> {
        if let Self::Doc { content, .. } = self {
            return content.clone();
        }

        eprintln!("unwrap_doc called on non-doc node");
        Vec::new()
    }

    pub fn is_task_item(&self) -> bool {
        matches!(self, Self::TaskItem { .. })
    }
}

#[derive(Clone, Deserialize, Debug, Serialize, Eq, PartialEq, Default)]
pub struct LinkMark {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub collection: Option<String>,
    pub href: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub occurrence_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

#[derive(Clone, Deserialize, Debug, Serialize, Eq, PartialEq, EnumIter, AsRefStr)]
#[serde(tag = "type", content = "attrs", rename_all = "camelCase")]
pub enum AdfMark {
    // Make sure to define the marks with longer markup strings first so they won't
    // be matched by shorter ones first. e.g. Strong (**) should be before Em (*).
    // The order here matters because we're using strum's EnumIter to iterate this,
    // which iterates in the order they are defined.
    Strong,
    Em,
    Code,
    Link(LinkMark),
    Strike,
    Subsup {
        #[serde(rename = "type")]
        type_: Subsup,
    },
    TextColor {
        color: String,
    },
    Underline,
    /// BackgroundColor doesn't actually exist in the Jira issue editor
    /// but exists in the ADF spec.
    BackgroundColor {
        color: String,
    },
}

impl AdfMark {
    pub fn markup_string(&self) -> Option<String> {
        Some(match self {
            AdfMark::Code => "`".to_owned(),
            AdfMark::Em => "*".to_owned(),
            AdfMark::Strike => "~~".to_owned(),
            AdfMark::Strong => "**".to_owned(),
            AdfMark::Underline => "__".to_owned(),
            AdfMark::Subsup { type_ } => match type_ {
                Subsup::Sub => "~".to_owned(),
                Subsup::Sup => "^".to_owned(),
            },
            AdfMark::TextColor { color } => {
                if let Some(text_color) = TextColor::from_hex_string(color) {
                    return Some(format!("{{color:{}}}", text_color.to_string()));
                }
                return None;
            }
            _ => {
                return None;
            }
        })
    }
}

pub enum ParseNextResponse {
    Char(char),
    Node(AdfNode),
    MarkAction(MarkAction),
}

pub enum MarkAction {
    Add(AdfMark),
    Remove(AdfMark),
}

impl MarkAction {
    pub fn apply_to(&self, marks: &mut Vec<AdfMark>) {
        match self {
            MarkAction::Add(mark) => {
                if !marks.contains(mark) {
                    match mark {
                        AdfMark::Code => {
                            *marks = vec![mark.clone()]; // Code cannot be combined with other marks
                        }
                        _ => {
                            marks.push(mark.clone());
                        }
                    }
                }
            }
            MarkAction::Remove(mark) => {
                marks.retain(|m| m != mark);
            }
        }
    }
}

#[derive(Debug, EnumString, Display, EnumIter, PartialEq, Clone, AsRefStr)]
#[strum(serialize_all = "snake_case")]
pub enum TextColor {
    BoldBlue,
    BoldTeal,
    BoldGreen,
    BoldOrange,
    BoldRed,
    BoldPurple,
    Gray,
    Blue,
    Teal,
    Green,
    Yellow,
    Red,
    Purple,
    White,
    SubtleBlue,
    SubtleTeal,
    SubtleGreen,
    SubtleYellow,
    SubtleRed,
    SubtlePurple,
}

impl TextColor {
    const fn mapping() -> &'static [(&'static str, TextColor)] {
        &[
            ("#0747a6", TextColor::BoldBlue),
            ("#008da6", TextColor::BoldTeal),
            ("#006644", TextColor::BoldGreen),
            ("#ff991f", TextColor::BoldOrange),
            ("#bf2600", TextColor::BoldRed),
            ("#403294", TextColor::BoldPurple),
            ("#97a0af", TextColor::Gray),
            ("#4c9aff", TextColor::Blue),
            ("#00b8d9", TextColor::Teal),
            ("#36b37e", TextColor::Green),
            ("#ffc400", TextColor::Yellow),
            ("#ff5630", TextColor::Red),
            ("#6554c0", TextColor::Purple),
            ("#ffffff", TextColor::White),
            ("#b3d4ff", TextColor::SubtleBlue),
            ("#b3f5ff", TextColor::SubtleTeal),
            ("#abf5d1", TextColor::SubtleGreen),
            ("#fff0b3", TextColor::SubtleYellow),
            ("#ffbdad", TextColor::SubtleRed),
            ("#eae6ff", TextColor::SubtlePurple),
        ]
    }

    pub fn as_hex_string(&self) -> String {
        for (hex, color) in Self::mapping() {
            if color == self {
                return hex.to_string();
            }
        }

        unreachable!("Missing color mapping for {:?}", self)
    }

    fn from_hex_string(s: &str) -> Option<Self> {
        for (hex, color) in Self::mapping() {
            if *hex == s {
                return Some(color.clone());
            }
        }

        None
    }
}

#[derive(Clone, Deserialize, Debug, Serialize, Eq, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub enum Subsup {
    #[default]
    Sub,
    Sup,
}

#[derive(Clone, Deserialize, Debug, Serialize, Eq, PartialEq, Default)]
pub struct HeadingAttrs {
    pub level: u8, // Heading level (1 to 6)
}

#[derive(Clone, Deserialize, Debug, Serialize, Eq, PartialEq, Default)]
pub struct ExpandAttrs {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

#[derive(Clone, Deserialize, Debug, Serialize, Eq, PartialEq, Default)]
pub struct CodeBlockAttrs {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>, // Optional programming language
}

#[derive(Clone, Deserialize, Debug, Serialize, Eq, PartialEq, Default)]
pub struct OrderedListAttrs {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order: Option<u32>,
}

#[derive(Clone, Deserialize, Serialize, Eq, PartialEq, Debug, Default)]
pub struct DateAttrs {
    pub timestamp: String,
}

#[derive(Clone, Deserialize, Serialize, Eq, PartialEq, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct EmojiAttrs {
    pub short_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

#[derive(Clone, Deserialize, Serialize, Eq, PartialEq, Debug, Default)]
pub struct InlineCardAttrs {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

#[derive(Clone, Deserialize, Serialize, Eq, PartialEq, Debug, Default)]
pub struct MediaNode {
    #[serde(rename = "type")]
    pub media_type: String,
    pub attrs: MediaAttrs,
    pub marks: Vec<MediaMark>,
}

#[derive(Clone, Deserialize, Debug, Serialize, Eq, PartialEq, Default)]
pub struct MediaAttrs {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alt: Option<String>,
    pub collection: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
    pub id: String,
    #[serde(rename = "type")]
    pub type_: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
}

#[derive(Clone, Deserialize, Debug, Serialize, Eq, PartialEq, EnumIter, AsRefStr)]
#[serde(tag = "type", content = "attrs", rename_all = "camelCase")]
pub enum MediaMark {
    Link(LinkMark),
    Border { color: String, size: u32 },
}

#[derive(Clone, Deserialize, Serialize, Eq, PartialEq, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct MentionAttrs {
    pub access_level: Option<String>,
    pub id: String,
    pub text: Option<String>,
    pub user_type: Option<String>,
}

#[derive(Clone, Deserialize, Serialize, Eq, PartialEq, Debug, Default)]
pub struct NestedAttrs {
    pub title: String,
}

#[derive(Clone, Deserialize, Serialize, Eq, PartialEq, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct PanelAttrs {
    pub panel_type: String,
}

#[derive(Clone, Deserialize, Serialize, Eq, PartialEq, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct StatusAttrs {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub local_id: Option<String>,
    pub text: String,
    pub color: String,
}

#[derive(Clone, Deserialize, Serialize, Eq, PartialEq, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct TableAttrs {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_number_column_enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub layout: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_mode: Option<String>,
}

#[derive(Clone, Deserialize, Serialize, Eq, PartialEq, Debug, Default)]
pub struct TableCellAttrs {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub colspan: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub colwidth: Option<Vec<u32>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rowspan: Option<u32>,
}

#[derive(Clone, Deserialize, Serialize, Eq, PartialEq, Debug, Default)]
pub struct MediaSingleAttrs {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub layout: Option<String>,
}

#[derive(Clone, Deserialize, Serialize, Eq, PartialEq, Debug, Default)]
#[serde(rename_all = "UPPERCASE")]
pub enum TaskItemState {
    #[default]
    Todo,
    Done,
}

#[derive(Clone, Deserialize, Serialize, Eq, PartialEq, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct TaskItemAttrs {
    pub local_id: String,
    pub state: TaskItemState,
}

#[derive(Clone, Deserialize, Serialize, Eq, PartialEq, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct DecisionItemAttrs {
    pub state: String,
    pub local_id: String,
}

#[derive(Clone, Deserialize, Serialize, Eq, PartialEq, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct LocalId {
    pub local_id: String,
}
