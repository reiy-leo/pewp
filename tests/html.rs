use pewp::render::Format;

fn html(json: &str) -> String {
    String::from_utf8(
        pewp::render_str(json, Format::Html, Some(std::path::Path::new("examples")))
            .unwrap()
            .bytes,
    )
    .unwrap()
}

#[test]
fn hello_example_renders() {
    let s = html(include_str!("../examples/hello.json"));
    assert!(s.contains("<h1 id=\"title\">季度报告</h1>"));
    assert!(s.contains("<th>格式</th>"));
    assert!(s.contains("color:#666666"));
    assert!(s.contains("@page { size: A4; margin: 20mm; }"));
}

#[test]
fn escapes_text_and_css() {
    let s = html(
        r##"{
      "root": "d",
      "elements": {
        "d": { "type": "Document", "children": ["t"] },
        "t": { "type": "Text", "props": { "text": [{ "text": "<b>x</b>", "color": "red;}body{display:none" }] } }
      }
    }"##,
    );
    assert!(s.contains("&lt;b&gt;x&lt;/b&gt;"));
    assert!(!s.contains("red;}"));
}

#[test]
fn rich_runs_and_slide() {
    let s = html(
        r##"{
      "root": "d",
      "elements": {
        "d": { "type": "Document", "children": ["s"] },
        "s": { "type": "Slide", "props": { "title": "封面" }, "children": ["t"] },
        "t": { "type": "Text", "props": { "text": [{ "text": "粗", "bold": true }, { "text": "链", "href": "https://example.com" }] } }
      }
    }"##,
    );
    assert!(s.contains("class=\"pewp-slide\""));
    assert!(s.contains("<strong>粗</strong>"));
    assert!(s.contains("<a href=\"https://example.com\">链</a>"));
}

fn err(json: &str) -> String {
    match pewp::render_str(json, Format::Html, None) {
        Ok(_) => panic!("应该报错"),
        Err(e) => format!("{e:#}"),
    }
}

#[test]
fn rejects_bad_documents() {
    assert!(err(r#"{"root":"x","elements":{}}"#).contains("`x` 不存在"));
    assert!(err(r#"{"root":"d","elements":{"d":{"type":"Nope"}}}"#).contains("没注册"));
    assert!(
        err(r#"{"root":"d","elements":{"d":{"type":"Heading","props":{"level":9,"text":"a"}}}}"#)
            .contains("level")
    );
    assert!(
        err(
            r#"{"root":"d","elements":{"d":{"type":"Text","props":{"text":"a"},"children":["d"]}}}"#
        )
        .contains("不能有 children")
    );
    assert!(
        err(r#"{"root":"d","elements":{"d":{"type":"Document","children":["d"]}}}"#)
            .contains("不止一次")
    );
    assert!(
        err(r#"{"root":"d","elements":{"d":{"type":"Text","props":{"text":"a"},"style":"nope"}}}"#)
            .contains("nope")
    );
}

#[test]
fn showcase_example_renders() {
    let s = html(include_str!("../examples/showcase.json"));
    assert!(s.contains("<ol id=\"steps\">"));
    assert!(s.contains("<col style=\"width:20.00%\">"));
    assert!(s.contains("class=\"pewp-pagebreak\""));
    assert!(s.contains("font-size:11pt"));
    assert!(s.contains("src=\"data:image/svg+xml;base64,"));
}
