use std::path::Path;

use pewp::render::Format;

fn has(bytes: &[u8], needle: &str) -> bool {
    bytes.windows(needle.len()).any(|w| w == needle.as_bytes())
}

#[test]
fn showcase_renders_to_docx() {
    let out = pewp::render_str(
        include_str!("../examples/showcase.json"),
        Format::Docx,
        Some(Path::new("examples")),
    )
    .unwrap();
    assert!(out.bytes.starts_with(b"PK"));
    assert!(has(&out.bytes, "word/document.xml"));
    // logo.svg 放不进 docx，应该给警告而不是报错
    assert!(out.warnings.iter().any(|w| w.contains("logo")));
}

#[test]
fn png_image_is_embedded() {
    let dir = std::env::temp_dir().join("pewp-docx-test");
    std::fs::create_dir_all(&dir).unwrap();
    image::RgbImage::from_pixel(4, 2, image::Rgb([200, 30, 30]))
        .save(dir.join("dot.png"))
        .unwrap();
    let json = r#"{"root":"d","elements":{"d":{"type":"Document","children":["i"]},"i":{"type":"Image","props":{"src":"dot.png","width":40}}}}"#;
    let out = pewp::render_str(json, Format::Docx, Some(&dir)).unwrap();
    assert!(out.warnings.is_empty(), "{:?}", out.warnings);
    assert!(has(&out.bytes, "word/media/"));
}
