//! HTML 渲染器：一个自带 CSS 的完整页面，能直接打开，也能打印。

use std::fmt::Write;

use base64::Engine;

use crate::{
    assets,
    components::{Component, Inline, Run},
    doc::{Doc, Node},
    render::{Options, Output},
    style::Style,
};

pub fn render(doc: &Doc, tree: &Node, opts: &Options) -> Output {
    let mut r = Html {
        out: String::new(),
        warnings: Vec::new(),
        opts,
    };
    let lang = doc.meta.lang.as_deref().unwrap_or("zh-CN");
    let title = doc.meta.title.as_deref().unwrap_or("pewp");
    let page = &doc.meta.page;
    let size = format!(
        "{}{}",
        page.size,
        if page.landscape { " landscape" } else { "" }
    );
    let _ = write!(
        r.out,
        "<!doctype html>\n<html lang=\"{}\">\n<head>\n<meta charset=\"utf-8\">\n\
         <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n\
         <meta name=\"generator\" content=\"pewp {}\">\n",
        esc(lang),
        env!("CARGO_PKG_VERSION")
    );
    if let Some(a) = &doc.meta.author {
        let _ = writeln!(r.out, "<meta name=\"author\" content=\"{}\">", esc(a));
    }
    let _ = write!(
        r.out,
        "<title>{}</title>\n<style>\n@page {{ size: {}; margin: {}; }}\n{}</style>\n</head>\n<body>\n",
        esc(title),
        esc(&size),
        esc(&page.margin),
        BASE_CSS
    );
    r.node(tree);
    r.out.push_str("</body>\n</html>\n");
    Output {
        bytes: r.out.into_bytes(),
        warnings: r.warnings,
    }
}

const BASE_CSS: &str = "\
body { font-family: -apple-system, \"PingFang SC\", \"Noto Sans SC\", \"Microsoft YaHei\", sans-serif; \
line-height: 1.6; color: #1a1a1a; max-width: 52rem; margin: 2rem auto; padding: 0 1.5rem; }
table { border-collapse: collapse; width: 100%; margin: 1em 0; }
th, td { border: 1px solid #ccc; padding: 0.4em 0.6em; text-align: left; vertical-align: top; }
th { background: #f3f3f3; font-weight: 600; }
figure { margin: 1em 0; }
figure img { max-width: 100%; height: auto; }
figcaption { color: #666; font-size: 0.9em; margin-top: 0.3em; }
code { font-family: ui-monospace, Menlo, monospace; background: #f3f3f3; padding: 0 0.2em; border-radius: 3px; }
.pewp-pagebreak { break-after: page; }
.pewp-slide { aspect-ratio: 16 / 9; border: 1px solid #ddd; border-radius: 6px; padding: 2rem 3rem; \
margin: 1.5rem 0; overflow: hidden; break-inside: avoid; break-after: page; }
@media screen { .pewp-pagebreak { border-top: 1px dashed #bbb; margin: 2rem 0; } }
@media print { body { max-width: none; margin: 0; padding: 0; } .pewp-slide { border: none; } }
";

struct Html<'a> {
    out: String,
    warnings: Vec<String>,
    opts: &'a Options,
}

impl Html<'_> {
    fn node(&mut self, n: &Node) {
        let id = esc(&n.id);
        let st = css(&n.style);
        let st = if st.is_empty() {
            String::new()
        } else {
            format!(" style=\"{}\"", esc(&st))
        };
        match &n.component {
            Component::Document(_) => {
                let _ = writeln!(self.out, "<main id=\"{id}\"{st}>");
                self.children(n);
                self.out.push_str("</main>\n");
            }
            Component::Slide(p) => {
                let _ = writeln!(self.out, "<section class=\"pewp-slide\" id=\"{id}\"{st}>");
                if let Some(t) = &p.title {
                    let _ = writeln!(self.out, "<h2>{}</h2>", esc(t));
                }
                self.children(n);
                self.out.push_str("</section>\n");
            }
            Component::Heading(p) => {
                let _ = write!(self.out, "<h{l} id=\"{id}\"{st}>", l = p.level);
                self.inline(&p.text);
                let _ = writeln!(self.out, "</h{}>", p.level);
            }
            Component::Text(p) => {
                let _ = write!(self.out, "<p id=\"{id}\"{st}>");
                self.inline(&p.text);
                self.out.push_str("</p>\n");
            }
            Component::List(p) => {
                let tag = if p.ordered { "ol" } else { "ul" };
                let _ = writeln!(self.out, "<{tag} id=\"{id}\"{st}>");
                for it in &p.items {
                    self.out.push_str("<li>");
                    self.inline(it);
                    self.out.push_str("</li>\n");
                }
                let _ = writeln!(self.out, "</{tag}>");
            }
            Component::Table(p) => {
                let _ = writeln!(self.out, "<table id=\"{id}\"{st}>");
                if let Some(w) = &p.widths {
                    let total: f32 = w.iter().sum();
                    if total > 0.0 {
                        self.out.push_str("<colgroup>");
                        for x in w {
                            let _ =
                                write!(self.out, "<col style=\"width:{:.2}%\">", x / total * 100.0);
                        }
                        self.out.push_str("</colgroup>\n");
                    }
                }
                if !p.columns.is_empty() {
                    self.out.push_str("<thead><tr>");
                    for c in &p.columns {
                        self.out.push_str("<th>");
                        self.inline(c);
                        self.out.push_str("</th>");
                    }
                    self.out.push_str("</tr></thead>\n");
                }
                self.out.push_str("<tbody>\n");
                for (i, row) in p.rows.iter().enumerate() {
                    if !p.columns.is_empty() && row.len() != p.columns.len() {
                        self.warnings.push(format!(
                            "表格 `{}` 第 {} 行有 {} 格，表头有 {} 列",
                            n.id,
                            i + 1,
                            row.len(),
                            p.columns.len()
                        ));
                    }
                    self.out.push_str("<tr>");
                    for c in row {
                        self.out.push_str("<td>");
                        self.inline(c);
                        self.out.push_str("</td>");
                    }
                    self.out.push_str("</tr>\n");
                }
                self.out.push_str("</tbody>\n</table>\n");
            }
            Component::Image(p) => {
                let mut size = String::new();
                if let Some(w) = p.width {
                    let _ = write!(size, " width=\"{}\"", pt_to_px(w));
                }
                if let Some(h) = p.height {
                    let _ = write!(size, " height=\"{}\"", pt_to_px(h));
                }
                let src = self.image_src(&n.id, &p.src);
                let _ = write!(
                    self.out,
                    "<figure id=\"{id}\"{st}><img src=\"{}\" alt=\"{}\"{size}>",
                    esc(&src),
                    esc(&p.alt)
                );
                if let Some(c) = &p.caption {
                    let _ = write!(self.out, "<figcaption>{}</figcaption>", esc(c));
                }
                self.out.push_str("</figure>\n");
            }
            Component::PageBreak(_) => {
                let _ = writeln!(self.out, "<div class=\"pewp-pagebreak\" id=\"{id}\"></div>");
            }
        }
    }

    /// 本地图片直接内嵌成 data URI，生成的 HTML 单独拿走也能看。
    fn image_src(&mut self, id: &str, src: &str) -> String {
        if assets::is_remote(src) {
            return src.to_string();
        }
        match assets::load(src, self.opts.base_dir.as_deref()) {
            Ok(a) => format!(
                "data:{};base64,{}",
                a.mime,
                base64::engine::general_purpose::STANDARD.encode(&a.bytes)
            ),
            Err(e) => {
                self.warnings
                    .push(format!("图片 `{id}`：{e:#}，保留原路径"));
                src.to_string()
            }
        }
    }

    fn children(&mut self, n: &Node) {
        for c in &n.children {
            self.node(c);
        }
    }

    fn inline(&mut self, t: &Inline) {
        for r in t.runs() {
            self.run(&r);
        }
    }

    fn run(&mut self, r: &Run) {
        let mut open = String::new();
        let mut close = Vec::new();
        if let Some(h) = &r.href {
            let _ = write!(open, "<a href=\"{}\">", esc(h));
            close.push("</a>");
        }
        if r.bold == Some(true) {
            open.push_str("<strong>");
            close.push("</strong>");
        }
        if r.italic == Some(true) {
            open.push_str("<em>");
            close.push("</em>");
        }
        if r.underline == Some(true) {
            open.push_str("<u>");
            close.push("</u>");
        }
        if r.code == Some(true) {
            open.push_str("<code>");
            close.push("</code>");
        }
        if let Some(c) = &r.color {
            let _ = write!(open, "<span style=\"color:{}\">", esc(&safe(c)));
            close.push("</span>");
        }
        self.out.push_str(&open);
        // 换行符变成 <br>
        let text = esc(&r.text).replace('\n', "<br>");
        self.out.push_str(&text);
        for c in close.iter().rev() {
            self.out.push_str(c);
        }
    }
}

fn pt_to_px(pt: f32) -> u32 {
    (pt * 96.0 / 72.0).round() as u32
}

fn css(s: &Style) -> String {
    let mut o = String::new();
    if let Some(v) = &s.font_family {
        let _ = write!(o, "font-family:\"{}\";", v.replace('"', ""));
    }
    if let Some(v) = s.font_size {
        let _ = write!(o, "font-size:{v}pt;");
    }
    if let Some(v) = &s.color {
        let _ = write!(o, "color:{};", safe(v));
    }
    if let Some(v) = s.bold {
        o.push_str(if v {
            "font-weight:700;"
        } else {
            "font-weight:400;"
        });
    }
    if let Some(v) = s.italic {
        o.push_str(if v {
            "font-style:italic;"
        } else {
            "font-style:normal;"
        });
    }
    if s.underline == Some(true) {
        o.push_str("text-decoration:underline;");
    }
    if let Some(v) = s.align {
        let _ = write!(o, "text-align:{};", v.as_str());
    }
    if let Some(v) = s.line_height {
        let _ = write!(o, "line-height:{v};");
    }
    if let Some(v) = s.space_before {
        let _ = write!(o, "margin-top:{v}pt;");
    }
    if let Some(v) = s.space_after {
        let _ = write!(o, "margin-bottom:{v}pt;");
    }
    if let Some(v) = &s.background {
        let _ = write!(o, "background:{};", safe(v));
    }
    if let Some(v) = &s.border {
        let _ = write!(o, "border:1pt solid {};", safe(v));
    }
    if let Some(v) = s.padding {
        let _ = write!(o, "padding:{v}pt;");
    }
    o
}

/// 去掉能跳出 CSS 声明的字符。
fn safe(v: &str) -> String {
    v.chars()
        .filter(|c| !matches!(c, ';' | '{' | '}' | '<' | '>' | '"' | '\\'))
        .collect()
}

pub fn esc(s: &str) -> String {
    let mut o = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => o.push_str("&amp;"),
            '<' => o.push_str("&lt;"),
            '>' => o.push_str("&gt;"),
            '"' => o.push_str("&quot;"),
            '\'' => o.push_str("&#39;"),
            _ => o.push(c),
        }
    }
    o
}
