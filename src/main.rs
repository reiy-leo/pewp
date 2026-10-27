//! pewp: 一份 JSON，按组件目录渲染成 docx / pdf / html。
//! 现在只有命令行骨架，渲染器还没写。

mod doc;

use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "pewp",
    version,
    about = "Render one JSON document to docx / pdf / html"
)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// 渲染文档：pewp render doc.json -o out.docx
    Render {
        input: PathBuf,
        #[arg(short, long)]
        output: PathBuf,
        /// 额外的组件定义文件或目录（YAML/JSON）
        #[arg(long)]
        catalog: Vec<PathBuf>,
    },
    /// 只校验，不出文件
    Check { input: PathBuf },
    /// 列出可用组件
    Catalog,
    /// 以 MCP server 方式运行（stdio）
    Mcp,
}

fn main() -> Result<()> {
    match Cli::parse().cmd {
        Cmd::Render { input, output, .. } => {
            let d = doc::load(&input)?;
            let ext = output.extension().and_then(|e| e.to_str()).unwrap_or("");
            bail!(
                "{ext} 渲染器还没实现（文档 {} 有 {} 个元素）",
                input.display(),
                d.elements.len()
            )
        }
        Cmd::Check { input } => {
            let d = doc::load(&input)?;
            d.validate()
                .with_context(|| format!("{} 校验没通过", input.display()))?;
            println!("ok: {} 个元素", d.elements.len());
            Ok(())
        }
        Cmd::Catalog => {
            for c in doc::BUILTINS {
                println!("{c}");
            }
            Ok(())
        }
        Cmd::Mcp => bail!("MCP server 还没实现"),
    }
}
