//! PDF 渲染器：先把元素树写成 Typst 标记，再用 typst 库编译成 PDF。
//!
//! 文字一律写成 Typst 字符串字面量（`#"..."`），不用转义标记语法。
//! 图片通过虚拟文件 `/assets/N.ext` 交给 typst。

use std::{collections::HashMap, fmt::Write, sync::OnceLock};

use typst::{
    Library, LibraryExt, World,
    diag::{FileError, FileResult},
    foundations::{Bytes, Datetime, Duration},
    syntax::{FileId, RootedPath, Source, VirtualPath, VirtualRoot},
    text::{Font, FontBook},
    utils::LazyHash,
};
use typst_kit::fonts::{self, FontStore};
use typst_layout::PagedDocument;

use crate::{
    assets,
    components::{Component, ImageProps, Inline, TableProps},
    doc::{Doc, Node},
    render::{Options, Output},
    style::{Align, Style},
    units,
};

const HEADING_PT: [f32; 6] = [20.0, 16.0, 14.0, 12.0, 11.0, 11.0];

/// 默认字体顺序：西文无衬线在前，中文按 macOS、Linux、Windows 常见字体排，
/// 最后是 typst 自带字体兜底。typst 按字形逐个往后找。
const FONTS: &[&str] = &[
    "Helvetica Neue",
    "Arial",
    "Liberation Sans",
    "PingFang SC",
    "Hiragino Sans GB",
    "Noto Sans CJK SC",
    "Source Han Sans SC",
    "Microsoft YaHei",
    "Libertinus Serif",
];
const MONO_FONTS: &[&str] = &["Menlo", "Consolas", "DejaVu Sans Mono"];

pub fn render(doc: &Doc, tree: &Node, opts: &Options) -> anyhow::Result<Output> {
    let (source, files, mut warnings) = markup(doc, tree, opts);
    let world = PewpWorld::new(source, files);
    let warned = typst::compile::<PagedDocument>(&world);
    for w in warned.warnings.iter() {
        // 默认字体列表本来就是按平台挑着用的，找不到不算问题
        let msg = w.message.to_string();
        let default_font = msg.strip_prefix("unknown font family: ").is_some_and(|f| {
            FONTS
                .iter()
                .chain(MONO_FONTS)
                .any(|d| d.eq_ignore_ascii_case(f))
        });
        if !default_font {
            warnings.push(format!("typst：{msg}"));
        }
    }
    let document = warned.output.map_err(|errs| {
        let msgs: Vec<String> = errs.iter().map(|e| e.message.to_string()).collect();
        anyhow::anyhow!("typst 排版失败：{}", msgs.join("；"))
    })?;
    let bytes = typst_pdf::pdf(&document, &typst_pdf::PdfOptions::default()).map_err(|errs| {
        let msgs: Vec<String> = errs.iter().map(|e| e.message.to_string()).collect();
        anyhow::anyhow!("生成 PDF 失败：{}", msgs.join("；"))
    })?;
    Ok(Output { bytes, warnings })
}

/// 生成 Typst 源码。返回源码、图片文件（虚拟路径 → 内容）、警告。
/// 图片虚拟文件：路径和内容。
pub type Files = Vec<(String, Vec<u8>)>;

pub fn markup(doc: &Doc, tree: &Node, opts: &Options) -> (String, Files, Vec<String>) {
    let mut m = Markup {
        out: String::new(),
        files: Vec::new(),
        warnings: Vec::new(),
        opts,
    };
    m.preamble(doc);
    m.node(tree, &Style::default());
    (m.out, m.files, m.warnings)
}

struct Markup<'a> {
    out: String,
    files: Files,
    warnings: Vec<String>,
    opts: &'a Options,
}

impl Markup<'_> {
    fn preamble(&mut self, doc: &Doc) {
        let page = &doc.meta.page;
        let paper = match page.size.to_ascii_uppercase().as_str() {
            "A3" => "a3",
            "A4" => "a4",
            "A5" => "a5",
            "B5" => "iso-b5",
            "LETTER" => "us-letter",
            "LEGAL" => "us-legal",
            other => {
                self.warnings
                    .push(format!("不认识的纸张 `{other}`，按 A4 处理"));
                "a4"
            }
        };
        let margin = units::parse_len_pt(&page.margin).unwrap_or_else(|| {
            self.warnings
                .push(format!("看不懂页边距 `{}`，按 20mm 处理", page.margin));
            units::parse_len_pt("20mm").unwrap()
        });
        let o = &mut self.out;
        let _ = writeln!(
            o,
            "#set page(paper: \"{paper}\", flipped: {}, margin: {margin}pt)",
            page.landscape
        );
        let mut docset = Vec::new();
        if let Some(t) = &doc.meta.title {
            docset.push(format!("title: {}", lit(t)));
        }
        if let Some(a) = &doc.meta.author {
            docset.push(format!("author: {}", lit(a)));
        }
        if !docset.is_empty() {
            let _ = writeln!(o, "#set document({})", docset.join(", "));
        }
        let lang = doc.meta.lang.as_deref().unwrap_or("zh");
        let lang = lang
            .split(['-', '_'])
            .next()
            .unwrap_or("zh")
            .to_ascii_lowercase();
        let _ = writeln!(
            o,
            "#set text(font: {}, size: 10.5pt, lang: {})",
            font_list(None),
            lit(&lang)
        );
        let _ = writeln!(o, "#set par(leading: 0.75em, spacing: 0.9em)");
        let _ = writeln!(
            o,
            "#show raw: set text(font: {}, size: 1.15em)",
            lits(MONO_FONTS)
        );
        let _ = writeln!(
            o,
            "#show raw.where(block: false): box.with(fill: rgb(\"#F3F3F3\"), inset: (x: 2pt), outset: (y: 2pt), radius: 2pt)"
        );
        let _ = writeln!(o, "#set figure(numbering: none)");
        let _ = writeln!(o, "#show link: underline");
        for (i, pt) in HEADING_PT.iter().enumerate() {
            let _ = writeln!(
                o,
                "#show heading.where(level: {}): set text(size: {pt}pt)",
                i + 1
            );
        }
        let _ = writeln!(o, "#show heading: set block(above: 1.4em, below: 0.7em)");
        let _ = writeln!(
            o,
            "#set table(stroke: 0.5pt + rgb(\"#999999\"), inset: (x: 5pt, y: 4pt))"
        );
        let _ = writeln!(
            o,
            "#show table.cell.where(y: 0): set text(weight: \"bold\")"
        );
    }

    fn node(&mut self, n: &Node, inherited: &Style) {
        let mut st = inherited.clone();
        st.merge(&n.style);
        match &n.component {
            Component::Document(_) => self.children(n, &st),
            Component::Slide(p) => {
                self.out.push_str("#pagebreak(weak: true)\n");
                if let Some(t) = &p.title {
                    let _ = writeln!(self.out, "#heading(level: 2)[#{}]", lit(t));
                }
                self.children(n, &st);
                self.out.push_str("#pagebreak(weak: true)\n");
            }
            Component::Heading(p) => {
                let body = self.inline(&p.text, &st, &n.id);
                let body = format!("#heading(level: {})[{body}]", p.level);
                self.block(body, &st, &n.id);
            }
            Component::Text(p) => {
                let body = self.inline(&p.text, &st, &n.id);
                self.block(format!("#par[{body}]"), &st, &n.id);
            }
            Component::List(p) => {
                let f = if p.ordered { "enum" } else { "list" };
                let items: Vec<String> = p
                    .items
                    .iter()
                    .map(|it| format!("[{}]", self.inline(it, &st, &n.id)))
                    .collect();
                self.block(format!("#{f}({})", items.join(", ")), &st, &n.id);
            }
            Component::Table(p) => self.table(n, p, &st),
            Component::Image(p) => self.image(n, p, &st),
            Component::PageBreak(_) => self.out.push_str("#pagebreak()\n"),
        }
    }

    fn children(&mut self, n: &Node, st: &Style) {
        for c in &n.children {
            self.node(c, st);
        }
    }

    /// 把一段内容包上段落级样式：对齐、行距、段前段后、背景、边框、字体。
    fn block(&mut self, body: String, s: &Style, id: &str) {
        let mut sets = String::new();
        let text = self.text_args(s, id);
        if !text.is_empty() {
            let _ = writeln!(sets, "#set text({text})");
        }
        if let Some(l) = s.line_height {
            // CSS 的倍数行高换成 typst 的行间距，大致对上
            let _ = writeln!(sets, "#set par(leading: {:.2}em)", (l - 0.5).max(0.2));
        }
        if s.align == Some(Align::Justify) {
            sets.push_str("#set par(justify: true)\n");
        }
        let mut body = body;
        if let Some(a) = s.align {
            let a = match a {
                Align::Left => "left",
                Align::Center => "center",
                Align::Right => "right",
                Align::Justify => "left",
            };
            body = format!("#align({a})[{body}]");
        }
        let mut args = Vec::new();
        if let Some(bg) = &s.background
            && let Some(c) = self.color(bg, id)
        {
            args.push(format!("fill: {c}"));
        }
        if let Some(b) = &s.border
            && let Some(c) = self.color(b, id)
        {
            args.push(format!("stroke: 0.75pt + {c}"));
        }
        if s.background.is_some() || s.border.is_some() || s.padding.is_some() {
            args.push(format!("inset: {}pt", s.padding.unwrap_or(6.0)));
            args.push("width: 100%".into());
            args.push("radius: 2pt".into());
        }
        if let Some(b) = s.space_before {
            args.push(format!("above: {b}pt"));
        }
        if let Some(a) = s.space_after {
            args.push(format!("below: {a}pt"));
        }
        if args.is_empty() && sets.is_empty() {
            let _ = writeln!(self.out, "{body}");
        } else {
            let _ = writeln!(self.out, "#block({})[\n{sets}{body}\n]", args.join(", "));
        }
    }

    /// `set text(...)` 的参数：字体、字号、颜色、粗斜体。
    fn text_args(&mut self, s: &Style, id: &str) -> String {
        let mut a = Vec::new();
        if let Some(f) = &s.font_family {
            a.push(format!("font: {}", font_list(Some(f))));
        }
        if let Some(sz) = s.font_size {
            a.push(format!("size: {sz}pt"));
        }
        if let Some(c) = &s.color
            && let Some(c) = self.color(c, id)
        {
            a.push(format!("fill: {c}"));
        }
        if s.bold == Some(true) {
            a.push("weight: \"bold\"".into());
        }
        if s.italic == Some(true) {
            a.push("style: \"italic\"".into());
        }
        a.join(", ")
    }

    fn inline(&mut self, t: &Inline, s: &Style, id: &str) -> String {
        let mut out = String::new();
        for r in t.runs() {
            let mut piece = String::new();
            for (i, line) in r.text.split('\n').enumerate() {
                if i > 0 {
                    piece.push_str("#linebreak()");
                }
                if !line.is_empty() {
                    if r.code == Some(true) {
                        let _ = write!(piece, "#raw({})", lit(line));
                    } else {
                        let _ = write!(piece, "#{}", lit(line));
                    }
                }
            }
            let mut args = Vec::new();
            if r.bold == Some(true) {
                args.push("weight: \"bold\"".to_string());
            }
            if r.italic == Some(true) {
                args.push("style: \"italic\"".to_string());
            }
            if let Some(c) = &r.color {
                if let Some(c) = self.color(c, id) {
                    args.push(format!("fill: {c}"));
                }
            } else if r.href.is_some() && s.color.is_none() {
                args.push("fill: rgb(\"#0563C1\")".into());
            }
            if !args.is_empty() {
                piece = format!("#text({})[{piece}]", args.join(", "));
            }
            if r.underline == Some(true) && r.href.is_none() {
                piece = format!("#underline[{piece}]");
            }
            if let Some(h) = &r.href {
                piece = format!("#link({})[{piece}]", lit(h));
            }
            out.push_str(&piece);
        }
        if s.underline == Some(true) {
            out = format!("#underline[{out}]");
        }
        out
    }

    fn table(&mut self, n: &Node, p: &TableProps, st: &Style) {
        let cols = p
            .columns
            .len()
            .max(p.rows.iter().map(Vec::len).max().unwrap_or(0))
            .max(1);
        let widths = match &p.widths {
            Some(ws) if ws.len() == cols && ws.iter().all(|w| *w > 0.0) => {
                ws.iter().map(|w| format!("{w}fr")).collect::<Vec<_>>()
            }
            Some(_) => {
                self.warnings.push(format!(
                    "表格 `{}` 的 widths 个数和列数对不上，改成平均分",
                    n.id
                ));
                vec!["1fr".to_string(); cols]
            }
            None => vec!["1fr".to_string(); cols],
        };
        let mut cells = Vec::new();
        if !p.columns.is_empty() {
            let head: Vec<String> = (0..cols)
                .map(|i| match p.columns.get(i) {
                    Some(c) => format!("[{}]", self.inline(c, st, &n.id)),
                    None => "[]".into(),
                })
                .collect();
            cells.push(format!("table.header({})", head.join(", ")));
        }
        for (ri, row) in p.rows.iter().enumerate() {
            if row.len() != cols {
                self.warnings.push(format!(
                    "表格 `{}` 第 {} 行有 {} 格，按 {} 列补齐",
                    n.id,
                    ri + 1,
                    row.len(),
                    cols
                ));
            }
            for i in 0..cols {
                cells.push(match row.get(i) {
                    Some(c) => format!("[{}]", self.inline(c, st, &n.id)),
                    None => "[]".into(),
                });
            }
        }
        let fill = if p.columns.is_empty() {
            ""
        } else {
            ", fill: (_, y) => if y == 0 { rgb(\"#F3F3F3\") }"
        };
        let body = format!(
            "#table(columns: ({},){fill}, {})",
            widths.join(", "),
            cells.join(", ")
        );
        self.block(body, st, &n.id);
    }

    fn image(&mut self, n: &Node, p: &ImageProps, st: &Style) {
        let loaded = if assets::is_remote(&p.src) {
            Err("pdf 暂不支持远程图片".to_string())
        } else {
            assets::load(&p.src, self.opts.base_dir.as_deref()).map_err(|e| format!("{e:#}"))
        };
        let asset = match loaded {
            Ok(a) => a,
            Err(why) => {
                self.warnings
                    .push(format!("图片 `{}`：{why}，改成显示 alt 文字", n.id));
                let alt = if p.alt.is_empty() { &p.src } else { &p.alt };
                self.block(format!("#emph[#{}]", lit(&format!("[{alt}]"))), st, &n.id);
                return;
            }
        };
        let ext = match asset.mime {
            "image/png" => "png",
            "image/jpeg" => "jpg",
            "image/gif" => "gif",
            "image/svg+xml" => "svg",
            "image/webp" => "webp",
            _ => "bin",
        };
        let path = format!("/assets/{}.{ext}", self.files.len());
        self.files.push((path.clone(), asset.bytes));
        let mut args = vec![lit(&path)];
        if let Some(w) = p.width {
            args.push(format!("width: {w}pt"));
        }
        if let Some(h) = p.height {
            args.push(format!("height: {h}pt"));
        }
        if !p.alt.is_empty() {
            args.push(format!("alt: {}", lit(&p.alt)));
        }
        let img = format!("image({})", args.join(", "));
        let body = match &p.caption {
            Some(c) => format!("#figure({img}, caption: [#{}])", lit(c)),
            None => format!("#box({img})"),
        };
        self.block(body, st, &n.id);
    }

    /// 只认 #rgb / #rrggbb / #rrggbbaa，其他写法给警告。
    fn color(&mut self, c: &str, id: &str) -> Option<String> {
        let hex = c.strip_prefix('#').unwrap_or("");
        if matches!(hex.len(), 3 | 4 | 6 | 8) && hex.chars().all(|ch| ch.is_ascii_hexdigit()) {
            Some(format!("rgb(\"#{hex}\")"))
        } else {
            self.warnings
                .push(format!("元素 `{id}`：pdf 只认十六进制颜色，`{c}` 跳过"));
            None
        }
    }
}

/// Typst 字符串字面量。
fn lit(s: &str) -> String {
    let mut o = String::with_capacity(s.len() + 2);
    o.push('"');
    for ch in s.chars() {
        match ch {
            '\\' => o.push_str("\\\\"),
            '"' => o.push_str("\\\""),
            '\n' => o.push_str("\\n"),
            '\r' => o.push_str("\\r"),
            '\t' => o.push_str("\\t"),
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

fn lits(xs: &[&str]) -> String {
    let v: Vec<String> = xs.iter().map(|x| lit(x)).collect();
    format!("({},)", v.join(", "))
}

fn font_list(first: Option<&str>) -> String {
    let mut v: Vec<&str> = Vec::new();
    if let Some(f) = first {
        v.push(f);
    }
    v.extend(FONTS);
    lits(&v)
}

/// 字体只扫一次：typst 自带字体加系统字体。
fn font_store() -> &'static FontStore {
    static STORE: OnceLock<FontStore> = OnceLock::new();
    STORE.get_or_init(|| {
        let mut s = FontStore::new();
        s.extend(fonts::embedded());
        s.extend(fonts::system());
        s
    })
}

fn library() -> &'static LazyHash<Library> {
    static LIB: OnceLock<LazyHash<Library>> = OnceLock::new();
    LIB.get_or_init(|| LazyHash::new(Library::default()))
}

struct PewpWorld {
    main: Source,
    files: HashMap<FileId, Bytes>,
}

fn file_id(path: &str) -> FileId {
    let vpath = VirtualPath::new(path).expect("内部生成的虚拟路径应当合法");
    RootedPath::new(VirtualRoot::Project, vpath).intern()
}

impl PewpWorld {
    fn new(source: String, files: Files) -> Self {
        let main = Source::new(file_id("/main.typ"), source);
        let files = files
            .into_iter()
            .map(|(p, b)| (file_id(&p), Bytes::new(b)))
            .collect();
        Self { main, files }
    }
}

impl World for PewpWorld {
    fn library(&self) -> &LazyHash<Library> {
        library()
    }

    fn book(&self) -> &LazyHash<FontBook> {
        font_store().book()
    }

    fn main(&self) -> FileId {
        self.main.id()
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        if id == self.main.id() {
            Ok(self.main.clone())
        } else {
            Err(FileError::AccessDenied)
        }
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        self.files.get(&id).cloned().ok_or(FileError::AccessDenied)
    }

    fn font(&self, index: usize) -> Option<Font> {
        font_store().font(index)
    }

    fn today(&self, _offset: Option<Duration>) -> Option<Datetime> {
        None
    }
}
