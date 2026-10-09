//! Helpers that mimic the Python semantics rivalcfg relies on (`str()`, `int()`,
//! `parse_param_string`, colors), so values behave exactly like in rivalcfg.

use std::collections::BTreeMap;

use serde_json::Value;

pub type Result<T> = std::result::Result<T, String>;

/// Python `str(value)`.
pub fn py_str(value: &Value) -> String {
    match value {
        Value::Null => "None".into(),
        Value::Bool(true) => "True".into(),
        Value::Bool(false) => "False".into(),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                i.to_string()
            } else if let Some(u) = n.as_u64() {
                u.to_string()
            } else {
                let f = n.as_f64().unwrap_or(0.0);
                if f.fract() == 0.0 && f.abs() < 1e16 {
                    format!("{f:.1}")
                } else {
                    f.to_string()
                }
            }
        }
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

/// Python `int(text)` (base 10).
pub fn parse_int(text: &str) -> Result<i64> {
    text.trim()
        .parse::<i64>()
        .map_err(|_| format!("invalid literal for int() with base 10: '{text}'"))
}

/// Python `int(value)` for numbers, numeric strings and booleans.
pub fn py_int(value: &Value) -> Result<i64> {
    match value {
        Value::Bool(b) => Ok(*b as i64),
        Value::Number(n) => n
            .as_i64()
            .or_else(|| n.as_f64().map(|f| f.trunc() as i64))
            .ok_or_else(|| format!("invalid number {n}")),
        Value::String(s) => parse_int(s),
        other => Err(format!("int() argument must be a string or a number, not '{other}'")),
    }
}

/// Integer channel check used for color tuples (`isinstance(c, int) and 0 <= c <= 255`).
pub fn as_channel(value: &Value) -> Option<u8> {
    value.as_i64().filter(|c| (0..=255).contains(c)).map(|c| c as u8)
}

/// `rivalcfg.helpers.uint_to_little_endian_bytearray`
pub fn uint_to_le(number: i64, size: usize) -> Result<Vec<i64>> {
    if size < 8 && number > (1i64 << (8 * size)) - 1 {
        return Err("integer overflow".into());
    }
    Ok((0..size).map(|i| (number >> (i * 8)) & 0xFF).collect())
}

// ---------------------------------------------------------------- colors ---

pub const NAMED_COLORS: [(&str, [u8; 3]); 16] = [
    ("white", [0xFF, 0xFF, 0xFF]),
    ("silver", [0xC0, 0xC0, 0xC0]),
    ("gray", [0x80, 0x80, 0x80]),
    ("black", [0x00, 0x00, 0x00]),
    ("red", [0xFF, 0x00, 0x00]),
    ("maroon", [0x80, 0x00, 0x00]),
    ("yellow", [0xFF, 0xFF, 0x00]),
    ("olive", [0x80, 0x80, 0x00]),
    ("lime", [0x00, 0xFF, 0x00]),
    ("green", [0x00, 0x80, 0x00]),
    ("aqua", [0x00, 0xFF, 0xFF]),
    ("teal", [0x00, 0x80, 0x80]),
    ("blue", [0x00, 0x00, 0xFF]),
    ("navy", [0x00, 0x00, 0x80]),
    ("fuchsia", [0xFF, 0x00, 0xFF]),
    ("purple", [0x80, 0x00, 0x80]),
];

fn named_color(name: &str) -> Option<[u8; 3]> {
    let lower = name.to_lowercase();
    NAMED_COLORS.iter().find(|(n, _)| *n == lower).map(|(_, c)| *c)
}

/// `rivalcfg.color_helpers.is_color`
pub fn is_color(text: &str) -> bool {
    if named_color(text).is_some() {
        return true;
    }
    let hex = text.strip_prefix('#').unwrap_or(text);
    (hex.len() == 3 || hex.len() == 6) && hex.chars().all(|c| c.is_ascii_hexdigit())
}

/// `rivalcfg.color_helpers.parse_color_string` (no validation, like the original)
pub fn parse_color_string(text: &str) -> Result<[u8; 3]> {
    if let Some(c) = named_color(text) {
        return Ok(c);
    }
    let mut hex: String = text.strip_prefix('#').unwrap_or(text).to_string();
    if hex.chars().count() == 3 {
        hex = hex.chars().flat_map(|c| [c, c]).collect();
    }
    let part = |range: std::ops::Range<usize>| -> Result<u8> {
        let s = hex.get(range.start.min(hex.len())..range.end.min(hex.len())).unwrap_or("");
        u8::from_str_radix(s.trim(), 16).map_err(|_| format!("invalid color '{text}'"))
    };
    Ok([part(0..2)?, part(2..4)?, part(4..usize::MAX)?])
}

/// Parses a color given as a string or an `[r, g, b]` list.
pub fn color_from_value(value: &Value) -> Result<[u8; 3]> {
    match value {
        Value::Array(items) if items.len() == 3 => {
            let channels: Option<Vec<u8>> = items.iter().map(as_channel).collect();
            channels
                .map(|c| [c[0], c[1], c[2]])
                .ok_or_else(|| format!("Not a valid color {value}"))
        }
        Value::String(s) if is_color(s) => parse_color_string(s),
        _ => Err(format!("Not a valid color {}", py_str(value))),
    }
}

pub fn color_to_hex(c: [u8; 3]) -> String {
    format!("#{:02x}{:02x}{:02x}", c[0], c[1], c[2])
}

#[derive(Debug, Clone, PartialEq)]
pub struct ColorStop {
    pub pos: i64,
    pub color: [u8; 3],
}

/// `rivalcfg.color_helpers.parse_color_gradient_string`
pub fn parse_color_gradient_string(gradient: &str) -> Result<Vec<ColorStop>> {
    let g = gradient.replace([' ', '%'], "");
    let invalid = || {
        format!("invalid color gradient '{g}'. It must looks like '<POS1>:<COLOR1>,<POS2>:<COLOR2>,...'")
    };
    // Prefix check equivalent to re.match(r"[0-9-]+:[a-zA-Z0-9#]+...")
    let first = g.split(',').next().unwrap_or("");
    let mut halves = first.splitn(2, ':');
    let (p, c) = (halves.next().unwrap_or(""), halves.next().unwrap_or(""));
    let color_prefix: String = c
        .chars()
        .take_while(|ch| ch.is_ascii_alphanumeric() || *ch == '#')
        .collect();
    if p.is_empty() || !p.chars().all(|ch| ch.is_ascii_digit() || ch == '-') || color_prefix.is_empty() {
        return Err(invalid());
    }

    let mut result = Vec::new();
    for item in g.split(',') {
        let parts: Vec<&str> = item.split(':').collect();
        if parts.len() != 2 {
            return Err(invalid());
        }
        let pos = parse_int(parts[0])?;
        if !(0..=100).contains(&pos) {
            return Err(format!("invalid color stop position '{pos}%'"));
        }
        if !is_color(parts[1]) {
            return Err(format!("invalid color '{}'", parts[1]));
        }
        result.push(ColorStop { pos, color: parse_color_string(parts[1])? });
    }
    Ok(result)
}

// ---------------------------------------------------------- param string ---

/// Matches `REGEXP_PARAM_STRING`; returns `(name, params)`.
pub fn match_param_string(text: &str) -> Option<(String, String)> {
    let s = text.trim_start();
    let name_len = s
        .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .unwrap_or(s.len());
    if name_len == 0 {
        return None;
    }
    let name = &s[..name_len];
    let rest = s[name_len..].trim_start();
    let rest = rest.strip_prefix('(')?;
    let close = rest.rfind(')')?;
    if !rest[close + 1..].chars().all(|c| c.is_whitespace() || c == ';') {
        return None;
    }
    let inner = &rest[..close];
    let trimmed = inner.trim_start();
    let inner = if trimmed.is_empty() { inner } else { trimmed };
    let params = inner.trim_end_matches(|c: char| c.is_whitespace() || c == ';');
    let params = if params.is_empty() { inner.get(..1)? } else { params };
    Some((name.to_string(), params.to_string()))
}

/// `rivalcfg.helpers.parse_param_string` (values left as strings).
pub fn parse_param_string(text: &str) -> Result<(String, BTreeMap<String, String>)> {
    let invalid = || format!("invalid parameter string '{text}'");
    let (name, params) = match_param_string(text).ok_or_else(invalid)?;
    let mut map = BTreeMap::new();
    for p in params.split(';') {
        let kv: Vec<&str> = p.split('=').collect();
        if kv.len() != 2 {
            return Err(invalid());
        }
        map.insert(kv[0].trim().to_string(), kv[1].trim().to_string());
    }
    Ok((name, map))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn param_string() {
        let (n, m) = parse_param_string("hello(name=world ) ;;").unwrap();
        assert_eq!(n, "hello");
        assert_eq!(m["name"], "world");
        assert!(parse_param_string("foobar[a=1]").is_err());
        let (_, m) = parse_param_string("foo(a=42; b=3.14;)").unwrap();
        assert_eq!(m.len(), 2);
    }

    #[test]
    fn colors() {
        assert_eq!(parse_color_string("#F00").unwrap(), [255, 0, 0]);
        assert_eq!(parse_color_string("RED").unwrap(), [255, 0, 0]);
        assert!(is_color("ff0000") && !is_color("#FF00") && !is_color("hello"));
        let g = parse_color_gradient_string("0%: red, 33%: #00ff00, 66: 00f").unwrap();
        assert_eq!(g.len(), 3);
        assert_eq!(g[2], ColorStop { pos: 66, color: [0, 0, 255] });
        assert!(parse_color_gradient_string("-1%: red").is_err());
        assert!(parse_color_gradient_string("42%: hello").is_err());
    }

    #[test]
    fn le_bytes() {
        assert_eq!(uint_to_le(0xFF42, 4).unwrap(), vec![0x42, 0xFF, 0, 0]);
        assert!(uint_to_le(0xFFFFFF, 2).is_err());
    }
}
