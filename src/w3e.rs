//! The terrain tools, on `w3e-rs`: `war3map.w3e`, a map's tilesets and,
//! for every point of its grid, height, water, texture, cliffs and flags.
//! A file is read from disk (`path`) or from inside a map (`path` inside
//! `archive`, `war3map.w3e` by default); changes are written to files on
//! disk.
//!
//! Points are addressed by column and row from the bottom left, (0, 0);
//! a point's world position is the offset plus 128 per step.

use serde_json::{Map, Value, json};
use w3e_rs::{Point, Terrain, Version, ZERO_HEIGHT};

/// The most points `w3e_points` gives at once.
const MAX_POINTS: u64 = 4096;

/// Every tool, for `tools/list`.
pub fn tools() -> Value {
    let path = json!({"type": "string", "description": "The terrain file: war3map.w3e on disk, or with `archive` its name inside (war3map.w3e when left out)"});
    let archive =
        json!({"type": "string", "description": "A map (.w3x, .w3m) to read the file from"});
    let region = json!({
        "x": {"type": "integer", "minimum": 0, "description": "The first column, from the left (0)"},
        "y": {"type": "integer", "minimum": 0, "description": "The first row, from the bottom (0)"},
        "width": {"type": "integer", "minimum": 1, "description": "Columns (default 1)"},
        "height": {"type": "integer", "minimum": 1, "description": "Rows (default 1)"},
    });
    let mut points_props = region.as_object().unwrap().clone();
    points_props.insert("path".into(), path.clone());
    points_props.insert("archive".into(), archive.clone());
    let mut set_props = region.as_object().unwrap().clone();
    set_props.insert(
        "path".into(),
        json!({"type": "string", "description": "The war3map.w3e on disk"}),
    );
    set_props.insert(
        "values".into(),
        json!({
            "type": "object",
            "additionalProperties": false,
            "description": "Fields to set on every point of the region",
            "properties": {
                "height": {"type": "integer", "minimum": 0, "maximum": 65535, "description": "Ground height as stored: 8192 is zero"},
                "water_height": {"type": "integer", "minimum": 0, "maximum": 16383},
                "edge": {"type": "boolean", "description": "Outside the camera bounds"},
                "texture": {"description": "Ground texture: an index into the ground list, or its rawcode (Ldrt), which must be in the list", "anyOf": [{"type": "integer", "minimum": 0}, {"type": "string"}]},
                "ramp": {"type": "boolean"},
                "blight": {"type": "boolean"},
                "water": {"type": "boolean", "description": "Water shows here"},
                "boundary": {"type": "boolean", "description": "Unwalkable, unbuildable boundary"},
                "ground_variation": {"type": "integer", "minimum": 0, "maximum": 31},
                "cliff_variation": {"type": "integer", "minimum": 0, "maximum": 7},
                "layer": {"type": "integer", "minimum": 0, "maximum": 15, "description": "Cliff level"},
                "cliff_texture": {"type": "integer", "minimum": 0, "maximum": 15, "description": "Index into the cliff list; 15 is none"},
            },
        }),
    );
    json!([
        {
            "name": "w3e_info",
            "description": "A map's terrain (war3map.w3e): version, base tileset, the ground and cliff textures with how many points use each, the grid's size, and its world bounds. With `game`, the tileset's and the textures' names from the game's files.",
            "inputSchema": {"type": "object", "properties": {
                "path": path,
                "archive": archive,
                "game": crate::source::game_schema(),
                "roots": crate::source::roots_schema(),
                "project": crate::source::project_schema(),
            }},
        },
        {
            "name": "w3e_image",
            "description": "A picture of a map's terrain from above (PNG, north up), as the World Editor's view draws it: each point in its ground texture's colour (with blight, and the cliffs' ground beside cliffs), lit by the slope, under its water by depth, darker on boundaries and past the camera bounds. Textures and the tileset's tables come through `game` and `roots` (by default the project's); a missing one is said. `show` marks the regions, the units, the bounds over it.",
            "inputSchema": {"type": "object", "properties": {
                "path": path,
                "archive": archive,
                "scale": {"type": "integer", "minimum": 1, "maximum": 16, "description": "Pixels per point (default: so that the picture is at least 512 pixels across, at most 4096)"},
                "show": {"type": "array", "items": {"type": "string", "enum": ["regions", "units", "bounds"]}, "description": "What to mark over it, from the map's files beside the terrain: the regions' outlines (war3map.w3r) in their colours, units and items (war3mapUnits.doo) a dot each in the owner's colour, the camera bounds (blue) and the playable area (yellow) (war3map.w3i)"},
                "game": crate::source::game_schema(),
                "roots": crate::source::roots_schema(),
                "project": crate::source::project_schema(),
            }},
        },
        {
            "name": "w3e_points",
            "description": "The terrain's points in a region (column x, row y from the bottom left; at most 4096 at once): world position, height (8192 is zero), water, ground texture, flags (ramp, blight, water, boundary, edge), variations, cliff level and texture.",
            "inputSchema": {"type": "object", "properties": points_props, "required": ["x", "y"]},
        },
        {
            "name": "w3e_set",
            "description": "Sets fields on every point of a region of a war3map.w3e on disk: height, water, texture, flags, cliff level ... Only the fields given change.",
            "inputSchema": {"type": "object", "properties": set_props, "required": ["path", "x", "y", "values"]},
        },
    ])
}

/// Runs the tool `name`, if it's one of these; an error is a message for
/// the model.
pub fn call(name: &str, args: &Value) -> Option<Result<Value, String>> {
    Some(match name {
        "w3e_info" => open(args).map(|t| {
            let names = crate::source::names(args);
            let mut out = info(&t);
            out["tileset_name"] = json!(names.tileset(t.tileset as char));
            for (key, table) in [("ground", &names.ground), ("cliffs", &names.cliffs)] {
                for entry in out[key].as_array_mut().into_iter().flatten() {
                    let code = entry["code"].as_str().unwrap_or_default().to_string();
                    entry["name"] = json!(table.get(&code));
                }
            }
            out
        }),
        "w3e_points" => open(args).and_then(|t| points(&t, args)),
        "w3e_image" => open(args).and_then(|t| image(&t, args)),
        "w3e_set" => set(args),
        _ => return None,
    })
}

/// The file at `args.path`, or `war3map.w3e` (or `args.path`) inside
/// `args.archive`.
fn open(args: &Value) -> Result<Terrain, String> {
    let archive = args.get("archive").and_then(Value::as_str);
    let path = match (args.get("path").and_then(Value::as_str), archive) {
        (Some(p), _) => p,
        (None, Some(_)) => "war3map.w3e",
        (None, None) => return Err("`path` is missing".into()),
    };
    let bytes = crate::source::read_bytes(path, archive)?;
    w3e_rs::parse(&bytes).map_err(|e| format!("{path}: {e}"))
}

/// The longest side `w3e_image` draws, in pixels.
const MAX_SIDE: u32 = 4096;

fn image(t: &Terrain, args: &Value) -> Result<Value, String> {
    let lookup = crate::source::lookup(args);
    if lookup.is_empty() {
        return Err(format!("No textures to draw it with: {}", crate::source::layers_named(args)));
    }
    let side = t.width.max(t.height).max(1);
    let scale = match args.get("scale").and_then(Value::as_u64) {
        Some(s) => s.clamp(1, 16) as u32,
        None => 512u32.div_ceil(side).clamp(1, (MAX_SIDE / side).max(1)),
    };
    if side * scale > MAX_SIDE {
        return Err(format!("{} pixels across is too big: at most {MAX_SIDE}", side * scale));
    }
    let (game, errors) = lookup.open();
    let mut p = wc3_editor::picture::picture(&game, t, scale);
    let shown = |what: &str| args.get("show").and_then(Value::as_array).is_some_and(|l| l.iter().any(|v| v.as_str() == Some(what)));
    let marks = wc3_editor::picture::Marks { regions: shown("regions"), units: shown("units"), bounds: shown("bounds") };
    if marks != wc3_editor::picture::Marks::default() {
        wc3_editor::picture::mark(&game, t, &mut p, scale, marks);
    }
    let png = crate::png::encode(p.width, p.height, &p.rgba);
    let mut text = format!(
        "Terrain, {}x{} points, north up, {scale} pixels per point ({}x{}).",
        t.width, t.height, p.width, p.height
    );
    for note in errors.iter().chain(&p.notes) {
        text += &format!("\n{note}");
    }
    Ok(json!({"mcp_content": [
        {"type": "text", "text": text},
        {"type": "image", "data": crate::png::base64(&png), "mimeType": "image/png"},
    ]}))
}

fn info(t: &Terrain) -> Value {
    let ground = t.texture_usage();
    let cliffs = t.cliff_usage();
    let list = |codes: &[w3e_rs::Rawcode], used: &[usize]| -> Vec<Value> {
        codes
            .iter()
            .enumerate()
            .map(|(i, c)| json!({"index": i, "code": c.to_string(), "points": used.get(i).copied().unwrap_or(0)}))
            .collect()
    };
    let (right, top) = t.world(t.width.saturating_sub(1), t.height.saturating_sub(1));
    json!({
        "version": t.version,
        "tileset": (t.tileset as char).to_string(),
        "custom_tilesets": t.custom_tilesets,
        "ground": list(&t.ground, &ground),
        "cliffs": list(&t.cliffs, &cliffs),
        "width": t.width,
        "height": t.height,
        "bounds": {"left": t.offset_x, "bottom": t.offset_y, "right": right, "top": top},
    })
}

/// The region `args` names, checked against the grid.
fn region(t: &Terrain, args: &Value) -> Result<(u32, u32, u32, u32), String> {
    let get = |k: &str, default: Option<u64>| {
        args.get(k)
            .and_then(Value::as_u64)
            .or(default)
            .ok_or(format!("`{k}` is missing"))
    };
    let (x, y) = (get("x", None)?, get("y", None)?);
    let (w, h) = (
        get("width", Some(1))?.max(1),
        get("height", Some(1))?.max(1),
    );
    if x + w > t.width as u64 || y + h > t.height as u64 {
        return Err(format!(
            "columns {x}..{} and rows {y}..{} don't fit the {}x{} grid",
            x + w,
            y + h,
            t.width,
            t.height
        ));
    }
    Ok((x as u32, y as u32, w as u32, h as u32))
}

fn point_json(t: &Terrain, v: Version, x: u32, y: u32, p: &Point) -> Value {
    let (wx, wy) = t.world(x, y);
    let texture = p.texture(v);
    json!({
        "x": x, "y": y, "world_x": wx, "world_y": wy,
        "height": p.height,
        "height_from_zero": p.height as i32 - ZERO_HEIGHT as i32,
        "water_height": p.water_height(),
        "edge": p.edge(),
        "texture": texture,
        "texture_code": t.ground.get(texture as usize).map(ToString::to_string),
        "ramp": p.ramp(v),
        "blight": p.blight(v),
        "water": p.has_water(v),
        "boundary": p.boundary(v),
        "ground_variation": p.ground_variation(),
        "cliff_variation": p.cliff_variation(),
        "layer": p.layer_height(),
        "cliff_texture": p.cliff_texture(),
    })
}

fn points(t: &Terrain, args: &Value) -> Result<Value, String> {
    let (x0, y0, w, h) = region(t, args)?;
    if w as u64 * h as u64 > MAX_POINTS {
        return Err(format!(
            "{} points: ask for {MAX_POINTS} at most",
            w as u64 * h as u64
        ));
    }
    let v = t.flags_version();
    let mut out = Vec::new();
    for y in y0..y0 + h {
        for x in x0..x0 + w {
            out.push(point_json(t, v, x, y, t.point(x, y).expect("in the grid")));
        }
    }
    Ok(json!({"points": out}))
}

/// Applies `values` to `p`; checked first by [`check`].
fn apply(p: &mut Point, v: Version, values: &Map<String, Value>, texture: Option<u16>) {
    for (key, value) in values {
        let n = value.as_u64().unwrap_or(0);
        let on = value.as_bool().unwrap_or(false);
        match key.as_str() {
            "height" => p.height = n as u16,
            "water_height" => p.set_water_height(n as u16),
            "edge" => p.set_edge(on),
            "texture" => p.set_texture(v, texture.expect("checked")),
            "ramp" => p.set_ramp(v, on),
            "blight" => p.set_blight(v, on),
            "water" => p.set_has_water(v, on),
            "boundary" => p.set_boundary(v, on),
            "ground_variation" => p.set_ground_variation(n as u8),
            "cliff_variation" => p.set_cliff_variation(n as u8),
            "layer" => p.set_layer_height(n as u8),
            "cliff_texture" => p.set_cliff_texture(n as u8),
            _ => {}
        }
    }
}

/// Checks `values` against the file; the ground texture's index, when
/// it's given.
fn check(t: &Terrain, values: &Map<String, Value>) -> Result<Option<u16>, String> {
    let limits: [(&str, u64); 6] = [
        ("height", 65535),
        ("water_height", 16383),
        ("ground_variation", 31),
        ("cliff_variation", 7),
        ("layer", 15),
        ("cliff_texture", 15),
    ];
    let texture_max = if t.version >= 12 { 63 } else { 15 };
    let mut texture = None;
    for (key, value) in values {
        match key.as_str() {
            "edge" | "ramp" | "blight" | "water" | "boundary" => {
                value.as_bool().ok_or(format!("`{key}` is true or false"))?;
            }
            "texture" => {
                let index = match value {
                    Value::String(code) => t
                        .ground
                        .iter()
                        .position(|c| c.to_string() == *code)
                        .ok_or(format!(
                            "{code} isn't one of the ground textures: {}",
                            codes(&t.ground)
                        ))? as u64,
                    _ => value.as_u64().ok_or("`texture` is an index or a rawcode")?,
                };
                if index >= t.ground.len() as u64 || index > texture_max {
                    return Err(format!(
                        "texture {index}: the ground list has {} ({})",
                        t.ground.len(),
                        codes(&t.ground)
                    ));
                }
                texture = Some(index as u16);
            }
            _ => {
                let max = limits
                    .iter()
                    .find(|(k, _)| k == key)
                    .map(|(_, m)| *m)
                    .ok_or(format!("no field `{key}`"))?;
                let n = value.as_u64().ok_or(format!("`{key}` is a whole number"))?;
                if n > max {
                    return Err(format!("`{key}` is at most {max}"));
                }
                if key == "cliff_texture" && n != 15 && n >= t.cliffs.len() as u64 {
                    return Err(format!(
                        "cliff texture {n}: the cliff list has {} ({}); 15 is none",
                        t.cliffs.len(),
                        codes(&t.cliffs)
                    ));
                }
            }
        }
    }
    Ok(texture)
}

fn codes(list: &[w3e_rs::Rawcode]) -> String {
    list.iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(", ")
}

fn set(args: &Value) -> Result<Value, String> {
    let path = args
        .get("path")
        .and_then(Value::as_str)
        .ok_or("`path` is missing")?;
    let values = args
        .get("values")
        .and_then(Value::as_object)
        .ok_or("`values` is an object")?;
    let bytes = crate::source::read_bytes(path, None)?;
    let mut t = w3e_rs::parse(&bytes).map_err(|e| format!("{path}: {e}"))?;
    let (x0, y0, w, h) = region(&t, args)?;
    let texture = check(&t, values)?;
    let v = t.flags_version();
    let mut changed = 0;
    for y in y0..y0 + h {
        for x in x0..x0 + w {
            let p = t.point_mut(x, y).expect("in the grid");
            let before = *p;
            apply(p, v, values, texture);
            changed += usize::from(*p != before);
        }
    }
    if changed > 0 {
        let out = t.to_bytes().map_err(|e| format!("{path}: {e}"))?;
        crate::source::write(path, out)?;
    }
    Ok(json!({"points": w as u64 * h as u64, "changed": changed}))
}

#[cfg(test)]
#[path = "w3e.test.rs"]
mod tests;
