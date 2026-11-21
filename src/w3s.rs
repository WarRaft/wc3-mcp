//! The sound tools, on `w3s-rs`: `war3map.w3s`, the sounds a map's Sound
//! Editor defines (`gg_snd_*` in its script). A file is read from disk
//! (`path`) or from inside a map (`path` inside `archive`, `war3map.w3s`
//! by default); changes are written to files on disk.
//!
//! A sound is named by its variable (`gg_snd_Rain`) or its index in the
//! file (from 0). A number may be `"default"`: World Editor's "use
//! default", the game's own value.

use serde_json::{Map, Value, json};
use w3s_rs::{DEFAULT_INT, DEFAULT_REAL, Sound, Sounds};

/// Every tool, for `tools/list`.
pub fn tools() -> Value {
    let path = json!({"type": "string", "description": "The sound file: war3map.w3s on disk, or with `archive` its name inside (war3map.w3s when left out)"});
    let disk = json!({"type": "string", "description": "The war3map.w3s on disk"});
    let archive =
        json!({"type": "string", "description": "A map (.w3x, .w3m) to read the file from"});
    let sound = json!({"description": "The sound: its variable (gg_snd_Rain) or its index in the file (from 0)", "anyOf": [{"type": "integer", "minimum": 0}, {"type": "string"}]});
    let mut props = Map::new();
    props.insert("name".into(), json!({"type": "string", "description": "The script's variable, gg_snd_...; unique"}));
    props.insert("file".into(), json!({"type": "string", "description": "The sound file, Sound\\Ambient\\Rain.wav"}));
    props.insert("eax".into(), json!({"type": "string", "description": "An EAX setting's label from UI\\SoundInfo\\EAXDefs.slk (DefaultEAXON, SpellsEAX ...)"}));
    props.insert("flags".into(), json!({"type": "integer", "minimum": 0, "description": "0x01 looping, 0x02 3D sound, 0x04 stop when out of range, 0x08 music, 0x10 imported; other bits are kept"}));
    let channels: Vec<String> = w3s_rs::CHANNELS.iter().skip(1).map(|(n, name)| format!("{n} {name}")).collect();
    for (name, value) in Sound::new("", "").numbers() {
        let kind = if matches!(value, w3s_rs::Value::Real(_)) { "number" } else { "integer" };
        let mut schema = json!({"anyOf": [{"type": kind}, {"const": "default"}]});
        if name == "channel" {
            schema["description"] = json!(format!("The channel: {}", channels.join(", ")));
        }
        props.insert(name.into(), schema);
    }
    let values = json!({"type": "object", "properties": props, "additionalProperties": false, "description": "Fields to change; a number may be \"default\" (World Editor's \"use default\")"});
    json!([
        {
            "name": "w3s_list",
            "description": "A map's sounds (war3map.w3s): index, variable, file, EAX setting, flags (with their names), volume, pitch, channel (with its name), fades, distances, cone. \"default\" is World Editor's \"use default\". With `game`, the names are the game's (its language), and the EAX setting's name too.",
            "inputSchema": {"type": "object", "properties": {"path": path, "archive": archive, "game": crate::source::game_schema(), "roots": crate::source::roots_schema()}},
        },
        {
            "name": "w3s_set",
            "description": "Changes a sound of a war3map.w3s on disk: some of its fields, its variable. Only the fields given change.",
            "inputSchema": {"type": "object", "properties": {"path": disk, "sound": sound, "values": values}, "required": ["path", "sound", "values"]},
        },
        {
            "name": "w3s_new",
            "description": "Adds a sound to a war3map.w3s on disk (a new file when there's none), as World Editor makes one for what `values` leaves out: 3D, stopping out of range, fades of 10, DefaultEAXON, every other number \"default\". `name` and `file` are needed.",
            "inputSchema": {"type": "object", "properties": {"path": disk, "values": values}, "required": ["path", "values"]},
        },
        {
            "name": "w3s_delete",
            "description": "Removes a sound from a war3map.w3s on disk. The script may still use its gg_snd_ variable.",
            "inputSchema": {"type": "object", "properties": {"path": disk, "sound": sound}, "required": ["path", "sound"]},
        },
    ])
}

/// Runs the tool `name`, if it's one of these; an error is a message for
/// the model.
pub fn call(name: &str, args: &Value) -> Option<Result<Value, String>> {
    Some(match name {
        "w3s_list" => open(args).map(|f| {
            // The game's words where it has them: its channels' names, its
            // EAX settings', its strings for the flags.
            let names = crate::source::names(args);
            let mut out = list(&f);
            for (s, json) in f.sounds.iter().zip(out["sounds"].as_array_mut().into_iter().flatten()) {
                let label = if s.eax.is_empty() { "DefaultEAXON" } else { &s.eax };
                json["eax_name"] = json!(names.eax(label));
                if let Some(name) = names.channel(s.channel) {
                    json["channel_name"] = json!(name);
                }
                let flags: Vec<String> = w3s_rs::FLAGS
                    .iter()
                    .filter(|(b, _, _)| s.flags & b != 0)
                    .map(|(_, english, key)| names.say(key, english))
                    .collect();
                json["flag_names"] = json!(flags);
            }
            out
        }),
        "w3s_set" => change(args, false, set),
        "w3s_new" => change(args, true, add),
        "w3s_delete" => change(args, false, delete),
        _ => return None,
    })
}

/// The file at `args.path`, or `war3map.w3s` (or `args.path`) inside
/// `args.archive`.
fn open(args: &Value) -> Result<Sounds, String> {
    let archive = args.get("archive").and_then(Value::as_str);
    let path = match (args.get("path").and_then(Value::as_str), archive) {
        (Some(p), _) => p,
        (None, Some(_)) => "war3map.w3s",
        (None, None) => return Err("`path` is missing".into()),
    };
    let bytes = crate::source::read_bytes(path, archive)?;
    w3s_rs::parse(&bytes).map_err(|e| format!("{path}: {e}"))
}

/// A number as JSON: `"default"`, or the shortest decimal that reads back
/// as it.
fn number(value: w3s_rs::Value) -> Value {
    match value {
        w3s_rs::Value::Int(DEFAULT_INT) => json!("default"),
        w3s_rs::Value::Int(i) => json!(i),
        w3s_rs::Value::Real(r) if r.to_bits() == DEFAULT_REAL.to_bits() => json!("default"),
        w3s_rs::Value::Real(r) => r
            .to_string()
            .parse::<f64>()
            .ok()
            .and_then(serde_json::Number::from_f64)
            .map_or(Value::Null, Value::Number),
    }
}

fn sound_json(index: usize, s: &Sound) -> Value {
    let mut out = Map::new();
    out.insert("index".into(), json!(index));
    out.insert("name".into(), json!(s.name));
    out.insert("file".into(), json!(s.file));
    out.insert("eax".into(), json!(s.eax));
    out.insert("flags".into(), json!(s.flags));
    out.insert("flag_names".into(), json!(w3s_rs::flag_names(s.flags)));
    for (name, value) in s.numbers() {
        out.insert(name.into(), number(value));
    }
    out.insert("channel_name".into(), json!(w3s_rs::channel_name(s.channel)));
    Value::Object(out)
}

fn list(file: &Sounds) -> Value {
    let sounds: Vec<Value> = file
        .sounds
        .iter()
        .enumerate()
        .map(|(i, s)| sound_json(i, s))
        .collect();
    json!({"count": sounds.len(), "sounds": sounds})
}

/// The index of `args.sound`: a number, or a variable.
fn find(file: &Sounds, args: &Value) -> Result<usize, String> {
    match args.get("sound") {
        Some(Value::Number(n)) => n
            .as_u64()
            .map(|n| n as usize)
            .filter(|&n| n < file.sounds.len())
            .ok_or(format!("no sound {n}: there are {}", file.sounds.len())),
        Some(Value::String(name)) => file
            .sounds
            .iter()
            .position(|s| &s.name == name)
            .ok_or(format!("no sound named {name:?}")),
        _ => Err("`sound` is an index or a variable".into()),
    }
}

/// Sets `values` on `sound`, at `at` in `file` (past the end for a new
/// one), then checks its name against the others.
fn apply(file: &Sounds, sound: &mut Sound, at: usize, values: &Map<String, Value>) -> Result<(), String> {
    for (key, value) in values {
        let text = || value.as_str().map(String::from).ok_or(format!("`{key}` is a string"));
        match key.as_str() {
            "name" => sound.name = text()?,
            "file" => sound.file = text()?,
            "eax" => sound.eax = text()?,
            "flags" => {
                sound.flags = value
                    .as_u64()
                    .and_then(|n| u32::try_from(n).ok())
                    .ok_or("`flags` is a whole number, 0 or more")?
            }
            _ => {
                let real = sound
                    .numbers()
                    .iter()
                    .find(|(n, _)| n == key)
                    .map(|(_, v)| matches!(v, w3s_rs::Value::Real(_)))
                    .ok_or(format!("no field `{key}`"))?;
                let v = match (value, real) {
                    (Value::String(s), true) if s == "default" => w3s_rs::Value::Real(DEFAULT_REAL),
                    (Value::String(s), false) if s == "default" => w3s_rs::Value::Int(DEFAULT_INT),
                    (v, true) => w3s_rs::Value::Real(
                        v.as_f64().ok_or(format!("`{key}` is a number or \"default\""))? as f32,
                    ),
                    (v, false) => w3s_rs::Value::Int(
                        v.as_i64()
                            .and_then(|n| i32::try_from(n).ok())
                            .ok_or(format!("`{key}` is a whole number or \"default\""))?,
                    ),
                };
                sound.set_number(key, v);
            }
        }
    }
    if sound.name.is_empty() {
        return Err("a sound has a name: its variable, gg_snd_...".into());
    }
    if let Some((i, _)) = file
        .sounds
        .iter()
        .enumerate()
        .find(|(i, s)| *i != at && s.name == sound.name)
    {
        return Err(format!("sound {i} is {:?} too: each is a variable", sound.name));
    }
    Ok(())
}

fn values(args: &Value) -> Result<&Map<String, Value>, String> {
    args.get("values")
        .and_then(Value::as_object)
        .ok_or("`values` is an object".into())
}

fn set(file: &mut Sounds, args: &Value) -> Result<Value, String> {
    let i = find(file, args)?;
    let mut sound = file.sounds[i].clone();
    apply(file, &mut sound, i, values(args)?)?;
    let before = sound_json(i, &file.sounds[i]);
    file.sounds[i] = sound;
    Ok(json!({"before": before, "after": sound_json(i, &file.sounds[i])}))
}

fn add(file: &mut Sounds, args: &Value) -> Result<Value, String> {
    let values = values(args)?;
    let missing: Vec<&str> = ["name", "file"]
        .into_iter()
        .filter(|k| !values.contains_key(*k))
        .collect();
    if !missing.is_empty() {
        return Err(format!("a new sound needs {}", missing.join(" and ")));
    }
    let mut sound = Sound::new("", "");
    apply(file, &mut sound, file.sounds.len(), values)?;
    file.sounds.push(sound);
    let i = file.sounds.len() - 1;
    Ok(json!({"added": sound_json(i, &file.sounds[i])}))
}

fn delete(file: &mut Sounds, args: &Value) -> Result<Value, String> {
    let i = find(file, args)?;
    let removed = file.sounds.remove(i);
    Ok(json!({"removed": sound_json(i, &removed)}))
}

/// Reads the file on disk at `args.path`, changes it with `change`, writes
/// it back. With `create` (`w3s_new`), a missing file starts empty.
fn change(
    args: &Value,
    create: bool,
    change: fn(&mut Sounds, &Value) -> Result<Value, String>,
) -> Result<Value, String> {
    let path = args
        .get("path")
        .and_then(Value::as_str)
        .ok_or("`path` is missing")?;
    let new = create && !std::path::Path::new(path).exists();
    let mut file = if new {
        Sounds::new()
    } else {
        let bytes = crate::source::read_bytes(path, None)?;
        w3s_rs::parse(&bytes).map_err(|e| format!("{path}: {e}"))?
    };
    let mut out = change(&mut file, args)?;
    let bytes = file.to_bytes().map_err(|e| format!("{path}: {e}"))?;
    if new {
        std::fs::write(path, bytes).map_err(|e| format!("{path}: {e}"))?;
    } else {
        crate::source::write(path, bytes)?;
    }
    out["count"] = json!(file.sounds.len());
    Ok(out)
}

#[cfg(test)]
#[path = "w3s.test.rs"]
mod tests;
