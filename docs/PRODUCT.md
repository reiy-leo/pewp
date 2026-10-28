# pewp 产品说明

pewp 把一份 JSON 文档渲染成 docx、pdf、html。JSON 里不写排版细节，只引用提前注册好的组件，再带一点样式。思路来自 [vercel-labs/json-render](https://github.com/vercel-labs/json-render)。

它是本地命令行工具，单个 Rust 二进制，另带一个 MCP server 给 AI 调用。通过 Homebrew 安装。

## 给谁用

- 想用一份数据同时出 Word、PDF 和网页的人。
- 想让 AI 直接产出办公文档的人：AI 只管写 JSON，pewp 负责排版。

## 第一版（v0.1）

- 格式：docx、pdf、html。
- 内置组件：Heading、Text、List、Table、Image、PageBreak、Slide，外加根组件 Document。
- 命令行：
  - `pewp render doc.json -o out.docx`：按输出文件的扩展名选格式。
  - `pewp check doc.json`：只校验，不出文件。
  - `pewp catalog`：列出能用的组件，加 `--json` 输出 schema。
  - 输入写 `-` 从标准输入读；输出写 `-` 时配 `--format` 写到标准输出。
  - `pewp mcp`：以 MCP server 方式运行（stdio）。
- MCP 工具：`render`（JSON 进，文件出）、`check`、`list_components`（带每个组件的 props 说明，方便 AI 写对）。
- 自定义组件：YAML 或 JSON 定义文件，用内置组件拼。

## 第二版

- xlsx、pptx。
- Slide 组件在 html 和 pdf 里按“一页一张”显示，到第二版再出真正的 pptx。

## 不做

- 不把 docx、pdf 解析回 JSON。
- 不做 Rust 插件。自定义组件只能拼内置组件。
- 不做图形界面，不做网站。
- 不依赖 LibreOffice 或 Word。

## 文档格式

结构沿用 json-render：所有元素平铺在 `elements` 里，用 id 互相引用，`root` 指向根元素。好处是 AI 可以一条一条地流式输出元素，也好做增量修改。

```json
{
  "version": "0.1",
  "meta": { "title": "季度报告", "lang": "zh-CN", "page": { "size": "A4", "margin": "20mm" } },
  "styles": { "muted": { "color": "#666666", "fontSize": 10 } },
  "root": "doc",
  "elements": {
    "doc":   { "type": "Document", "children": ["title", "intro"] },
    "title": { "type": "Heading", "props": { "level": 1, "text": "季度报告" } },
    "intro": { "type": "Text", "props": { "text": "一份 JSON，出三种格式。" }, "style": "muted" }
  }
}
```

- `type`：组件名，必须在组件目录里。`pewp catalog --json` 能看到每个组件 props 的 JSON Schema。
- `props`：组件自己的参数，多写、少写、写错类型都会报错。
- `style`：三种写法都行：样式名 `"muted"`；样式名数组 `["muted", "center"]`，后面的覆盖前面的；样式对象 `{ "fontSize": 11 }`。数组里也能混写对象。
- `children`：子元素 id 列表，只有容器组件（Document、Slide）能用。每个元素只能被引用一次。

文字类的 props（Heading 的 `text`、Text 的 `text`、List 的每一项、表格的每一格）都能写成纯字符串，也能写成 runs 数组，每段单独设置 `bold`、`italic`、`underline`、`code`、`color`、`href`。

`meta.page` 管纸张：`size`（默认 A4）、`landscape`、`margin`（默认 20mm）。

样式字段（完整设计见 [STYLE.md](STYLE.md)，草案）：`fontFamily`、`fontSize`（pt）、`color`、`bold`、`italic`、`underline`、`align`（left/center/right/justify）、`lineHeight`（倍数）、`spaceBefore`、`spaceAfter`、`background`、`border`（边框颜色，1pt 实线）、`padding`。颜色写 `#RRGGBB`。某个格式表达不了的样式，渲染时忽略并给出警告，不报错。

完整例子见 `examples/showcase.json`。

## 各格式的差别

同一份 JSON 在三种格式里尽量长得一样。docx 有这些不同：

- 图片只收 png、jpeg、gif、webp。SVG 和远程图片放不进去，改成显示 alt 文字，并给警告。
- `padding` 只在有 `border` 时生效，用作边框和文字的间距。
- Slide 变成一个二级标题加正文，后面分页。
- 纸张、方向、页边距按 `meta.page` 设置；表格铺满版心，`widths` 按比例分。

pdf 用 Typst 排版，有这些不同：

- 中文用系统字体：macOS 上是苹方，Linux 上是 Noto Sans CJK。都没有时退回 Typst 自带字体，中文可能显示不全。
- 中文字体没有斜体，`italic` 对中文不起作用。
- 支持 SVG；远程图片暂不支持，改成显示 alt 文字。
- 颜色只认十六进制写法，比如 `#c00`、`#1a73e8`。

## 自定义组件

自定义组件把内置组件组合起来，JSON 里像内置组件一样引用。渲染前先展开成内置组件，所以三个渲染器只认内置组件。

定义放在三个地方，都能用：

- 文档旁边的 `catalog/` 目录，自动读取。
- `--catalog` 指定的文件或目录，可以写多次。`render`、`check`、`catalog` 三个命令都认。
- 文档 JSON 里的 `components` 数组，适合 AI 通过 MCP 一次传完。

一个文件一个组件，YAML 或 JSON 都行：

```yaml
name: Callout                # 大写字母开头，不能和内置组件重名
description: 带标题的提示框
container: true              # 能放子元素；模板里要写一次 $children
props:
  title: { type: inline, required: true }
  tone:  { type: string, default: "#1a73e8" }
template:
  root: box
  elements:
    box:  { type: Document, children: [head, $children] }
    head: { type: Text, props: { text: "{{title}}" }, style: { bold: true, color: "{{tone}}" } }
```

规则：

- prop 的 `type`：`string`（默认）、`number`、`integer`、`boolean`、`array`、`object`、`inline`（纯字符串或 runs 数组）、`any`。
- 字符串正好是 `"{{title}}"` 时，整个换成 prop 的值，数组、对象也行；夹在文字里时按文字替换，比如 `"第 {{n}} 章"`。没给值的可选 prop 换成空。
- 实例的 id 留给模板根元素，其他模板元素的 id 变成 `实例id.模板id`，比如 `tip.head`。
- 实例的 `style` 叠在模板根元素的 `style` 后面。
- 模板里可以用别的自定义组件，最多套 32 层，超过就当成环报错。
- 少写必填 prop、多写没定义的 prop、类型不对，都会报错。

完整例子见 `examples/custom.json` 和 `examples/catalog/`。

## 技术选型

版本在 2026-10-28 核对过 crates.io，动手加依赖前再看一次：

| 用途 | crate | 当时最新版 |
| --- | --- | --- |
| docx | docx-rs | 0.4.22 |
| YAML | serde-saphyr | 2.0 |
| pdf | typst（当库用） | 0.15.1 |
| xlsx（第二版） | rust_xlsxwriter | 0.101.0 |
| pptx（第二版） | ppt-rs | 0.2.37 |
| MCP | rmcp | 3.8.0 |
| 命令行 | clap | 4.6.7 |
| JSON Schema | schemars | 1.2.3 |

- PDF 走 Typst：先把文档转成 Typst 源码，再编译成 PDF。中文排版好，不用装 LibreOffice。字体要打包进二进制，或者读系统字体。
- HTML 自己拼字符串，不引模板引擎。
- YAML 解析：`serde_yaml` 已经停更，改用 serde-saphyr。

## 发布

- cargo-dist 出 GitHub Release，同时生成 Homebrew formula，推到自建 tap `reiy-leo/homebrew-tap`。
- 预编译 macOS Intel、macOS Apple Silicon、Linux x86_64。Rei 的 Mac 是 2018 年的 Intel 机器，不能让它从源码编译。
- 每次发版前先问 Rei。
