//! `rivalcfg.handlers.rgbgradient` and `rivalcfg.handlers.rgbgradientv2`

use serde_json::Value;

use crate::core::profile::Setting;
use crate::core::pyval::{
    as_channel, is_color, parse_color_gradient_string, parse_color_string, parse_int,
    parse_param_string, py_str, uint_to_le, ColorStop, Result,
};

pub const DEFAULT_DURATION: i64 = 1000;

/// A parsed gradient value.
#[derive(Debug, Clone, PartialEq)]
pub struct Gradient {
    /// `false` for a plain color
    pub is_gradient: bool,
    pub duration: i64,
    pub stops: Vec<ColorStop>,
}

/// `is_rgbgradient` + `parse_param_string` with the gradient value parsers.
/// Returns the duration and the stops as written (without smoothing).
pub fn parse_rgbgradient_string(text: &str) -> Result<(i64, Vec<ColorStop>)> {
    let (name, params) = parse_param_string(text)?;
    if name != "rgbgradient" {
        return Err("invalid rgbgradient expression. It must looks like 'rgbgradient(<PARAMS>)'.".into());
    }
    // Value parsers run on every known key before validation
    let duration = params.get("duration").map(|d| parse_int(d)).transpose()?;
    let colors = params.get("colors").map(|c| parse_color_gradient_string(c)).transpose()?;
    let colors = colors.ok_or(
        "invalid rgbgradient expression. You must provide at least one color: 'rgbgradient(colors=<POS>: <COLOR>)'.",
    )?;
    if let Some(key) = params.keys().find(|k| *k != "colors" && *k != "duration") {
        return Err(format!("unknown parameter '{key}'. Allowed parameters are 'colors', 'duration'."));
    }
    Ok((duration.unwrap_or(DEFAULT_DURATION), colors))
}

/// `_handle_rgbgradient_dict` for the Python dict form `{"duration": .., "colors": [{"pos", "color"}]}`.
fn stops_from_dict(map: &serde_json::Map<String, Value>) -> Result<(i64, Vec<ColorStop>)> {
    let duration = match map.get("duration") {
        Some(d) => d.as_i64().ok_or_else(|| format!("invalid duration {d}"))?,
        None => DEFAULT_DURATION,
    };
    let mut stops = Vec::new();
    if let Some(colors) = map.get("colors") {
        let colors = colors.as_array().ok_or("colors must be a list")?;
        for stop in colors {
            let color = &stop["color"];
            let rgb = match color {
                Value::String(s) => parse_color_string(s)?,
                Value::Array(items) if items.len() == 3 => {
                    let c: Option<Vec<u8>> = items.iter().map(as_channel).collect();
                    let c = c.ok_or_else(|| format!("Not a valid color {color}"))?;
                    [c[0], c[1], c[2]]
                }
                _ => return Err(format!("Not a valid color {color}")),
            };
            let pos = match stop.get("pos") {
                Some(p) => p.as_i64().ok_or_else(|| format!("invalid position {p}"))?,
                None => 0,
            };
            stops.push(ColorStop { pos, color: rgb });
        }
    }
    Ok((duration, stops))
}

/// Adds a final stop at 100% (with the first color) to smooth the loop.
fn smooth(mut stops: Vec<ColorStop>) -> Result<Vec<ColorStop>> {
    let last = stops.last().ok_or("list index out of range")?;
    if stops.len() < 14 && last.pos != 100 {
        let first = stops[0].color;
        stops.push(ColorStop { pos: 100, color: first });
    }
    Ok(stops)
}

/// Interprets a value the way both gradient handlers do (with the smoothing stop).
pub fn parse_gradient_value(value: &Value) -> Result<Gradient> {
    let mut gradient = parse_gradient_raw(value)?;
    if gradient.is_gradient {
        gradient.stops = smooth(gradient.stops)?;
    }
    Ok(gradient)
}

/// Like [`parse_gradient_value`] but keeps the stops as written.
pub fn parse_gradient_raw(value: &Value) -> Result<Gradient> {
    match value {
        Value::Array(items) => {
            let c: Option<Vec<u8>> = (items.len() == 3)
                .then(|| items.iter().map(as_channel).collect())
                .flatten();
            let c = c.ok_or_else(|| format!("Not a valid color {value}"))?;
            Ok(Gradient {
                is_gradient: false,
                duration: DEFAULT_DURATION,
                stops: vec![ColorStop { pos: 0, color: [c[0], c[1], c[2]] }],
            })
        }
        Value::String(s) if is_color(s) => Ok(Gradient {
            is_gradient: false,
            duration: DEFAULT_DURATION,
            stops: vec![ColorStop { pos: 0, color: parse_color_string(s)? }],
        }),
        Value::Object(map) => {
            let (duration, stops) = stops_from_dict(map)?;
            Ok(Gradient { is_gradient: true, duration, stops })
        }
        Value::String(s) => match parse_rgbgradient_string(s) {
            Ok((duration, stops)) => Ok(Gradient { is_gradient: true, duration, stops }),
            Err(_) => Err(format!("Not a valid color or rgbgradient {s}")),
        },
        other => Err(format!("Not a valid color or rgbgradient {}", py_str(other))),
    }
}

pub fn process_v1(setting: &Setting, value: &Value) -> Result<Vec<i64>> {
    let h = setting.rgbgradient_header.as_ref().ok_or("missing rgbgradient_header")?;
    let led_id = setting.led_id.ok_or("missing led_id")? as i64;
    let gradient = parse_gradient_value(value)?;
    let triggers = 0x00;
    let repeat = if !gradient.is_gradient || triggers != 0x00 { 0x01 } else { 0x00 };

    if gradient.stops.is_empty() {
        return Err(format!("no color: {}", py_str(value)));
    }
    if gradient.stops.len() > 14 {
        return Err("a maximum of 14 color stops are allowed".into());
    }

    let mut header = vec![0i64; h.header_length];
    let mut set = |offset: usize, v: i64| -> Result<()> {
        *header.get_mut(offset).ok_or("list assignment index out of range")? = v;
        Ok(())
    };
    set(h.repeat_offset, repeat)?;
    set(h.triggers_offset, triggers)?;
    set(h.color_count_offset, gradient.stops.len() as i64)?;
    for offset in &h.led_id_offsets {
        set(*offset, led_id)?;
    }
    for (i, b) in uint_to_le(gradient.duration, h.duration_length)?.into_iter().enumerate() {
        set(h.duration_offset + i, b)?;
    }

    let mut body: Vec<i64> = gradient.stops[0].color.iter().map(|c| *c as i64).collect();
    let mut last_real_pos = 0;
    for stop in &gradient.stops {
        let real_pos = ((stop.pos * 255) as f64 / 100.0).trunc() as i64;
        body.extend(stop.color.iter().map(|c| *c as i64));
        body.push(real_pos - last_real_pos);
        last_real_pos = real_pos;
    }
    header.extend(body);
    Ok(header)
}

pub fn process_v2(setting: &Setting, value: &Value) -> Result<Vec<i64>> {
    let h = setting.rgbgradientv2_header.as_ref().ok_or("missing rgbgradientv2_header")?;
    let led_id = setting.led_id.ok_or("missing led_id")? as i64;
    let gradient = parse_gradient_value(value)?;
    let duration = gradient.duration;
    let stops = gradient.stops;

    if stops.is_empty() {
        return Err(format!("no color: {}", py_str(value)));
    }
    let length = stops.len();
    if length > h.maxgradient {
        return Err(format!("a maximum of {} color stops are allowed", h.maxgradient));
    }
    let minimum_duration = (length as f64 * 33.3).trunc() as i64;
    if duration < minimum_duration {
        return Err(format!("a duration of {minimum_duration} or above is need for {length} gradient"));
    }
    if duration > 30000 {
        return Err("a maximum duration of 30000ms is allowed".into());
    }

    let mut header: Vec<i64> = vec![led_id, 0x1D, 0x01, 0x02, 0x31, 0x51, 0xFF, 0xC8, 0x00];
    let start_color = stops[0].color;
    let mut last_real_pos = stops[0].pos;
    let mut old_color = start_color.map(|c| c as i64);
    for (index, stop) in stops.iter().skip(1).enumerate() {
        if stop.pos <= last_real_pos {
            return Err(
                "Incorrect order for gradient or duplicate order found please check position order".into(),
            );
        }
        header.push(index as i64);
        header.push(0);
        let time = ((duration as f64 / 100.0) * (stop.pos - last_real_pos) as f64).trunc() as i64;
        last_real_pos = stop.pos;
        if time == 0 {
            return Err("Incompatble timings set, please set different timings".into());
        }
        for (i, channel) in stop.color.iter().enumerate() {
            let diff = *channel as i64 - old_color[i];
            let ramp = (diff as f64 / time as f64 * 16.0).trunc() as i64;
            old_color[i] = *channel as i64;
            header.push(ramp & 255);
        }
        header.push(0);
        header.extend(uint_to_le(time, 2)?);
    }

    if h.color_field_length > header.len() {
        header.resize(h.color_field_length, 0);
    }

    for channel in start_color {
        let (high, low) = ((channel >> 4) as i64, (channel & 0x0F) as i64);
        header.push(low << 4);
        header.push(high);
    }
    header.extend([0xFF, 0x00]);
    header.extend(uint_to_le(1500, 2)?);
    header.extend(uint_to_le(650, 2)?);
    header.extend([0x00, 0x00, 0x00, 0x00, 0x01, 0x00]);
    header.extend(uint_to_le(length as i64 - 1, 2)?);
    header.extend(uint_to_le(duration, h.duration_length)?);
    Ok(header)
}
