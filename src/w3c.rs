//! The camera tools, on `w3c-rs`: `war3map.w3c`, the cameras a map's
//! World Editor places (`gg_cam_*` in its script). A file is read from
//! disk (`path`) or from inside a map (`path` inside `archive`,
//! `war3map.w3c` by default); changes are written to files on disk.
//!
//! The file doesn't say its layout: since 1.31 each camera has a local
//! pitch, yaw and roll too. The map's `war3map.w3i` says which game it's
//! for (its first four bytes, the format version: 28 and later is 1.31
//! and later), read next to the file (the same folder, or the same map);
//! without one the layout is 1.27a's. `w3i_version` sets it by hand.

use serde_json::{Value, json};
use w3c_rs::{Camera, Cameras, Layout};

/// The number fields, in file order, for the tools' schemas.
const FIELDS: [&str; 16] = [
    "target_x",
    "target_y",
    "z_offset",
    "rotation",
    "angle_of_attack",
    "distance",
    "roll",
    "field_of_view",
    "far_z",
    "near_z",
    "local_pitch",
    "local_yaw",
    "local_roll",
    "depth_of_field",
    "depth_of_field_scale",
    "absolute_z",
];

/// Every tool, for `tools/list`.
pub fn tools() -> Value {
    let path = json!({"type": "string", "description": "The camera file: war3map.w3c on disk, or with `archive` its name inside (war3map.w3c when left out)"});
    let disk = json!({"type": "string", "description": "The war3map.w3c on disk"});
    let archive =
        json!({"type": "string", "description": "A map (.w3x, .w3m) to read the file from"});
    let w3i = json!({"type": "integer", "minimum": 0, "description": "The map's war3map.w3i format version, when there's no war3map.w3i next to the file: 18 Reign of Chaos, 25 The Frozen Throne, 28 1.31, 31 1.32, 39 2.0 (a new file then version 3). Without either, 1.27a's layout"});
    let camera = json!({"description": "The camera: its index (from 0) or its name", "anyOf": [{"type": "integer", "minimum": 0}, {"type": "string"}]});
    let mut values = serde_json::Map::new();
    values.insert("name".into(), json!({"type": "string"}));
    for f in FIELDS {
        values.insert(f.into(), json!({"type": "number"}));
    }
    values.insert("free".into(), json!({"type": "integer", "minimum": 0}));
    let values = json!({"type": "object", "properties": values, "additionalProperties": false, "description": "Fields to change: name, target_x / target_y / z_offset, rotation, angle_of_attack, field_of_view (degrees), distance, roll, far_z, near_z; local_pitch / local_yaw / local_roll since 1.31; depth_of_field, depth_of_field_scale, absolute_z, free (1 a free camera) in version 3 (Reforged 2.0)"});
    json!([
        {
            "name": "w3c_list",
            "description": "A map's cameras (war3map.w3c): index, name, the script's variable (gg_cam_...), and every field. The layout (1.31 added local_pitch, local_yaw, local_roll) comes from the war3map.w3i next to the file, else 1.27a's.",
            "inputSchema": {"type": "object", "properties": {"path": path, "archive": archive, "w3i_version": w3i}},
        },
        {
            "name": "w3c_set",
            "description": "Changes a camera of a war3map.w3c on disk: some of its fields, its name.",
            "inputSchema": {
                "type": "object",
                "properties": {"path": disk, "camera": camera, "values": values, "w3i_version": w3i},
                "required": ["path", "camera", "values"],
            },
        },
        {
            "name": "w3c_new",
            "description": "Adds a camera to a war3map.w3c on disk (a new file when there's none), with World Editor's defaults for what `values` leaves out (rotation 90, angle of attack 304, distance 1650, field of view 70, far z 5000, near z 100).",
            "inputSchema": {
                "type": "object",
                "properties": {"path": disk, "values": values, "w3i_version": w3i},
                "required": ["path", "values"],
            },
        },
        {
            "name": "w3c_delete",
            "description": "Removes a camera from a war3map.w3c on disk. The script may still use its gg_cam_ variable.",
            "inputSchema": {
                "type": "object",
                "properties": {"path": disk, "camera": camera, "w3i_version": w3i},
                "required": ["path", "camera"],
            },
        },
    ])
}

/// Runs the tool `name`, if it's one of these; an error is a message for
/// the model.
pub fn call(name: &str, args: &Value) -> Option<Result<Value, String>> {
    Some(match name {
        "w3c_list" => open(args).map(|(file, from)| list(&file, &from)),
        "w3c_set" => change(args, false, set),
        "w3c_new" => change(args, true, add),
        "w3c_delete" => change(args, false, delete),
        _ => return None,
    })
}

/// The layout for the file at `path` (inside `archive`): from
/// `args.w3i_version`, else the `war3map.w3i` next to it, else 1.27a's;
/// with where it came from, and that w3i version.
fn layout(args: &Value, path: &str, archive: Option<&str>) -> (Option<Layout>, String, Option<u32>) {
    if let Some(v) = args.get("w3i_version").and_then(Value::as_u64) {
        return (
            Some(Layout::from_w3i_version(v as u32)),
            format!("w3i_version {v}"),
            Some(v as u32),
        );
    }
    let w3i = match archive {
        Some(_) => match path.rfind(['\\', '/']) {
            Some(i) => format!("{}war3map.w3i", &path[..=i]),
            None => "war3map.w3i".to_string(),
        },
        None => std::path::Path::new(path)
            .with_file_name("war3map.w3i")
            .to_string_lossy()
            .into_owned(),
    };
    match crate::source::read_bytes(&w3i, archive).ok().and_then(|b| {
        Some((
            Layout::from_w3i(&b)?,
            u32::from_le_bytes(b[..4].try_into().ok()?),
        ))
    }) {
        Some((layout, v)) => (Some(layout), format!("war3map.w3i, version {v}"), Some(v)),
        None => (None, "no war3map.w3i: 1.27a".to_string(), None),
    }
}

/// Reads the file with its layout; and where the layout came from.
fn read(args: &Value, path: &str, archive: Option<&str>) -> Result<(Cameras, String), String> {
    let bytes = crate::source::read_bytes(path, archive)?;
    let (layout, from, _) = layout(args, path, archive);
    let layout = layout.unwrap_or_default();
    let file = w3c_rs::parse(&bytes, layout).map_err(|e| {
        format!(
            "{path}: {e} (layout for {}, from {from})",
            layout.describe()
        )
    })?;
    Ok((file, from))
}

/// The file at `args.path`, or `war3map.w3c` (or `args.path`) inside
/// `args.archive`.
fn open(args: &Value) -> Result<(Cameras, String), String> {
    let archive = args.get("archive").and_then(Value::as_str);
    let path = match (args.get("path").and_then(Value::as_str), archive) {
        (Some(p), _) => p,
        (None, Some(_)) => "war3map.w3c",
        (None, None) => return Err("`path` is missing".into()),
    };
    read(args, path, archive)
}

/// `gg_cam_` and the name with spaces as `_`, as World Editor names it in
/// the script.
fn variable(name: &str) -> String {
    format!("gg_cam_{}", name.replace(' ', "_"))
}

/// `value` as the shortest decimal that reads back as it (`-2728.98`, not
/// the `-2728.97998046875` widening to f64 gives).
fn number(value: f32) -> Value {
    value
        .to_string()
        .parse::<f64>()
        .ok()
        .and_then(serde_json::Number::from_f64)
        .map_or(Value::Null, Value::Number)
}

fn camera_json(index: usize, c: &Camera, layout: Layout, version: u32) -> Value {
    let mut out = serde_json::Map::new();
    out.insert("index".into(), json!(index));
    out.insert("name".into(), json!(c.name));
    out.insert("variable".into(), json!(variable(&c.name)));
    let fields = c.fields();
    for i in w3c_rs::field_indices(layout, version) {
        let (name, value) = fields[i];
        out.insert(name.into(), number(value));
    }
    if version >= 3 {
        out.insert("free".into(), json!(c.free));
    }
    Value::Object(out)
}

fn list(file: &Cameras, from: &str) -> Value {
    let cameras: Vec<Value> = file
        .cameras
        .iter()
        .enumerate()
        .map(|(i, c)| camera_json(i, c, file.layout, file.version))
        .collect();
    json!({"version": file.version, "layout": file.layout.describe(), "layout_from": from, "count": cameras.len(), "cameras": cameras})
}

/// The index of `args.camera`: a number, or a name.
fn find(file: &Cameras, args: &Value) -> Result<usize, String> {
    match args.get("camera") {
        Some(Value::Number(n)) => n
            .as_u64()
            .map(|n| n as usize)
            .filter(|&n| n < file.cameras.len())
            .ok_or(format!("no camera {n}: there are {}", file.cameras.len())),
        Some(Value::String(name)) => file
            .cameras
            .iter()
            .position(|c| &c.name == name)
            .ok_or(format!("no camera named {name:?}")),
        _ => Err("`camera` is an index or a name".into()),
    }
}

/// Sets `args.values` on `camera`.
fn apply(camera: &mut Camera, args: &Value, layout: Layout, version: u32) -> Result<(), String> {
    let values = args
        .get("values")
        .and_then(Value::as_object)
        .ok_or("`values` is an object")?;
    for (key, value) in values {
        if key == "name" {
            camera.name = value.as_str().ok_or("`name` is a string")?.to_string();
            continue;
        }
        let reforged = matches!(key.as_str(), "free" | "depth_of_field" | "depth_of_field_scale" | "absolute_z");
        if reforged && version < 3 {
            return Err(format!("`{key}` is in version 3 (Reforged 2.0) and later; this file is {version}"));
        }
        if key == "free" {
            camera.free = value.as_u64().ok_or("`free` is a whole number")? as u32;
            continue;
        }
        let number = value.as_f64().ok_or(format!("`{key}` is a number"))? as f32;
        if key.starts_with("local_") && layout != Layout::Local {
            return Err(format!(
                "`{key}` is only in 1.31 and later; this file is for {}",
                layout.describe()
            ));
        }
        *camera.field_mut(key).ok_or(format!("no field `{key}`"))? = number;
    }
    Ok(())
}

fn set(file: &mut Cameras, args: &Value) -> Result<Value, String> {
    let i = find(file, args)?;
    let before = camera_json(i, &file.cameras[i], file.layout, file.version);
    apply(&mut file.cameras[i], args, file.layout, file.version)?;
    Ok(json!({"before": before, "after": camera_json(i, &file.cameras[i], file.layout, file.version)}))
}

fn add(file: &mut Cameras, args: &Value) -> Result<Value, String> {
    let mut camera = Camera::new(format!("Camera {:03}", file.cameras.len() + 1), 0.0, 0.0);
    apply(&mut camera, args, file.layout, file.version)?;
    file.cameras.push(camera);
    let i = file.cameras.len() - 1;
    Ok(json!({"added": camera_json(i, &file.cameras[i], file.layout, file.version)}))
}

fn delete(file: &mut Cameras, args: &Value) -> Result<Value, String> {
    let i = find(file, args)?;
    let removed = file.cameras.remove(i);
    Ok(json!({"removed": camera_json(i, &removed, file.layout, file.version)}))
}

/// Reads the file on disk at `args.path`, changes it with `change`, writes
/// it back. With `create` (`w3c_new`), a missing file starts empty.
fn change(
    args: &Value,
    create: bool,
    change: fn(&mut Cameras, &Value) -> Result<Value, String>,
) -> Result<Value, String> {
    let path = args
        .get("path")
        .and_then(Value::as_str)
        .ok_or("`path` is missing")?;
    let new = create && !std::path::Path::new(path).exists();
    let (mut file, from) = if new {
        let (layout, from, w3i) = layout(args, path, None);
        let mut file = Cameras::new(layout.unwrap_or_default());
        // A Reforged 2.0 map's (w3i 39 and later) is version 3.
        if w3i.is_some_and(|v| v >= 39) {
            file.version = 3;
        }
        (file, from)
    } else {
        read(args, path, None)?
    };
    let mut out = change(&mut file, args)?;
    let bytes = file.to_bytes().map_err(|e| format!("{path}: {e}"))?;
    if new {
        std::fs::write(path, bytes).map_err(|e| format!("{path}: {e}"))?;
    } else {
        crate::source::write(path, bytes)?;
    }
    out["layout"] = json!(file.layout.describe());
    out["layout_from"] = json!(from);
    out["count"] = json!(file.cameras.len());
    Ok(out)
}

#[cfg(test)]
#[path = "w3c.test.rs"]
mod tests;
