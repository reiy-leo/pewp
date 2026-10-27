//! 读本地图片等资源。相对路径按文档所在目录找。

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

pub struct Asset {
    pub bytes: Vec<u8>,
    pub mime: &'static str,
}

pub fn is_remote(src: &str) -> bool {
    src.starts_with("http://") || src.starts_with("https://") || src.starts_with("data:")
}

pub fn resolve(src: &str, base: Option<&Path>) -> PathBuf {
    let p = Path::new(src);
    match base {
        Some(b) if p.is_relative() => b.join(p),
        _ => p.to_path_buf(),
    }
}

pub fn load(src: &str, base: Option<&Path>) -> Result<Asset> {
    let path = resolve(src, base);
    let bytes = std::fs::read(&path).with_context(|| format!("读不了图片 {}", path.display()))?;
    Ok(Asset {
        mime: mime(&path, &bytes),
        bytes,
    })
}

fn mime(path: &Path, bytes: &[u8]) -> &'static str {
    if bytes.starts_with(b"\x89PNG") {
        return "image/png";
    }
    if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return "image/jpeg";
    }
    if bytes.starts_with(b"GIF8") {
        return "image/gif";
    }
    if bytes.len() > 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        return "image/webp";
    }
    match path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .as_deref()
    {
        Some("svg") => "image/svg+xml",
        _ => "application/octet-stream",
    }
}
