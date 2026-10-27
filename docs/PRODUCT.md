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
  - `pewp catalog`：列出能用的组件。
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

- `type`：组件名，必须在组件目录里。
- `props`：组件自己的参数，由组件的 schema 校验。
- `style`：可以写 `styles` 里的名字，也可以直接写一个样式对象。
- `children`：子元素 id 列表，只有容器组件（Document、Slide）用。

样式只收各格式都能表达的那部分：字体、字号、颜色、粗细、斜体、对齐、行距、段前段后、背景色、边框。某个格式表达不了的样式，渲染时忽略并给出警告，不报错。

## 自定义组件

一个组件一个文件，放在 `catalog/` 或用 `--catalog` 指定。例子：

```yaml
name: Callout
description: 带标题的提示框
props:
  title: { type: string }
  body:  { type: string }
template:
  root: box
  elements:
    box:   { type: Document, children: [head, text], style: { background: "#F5F5F5", padding: 8 } }
    head:  { type: Heading, props: { level: 3, text: "{{title}}" } }
    text:  { type: Text, props: { text: "{{body}}" } }
```

渲染前先把自定义组件展开成内置组件，所以各个渲染器只认内置组件。具体写法第一版动手时再定，以代码里的 schema 为准。

## 技术选型

版本在 2026-10-28 核对过 crates.io，动手加依赖前再看一次：

| 用途 | crate | 当时最新版 |
| --- | --- | --- |
| docx | docx-rs | 0.4.22 |
| pdf | typst（当库用） | 0.15.1 |
| xlsx（第二版） | rust_xlsxwriter | 0.101.0 |
| pptx（第二版） | ppt-rs | 0.2.37 |
| MCP | rmcp | 3.8.0 |
| 命令行 | clap | 4.6.7 |
| JSON Schema | schemars | 1.2.3 |

- PDF 走 Typst：先把文档转成 Typst 源码，再编译成 PDF。中文排版好，不用装 LibreOffice。字体要打包进二进制，或者读系统字体。
- HTML 自己拼字符串，不引模板引擎。
- YAML 解析：`serde_yaml` 已经停更，第一版动手时另选。

## 发布

- cargo-dist 出 GitHub Release，同时生成 Homebrew formula，推到自建 tap `reiy-leo/homebrew-tap`。
- 预编译 macOS Intel、macOS Apple Silicon、Linux x86_64。Rei 的 Mac 是 2018 年的 Intel 机器，不能让它从源码编译。
- 每次发版前先问 Rei。
