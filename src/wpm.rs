//! The pathing tools, on `wpm-rs`: `war3map.wpm`, for every cell of a
//! map's grid (four to a terrain cell, 32 world units), whether units can
//! walk, fly, build or sail there, blight, and the map's boundary. A file
//! is read from disk (`path`) or from inside a map (`path` inside
//! `archive`, `war3map.wpm` by default); changes are written to files on
//! disk.
//!
//! A cell is addressed by column `x` and row `y` from the bottom left, as
//! the file and the terrain count. With the map's `war3map.w3e` beside
//! the file (the same folder or map), cells also have world coordinates.
//! `wpm_image` answers with a picture, drawn as the game's pathing
//! textures draw pathing.

use serde_json::{Map, Value, json};
use wpm_rs::{CELL, FLAGS, Pathing, flag_names};

/// The most cells `wpm_cells` gives at once.
const MAX_CELLS: u64 = 4096;

/// The longest side `wpm_image` draws, in pixels.
const MAX_SIDE: u32 = 4096;

/// Every tool, for `tools/list`.
pub fn tools() -> Value {
    let path = json!({"type": "string", "description": "The pathing map: war3map.wpm on disk, or with `archive` its name inside (war3map.wpm when left out)"});
    let archive =
        json!({"type": "string", "description": "A map (.w3x, .w3m) to read the file from"});
    let names: Vec<String> = FLAGS.iter().map(|(b, n)| format!("0x{b:02X} {n}")).collect();
    let flag = json!({"description": format!("A flag: its bit or its name ({})", names.join(", ")), "anyOf": [{"type": "integer"}, {"type": "string"}]});
    let mut region = Map::new();
    region.insert("x".into(), json!({"type": "integer", "minimum": 0, "description": "The first column, from the left (0)"}));
    region.insert("y".into(), json!({"type": "integer", "minimum": 0, "description": "The first row, from the bottom (0)"}));
    region.insert("width".into(), json!({"type": "integer", "minimum": 1, "description": "Columns"}));
    region.insert("height".into(), json!({"type": "integer", "minimum": 1, "description": "Rows"}));
    let with = |extra: Value| {
        let mut props = region.clone();
        props.insert("path".into(), path.clone());
        props.insert("archive".into(), archive.clone());
        props.extend(extra.as_object().expect("an object").clone());
        Value::Object(props)
    };
    json!([
        {
            "name": "wpm_info",
            "description": "A map's pathing (war3map.wpm): its size in cells (four per terrain cell, 32 world units each), how many cells have each flag (no walking, no flying, no building, blight, no water, boundary ...), and, with war3map.w3e beside it, its world bounds.",
            "inputSchema": {"type": "object", "properties": {"path": path, "archive": archive}},
        },
        {
            "name": "wpm_image",
            "description": "A picture of a map's pathing, a PNG, north up. `show`: \"pathing\" (the default) draws it as the game's pathing textures do, red where units can't walk, green where they can't fly, blue where nothing can be built (white: none of them, black: all allowed); a flag draws that flag alone, white where it's set. A region (x, y from the bottom left, width, height, in cells) crops it; `scale` makes each cell that many pixels (default: so that the picture is at least 512 pixels across, at most 4096).",
            "inputSchema": {"type": "object", "properties": with(json!({
                "show": {"description": "\"pathing\", or a flag (its bit or name) to draw alone", "anyOf": [{"type": "integer"}, {"type": "string"}]},
                "scale": {"type": "integer", "minimum": 1, "maximum": 64, "description": "Pixels per cell"},
            }))},
        },
        {
            "name": "wpm_cells",
            "description": "The cells of a region of a map's pathing (x, y from the bottom left; at most 4096 at once): each one's flags as a number and by name, and its world position (the cell's bottom left corner) when war3map.w3e is beside the file.",
            "inputSchema": {"type": "object", "properties": with(json!({})), "required": ["x", "y"]},
        },
        {
            "name": "wpm_set",
            "description": "Sets and clears flags on every cell of a region of a war3map.wpm on disk: `set` and `clear` are lists of flags (bits or names). A set flag forbids: setting \"no walking\" makes the cells unwalkable.",
            "inputSchema": {"type": "object", "properties": with(json!({
                "path": {"type": "string", "description": "The war3map.wpm on disk"},
                "set": {"type": "array", "items": flag.clone()},
                "clear": {"type": "array", "items": flag},
            })), "required": ["path", "x", "y"]},
        },
    ])
}

/// Runs the tool `name`, if it's one of these; an error is a message for
/// the model.
pub fn call(name: &str, args: &Value) -> Option<Result<Value, String>> {
    Some(match name {
        "wpm_info" => open(args).map(|(m, origin)| info(&m, origin)),
        "wpm_image" => open(args).and_then(|(m, _)| image(&m, args)),
        "wpm_cells" => open(args).and_then(|(m, origin)| cells(&m, origin, args)),
        "wpm_set" => set(args),
        _ => return None,
    })
}

/// The file at `args.path`, or `war3map.wpm` (or `args.path`) inside
/// `args.archive`; and the world position of its bottom left corner, from
/// the `war3map.w3e` beside it.
fn open(args: &Value) -> Result<(Pathing, Option<(f32, f32)>), String> {
    let archive = args.get("archive").and_then(Value::as_str);
    let path = match (args.get("path").and_then(Value::as_str), archive) {
        (Some(p), _) => p,
        (None, Some(_)) => "war3map.wpm",
        (None, None) => return Err("`path` is missing".into()),
    };
    let bytes = crate::source::read_bytes(path, archive)?;
    let map = wpm_rs::parse(&bytes).map_err(|e| format!("{path}: {e}"))?;
    let w3e = match archive {
        Some(_) => match path.rfind(['\\', '/']) {
            Some(i) => format!("{}war3map.w3e", &path[..=i]),
            None => "war3map.w3e".to_string(),
        },
        None => std::path::Path::new(path)
            .with_file_name("war3map.w3e")
            .to_string_lossy()
            .into_owned(),
    };
    let origin = crate::source::read_bytes(&w3e, archive)
        .ok()
        .and_then(|b| w3e_rs::parse(&b).ok())
        .map(|t| (t.offset_x, t.offset_y));
    Ok((map, origin))
}

/// A flag named by its bit or its name.
fn flag(v: &Value) -> Result<u8, String> {
    let found = match v {
        Value::Number(n) => n.as_u64().and_then(|n| FLAGS.iter().find(|(b, _)| u64::from(*b) == n)),
        Value::String(s) => FLAGS.iter().find(|(_, name)| name.eq_ignore_ascii_case(s.trim())),
        _ => None,
    };
    let names: Vec<String> = FLAGS.iter().map(|(b, n)| format!("{b} ({n})")).collect();
    found
        .map(|(b, _)| *b)
        .ok_or(format!("{v} isn't a flag: {}", names.join(", ")))
}

fn info(m: &Pathing, origin: Option<(f32, f32)>) -> Value {
    let total = m.cells.len().max(1) as f64;
    let flags: Vec<Value> = FLAGS
        .iter()
        .map(|(bit, name)| {
            let n = m.cells.iter().filter(|&&c| c & bit != 0).count();
            json!({"bit": bit, "name": name, "cells": n, "percent": (n as f64 * 1000.0 / total).round() / 10.0})
        })
        .collect();
    let mut out = json!({
        "width": m.width,
        "height": m.height,
        "cell_size": CELL,
        "flags": flags,
    });
    if let Some((x, y)) = origin {
        out["bounds"] = json!({"left": x, "bottom": y, "right": x + m.width as f32 * CELL, "top": y + m.height as f32 * CELL});
    }
    out
}

/// The region `args` names (the whole map by default), checked against
/// the grid.
fn region(m: &Pathing, args: &Value, whole: bool) -> Result<(u32, u32, u32, u32), String> {
    let get = |k: &str, default: Option<u64>| {
        args.get(k).and_then(Value::as_u64).or(default).ok_or(format!("`{k}` is missing"))
    };
    let (x, y) = if whole { (get("x", Some(0))?, get("y", Some(0))?) } else { (get("x", None)?, get("y", None)?) };
    let (w, h) = if whole {
        (get("width", Some(m.width as u64 - x.min(m.width as u64)))?, get("height", Some(m.height as u64 - y.min(m.height as u64)))?)
    } else {
        (get("width", Some(1))?, get("height", Some(1))?)
    };
    if w == 0 || h == 0 || x + w > m.width as u64 || y + h > m.height as u64 {
        return Err(format!(
            "columns {x}..{} and rows {y}..{} don't fit the {}x{} grid",
            x + w,
            y + h,
            m.width,
            m.height
        ));
    }
    Ok((x as u32, y as u32, w as u32, h as u32))
}

fn image(m: &Pathing, args: &Value) -> Result<Value, String> {
    let (x0, y0, w, h) = region(m, args, true)?;
    let show = match args.get("show") {
        None => None,
        Some(Value::String(s)) if s.eq_ignore_ascii_case("pathing") => None,
        Some(v) => Some(flag(v)?),
    };
    let side = w.max(h);
    let scale = match args.get("scale").and_then(Value::as_u64) {
        Some(s) => s.clamp(1, 64) as u32,
        None => 512u32.div_ceil(side).clamp(1, (MAX_SIDE / side).max(1)),
    };
    if side * scale > MAX_SIDE {
        return Err(format!("{}x{} pixels is too big: at most {MAX_SIDE} across", w * scale, h * scale));
    }
    let (pw, ph) = (w * scale, h * scale);
    let mut rgba = Vec::with_capacity((pw * ph * 4) as usize);
    // North up: the region's top row first.
    for py in 0..ph {
        let y = y0 + h - 1 - py / scale;
        for px in 0..pw {
            let c = m.cell(x0 + px / scale, y).expect("in the region");
            let on = |bit: u8| if c & bit != 0 { 255 } else { 0 };
            rgba.extend(match show {
                None => [on(wpm_rs::NO_WALK), on(wpm_rs::NO_FLY), on(wpm_rs::NO_BUILD), 255],
                Some(bit) => [on(bit), on(bit), on(bit), 255],
            });
        }
    }
    let png = crate::png::encode(pw, ph, &rgba);
    let what = match show {
        None => "red: no walking, green: no flying, blue: no building (white: none allowed, black: all)".to_string(),
        Some(bit) => format!("white where \"{}\" is set", flag_names(bit).join("")),
    };
    let text = format!(
        "Pathing, columns {x0}..{} and rows {y0}..{} of {}x{} cells, north up, {scale} pixels per cell ({pw}x{ph}): {what}.",
        x0 + w,
        y0 + h,
        m.width,
        m.height
    );
    Ok(json!({"mcp_content": [
        {"type": "text", "text": text},
        {"type": "image", "data": crate::png::base64(&png), "mimeType": "image/png"},
    ]}))
}

fn cells(m: &Pathing, origin: Option<(f32, f32)>, args: &Value) -> Result<Value, String> {
    let (x0, y0, w, h) = region(m, args, false)?;
    if w as u64 * h as u64 > MAX_CELLS {
        return Err(format!("{} cells: at most {MAX_CELLS} at once", w * h));
    }
    let mut out = Vec::new();
    for y in y0..y0 + h {
        for x in x0..x0 + w {
            let c = m.cell(x, y).expect("in the region");
            let mut cell = json!({"x": x, "y": y, "flags": c, "names": flag_names(c)});
            if let Some((ox, oy)) = origin {
                cell["world_x"] = json!(ox + x as f32 * CELL);
                cell["world_y"] = json!(oy + y as f32 * CELL);
            }
            out.push(cell);
        }
    }
    Ok(json!({"count": out.len(), "cells": out}))
}

fn set(args: &Value) -> Result<Value, String> {
    let path = args.get("path").and_then(Value::as_str).ok_or("`path` is missing")?;
    let bytes = crate::source::read_bytes(path, None)?;
    let mut m = wpm_rs::parse(&bytes).map_err(|e| format!("{path}: {e}"))?;
    let (x0, y0, w, h) = region(&m, args, false)?;
    let bits = |key: &str| -> Result<u8, String> {
        match args.get(key) {
            None => Ok(0),
            Some(Value::Array(list)) => list.iter().try_fold(0, |acc, v| Ok(acc | flag(v)?)),
            Some(_) => Err(format!("`{key}` is a list of flags")),
        }
    };
    let (on, off) = (bits("set")?, bits("clear")?);
    if on & off != 0 {
        return Err(format!("{} both set and cleared", flag_names(on & off).join(", ")));
    }
    let mut changed = 0;
    for y in y0..y0 + h {
        for x in x0..x0 + w {
            let i = (y * m.width + x) as usize;
            let next = (m.cells[i] | on) & !off;
            if next != m.cells[i] {
                m.cells[i] = next;
                changed += 1;
            }
        }
    }
    if changed > 0 {
        crate::source::write(path, m.to_bytes().map_err(|e| format!("{path}: {e}"))?)?;
    }
    Ok(json!({"cells": w * h, "changed": changed}))
}

#[cfg(test)]
#[path = "wpm.test.rs"]
mod tests;
