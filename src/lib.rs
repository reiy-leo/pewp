//! pewp：一份 JSON，按组件目录渲染成 docx / pdf / html。

pub mod assets;
pub mod components;
pub mod custom;
pub mod doc;
pub mod render;
pub mod style;
pub mod units;

use std::path::Path;

use anyhow::{Context, Result};

/// 解析、展开自定义组件、校验，得到可以渲染的文档和元素树。
/// `catalog` 是外部定义文件里的组件；文档自己 `components` 里写的会合进来。
pub fn prepare(json: &str, catalog: &custom::Catalog) -> Result<(doc::Doc, doc::Node)> {
    let mut d = doc::parse(json)?;
    let inline = std::mem::take(&mut d.components);
    let expanded = if inline.is_empty() {
        catalog.expand(&mut d)
    } else {
        let mut all = catalog.clone();
        for def in inline {
            let name = def.name.clone();
            all.add(def)
                .with_context(|| format!("文档里的组件定义 `{name}` 有问题"))?;
        }
        all.expand(&mut d)
    };
    expanded?;
    let tree = d.tree()?;
    Ok((d, tree))
}

/// 从 JSON 文本直接渲染，命令行和 MCP 都走这里。
/// `base_dir` 是文档所在目录，用来找相对路径的图片；没有就按当前目录找。
pub fn render_with(
    json: &str,
    format: render::Format,
    base_dir: Option<&Path>,
    catalog: &custom::Catalog,
) -> Result<render::Output> {
    let (d, tree) = prepare(json, catalog)?;
    render::render(
        &d,
        &tree,
        format,
        &render::Options {
            base_dir: base_dir.map(Into::into),
        },
    )
}

/// 不带外部组件定义的 [`render_with`]。
pub fn render_str(
    json: &str,
    format: render::Format,
    base_dir: Option<&Path>,
) -> Result<render::Output> {
    render_with(json, format, base_dir, &custom::Catalog::new())
}

/// 读组件定义：先读文档目录下的 `catalog/`（有的话），再读 `--catalog` 指定的文件或目录。
pub fn load_catalog(
    doc_dir: Option<&Path>,
    extra: &[std::path::PathBuf],
) -> Result<custom::Catalog> {
    let mut c = custom::Catalog::new();
    if let Some(dir) = doc_dir {
        let auto = dir.join("catalog");
        if auto.is_dir() {
            c.load_path(&auto)?;
        }
    }
    for p in extra {
        c.load_path(p)?;
    }
    Ok(c)
}
