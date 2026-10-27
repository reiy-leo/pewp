//! pewp：一份 JSON，按组件目录渲染成 docx / pdf / html。

pub mod assets;
pub mod components;
pub mod doc;
pub mod render;
pub mod style;

use anyhow::Result;

/// 从 JSON 文本直接渲染，命令行和 MCP 都走这里。
/// `base_dir` 是文档所在目录，用来找相对路径的图片；没有就按当前目录找。
pub fn render_str(
    json: &str,
    format: render::Format,
    base_dir: Option<&std::path::Path>,
) -> Result<render::Output> {
    let d = doc::parse(json)?;
    let tree = d.tree()?;
    render::render(
        &d,
        &tree,
        format,
        &render::Options {
            base_dir: base_dir.map(Into::into),
        },
    )
}
