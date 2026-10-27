//! 长度和纸张换算。docx 用 twip（1/20 pt），图片用 EMU（1pt = 12700）。

/// 解析 CSS 风格的长度（mm、cm、in、pt、px），返回 pt。
pub fn parse_len_pt(s: &str) -> Option<f32> {
    let s = s.trim();
    let split = s.find(|c: char| c.is_ascii_alphabetic()).unwrap_or(s.len());
    let (num, unit) = s.split_at(split);
    let n: f32 = num.trim().parse().ok()?;
    Some(match unit.trim() {
        "mm" => n * 72.0 / 25.4,
        "cm" => n * 72.0 / 2.54,
        "in" => n * 72.0,
        "pt" | "" => n,
        "px" => n * 0.75,
        _ => return None,
    })
}

/// 纸张宽高，pt。
pub fn page_size_pt(name: &str) -> Option<(f32, f32)> {
    let mm = |w: f32, h: f32| (w * 72.0 / 25.4, h * 72.0 / 25.4);
    Some(match name.to_ascii_uppercase().as_str() {
        "A3" => mm(297.0, 420.0),
        "A4" => mm(210.0, 297.0),
        "A5" => mm(148.0, 210.0),
        "B5" => mm(176.0, 250.0),
        "LETTER" => (612.0, 792.0),
        "LEGAL" => (612.0, 1008.0),
        _ => return None,
    })
}

pub fn twip(pt: f32) -> i32 {
    (pt * 20.0).round() as i32
}

pub fn emu(pt: f32) -> u32 {
    (pt * 12700.0).round() as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lengths() {
        assert!((parse_len_pt("25.4mm").unwrap() - 72.0).abs() < 0.01);
        assert_eq!(parse_len_pt("1in"), Some(72.0));
        assert_eq!(parse_len_pt("12"), Some(12.0));
        assert_eq!(parse_len_pt("3furlongs"), None);
        assert_eq!(twip(1.5), 30);
    }
}
