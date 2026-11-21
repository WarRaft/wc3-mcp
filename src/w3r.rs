//! The region tools, on `w3r-rs`: `war3map.w3r`, the regions a map's
//! World Editor places (`gg_rct_*` in its script). A file is read from
//! disk (`path`) or from inside a map (`path` inside `archive`,
//! `war3map.w3r` by default); changes are written to files on disk.
//!
//! A region is named by its name, or by its `index`: the number World
//! Editor gave it, which stays when others are removed.

use serde_json::{Map, Value, json};
use w3r_rs::{Rawcode, Region, Regions};

/// Every tool, for `tools/list`.
pub fn tools() -> Value {
    let path = json!({"type": "string", "description": "The region file: war3map.w3r on disk, or with `archive` its name inside (war3map.w3r when left out)"});
    let disk = json!({"type": "string", "description": "The war3map.w3r on disk"});
    let archive =
        json!({"type": "string", "description": "A map (.w3x, .w3m) to read the file from"});
    let region = json!({"description": "The region: its name, or its `index` (World Editor's number for it)", "anyOf": [{"type": "integer"}, {"type": "string"}]});
    let byte = json!({"type": "integer", "minimum": 0, "maximum": 255});
    let values = json!({
        "type": "object",
        "additionalProperties": false,
        "description": "Fields to change",
        "properties": {
            "name": {"type": "string", "description": "The script's variable is gg_rct_ and the name, spaces as _; names are unique"},
            "index": {"type": "integer", "description": "World Editor's number for it; unique"},
            "left": {"type": "number"},
            "bottom": {"type": "number"},
            "right": {"type": "number"},
            "top": {"type": "number"},
            "weather": {"type": "string", "description": "A weather effect's id (RAhr, SNls ...), or \"\" for none"},
            "sound": {"type": "string", "description": "An ambient sound variable of the map (gg_snd_...), or \"\""},
            "red": byte, "green": byte, "blue": byte, "alpha": byte,
        },
    });
    json!([
        {
            "name": "w3r_list",
            "description": "A map's regions (war3map.w3r): name, the script's variable (gg_rct_...), index, the rectangle (left, bottom, right, top in world coordinates), weather, ambient sound, color.",
            "inputSchema": {"type": "object", "properties": {"path": path, "archive": archive}},
        },
        {
            "name": "w3r_set",
            "description": "Changes a region of a war3map.w3r on disk: its rectangle, name, weather, sound, color. Only the fields given change.",
            "inputSchema": {"type": "object", "properties": {"path": disk, "region": region, "values": values}, "required": ["path", "region", "values"]},
        },
        {
            "name": "w3r_new",
            "description": "Adds a region to a war3map.w3r on disk (a new file when there's none). Left out: the next index, the name `Region NNN` after it, no weather, no sound, white. The rectangle is needed: left, bottom, right, top.",
            "inputSchema": {"type": "object", "properties": {"path": disk, "values": values}, "required": ["path", "values"]},
        },
        {
            "name": "w3r_delete",
            "description": "Removes a region from a war3map.w3r on disk. The script may still use its gg_rct_ variable.",
            "inputSchema": {"type": "object", "properties": {"path": disk, "region": region}, "required": ["path", "region"]},
        },
    ])
}

/// Runs the tool `name`, if it's one of these; an error is a message for
/// the model.
pub fn call(name: &str, args: &Value) -> Option<Result<Value, String>> {
    Some(match name {
        "w3r_list" => open(args).map(|f| list(&f)),
        "w3r_set" => change(args, false, set),
        "w3r_new" => change(args, true, add),
        "w3r_delete" => change(args, false, delete),
        _ => return None,
    })
}

/// The file at `args.path`, or `war3map.w3r` (or `args.path`) inside
/// `args.archive`.
fn open(args: &Value) -> Result<Regions, String> {
    let archive = args.get("archive").and_then(Value::as_str);
    let path = match (args.get("path").and_then(Value::as_str), archive) {
        (Some(p), _) => p,
        (None, Some(_)) => "war3map.w3r",
        (None, None) => return Err("`path` is missing".into()),
    };
    let bytes = crate::source::read_bytes(path, archive)?;
    w3r_rs::parse(&bytes).map_err(|e| format!("{path}: {e}"))
}

/// `value` as the shortest decimal that reads back as it.
fn number(value: f32) -> Value {
    value
        .to_string()
        .parse::<f64>()
        .ok()
        .and_then(serde_json::Number::from_f64)
        .map_or(Value::Null, Value::Number)
}

fn region_json(r: &Region) -> Value {
    json!({
        "name": r.name,
        "variable": r.variable(),
        "index": r.index,
        "left": number(r.left),
        "bottom": number(r.bottom),
        "right": number(r.right),
        "top": number(r.top),
        "weather": if r.weather.is_none() { String::new() } else { r.weather.to_string() },
        "sound": r.sound,
        "red": r.red, "green": r.green, "blue": r.blue, "alpha": r.alpha,
    })
}

fn list(file: &Regions) -> Value {
    let regions: Vec<Value> = file.regions.iter().map(region_json).collect();
    json!({"count": regions.len(), "regions": regions})
}

/// Where `args.region` is in the list: by name, or by its index.
fn find(file: &Regions, args: &Value) -> Result<usize, String> {
    match args.get("region") {
        Some(Value::Number(n)) => {
            let n = n.as_i64().ok_or("`region` is an index or a name")?;
            file.regions
                .iter()
                .position(|r| i64::from(r.index) == n)
                .ok_or(format!("no region with index {n}"))
        }
        Some(Value::String(name)) => file
            .regions
            .iter()
            .position(|r| &r.name == name)
            .ok_or(format!("no region named {name:?}")),
        _ => Err("`region` is an index or a name".into()),
    }
}

/// Sets `args.values` on region `at` of `file` (a new one past the end),
/// then checks it against the others.
fn apply(file: &Regions, region: &mut Region, at: usize, values: &Map<String, Value>) -> Result<(), String> {
    for (key, value) in values {
        match key.as_str() {
            "name" => region.name = value.as_str().ok_or("`name` is a string")?.to_string(),
            "sound" => region.sound = value.as_str().ok_or("`sound` is a string")?.to_string(),
            "index" => {
                region.index = value
                    .as_i64()
                    .and_then(|n| i32::try_from(n).ok())
                    .ok_or("`index` is a whole number")?
            }
            "weather" => {
                let s = value.as_str().ok_or("`weather` is an id or \"\"")?;
                region.weather = if s.is_empty() {
                    Rawcode::NONE
                } else {
                    Rawcode::parse(s).ok_or(format!("`weather`: {s:?} isn't an id (four characters)"))?
                };
            }
            "red" | "green" | "blue" | "alpha" => {
                let n = value
                    .as_u64()
                    .and_then(|n| u8::try_from(n).ok())
                    .ok_or(format!("`{key}` is a number from 0 to 255"))?;
                match key.as_str() {
                    "red" => region.red = n,
                    "green" => region.green = n,
                    "blue" => region.blue = n,
                    _ => region.alpha = n,
                }
            }
            _ => {
                let n = value.as_f64().ok_or(format!("`{key}` is a number"))? as f32;
                *region.bound_mut(key).ok_or(format!("no field `{key}`"))? = n;
            }
        }
    }
    if region.left > region.right || region.bottom > region.top {
        return Err(format!(
            "left {} is past right {}, or bottom {} past top {}",
            region.left, region.right, region.bottom, region.top
        ));
    }
    if region.name.is_empty() {
        return Err("a region has a name".into());
    }
    for (i, other) in file.regions.iter().enumerate().filter(|(i, _)| *i != at) {
        if other.name == region.name {
            return Err(format!("region {i} is named {:?} too: each is a variable, gg_rct_...", other.name));
        }
        if other.index == region.index {
            return Err(format!("region {:?} has index {} too", other.name, other.index));
        }
    }
    Ok(())
}

fn values(args: &Value) -> Result<&Map<String, Value>, String> {
    args.get("values")
        .and_then(Value::as_object)
        .ok_or("`values` is an object".into())
}

fn set(file: &mut Regions, args: &Value) -> Result<Value, String> {
    let i = find(file, args)?;
    let mut region = file.regions[i].clone();
    apply(file, &mut region, i, values(args)?)?;
    let before = region_json(&file.regions[i]);
    file.regions[i] = region;
    Ok(json!({"before": before, "after": region_json(&file.regions[i])}))
}

fn add(file: &mut Regions, args: &Value) -> Result<Value, String> {
    let values = values(args)?;
    let missing: Vec<&str> = ["left", "bottom", "right", "top"]
        .into_iter()
        .filter(|k| !values.contains_key(*k))
        .collect();
    if !missing.is_empty() {
        return Err(format!("a new region needs {}", missing.join(", ")));
    }
    let index = file.next_index();
    let mut region = Region::new(format!("Region {index:03}"), 0.0, 0.0, 0.0, 0.0, index);
    apply(file, &mut region, file.regions.len(), values)?;
    file.regions.push(region);
    Ok(json!({"added": region_json(file.regions.last().expect("just added"))}))
}

fn delete(file: &mut Regions, args: &Value) -> Result<Value, String> {
    let i = find(file, args)?;
    let removed = file.regions.remove(i);
    Ok(json!({"removed": region_json(&removed)}))
}

/// Reads the file on disk at `args.path`, changes it with `change`, writes
/// it back. With `create` (`w3r_new`), a missing file starts empty.
fn change(
    args: &Value,
    create: bool,
    change: fn(&mut Regions, &Value) -> Result<Value, String>,
) -> Result<Value, String> {
    let path = args
        .get("path")
        .and_then(Value::as_str)
        .ok_or("`path` is missing")?;
    let new = create && !std::path::Path::new(path).exists();
    let mut file = if new {
        Regions::new()
    } else {
        let bytes = crate::source::read_bytes(path, None)?;
        w3r_rs::parse(&bytes).map_err(|e| format!("{path}: {e}"))?
    };
    let mut out = change(&mut file, args)?;
    let bytes = file.to_bytes().map_err(|e| format!("{path}: {e}"))?;
    if new {
        std::fs::write(path, bytes).map_err(|e| format!("{path}: {e}"))?;
    } else {
        crate::source::write(path, bytes)?;
    }
    out["count"] = json!(file.regions.len());
    Ok(out)
}

#[cfg(test)]
#[path = "w3r.test.rs"]
mod tests;
