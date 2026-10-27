//! 文档模型：沿用 json-render 的扁平 elements + root。

use std::{collections::BTreeMap, path::Path};

use anyhow::{Context, Result, bail};
use serde::Deserialize;
use serde_json::Value;

pub const BUILTINS: &[&str] = &[
    "Document",
    "Heading",
    "Text",
    "List",
    "Table",
    "Image",
    "PageBreak",
    "Slide",
];

#[derive(Debug, Deserialize)]
pub struct Doc {
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub meta: Value,
    #[serde(default)]
    pub styles: BTreeMap<String, Value>,
    pub root: String,
    pub elements: BTreeMap<String, Element>,
}

#[derive(Debug, Deserialize)]
pub struct Element {
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub props: Value,
    #[serde(default)]
    pub style: Option<Value>,
    #[serde(default)]
    pub children: Vec<String>,
}

pub fn load(path: &Path) -> Result<Doc> {
    let text =
        std::fs::read_to_string(path).with_context(|| format!("读不了 {}", path.display()))?;
    serde_json::from_str(&text).with_context(|| format!("{} 不是合法的 pewp JSON", path.display()))
}

impl Doc {
    pub fn validate(&self) -> Result<()> {
        if !self.elements.contains_key(&self.root) {
            bail!("root 指向的元素 `{}` 不存在", self.root);
        }
        for (id, el) in &self.elements {
            if !BUILTINS.contains(&el.kind.as_str()) {
                bail!("元素 `{id}` 的组件 `{}` 没注册", el.kind);
            }
            for c in &el.children {
                if !self.elements.contains_key(c) {
                    bail!("元素 `{id}` 的子元素 `{c}` 不存在");
                }
            }
        }
        Ok(())
    }
}
