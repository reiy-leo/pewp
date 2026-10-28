# 样式模型（草案）

> 状态：草案，等 Rei 拍板后再改代码。现在代码里的样式只有 13 个字段，见 `src/style.rs`。

pewp 的样式以 OOXML 为准：先把 Word、PowerPoint、Excel 能配的属性列全，再归并成一套字段。HTML 和 PDF（Typst）按这套字段去对应，表达不了的就忽略并给警告。

依据：ECMA-376 第 5 版 Part 1（WordprocessingML 第 17 章、SpreadsheetML 第 18 章、DrawingML 第 20–21 章）。

## 一、写法约定

- 字段名用 camelCase，和现在一致。
- 按 OOXML 的层级分组：文字（rPr）、段落（pPr）、边框、背景、盒子（表格单元格、文本框）、表格、页面。分组只用来写文档；JSON 里字段平铺在 `style` 下，不用嵌套（待定，见第九节）。
- 长度：数字就是 pt；字符串可以带单位：`pt`、`mm`、`cm`、`in`、`px`（按 96 dpi 折算），还有两个排版单位：
  - `em`：当前字号的倍数，`"2em"` 就是首行缩进两个字。Word 里对应 `firstLineChars` 这类按字符计的属性。
  - `line`：行数，只用在段前段后，`"0.5line"` 对应 Word 的 `beforeLines`。
- 颜色：`#RRGGBB`，以后加 `#RRGGBBAA`（只有 PPT、HTML、PDF 认透明度）。另外接受主题色名（待定）。
- 同一个属性分「全部」和「单边」两种写法，单边覆盖全部：`border` 和 `borderTop`，`padding` 和 `paddingLeft`。

### 单位换算（实现时用）

| OOXML 单位 | 用在哪 | 1pt 等于 |
|---|---|---|
| twip（1/20 pt） | w:spacing、w:ind、页边距、表格宽 | 20 |
| 半磅 | w:sz（字号） | 2 |
| 1/8 pt | w:pBdr、w:tcBorders 的 sz | 8 |
| 1/100 pt | a:rPr sz、a:spcPts | 100 |
| EMU | DrawingML 的长度、线宽 | 12700 |
| 1/240 行 | w:spacing line（lineRule=auto） | — |
| 1/1000 % | a:spcPct、a:lnSpc | 100% = 100000 |

## 二、文字（对应 w:rPr、a:rPr、xlsx font）

| 字段 | 值 | Word w:rPr | PPT a:rPr | Excel font | HTML | PDF |
|---|---|---|---|---|---|---|
| `fontFamily` | 字符串，或按文种分开的对象（见下） | w:rFonts | a:latin / a:ea / a:cs | name（只有一个） | font-family | font |
| `fontSize` | 长度 | w:sz + w:szCs | sz | sz | font-size | size |
| `color` | 颜色 | w:color | a:solidFill | color | color | fill |
| `bold` | 布尔 | w:b + w:bCs | b | b | font-weight | weight |
| `italic` | 布尔 | w:i + w:iCs | i | i | font-style | style |
| `underline` | 布尔，或 `single` `double` `thick` `dotted` `dashed` `wavy` | w:u val | u | u（single、double，另有会计用） | text-decoration-style | underline（只有实线） |
| `underlineColor` | 颜色 | w:u color | a:uFill | — | text-decoration-color | stroke |
| `strike` | 布尔，或 `double` | w:strike / w:dstrike | strike（sngStrike / dblStrike） | strike（只有单线） | line-through | strike |
| `verticalAlign` | `super` `sub` | w:vertAlign | baseline（±30000） | vertAlign | vertical-align | super / sub |
| `caps` | `all` `small` | w:caps / w:smallCaps | cap | — | text-transform / font-variant | smallcaps / upper |
| `letterSpacing` | 长度 | w:spacing（twip） | spc（1/100 pt） | — | letter-spacing | tracking |
| `highlight` | 颜色 | w:highlight（只有 16 种固定色）或 w:shd | a:highlight | — | background | highlight |
| `emphasisMark` | `dot` `comma` `circle` `underDot` | w:em（着重号） | — | — | text-emphasis | — |
| `scale` | 百分比数字，如 `80` | w:w | — | — | transform（不准） | — |
| `kerning` | 布尔 | w:kern | kern | — | font-kerning | kerning |
| `lang` | BCP 47，如 `zh-CN` | w:lang val / eastAsia | lang / altLang | — | lang | lang |
| `hidden` | 布尔 | w:vanish | — | — | display:none | — |

`fontFamily` 按文种分开写：

```yaml
fontFamily:
  latin: Inter          # 西文：w:rFonts ascii + hAnsi，a:latin
  eastAsia: 思源黑体     # 中日韩：w:rFonts eastAsia，a:ea
  complex: Noto Naskh   # 阿拉伯文等：w:rFonts cs，a:cs
  mono: JetBrains Mono  # 代码 run 用，OOXML 没有这一项，pewp 自己加
```

写成字符串时四项都用它。Excel 一个单元格只能有一种字体：有中文内容取 `eastAsia`，否则取 `latin`。HTML、PDF 拼成回退列表：`latin, eastAsia`。

## 三、段落（对应 w:pPr、a:pPr、xlsx alignment）

| 字段 | 值 | Word w:pPr | PPT a:pPr | Excel alignment | HTML | PDF |
|---|---|---|---|---|---|---|
| `align` | `left` `center` `right` `justify` `distribute` | w:jc（start、center、end、both、distribute） | algn（l、ctr、r、just、dist） | horizontal | text-align | align / justify |
| `spaceBefore` | 长度或 `"Nline"` | w:spacing before / beforeLines | a:spcBef | — | margin-top | block above |
| `spaceAfter` | 同上 | w:spacing after / afterLines | a:spcAft | — | margin-bottom | block below |
| `lineHeight` | 数字是倍数；长度是固定值；`{ atLeast: 长度 }` 是最小值 | w:spacing line + lineRule（auto / exact / atLeast） | a:lnSpc（spcPct / spcPts） | — | line-height | par leading（近似） |
| `indentLeft` | 长度 | w:ind start | marL | indent（按字符数，只有整数） | padding-left | pad |
| `indentRight` | 长度 | w:ind end | — | — | padding-right | pad |
| `firstLineIndent` | 长度，负数是悬挂缩进 | w:ind firstLine / hanging（`em` 时用 firstLineChars） | indent | — | text-indent | first-line-indent |
| `keepWithNext` | 布尔 | w:keepNext | — | — | break-after: avoid | 用 block(breakable) 近似 |
| `keepTogether` | 布尔 | w:keepLines | — | — | break-inside: avoid | block(breakable: false) |
| `pageBreakBefore` | 布尔 | w:pageBreakBefore | — | — | break-before | pagebreak |
| `widowControl` | 布尔，默认开 | w:widowControl | — | — | orphans / widows | — |
| `contextualSpacing` | 布尔：同样式段落之间不加段距 | w:contextualSpacing | — | — | — | — |
| `autoSpace` | 布尔：中英文、中文和数字之间自动加空隙 | w:autoSpaceDE + w:autoSpaceDN | — | — | text-autospace（新） | — |
| `kinsoku` | 布尔：中文避头尾 | w:kinsoku | — | — | line-break: strict | 默认就有 |
| `direction` | `ltr` `rtl` | w:bidi | rtl | readingOrder | direction | dir |
| `wrap` | 布尔：自动换行 | — | a:bodyPr wrap | wrapText | white-space | — |
| `outlineLevel` | 1–9 | w:outlineLvl | — | — | — | — |

## 四、边框（对应 w:pBdr、w:tblBorders、w:tcBorders、a:ln、xlsx border）

一条边写成对象，也能用简写字符串 `"1pt solid #CCCCCC"`：

```yaml
border: { width: 1, style: solid, color: "#CCCCCC" }
borderBottom: { width: 2, style: double, color: "#1A73E8" }
borderBetween: none     # 段落之间的线，w:pBdr between
```

| 字段 | 用在哪 | Word | PPT | Excel | HTML | PDF |
|---|---|---|---|---|---|---|
| `border` / `borderTop` / `borderRight` / `borderBottom` / `borderLeft` | 段落、单元格、表格、文本框 | w:top … w:start、w:end | 形状只有一条 a:ln，四边不能分开 | left right top bottom | border-* | stroke |
| `borderBetween` | 相邻同样式段落之间 | w:pBdr between | — | — | — | — |
| `borderInsideH` / `borderInsideV` | 表格内部横线、竖线 | w:tblBorders insideH / insideV | a:tcPr 的 lnB 等逐格写 | 逐格写 | 逐格写 | table stroke |
| `borderDiagonal` | 单元格斜线 | w:tl2br / w:tr2bl | a:lnTlToBr | diagonal | — | — |

线型 `style` 只收各格式都有的几种，再按格式换：

| pewp | Word val | PPT prstDash / cmpd | Excel style | CSS |
|---|---|---|---|---|
| `none` | nil | a:noFill | 不写 | none |
| `solid` | single | solid | 按粗细：hair（≤0.25pt）、thin（≤1）、medium（≤2）、thick | solid |
| `dashed` | dashed | dash | dashed / mediumDashed | dashed |
| `dotted` | dotted | sysDot | dotted | dotted |
| `dashDot` | dotDash | dashDot | dashDot / mediumDashDot | dashed（近似） |
| `double` | double | cmpd="dbl" | double | double |
| `wave` | wave | — | — | wavy 只有下划线有，边框退成 solid |

Word 的线宽只能在 0.25pt 到 12pt 之间（sz 2–96），Excel 只有四档，换算时就近取。

## 五、背景（对应 w:shd、a:solidFill / a:gradFill / a:pattFill、xlsx fill）

```yaml
background: "#F5F7FA"                                   # 纯色
background: { pattern: pct20, color: "#000000", fill: "#FFFFFF" }   # 底纹
background: { gradient: linear, angle: 90, stops: ["#FFFFFF", "#E8F0FE"] }
```

| 写法 | Word | PPT | Excel | HTML | PDF |
|---|---|---|---|---|---|
| 纯色 | w:shd val=clear fill | a:solidFill | patternFill solid | background-color | fill |
| 底纹 `pattern` | w:shd val（pct5…pct95、horzStripe 等） | a:pattFill prst | patternFill patternType（gray125、darkGrid 等） | 只能近似成纯色 | 近似成纯色 |
| 渐变 | 段落和单元格不支持，退成第一个色 | a:gradFill | gradientFill | linear-gradient | gradient.linear |
| 图片 | 不支持 | a:blipFill | 不支持 | background-image | 不支持 |

`pattern` 的名字取 Word 和 Excel 的交集，具体列表实现时定。

## 六、盒子（单元格、文本框、提示框这类容器）

| 字段 | 值 | Word | PPT | Excel | HTML | PDF |
|---|---|---|---|---|---|---|
| `padding` / `paddingTop` … | 长度 | w:tcMar；段落用边框 space 近似 | a:bodyPr lIns tIns rIns bIns | — | padding | inset |
| `verticalAlign` | `top` `middle` `bottom` | w:vAlign | a:bodyPr anchor | vertical | vertical-align | align |
| `width` / `height` | 长度或百分比 | w:tcW、w:trHeight | a:xfrm ext | 列宽、行高 | width / height | width / height |
| `textRotation` | `0` `90` `270` | w:textDirection | a:bodyPr vert | textRotation | writing-mode | rotate |
| `radius` | 长度 | — | prstGeom roundRect | — | border-radius | radius |

## 七、表格（对应 w:tblPr、w:trPr、xlsx 工作表）

这些是表格组件的样式，不是每个元素都能用。

| 字段 | 值 | Word | PPT | Excel | HTML | PDF |
|---|---|---|---|---|---|---|
| `tableLayout` | `fixed` `auto` | w:tblLayout | 固定 | 固定 | table-layout | 列宽 |
| `cellPadding` | 长度 | w:tblCellMar | 逐格 | — | td padding | inset |
| `repeatHeader` | 布尔，默认开 | w:tblHeader | — | 打印标题行 | thead | table.header(repeat) |
| `rowBreak` | 布尔：行内能不能跨页 | w:cantSplit | — | — | break-inside | — |
| `stripe` | 颜色：隔行底色 | 逐行写 w:shd | 逐行写 | 逐行写 | nth-child | fill 函数 |
| `headerStyle` | 样式名或样式对象：表头行用 | — | — | — | — | — |

合并单元格（w:gridSpan、w:vMerge）放在 Table 的 props 里，不算样式。

## 八、页面（`meta.page`，对应 w:sectPr）

| 字段 | 值 | Word | PPT | Excel | HTML | PDF |
|---|---|---|---|---|---|---|
| `size` | `A4` `Letter` …，或 `{ width, height }` | w:pgSz | p:sldSz | pageSetup paperSize | @page size | page width / height |
| `landscape` | 布尔 | w:pgSz orient | — | orientation | @page | flipped |
| `margin` / `marginTop` … | 长度 | w:pgMar | — | pageMargins | @page margin | margin |
| `headerDistance` / `footerDistance` | 长度 | w:pgMar header / footer | — | pageMargins header / footer | — | header-ascent |
| `gutter` | 长度 | w:pgMar gutter | — | — | — | binding |
| `columns` | 数字，或 `{ count, gap }` | w:cols | — | — | columns | columns |
| `verticalAlign` | `top` `center` | w:vAlign | — | — | — | — |

页眉页脚、页码要单独设计，先不放进这一版。

## 九、样式怎么继承

1. 内置默认样式（每个组件一份，比如 Heading 1 是 22pt 加粗）。
2. `styles` 里以组件名为键的样式：`Heading`、`Text`、`Table`，管这类组件的全部元素；`Heading.1` 只管一级标题。
3. 元素的 `style`：样式名、样式名数组、样式对象，后面的覆盖前面的。
4. runs 上的文字属性，只能写第二节的字段。

文字属性往子元素继承（像 CSS），边框、背景、盒子属性不继承。docx 里第 2 层写成 Word 的样式（styles.xml 里的 w:style），用户在 Word 里改「标题 1」能一起改。

## 十、要 Rei 定的

1. **字段平铺还是分组。** 推荐平铺：`style: { fontSize: 12, spaceAfter: 6, border: … }`，和 CSS 一样好写；分组写法是 `style: { run: { size: 12 }, paragraph: { after: 6 } }`，离 OOXML 更近，但每次多一层。
2. **字段名跟谁。** 推荐跟 CSS 走（`lineHeight`、`letterSpacing`、`textRotation`），OOXML 名字只写在本文档的对照表里；另一种是直接用 OOXML 名（`jc`、`ind`、`shd`），对熟悉 Word XML 的人直观，但对 AI 和多数人难懂。
3. **主题色。** 要不要支持 `accent1`、`text1` 这类主题色名（对应 Word/PPT 主题），换主题就换一套颜色。推荐先不做。
