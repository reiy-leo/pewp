use std::path::Path;

use pewp::render::Format;

#[test]
fn showcase_renders_to_pdf() {
    let out = pewp::render_str(
        include_str!("../examples/showcase.json"),
        Format::Pdf,
        Some(Path::new("examples")),
    )
    .unwrap();
    assert!(out.bytes.starts_with(b"%PDF-"));
    // SVG 在 pdf 里能直接用，不该有图片警告
    assert!(
        !out.warnings.iter().any(|w| w.contains("图片")),
        "{:?}",
        out.warnings
    );
}

#[test]
fn quotes_and_markup_chars_survive() {
    let json = r##"{"root":"d","elements":{"d":{"type":"Document","children":["t"]},"t":{"type":"Text","props":{"text":"a \"b\" \\ #c *d* _e_ $f$ <g> @h [i]"}}}}"##;
    let out = pewp::render_str(json, Format::Pdf, None).unwrap();
    assert!(out.bytes.starts_with(b"%PDF-"));
}
