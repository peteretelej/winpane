use winpane::Color;

/// Parses a CSS hex color string to a `Color`.
///
/// Supported formats: `#rgb`, `#rrggbb`, `#rrggbbaa`.
pub fn parse_color(s: &str) -> Result<Color, String> {
    let hex = s.strip_prefix('#').unwrap_or(s);

    let parse_byte = |h: &str| -> Result<u8, String> {
        u8::from_str_radix(h, 16).map_err(|_| format!("invalid hex color: {s}"))
    };

    match hex.len() {
        3 => {
            let r = parse_byte(&hex[0..1])?;
            let g = parse_byte(&hex[1..2])?;
            let b = parse_byte(&hex[2..3])?;
            Ok(Color::rgba(r << 4 | r, g << 4 | g, b << 4 | b, 255))
        }
        6 => {
            let r = parse_byte(&hex[0..2])?;
            let g = parse_byte(&hex[2..4])?;
            let b = parse_byte(&hex[4..6])?;
            Ok(Color::rgba(r, g, b, 255))
        }
        8 => {
            let r = parse_byte(&hex[0..2])?;
            let g = parse_byte(&hex[2..4])?;
            let b = parse_byte(&hex[4..6])?;
            let a = parse_byte(&hex[6..8])?;
            Ok(Color::rgba(r, g, b, a))
        }
        _ => Err(format!("invalid hex color length: {s}")),
    }
}

/// Loads an image file and returns RGBA8 pixel data.
///
/// If `premultiply` is true, premultiplies alpha into RGB channels
/// (required for `ImageElement.data`). Pass `false` for tray icons
/// which expect straight alpha.
pub fn load_image_rgba(path: &str, premultiply: bool) -> Result<(Vec<u8>, u32, u32), String> {
    let img = image::open(path).map_err(|e| format!("failed to load image {path}: {e}"))?;
    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();
    let mut pixels = rgba.into_raw();

    if premultiply {
        for chunk in pixels.chunks_exact_mut(4) {
            let a = chunk[3] as u16;
            chunk[0] = ((chunk[0] as u16 * a) / 255) as u8;
            chunk[1] = ((chunk[1] as u16 * a) / 255) as u8;
            chunk[2] = ((chunk[2] as u16 * a) / 255) as u8;
        }
    }

    Ok((pixels, width, height))
}

/// Extracts an optional color field from a JSON object.
///
/// Returns `Ok(None)` if the key is missing or null, `Ok(Some(color))`
/// if present and valid, or `Err` if present but invalid.
pub fn extract_optional_color(
    value: &serde_json::Value,
    key: &str,
) -> Result<Option<Color>, String> {
    match value.get(key) {
        None | Some(serde_json::Value::Null) => Ok(None),
        Some(serde_json::Value::String(s)) => parse_color(s).map(Some),
        Some(_) => Err(format!("{key} must be a hex color string")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_color_3_digit() {
        let c = parse_color("#f0a").unwrap();
        assert_eq!((c.r, c.g, c.b, c.a), (0xff, 0x00, 0xaa, 0xff));
    }

    #[test]
    fn parse_color_6_digit() {
        let c = parse_color("#1a2b3c").unwrap();
        assert_eq!((c.r, c.g, c.b, c.a), (0x1a, 0x2b, 0x3c, 0xff));
    }

    #[test]
    fn parse_color_8_digit() {
        let c = parse_color("#1a2b3c80").unwrap();
        assert_eq!((c.r, c.g, c.b, c.a), (0x1a, 0x2b, 0x3c, 0x80));
    }

    #[test]
    fn parse_color_no_hash() {
        let c = parse_color("ff0000").unwrap();
        assert_eq!((c.r, c.g, c.b, c.a), (0xff, 0x00, 0x00, 0xff));
    }

    #[test]
    fn parse_color_invalid() {
        assert!(parse_color("#xyz").is_err());
        assert!(parse_color("#12345").is_err());
    }

    #[test]
    fn extract_optional_color_missing() {
        let obj = serde_json::json!({});
        assert_eq!(extract_optional_color(&obj, "color").unwrap(), None);
    }

    #[test]
    fn extract_optional_color_null() {
        let obj = serde_json::json!({ "color": null });
        assert_eq!(extract_optional_color(&obj, "color").unwrap(), None);
    }

    #[test]
    fn extract_optional_color_valid() {
        let obj = serde_json::json!({ "color": "#ff0000" });
        let c = extract_optional_color(&obj, "color").unwrap().unwrap();
        assert_eq!((c.r, c.g, c.b, c.a), (0xff, 0x00, 0x00, 0xff));
    }

    #[test]
    fn extract_optional_color_invalid_type() {
        let obj = serde_json::json!({ "color": 123 });
        assert!(extract_optional_color(&obj, "color").is_err());
    }
}
