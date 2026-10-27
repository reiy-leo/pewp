//! 各格式的渲染器。

pub mod docx;
pub mod html;

use std::path::{Path, PathBuf};

use anyhow::{Result, bail};

use crate::doc::{Doc, Node};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Html,
    Docx,
    Pdf,
}

impl Format {
    pub fn from_path(p: &Path) -> Result<Format> {
        let ext = p
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        Format::from_name(&ext)
    }

    pub fn from_name(s: &str) -> Result<Format> {
        Ok(match s {
            "html" | "htm" => Format::Html,
            "docx" => Format::Docx,
            "pdf" => Format::Pdf,
            "xlsx" | "pptx" => bail!("{s} 要到第二版才支持"),
            other => bail!("不认识的输出格式 `{other}`，现在支持 html、docx、pdf"),
        })
    }
}

/// 渲染选项。
#[derive(Debug, Default, Clone)]
pub struct Options {
    /// 文档所在目录，相对路径的图片按它找。
    pub base_dir: Option<PathBuf>,
}

/// 渲染结果：文件内容加警告。
pub struct Output {
    pub bytes: Vec<u8>,
    pub warnings: Vec<String>,
}

pub fn render(doc: &Doc, tree: &Node, format: Format, opts: &Options) -> Result<Output> {
    match format {
        Format::Html => Ok(html::render(doc, tree, opts)),
        Format::Docx => docx::render(doc, tree, opts),
        Format::Pdf => bail!("pdf 渲染器还没写"),
    }
}
