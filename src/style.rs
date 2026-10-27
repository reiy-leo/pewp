//! 样式：各格式都能表达的那一小部分。

use std::collections::BTreeMap;

use anyhow::{Result, bail};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Style {
    /// 字体名，如 "Noto Sans SC"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_family: Option<String>,
    /// 字号，单位 pt
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_size: Option<f32>,
    /// 颜色，#RRGGBB
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bold: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub italic: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub underline: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub align: Option<Align>,
    /// 行距倍数，如 1.5
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_height: Option<f32>,
    /// 段前，pt
    #[serde(skip_serializing_if = "Option::is_none")]
    pub space_before: Option<f32>,
    /// 段后，pt
    #[serde(skip_serializing_if = "Option::is_none")]
    pub space_after: Option<f32>,
    /// 背景色，#RRGGBB
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background: Option<String>,
    /// 边框颜色，#RRGGBB（1pt 实线）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub border: Option<String>,
    /// 内边距，pt
    #[serde(skip_serializing_if = "Option::is_none")]
    pub padding: Option<f32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Align {
    Left,
    Center,
    Right,
    Justify,
}

impl Align {
    pub fn as_str(self) -> &'static str {
        match self {
            Align::Left => "left",
            Align::Center => "center",
            Align::Right => "right",
            Align::Justify => "justify",
        }
    }
}

impl Style {
    /// 用 `other` 里有值的字段覆盖自己。
    pub fn merge(&mut self, other: &Style) {
        macro_rules! m { ($($f:ident),*) => { $( if other.$f.is_some() { self.$f = other.$f.clone(); } )* } }
        m!(
            font_family,
            font_size,
            color,
            bold,
            italic,
            underline,
            align,
            line_height,
            space_before,
            space_after,
            background,
            border,
            padding
        );
    }

    pub fn is_empty(&self) -> bool {
        *self == Style::default()
    }
}

/// 解析元素上的 `style`：可以是样式名、样式名数组，也可以是样式对象。
pub fn resolve(v: Option<&Value>, named: &BTreeMap<String, Style>) -> Result<Style> {
    let mut out = Style::default();
    let Some(v) = v else { return Ok(out) };
    match v {
        Value::Null => {}
        Value::String(name) => out.merge(lookup(name, named)?),
        Value::Array(items) => {
            for it in items {
                out.merge(&resolve(Some(it), named)?);
            }
        }
        Value::Object(_) => out.merge(&serde_json::from_value(v.clone())?),
        _ => bail!("style 只能是样式名、样式名数组或样式对象"),
    }
    Ok(out)
}

fn lookup<'a>(name: &str, named: &'a BTreeMap<String, Style>) -> Result<&'a Style> {
    named
        .get(name)
        .ok_or_else(|| anyhow::anyhow!("样式 `{name}` 没在 styles 里定义"))
}
