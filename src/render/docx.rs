//! docx 渲染器，基于 docx-rs。

use docx_rs::{
    AbstractNumbering, AlignmentType, BorderType, BreakType, Docx, Hyperlink, HyperlinkType,
    IndentLevel, Level, LevelJc, LevelText, LineSpacing, LineSpacingType, NumberFormat, Numbering,
    NumberingId, PageMargin, Paragraph, ParagraphBorder, ParagraphBorderPosition, ParagraphBorders,
    Pic, Run, RunFonts, Shading, ShdType, SpecialIndentType, Start, Style, StyleType, Table,
    TableCell, TableCellMargins, TableLayoutType, TableRow, WidthType,
};

use crate::{
    assets,
    components::{Component, ImageProps, Inline, Run as PRun},
    doc::{Doc, Node},
    render::{Options, Output},
    style::{Align, Style as PStyle},
    units,
};

const HEADING_PT: [f32; 6] = [20.0, 16.0, 14.0, 12.0, 11.0, 11.0];
const MONO: &str = "Menlo";

pub fn render(doc: &Doc, tree: &Node, opts: &Options) -> anyhow::Result<Output> {
    let mut warnings = Vec::new();
    let page = &doc.meta.page;
    let (mut w, mut h) = units::page_size_pt(&page.size).unwrap_or_else(|| {
        warnings.push(format!("不认识的纸张 `{}`，按 A4 处理", page.size));
        units::page_size_pt("A4").unwrap()
    });
    if page.landscape {
        std::mem::swap(&mut w, &mut h);
    }
    let margin = units::parse_len_pt(&page.margin).unwrap_or_else(|| {
        warnings.push(format!("看不懂页边距 `{}`，按 20mm 处理", page.margin));
        units::parse_len_pt("20mm").unwrap()
    });
    let m = units::twip(margin);

    let mut docx = Docx::new()
        .page_size(units::twip(w) as u32, units::twip(h) as u32)
        .page_margin(PageMargin::new().top(m).bottom(m).left(m).right(m))
        .default_fonts(
            RunFonts::new()
                .ascii("Arial")
                .hi_ansi("Arial")
                .east_asia("PingFang SC")
                .cs("Arial"),
        )
        .default_size(21); // 10.5pt
    for (i, pt) in HEADING_PT.iter().enumerate() {
        let lvl = i + 1;
        docx = docx.add_style(
            Style::new(format!("Heading{lvl}"), StyleType::Paragraph)
                .name(format!("heading {lvl}"))
                .based_on("Normal")
                .next("Normal")
                .q_format(true)
                .outline_lvl(i)
                .size((pt * 2.0) as usize)
                .bold()
                .line_spacing(
                    LineSpacing::new()
                        .before(units::twip(12.0) as u32)
                        .after(units::twip(6.0) as u32),
                ),
        );
    }

    let mut r = Ctx {
        warnings,
        opts,
        content_width_pt: w - 2.0 * margin,
        next_num: 1,
        blocks: Vec::new(),
        numberings: Vec::new(),
    };
    r.node(tree, &PStyle::default());
    for (abs, num) in std::mem::take(&mut r.numberings) {
        docx = docx.add_abstract_numbering(abs).add_numbering(num);
    }
    for b in std::mem::take(&mut r.blocks) {
        docx = match b {
            Block::P(p) => docx.add_paragraph(p),
            Block::T(t) => docx.add_table(t),
        };
    }
    let mut buf = std::io::Cursor::new(Vec::new());
    docx.build().pack(&mut buf)?;
    Ok(Output {
        bytes: buf.into_inner(),
        warnings: r.warnings,
    })
}

// 段落和表格都只活到 render 结束，不值得为大小差异装箱
#[allow(clippy::large_enum_variant)]
enum Block {
    P(Paragraph),
    T(Table),
}

struct Ctx<'a> {
    warnings: Vec<String>,
    opts: &'a Options,
    content_width_pt: f32,
    next_num: usize,
    blocks: Vec<Block>,
    numberings: Vec<(AbstractNumbering, Numbering)>,
}

impl Ctx<'_> {
    fn node(&mut self, n: &Node, inherited: &PStyle) {
        let mut st = inherited.clone();
        st.merge(&n.style);
        match &n.component {
            Component::Document(_) => self.children(n, &st),
            Component::Slide(p) => {
                if let Some(t) = &p.title {
                    let para = Paragraph::new()
                        .style("Heading2")
                        .add_run(Run::new().add_text(t));
                    self.blocks.push(Block::P(para));
                }
                self.children(n, &st);
                self.page_break();
            }
            Component::Heading(p) => {
                let para = Paragraph::new()
                    .style(&format!("Heading{}", p.level))
                    .keep_next(true);
                let para = self.para_style(para, &st, &n.id);
                let para = self.inline(para, &p.text, &st);
                self.blocks.push(Block::P(para));
            }
            Component::Text(p) => {
                let para = self.para_style(Paragraph::new(), &st, &n.id);
                let para = self.inline(para, &p.text, &st);
                self.blocks.push(Block::P(para));
            }
            Component::List(p) => {
                let id = self.list_numbering(p.ordered);
                for it in &p.items {
                    let para =
                        Paragraph::new().numbering(NumberingId::new(id), IndentLevel::new(0));
                    let para = self.para_style(para, &st, &n.id);
                    let para = self.inline(para, it, &st);
                    self.blocks.push(Block::P(para));
                }
            }
            Component::Table(p) => self.table(n, p, &st),
            Component::Image(p) => self.image(n, p, &st),
            Component::PageBreak(_) => self.page_break(),
        }
    }

    fn children(&mut self, n: &Node, st: &PStyle) {
        for c in &n.children {
            self.node(c, st);
        }
    }

    fn page_break(&mut self) {
        self.blocks.push(Block::P(
            Paragraph::new().add_run(Run::new().add_break(BreakType::Page)),
        ));
    }

    fn table(&mut self, n: &Node, p: &crate::components::TableProps, st: &PStyle) {
        let cols = p
            .columns
            .len()
            .max(p.rows.iter().map(Vec::len).max().unwrap_or(0))
            .max(1);
        let total = units::twip(self.content_width_pt) as f32;
        let even = vec![(total / cols as f32) as usize; cols];
        let grid: Vec<usize> = match &p.widths {
            Some(ws) if ws.len() == cols && ws.iter().sum::<f32>() > 0.0 => {
                let s: f32 = ws.iter().sum();
                ws.iter().map(|x| (total * x / s) as usize).collect()
            }
            Some(_) => {
                self.warnings.push(format!(
                    "表格 `{}` 的 widths 个数和列数对不上，改成平均分",
                    n.id
                ));
                even
            }
            None => even,
        };
        let mut rows = Vec::new();
        if !p.columns.is_empty() {
            let mut head = st.clone();
            head.bold = Some(true);
            let mut cells = Vec::new();
            for (i, width) in grid.iter().enumerate() {
                let para = match p.columns.get(i) {
                    Some(c) => self.inline(Paragraph::new(), c, &head),
                    None => Paragraph::new(),
                };
                cells.push(
                    TableCell::new()
                        .add_paragraph(para)
                        .width(*width, WidthType::Dxa)
                        .shading(Shading::new().shd_type(ShdType::Clear).fill("F3F3F3")),
                );
            }
            rows.push(TableRow::new(cells));
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
            let mut cells = Vec::new();
            for (i, width) in grid.iter().enumerate() {
                let para = match row.get(i) {
                    Some(c) => self.inline(Paragraph::new(), c, st),
                    None => Paragraph::new(),
                };
                cells.push(
                    TableCell::new()
                        .add_paragraph(para)
                        .width(*width, WidthType::Dxa),
                );
            }
            rows.push(TableRow::new(cells));
        }
        let t = Table::new(rows)
            .set_grid(grid)
            .layout(TableLayoutType::Fixed)
            .margins(TableCellMargins::new().margin(60, 100, 60, 100))
            .width(total as usize, WidthType::Dxa);
        self.blocks.push(Block::T(t));
        // 表格后面留一点空
        self.blocks.push(Block::P(Paragraph::new()));
    }

    fn image(&mut self, n: &Node, p: &ImageProps, st: &PStyle) {
        let asset = if assets::is_remote(&p.src) {
            Err("docx 暂不支持远程图片".to_string())
        } else {
            assets::load(&p.src, self.opts.base_dir.as_deref()).map_err(|e| format!("{e:#}"))
        };
        let decoded = asset.and_then(|a| match image::load_from_memory(&a.bytes) {
            Ok(img) => Ok((a, img.width() as f32, img.height() as f32)),
            Err(_) => Err(format!(
                "{} 格式放不进 docx（支持 png、jpeg、gif、webp）",
                a.mime
            )),
        });
        let (asset, px_w, px_h) = match decoded {
            Ok(x) => x,
            Err(why) => {
                self.warnings
                    .push(format!("图片 `{}`：{why}，改成显示 alt 文字", n.id));
                let alt = if p.alt.is_empty() { &p.src } else { &p.alt };
                let para = self
                    .para_style(Paragraph::new(), st, &n.id)
                    .add_run(Run::new().add_text(format!("[{alt}]")).italic());
                self.blocks.push(Block::P(para));
                return;
            }
        };
        let (pw, ph) = (px_w * 0.75, px_h * 0.75); // px → pt
        let (mut w, mut h) = match (p.width, p.height) {
            (Some(w), Some(h)) => (w, h),
            (Some(w), None) => (w, w * ph / pw),
            (None, Some(h)) => (h * pw / ph, h),
            (None, None) => (pw, ph),
        };
        if w > self.content_width_pt {
            h *= self.content_width_pt / w;
            w = self.content_width_pt;
        }
        let pic = Pic::new(&asset.bytes).size(units::emu(w), units::emu(h));
        let para = self
            .para_style(Paragraph::new(), st, &n.id)
            .add_run(Run::new().add_image(pic));
        self.blocks.push(Block::P(para));
        if let Some(c) = &p.caption {
            let para = Paragraph::new().add_run(Run::new().add_text(c).size(18).color("666666"));
            self.blocks.push(Block::P(para));
        }
    }

    fn list_numbering(&mut self, ordered: bool) -> usize {
        let id = self.next_num;
        self.next_num += 1;
        let (fmt, text) = if ordered {
            ("decimal", "%1.")
        } else {
            ("bullet", "•")
        };
        let level = Level::new(
            0,
            Start::new(1),
            NumberFormat::new(fmt),
            LevelText::new(text),
            LevelJc::new("left"),
        )
        .indent(Some(720), Some(SpecialIndentType::Hanging(360)), None, None);
        self.numberings.push((
            AbstractNumbering::new(id).add_level(level),
            Numbering::new(id, id),
        ));
        id
    }

    /// 段落级样式：对齐、行距、段前段后、背景、边框。
    fn para_style(&mut self, mut p: Paragraph, s: &PStyle, id: &str) -> Paragraph {
        if let Some(a) = s.align {
            p = p.align(match a {
                Align::Left => AlignmentType::Left,
                Align::Center => AlignmentType::Center,
                Align::Right => AlignmentType::Right,
                Align::Justify => AlignmentType::Both,
            });
        }
        if s.line_height.is_some() || s.space_before.is_some() || s.space_after.is_some() {
            let mut ls = LineSpacing::new();
            if let Some(l) = s.line_height {
                ls = ls.line((l * 240.0) as i32).line_rule(LineSpacingType::Auto);
            }
            if let Some(b) = s.space_before {
                ls = ls.before(units::twip(b) as u32);
            }
            if let Some(a) = s.space_after {
                ls = ls.after(units::twip(a) as u32);
            }
            p = p.line_spacing(ls);
        }
        if let Some(bg) = &s.background {
            p.property = p
                .property
                .shading(Shading::new().shd_type(ShdType::Clear).fill(hex(bg)));
        }
        if let Some(c) = &s.border {
            let space = s.padding.map(|x| x as usize).unwrap_or(4);
            let mut bs = ParagraphBorders::new();
            for pos in [
                ParagraphBorderPosition::Top,
                ParagraphBorderPosition::Bottom,
                ParagraphBorderPosition::Left,
                ParagraphBorderPosition::Right,
            ] {
                bs = bs.set(
                    ParagraphBorder::new(pos)
                        .val(BorderType::Single)
                        .size(8)
                        .space(space)
                        .color(hex(c)),
                );
            }
            p.property = p.property.set_borders(bs);
        } else if s.padding.is_some() {
            self.warnings
                .push(format!("元素 `{id}`：docx 里 padding 只在有 border 时生效"));
        }
        p
    }

    fn inline(&mut self, mut p: Paragraph, t: &Inline, s: &PStyle) -> Paragraph {
        for r in t.runs() {
            let run = run(&r, s);
            p = match &r.href {
                Some(h) => p.add_hyperlink(Hyperlink::new(h, HyperlinkType::External).add_run(run)),
                None => p.add_run(run),
            };
        }
        p
    }
}

/// 字符级样式：先用元素样式，再用 run 自己的设置覆盖。
fn run(r: &PRun, s: &PStyle) -> Run {
    let mut run = Run::new();
    for (i, line) in r.text.split('\n').enumerate() {
        if i > 0 {
            run = run.add_break(BreakType::TextWrapping);
        }
        run = run.add_text(line);
    }
    if let Some(f) = &s.font_family {
        run = run.fonts(RunFonts::new().ascii(f).hi_ansi(f).east_asia(f).cs(f));
    }
    if let Some(sz) = s.font_size {
        run = run.size((sz * 2.0).round() as usize);
    }
    let color = r
        .color
        .clone()
        .or_else(|| s.color.clone())
        .or_else(|| r.href.as_ref().map(|_| "#0563C1".to_string()));
    if let Some(c) = color {
        run = run.color(hex(&c));
    }
    if r.bold.or(s.bold) == Some(true) {
        run = run.bold();
    }
    if r.italic.or(s.italic) == Some(true) {
        run = run.italic();
    }
    if r.underline.or(s.underline) == Some(true) || r.href.is_some() {
        run = run.underline("single");
    }
    if r.code == Some(true) {
        run = run
            .fonts(RunFonts::new().ascii(MONO).hi_ansi(MONO).cs(MONO))
            .shading(Shading::new().shd_type(ShdType::Clear).fill("F3F3F3"));
    }
    run
}

fn hex(c: &str) -> String {
    c.trim_start_matches('#').to_ascii_uppercase()
}
