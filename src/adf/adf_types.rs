use serde::{Deserialize, Serialize};
use strum_macros::{AsRefStr, Display, EnumIter, EnumString};

macro_rules! fixed_type_tag {
    ($name:ident, $val:expr) => {
        #[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
        pub struct $name;

        impl ::serde::Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: ::serde::Serializer,
            {
                serializer.serialize_str($val)
            }
        }

        impl ::std::fmt::Display for $name {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                f.write_str($val)
            }
        }

        impl<'de> ::serde::Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: ::serde::Deserializer<'de>,
            {
                let s = <String>::deserialize(deserializer)?;
                if s == $val {
                    Ok($name)
                } else {
                    Err(::serde::de::Error::custom(format!(
                        "expected '{}', got '{}'",
                        $val, s
                    )))
                }
            }
        }
    };
}

fixed_type_tag!(TableHeaderType, "tableHeader");
#[derive(Clone, Deserialize, Serialize, Eq, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TableHeader {
    #[serde(rename = "type")]
    type_: TableHeaderType,
    #[serde(skip_serializing_if = "Option::is_none")]
    attrs: Option<TableCellAttrs>,
    content: Vec<AdfBlockNode>,
}

impl TableHeader {
    pub fn attrs(&self) -> &Option<TableCellAttrs> {
        &self.attrs
    }

    pub fn content(&self) -> &Vec<AdfBlockNode> {
        &self.content
    }

    pub fn unwrap(self) -> (Vec<AdfBlockNode>, Option<TableCellAttrs>) {
        let Self { content, attrs, .. } = self;
        (content, attrs)
    }
}

fixed_type_tag!(TableCellType, "tableCell");
#[derive(Clone, Deserialize, Serialize, Eq, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TableCell {
    #[serde(rename = "type")]
    type_: TableCellType,
    #[serde(skip_serializing_if = "Option::is_none")]
    attrs: Option<TableCellAttrs>,
    content: Vec<AdfBlockNode>,
}

impl TableCell {
    pub fn attrs(&self) -> &Option<TableCellAttrs> {
        &self.attrs
    }

    pub fn content(&self) -> &Vec<AdfBlockNode> {
        &self.content
    }

    pub fn unwrap(self) -> (Vec<AdfBlockNode>, Option<TableCellAttrs>) {
        let Self { content, attrs, .. } = self;
        (content, attrs)
    }
}

#[derive(Clone, Deserialize, Serialize, Eq, PartialEq, Debug, Display)]
#[strum(serialize_all = "camelCase")]
#[serde(untagged)]
pub enum TableRowEntry {
    TableHeader(TableHeader),
    TableCell(TableCell),
}

impl TableRowEntry {
    pub fn new_table_header(content: Vec<AdfBlockNode>, attrs: Option<TableCellAttrs>) -> Self {
        Self::TableHeader(TableHeader {
            type_: TableHeaderType,
            content,
            attrs,
        })
    }

    pub fn new_table_cell(content: Vec<AdfBlockNode>, attrs: Option<TableCellAttrs>) -> Self {
        Self::TableCell(TableCell {
            type_: TableCellType,
            content,
            attrs,
        })
    }
}

fixed_type_tag!(TableRowType, "tableRow");
#[derive(Clone, Deserialize, Serialize, Eq, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TableRow {
    #[serde(rename = "type")]
    type_: TableRowType,
    content: Vec<TableRowEntry>,
}

impl TableRow {
    pub fn new(content: Vec<TableRowEntry>) -> Self {
        Self {
            type_: TableRowType,
            content,
        }
    }

    pub fn content(&self) -> &Vec<TableRowEntry> {
        &self.content
    }

    pub fn unwrap(self) -> Vec<TableRowEntry> {
        self.content
    }
}

fixed_type_tag!(TaskItemType, "taskItem");
#[derive(Clone, Deserialize, Serialize, Eq, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TaskItem {
    #[serde(rename = "type")]
    type_: TaskItemType,
    content: Vec<AdfNode>,
    attrs: TaskItemAttrs,
}

impl TaskItem {
    pub fn new(content: Vec<AdfNode>, attrs: TaskItemAttrs) -> Self {
        Self {
            type_: TaskItemType,
            content,
            attrs,
        }
    }

    pub fn content(&self) -> &Vec<AdfNode> {
        &self.content
    }

    pub fn attrs(&self) -> &TaskItemAttrs {
        &self.attrs
    }

    pub fn unwrap(self) -> (Vec<AdfNode>, TaskItemAttrs) {
        let Self { content, attrs, .. } = self;
        (content, attrs)
    }
}

fixed_type_tag!(DecisionItemType, "decisionItem");
#[derive(Clone, Deserialize, Serialize, Eq, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DecisionItem {
    #[serde(rename = "type")]
    type_: DecisionItemType,
    content: Vec<AdfNode>,
    attrs: DecisionItemAttrs,
}

impl DecisionItem {
    pub fn new(content: Vec<AdfNode>, attrs: DecisionItemAttrs) -> Self {
        Self {
            type_: DecisionItemType,
            content,
            attrs,
        }
    }

    pub fn content(&self) -> &Vec<AdfNode> {
        &self.content
    }

    pub fn attrs(&self) -> &DecisionItemAttrs {
        &self.attrs
    }

    pub fn unwrap(self) -> (Vec<AdfNode>, DecisionItemAttrs) {
        let Self { content, attrs, .. } = self;
        (content, attrs)
    }
}

fixed_type_tag!(ListItemType, "listItem");
#[derive(Clone, Deserialize, Serialize, Eq, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ListItem {
    #[serde(rename = "type")]
    type_: ListItemType,
    content: Vec<AdfBlockNode>,
}

impl ListItem {
    pub fn new(content: Vec<AdfBlockNode>) -> Self {
        Self {
            type_: ListItemType,
            content,
        }
    }

    pub fn content(&self) -> &Vec<AdfBlockNode> {
        &self.content
    }

    pub fn unwrap(self) -> Vec<AdfBlockNode> {
        self.content
    }
}

#[derive(Clone, Deserialize, Serialize, Eq, PartialEq, Debug, EnumString, Display)]
#[strum(serialize_all = "camelCase")]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum AdfNode {
    HardBreak,
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
    Mention {
        attrs: MentionAttrs,
    },
    Status {
        attrs: StatusAttrs,
    },
    #[serde(other)]
    Unknown,
}

#[derive(Clone, Deserialize, Serialize, Eq, PartialEq, Debug, EnumString, Display)]
#[strum(serialize_all = "camelCase")]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum AdfBlockNode {
    Doc {
        content: Vec<AdfBlockNode>,
        version: i32,
    },
    Blockquote {
        content: Vec<AdfBlockNode>,
    },
    BulletList {
        content: Vec<ListItem>,
    },
    CodeBlock {
        #[serde(skip_serializing_if = "Option::is_none")]
        attrs: Option<CodeBlockAttrs>,
        #[serde(skip_serializing_if = "Option::is_none")]
        content: Option<Vec<AdfNode>>,
    },
    Expand {
        content: Vec<AdfBlockNode>,
        attrs: ExpandAttrs,
    },
    NestedExpand {
        attrs: NestedAttrs,
        content: Vec<AdfBlockNode>,
    },
    Paragraph {
        #[serde(skip_serializing_if = "Option::is_none")]
        content: Option<Vec<AdfNode>>,
    },
    Rule,
    Heading {
        attrs: HeadingAttrs,
        #[serde(skip_serializing_if = "Option::is_none")]
        content: Option<Vec<AdfNode>>,
    },
    Panel {
        attrs: PanelAttrs,
        content: Vec<AdfBlockNode>,
    },
    MediaGroup {
        content: Vec<MediaNode>,
    },
    MediaSingle {
        attrs: MediaSingleAttrs,
        content: Vec<MediaNode>,
    },
    Table {
        #[serde(skip_serializing_if = "Option::is_none")]
        attrs: Option<TableAttrs>,
        content: Vec<TableRow>,
    },
    OrderedList {
        #[serde(skip_serializing_if = "Option::is_none")]
        attrs: Option<OrderedListAttrs>,
        content: Vec<ListItem>,
    },
    // The following nodes exist but are not documented in the ADF spec:
    BlockCard {
        attrs: BlockCardAttrs,
    },
    TaskList {
        attrs: LocalId,
        content: Vec<TaskItem>,
    },
    DecisionList {
        content: Vec<DecisionItem>,
        attrs: LocalId,
    },
    #[serde(other)]
    Unknown,
}

impl AdfBlockNode {
    pub fn unwrap_doc(&mut self) -> Vec<AdfBlockNode> {
        if let Self::Doc { content, .. } = self {
            return content.clone();
        }
        Vec::new()
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
#[serde(rename_all = "lowercase")]
pub enum MediaType {
    #[default]
    Media,
}

#[derive(Clone, Deserialize, Serialize, Eq, PartialEq, Debug, Default)]
#[serde(rename_all = "lowercase")]
pub enum MediaDataType {
    #[default]
    File,
    Link,
}

#[derive(Clone, Deserialize, Serialize, Eq, PartialEq, Debug, Default)]
pub struct MediaNode {
    #[serde(rename = "type")]
    pub media_type: MediaType,
    pub attrs: MediaAttrs,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub marks: Option<Vec<MediaMark>>,
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
    pub type_: MediaDataType,
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
#[serde(rename_all = "UPPERCASE")]
pub enum AccessLevel {
    #[default]
    None,
    Site,
    Application,
}

#[derive(Clone, Deserialize, Serialize, Eq, PartialEq, Debug, Default)]
#[serde(rename_all = "UPPERCASE")]
pub enum UserType {
    #[default]
    Default,
    Special,
    App,
}

#[derive(Clone, Deserialize, Serialize, Eq, PartialEq, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct MentionAttrs {
    pub access_level: Option<AccessLevel>,
    pub id: String,
    pub text: Option<String>,
    pub user_type: Option<UserType>,
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
    pub layout: String,
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

fixed_type_tag!(DecisionItemState, "DECIDED");
#[derive(Clone, Deserialize, Serialize, Eq, PartialEq, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct DecisionItemAttrs {
    pub state: DecisionItemState,
    pub local_id: String,
}

#[derive(Clone, Deserialize, Serialize, Eq, PartialEq, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct LocalId {
    pub local_id: String,
}

#[derive(Clone, Deserialize, Serialize, Eq, PartialEq, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct BlockCardAttrs {
    pub datasource: DataSourceAttrs,
    pub url: String,
}

#[derive(Clone, Deserialize, Serialize, Eq, PartialEq, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct DataSourceAttrs {
    pub id: String,
    pub parameters: DataSourceParameters,
    pub views: Vec<DataSourceView>,
}

#[derive(Clone, Deserialize, Serialize, Eq, PartialEq, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct DataSourceParameters {
    pub cloud_id: String,
    pub jql: String,
}

#[derive(Clone, Deserialize, Serialize, Eq, PartialEq, Debug)]
#[serde(tag = "type", content = "properties", rename_all = "camelCase")]
pub enum DataSourceView {
    Table(TableViewProperties),
}

#[derive(Clone, Deserialize, Serialize, Eq, PartialEq, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct TableViewProperties {
    pub columns: Vec<TableColumn>,
}

#[derive(Clone, Deserialize, Serialize, Eq, PartialEq, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct TableColumn {
    pub key: String,
}
