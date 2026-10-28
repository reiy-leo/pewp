use std::{
    io::{Read, Write},
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use pewp::{components, render};

#[derive(Parser)]
#[command(name = "pewp", version, about = "把一份 JSON 渲染成 docx / pdf / html")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// 渲染文档：pewp render doc.json -o out.html（输入写 - 表示从标准输入读）
    Render {
        input: PathBuf,
        /// 输出文件，按扩展名选格式；写 - 表示输出到标准输出，这时要配 --format
        #[arg(short, long)]
        output: PathBuf,
        /// 指定格式（html、docx、pdf），不写就看输出文件的扩展名
        #[arg(short, long)]
        format: Option<String>,
        /// 自定义组件的定义文件或目录（JSON），可写多次；文档目录下的 catalog/ 会自动读
        #[arg(long)]
        catalog: Vec<PathBuf>,
    },
    /// 只校验，不出文件
    Check {
        input: PathBuf,
        /// 同 render 的 --catalog
        #[arg(long)]
        catalog: Vec<PathBuf>,
    },
    /// 列出可用组件；加 --json 输出每个组件 props 的 JSON Schema
    Catalog {
        #[arg(long)]
        json: bool,
        /// 一起列出这些定义文件或目录里的自定义组件
        #[arg(long)]
        catalog: Vec<PathBuf>,
    },
    /// 以 MCP server 方式运行（stdio），还没实现
    Mcp,
}

fn read_input(p: &Path) -> Result<String> {
    if p == Path::new("-") {
        let mut s = String::new();
        std::io::stdin().read_to_string(&mut s)?;
        Ok(s)
    } else {
        std::fs::read_to_string(p).with_context(|| format!("读不了 {}", p.display()))
    }
}

/// 文档所在目录；从标准输入读时没有。
fn doc_dir(input: &Path) -> Option<PathBuf> {
    if input == Path::new("-") {
        None
    } else {
        input.parent().map(Path::to_path_buf)
    }
}

fn main() -> Result<()> {
    match Cli::parse().cmd {
        Cmd::Render {
            input,
            output,
            format,
            catalog,
        } => {
            let fmt = match &format {
                Some(f) => render::Format::from_name(&f.to_ascii_lowercase())?,
                None if output == Path::new("-") => bail!("输出到标准输出时要用 --format 指定格式"),
                None => render::Format::from_path(&output)?,
            };
            let text = read_input(&input)?;
            let base_dir = doc_dir(&input);
            let cat = pewp::load_catalog(base_dir.as_deref(), &catalog)?;
            let (d, tree) = pewp::prepare(&text, &cat)
                .with_context(|| format!("{} 不是合法的 pewp 文档", input.display()))?;
            let out = render::render(&d, &tree, fmt, &render::Options { base_dir })?;
            for w in &out.warnings {
                eprintln!("警告：{w}");
            }
            if output == Path::new("-") {
                std::io::stdout().write_all(&out.bytes)?;
            } else {
                std::fs::write(&output, &out.bytes)
                    .with_context(|| format!("写不了 {}", output.display()))?;
                eprintln!("已生成 {}", output.display());
            }
            Ok(())
        }
        Cmd::Check { input, catalog } => {
            let text = read_input(&input)?;
            let cat = pewp::load_catalog(doc_dir(&input).as_deref(), &catalog)?;
            let (d, _) = pewp::prepare(&text, &cat)
                .with_context(|| format!("{} 校验没通过", input.display()))?;
            println!("ok：展开后 {} 个元素", d.elements.len());
            Ok(())
        }
        Cmd::Catalog { json, catalog } => {
            let cat = pewp::load_catalog(None, &catalog)?;
            if json {
                let mut m = serde_json::Map::new();
                for s in components::BUILTINS {
                    m.insert(
                        s.name.into(),
                        serde_json::json!({
                            "description": s.description,
                            "container": s.container,
                            "props": (s.schema)(),
                        }),
                    );
                }
                for d in cat.defs() {
                    m.insert(
                        d.name.clone(),
                        serde_json::json!({
                            "description": d.description,
                            "container": d.container,
                            "custom": true,
                            "props": d.props_schema(),
                        }),
                    );
                }
                println!("{}", serde_json::to_string_pretty(&m)?);
            } else {
                for s in components::BUILTINS {
                    println!(
                        "{:<10} {}{}",
                        s.name,
                        s.description,
                        if s.container { "（容器）" } else { "" }
                    );
                }
                for d in cat.defs() {
                    println!(
                        "{:<10} {}（自定义{}）",
                        d.name,
                        d.description,
                        if d.container { "，容器" } else { "" }
                    );
                }
            }
            Ok(())
        }
        Cmd::Mcp => bail!("MCP server 还没实现"),
    }
}
