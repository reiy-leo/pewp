//! 内置组件和它们的 props。

use schemars::{JsonSchema, Schema, schema_for};
use serde::{Deserialize, Serialize};

/// 一段文字里的一小段，可以单独加粗、变色或加链接。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Run {
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bold: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub italic: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub underline: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub href: Option<String>,
}

/// 纯文本，或者分段的富文本。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum Inline {
    Plain(String),
    Runs(Vec<Run>),
}

impl Inline {
    pub fn runs(&self) -> Vec<Run> {
        match self {
            Inline::Plain(s) => vec![Run {
                text: s.clone(),
                bold: None,
                italic: None,
                underline: None,
                code: None,
                color: None,
                href: None,
            }],
            Inline::Runs(r) => r.clone(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DocumentProps {}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct HeadingProps {
    /// 1 到 6
    #[schemars(range(min = 1, max = 6))]
    pub level: u8,
    pub text: Inline,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TextProps {
    pub text: Inline,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ListProps {
    /// true 是编号列表
    #[serde(default)]
    pub ordered: bool,
    pub items: Vec<Inline>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TableProps {
    /// 表头；不写就没有表头行
    #[serde(default)]
    pub columns: Vec<Inline>,
    pub rows: Vec<Vec<Inline>>,
    /// 每列宽度占比，如 [1, 2, 1]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub widths: Option<Vec<f32>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ImageProps {
    /// 本地路径（相对文档所在目录）或 http(s) 链接
    pub src: String,
    #[serde(default)]
    pub alt: String,
    /// 宽度，pt
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<f32>,
    /// 高度，pt
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub caption: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PageBreakProps {}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SlideProps {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

/// 解析好的组件。
#[derive(Debug, Clone)]
pub enum Component {
    Document(DocumentProps),
    Heading(HeadingProps),
    Text(TextProps),
    List(ListProps),
    Table(TableProps),
    Image(ImageProps),
    PageBreak(PageBreakProps),
    Slide(SlideProps),
}

pub struct Spec {
    pub name: &'static str,
    pub description: &'static str,
    /// 能不能有 children
    pub container: bool,
    pub schema: fn() -> Schema,
}

pub const BUILTINS: &[Spec] = &[
    Spec {
        name: "Document",
        description: "根容器，按顺序排列子元素",
        container: true,
        schema: || schema_for!(DocumentProps),
    },
    Spec {
        name: "Heading",
        description: "标题，level 1 到 6",
        container: false,
        schema: || schema_for!(HeadingProps),
    },
    Spec {
        name: "Text",
        description: "一段正文，可以是纯文本或富文本 runs",
        container: false,
        schema: || schema_for!(TextProps),
    },
    Spec {
        name: "List",
        description: "列表，ordered 为 true 时带编号",
        container: false,
        schema: || schema_for!(ListProps),
    },
    Spec {
        name: "Table",
        description: "表格，columns 是表头，rows 是数据行",
        container: false,
        schema: || schema_for!(TableProps),
    },
    Spec {
        name: "Image",
        description: "图片，src 是本地路径或链接",
        container: false,
        schema: || schema_for!(ImageProps),
    },
    Spec {
        name: "PageBreak",
        description: "分页",
        container: false,
        schema: || schema_for!(PageBreakProps),
    },
    Spec {
        name: "Slide",
        description: "一页幻灯片（16:9），子元素排在里面",
        container: true,
        schema: || schema_for!(SlideProps),
    },
];

pub fn spec(name: &str) -> Option<&'static Spec> {
    BUILTINS.iter().find(|s| s.name == name)
}

pub fn parse(kind: &str, props: &serde_json::Value) -> anyhow::Result<Component> {
    let p = if props.is_null() {
        serde_json::json!({})
    } else {
        props.clone()
    };
    Ok(match kind {
        "Document" => Component::Document(serde_json::from_value(p)?),
        "Heading" => {
            let h: HeadingProps = serde_json::from_value(p)?;
            if !(1..=6).contains(&h.level) {
                anyhow::bail!("level 要在 1 到 6 之间，现在是 {}", h.level);
            }
            Component::Heading(h)
        }
        "Text" => Component::Text(serde_json::from_value(p)?),
        "List" => Component::List(serde_json::from_value(p)?),
        "Table" => Component::Table(serde_json::from_value(p)?),
        "Image" => Component::Image(serde_json::from_value(p)?),
        "PageBreak" => Component::PageBreak(serde_json::from_value(p)?),
        "Slide" => Component::Slide(serde_json::from_value(p)?),
        other => anyhow::bail!("组件 `{other}` 没注册"),
    })
}
