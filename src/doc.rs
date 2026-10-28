//! 文档模型：沿用 json-render 的扁平 elements + root。

use std::{
    collections::{BTreeMap, HashSet},
    path::Path,
};

use anyhow::{Context, Result, bail};
use serde::Deserialize;
use serde_json::Value;

use crate::{
    components::{self, Component},
    style::{self, Style},
};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Doc {
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub meta: Meta,
    #[serde(default)]
    pub styles: BTreeMap<String, Style>,
    /// 文档里直接写的自定义组件定义，和定义文件写法一样
    #[serde(default)]
    pub components: Vec<crate::custom::Def>,
    pub root: String,
    pub elements: BTreeMap<String, Element>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Meta {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub lang: Option<String>,
    #[serde(default)]
    pub page: Page,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Page {
    /// A4、A5、Letter 等
    #[serde(default = "default_size")]
    pub size: String,
    #[serde(default)]
    pub landscape: bool,
    /// CSS 长度，如 "20mm"
    #[serde(default = "default_margin")]
    pub margin: String,
}

fn default_size() -> String {
    "A4".into()
}
fn default_margin() -> String {
    "20mm".into()
}
impl Default for Page {
    fn default() -> Self {
        Page {
            size: default_size(),
            landscape: false,
            margin: default_margin(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
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

/// 校验并解析好的元素树，渲染器只读它。
#[derive(Debug)]
pub struct Node {
    pub id: String,
    pub component: Component,
    pub style: Style,
    pub children: Vec<Node>,
}

pub fn load(path: &Path) -> Result<Doc> {
    let text =
        std::fs::read_to_string(path).with_context(|| format!("读不了 {}", path.display()))?;
    parse(&text).with_context(|| format!("{} 不是合法的 pewp JSON", path.display()))
}

pub fn parse(text: &str) -> Result<Doc> {
    Ok(serde_json::from_str(text)?)
}

impl Doc {
    /// 校验全部元素，并从 root 建出元素树。
    pub fn tree(&self) -> Result<Node> {
        let mut seen = HashSet::new();
        let node = self.build(&self.root, &mut seen)?;
        let unused: Vec<_> = self
            .elements
            .keys()
            .filter(|k| !seen.contains(k.as_str()))
            .collect();
        if !unused.is_empty() {
            eprintln!("警告：这些元素没被引用，不会渲染：{unused:?}");
        }
        Ok(node)
    }

    fn build<'a>(&'a self, id: &'a str, seen: &mut HashSet<&'a str>) -> Result<Node> {
        let el = self
            .elements
            .get(id)
            .ok_or_else(|| anyhow::anyhow!("元素 `{id}` 不存在"))?;
        if !seen.insert(id) {
            bail!("元素 `{id}` 被引用了不止一次（或者成环了）");
        }
        let spec = components::spec(&el.kind)
            .ok_or_else(|| anyhow::anyhow!("元素 `{id}` 的组件 `{}` 没注册", el.kind))?;
        if !spec.container && !el.children.is_empty() {
            bail!("元素 `{id}`：{} 不能有 children", el.kind);
        }
        let component = components::parse(&el.kind, &el.props)
            .with_context(|| format!("元素 `{id}`（{}）的 props 不对", el.kind))?;
        let style = style::resolve(el.style.as_ref(), &self.styles)
            .with_context(|| format!("元素 `{id}` 的 style 不对"))?;
        let mut children = Vec::with_capacity(el.children.len());
        for c in &el.children {
            children.push(
                self.build(c, seen)
                    .with_context(|| format!("在 `{id}` 的子元素里"))?,
            );
        }
        Ok(Node {
            id: id.to_string(),
            component,
            style,
            children,
        })
    }
}
