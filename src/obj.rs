//! The object data tools, on `w3abdhqtu-rs`: a map's changes to the
//! game's abilities (`.w3a`), destructables (`.w3b`), doodads (`.w3d`),
//! buffs (`.w3h`), upgrades (`.w3q`), items (`.w3t`) and units (`.w3u`).
//!
//! A file is read from disk (`path`) or from inside a map or archive
//! (`path` inside `archive`); changes are written to files on disk. The
//! kind comes from the extension. `game`, the game's folders (like a
//! project's `game=`), gives the fields' names and types and the checks,
//! from the game's metadata tables; without it fields are ids and numbers
//! are integers or reals by how they're written.
//!
//! An object is named by its id: a custom one's own id (`A000`), a changed
//! standard one's (`AHbz`). A field is its id (`Hbz1`), and for abilities,
//! upgrades and doodads its level and data column (`Data A` is 1).

use serde_json::{Map, Value, json};
use w3abdhqtu_rs::{
    FieldMeta, Kind, Metadata, Mod, Object, ObjectData, Rawcode, Set, Storage, id_text,
};

/// The most lines `obj_text` gives at once.
const MAX_LINES: usize = 5000;

/// Every tool, for `tools/list`.
pub fn tools() -> Value {
    let path = json!({"type": "string", "description": "The object data file: war3map.w3a (abilities), .w3b, .w3d, .w3h, .w3q, .w3t or .w3u; with `archive`, its name inside"});
    let disk = json!({"type": "string", "description": "The object data file on disk: .w3a .w3b .w3d .w3h .w3q .w3t .w3u"});
    let archive = json!({"type": "string", "description": "A map or archive (.w3x, .w3m, .mpq) to read `path` from"});
    let game = json!({"type": "array", "items": {"type": "string"}, "description": "The game's folders, lowest first (as a project's game=): their War3*.mpq and files give the fields' names, types and checks"});
    let object = json!({"type": "string", "description": "The object's id: a custom one's (A000), a changed standard one's (AHbz)"});
    let strings = json!({"type": "string", "description": "The map's string table (war3map.wts) on disk, to show TRIGSTR_<n> values as their text; read from `archive` by itself"});
    let source = json!({"path": path, "archive": archive, "game": game, "roots": crate::source::roots_schema(), "project": crate::source::project_schema(), "strings": strings});
    let with = |extra: Value| {
        let mut props = source.as_object().expect("an object").clone();
        props.extend(extra.as_object().expect("an object").clone());
        Value::Object(props)
    };
    json!([
        {
            "name": "obj_list",
            "description": "The objects of an object data file: id, base (the standard object it starts from), table (original: a changed standard object; custom: a new one), name (its name field, when the file sets it) and how many fields it changes. Filter by text in the id, base or name, or by table; page with offset / limit.",
            "inputSchema": {
                "type": "object",
                "properties": with(json!({
                    "contains": {"type": "string", "description": "Keep objects whose id, base or name contains this, in any case"},
                    "table": {"type": "string", "enum": ["original", "custom"]},
                    "offset": {"type": "integer", "minimum": 0},
                    "limit": {"type": "integer", "minimum": 1, "maximum": 5000, "description": "Objects to give at most (default 500)"},
                })),
                "required": ["path"],
            },
        },
        {
            "name": "obj_get",
            "description": "One object of an object data file and every field it changes: the key as the text form writes it (Hbz1[3].A), field id, level, column, value, and with `game` the field's name and type.",
            "inputSchema": {"type": "object", "properties": with(json!({"object": object})), "required": ["path", "object"]},
        },
        {
            "name": "obj_text",
            "description": "An object data file as text (BNI, what the wc3-rs editor shows): a section per object, a field per line, with `game` each field's name and type as a comment. Paged by lines (offset, limit; 5000 at most).",
            "inputSchema": {
                "type": "object",
                "properties": with(json!({
                    "offset": {"type": "integer", "minimum": 0, "description": "Lines to skip"},
                    "limit": {"type": "integer", "minimum": 1, "maximum": 5000, "description": "Lines to give at most (default 1000)"},
                })),
                "required": ["path"],
            },
        },
        {
            "name": "obj_check",
            "description": "Checks an object data file against the game's metadata (`game` is needed): values of the wrong type, past World Editor's limits, unknown fields, levels or data columns that don't fit, fields of other objects. Each problem with its object, key, rule and message.",
            "inputSchema": {"type": "object", "properties": source, "required": ["path"]},
        },
        {
            "name": "obj_set",
            "description": "Writes one field of one object in an object data file on disk, or removes it (value null). A number takes the field's type (integer, real, unreal) from `game`'s metadata; without it a whole number is an integer and any other a real. Abilities, upgrades and doodads need `level` for a field with levels; the data column defaults to the field's own.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": disk,
                    "game": game,
                    "roots": crate::source::roots_schema(),
                    "project": crate::source::project_schema(),
                    "object": object,
                    "field": {"type": "string", "description": "The field: its name in the game's tables as obj_get keys it (Cool, DataA, HP) or its id (acdn, Hbz1, uhpm)"},
                    "level": {"type": "integer", "minimum": 0},
                    "column": {"type": "integer", "minimum": 0, "description": "The data column, Data A is 1"},
                    "value": {"type": ["string", "number", "null"], "description": "The new value; null removes the field"},
                },
                "required": ["path", "object", "field", "value"],
            },
        },
        {
            "name": "obj_new",
            "description": "Adds a custom object to an object data file on disk: its own id and the standard object it starts from, with no fields changed yet.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": disk,
                    "id": {"type": "string", "description": "The new object's id: A000, h001"},
                    "base": {"type": "string", "description": "The standard object it starts from: AHbz, hfoo"},
                },
                "required": ["path", "id", "base"],
            },
        },
        {
            "name": "obj_delete",
            "description": "Removes an object from an object data file on disk: a custom one, or a standard one's changes.",
            "inputSchema": {"type": "object", "properties": {"path": disk, "object": object}, "required": ["path", "object"]},
        },
    ])
}

/// Runs the tool `name`, if it's one of these; an error is a message for
/// the model.
pub fn call(name: &str, args: &Value) -> Option<Result<Value, String>> {
    Some(match name {
        "obj_list" => open(args).and_then(|f| list(&f, args)),
        "obj_get" => open(args).and_then(|f| get(&f, args)),
        "obj_text" => open(args).and_then(|f| text(&f, args)),
        "obj_check" => open(args).and_then(|f| check(&f)),
        "obj_set" => set(args),
        "obj_new" => new(args),
        "obj_delete" => delete(args),
        _ => return None,
    })
}

/// A file, the metadata of its kind, and the map's string table.
struct File {
    data: ObjectData,
    meta: Metadata,
    strings: Option<wts_rs::Document>,
}

impl File {
    /// A value as JSON, a `TRIGSTR_<n>` as its text when the string
    /// table has it.
    fn value(&self, v: &w3abdhqtu_rs::Value) -> Value {
        match (v, &self.strings) {
            (w3abdhqtu_rs::Value::Str(s), Some(table)) => json!(table.resolve(s)),
            _ => value_json(v),
        }
    }
}

fn string<'a>(args: &'a Value, key: &str) -> Result<&'a str, String> {
    args.get(key)
        .and_then(Value::as_str)
        .ok_or(format!("`{key}` is missing"))
}

fn kind(path: &str) -> Result<Kind, String> {
    Kind::from_path(path).ok_or(format!(
        "{path} isn't object data: .w3a .w3b .w3d .w3h .w3q .w3t .w3u"
    ))
}

/// The metadata of `kind` from `args.game` and `args.roots`, read as the
/// editor reads it (the server's `objects::metadata`); empty without
/// them.
fn metadata(args: &Value, kind: Kind) -> Metadata {
    wc3_server::objects::metadata(&crate::source::lookup(args), kind)
}

fn open(args: &Value) -> Result<File, String> {
    let path = string(args, "path")?;
    let kind = kind(path)?;
    let bytes = crate::source::read_bytes(path, args.get("archive").and_then(Value::as_str))?;
    let data = w3abdhqtu_rs::parse(&bytes, kind).map_err(|e| format!("{path}: {e}"))?;
    Ok(File {
        data,
        meta: metadata(args, kind),
        strings: crate::wts::table_for(args),
    })
}

/// The file on disk at `args.path`, to change.
fn open_disk(args: &Value) -> Result<(String, ObjectData), String> {
    let path = string(args, "path")?;
    let kind = kind(path)?;
    let bytes = crate::source::read_bytes(path, None)?;
    let data = w3abdhqtu_rs::parse(&bytes, kind).map_err(|e| format!("{path}: {e}"))?;
    Ok((path.to_string(), data))
}

fn save(path: &str, data: &ObjectData) -> Result<(), String> {
    let bytes = data.to_bytes().map_err(|e| format!("{path}: {e}"))?;
    crate::source::write(path, bytes)
}

fn parse_id(text: &str) -> Result<Rawcode, String> {
    Rawcode::parse(text.trim()).ok_or(format!(
        "'{text}' isn't an id: four characters, or 0x and eight hex digits"
    ))
}

/// An object's id as the user names it: its own, or a standard one's base.
fn object_id(o: &Object) -> Rawcode {
    if o.id.is_none() { o.base } else { o.id }
}

/// Where the object named `id` is: (custom?, index).
fn find(data: &ObjectData, id: Rawcode) -> Option<(bool, usize)> {
    data.customs
        .iter()
        .position(|o| o.id == id)
        .map(|i| (true, i))
        .or_else(|| {
            data.originals
                .iter()
                .position(|o| object_id(o) == id)
                .map(|i| (false, i))
        })
}

/// The field that names objects of this kind.
fn name_field(kind: Kind) -> &'static [u8; 4] {
    match kind {
        Kind::Ability => b"anam",
        Kind::Destructable => b"bnam",
        Kind::Doodad => b"dnam",
        Kind::Buff => b"fnam",
        Kind::Upgrade => b"gnam",
        Kind::Item | Kind::Unit => b"unam",
    }
}

fn name(f: &File, o: &Object) -> Value {
    let field = Rawcode(*name_field(f.data.kind));
    o.mods()
        .find(|m| m.field == field)
        .map_or(Value::Null, |m| f.value(&m.value))
}

fn value_json(v: &w3abdhqtu_rs::Value) -> Value {
    match v {
        w3abdhqtu_rs::Value::Int(i) => json!(i),
        w3abdhqtu_rs::Value::Real(r) | w3abdhqtu_rs::Value::Unreal(r) => {
            serde_json::Number::from_f64(f64::from(*r)).map_or(json!(r.to_string()), Value::Number)
        }
        w3abdhqtu_rs::Value::Str(s) => json!(s),
    }
}

fn type_name(v: &w3abdhqtu_rs::Value) -> &'static str {
    match v {
        w3abdhqtu_rs::Value::Int(_) => "int",
        w3abdhqtu_rs::Value::Real(_) => "real",
        w3abdhqtu_rs::Value::Unreal(_) => "unreal",
        w3abdhqtu_rs::Value::Str(_) => "string",
    }
}

fn list(f: &File, args: &Value) -> Result<Value, String> {
    let kind = f.data.kind;
    let needle = args
        .get("contains")
        .and_then(Value::as_str)
        .map(str::to_lowercase);
    let table = args.get("table").and_then(Value::as_str);
    let offset = args.get("offset").and_then(Value::as_u64).unwrap_or(0) as usize;
    let limit = args
        .get("limit")
        .and_then(Value::as_u64)
        .unwrap_or(500)
        .clamp(1, 5000) as usize;
    let all = f
        .data
        .originals
        .iter()
        .map(|o| ("original", o))
        .chain(f.data.customs.iter().map(|o| ("custom", o)))
        .filter(|(t, _)| table.is_none_or(|want| want == *t))
        .map(|(t, o)| {
            json!({
                "id": id_text(object_id(o)),
                "base": id_text(o.base),
                "table": t,
                "name": name(f, o),
                "fields": o.mods().count(),
            })
        })
        .filter(|o| {
            needle.as_ref().is_none_or(|n| {
                ["id", "base", "name"]
                    .iter()
                    .any(|k| o[k].as_str().is_some_and(|s| s.to_lowercase().contains(n)))
            })
        });
    let mut total = 0;
    let mut objects = Vec::new();
    for o in all {
        if total >= offset && objects.len() < limit {
            objects.push(o);
        }
        total += 1;
    }
    Ok(
        json!({"kind": kind.title(), "version": f.data.version, "total": total, "offset": offset, "objects": objects}),
    )
}

fn field_json(f: &File, m: &Mod, keys: &w3abdhqtu_rs::Keys) -> Value {
    let (kind, meta) = (f.data.kind, &f.meta);
    let mut obj = Map::new();
    // As the text form writes it: by its name in the game's tables.
    let key = w3abdhqtu_rs::field_key(m, kind, meta, keys);
    obj.insert("key".into(), json!(key));
    obj.insert("field".into(), json!(id_text(m.field)));
    if let Some(f) = meta.get(m.field) {
        obj.insert("name".into(), json!(f.name));
        obj.insert("type".into(), json!(f.ty));
    }
    obj.insert("level".into(), json!(m.level));
    obj.insert("column".into(), json!(m.column));
    obj.insert("stored".into(), json!(type_name(&m.value)));
    obj.insert("value".into(), value_json(&m.value));
    // A TRIGSTR_<n> with the text the map's string table gives it.
    let text = f.value(&m.value);
    if text != obj["value"] {
        obj.insert("text".into(), text);
    }
    Value::Object(obj)
}

fn get(f: &File, args: &Value) -> Result<Value, String> {
    let id = parse_id(string(args, "object")?)?;
    let (custom, i) = find(&f.data, id).ok_or(format!("no object {} in the file", id_text(id)))?;
    let o = if custom {
        &f.data.customs[i]
    } else {
        &f.data.originals[i]
    };
    let keys = f.meta.keys(f.data.kind, o.base);
    let sets: Vec<Value> = o
        .sets
        .iter()
        .map(|s| {
            let fields: Vec<Value> = s.mods.iter().map(|m| field_json(f, m, &keys)).collect();
            json!({"flags": s.flags, "fields": fields})
        })
        .collect();
    Ok(json!({
        "id": id_text(object_id(o)),
        "base": id_text(o.base),
        "table": if custom { "custom" } else { "original" },
        "name": name(f, o),
        "sets": sets,
    }))
}

fn text(f: &File, args: &Value) -> Result<Value, String> {
    let text = f.data.to_text_named(&f.meta);
    let offset = args.get("offset").and_then(Value::as_u64).unwrap_or(0) as usize;
    let limit = args
        .get("limit")
        .and_then(Value::as_u64)
        .unwrap_or(1000)
        .clamp(1, MAX_LINES as u64) as usize;
    let lines: Vec<&str> = text.lines().collect();
    let page = lines
        .iter()
        .skip(offset)
        .take(limit)
        .copied()
        .collect::<Vec<_>>()
        .join("\n");
    Ok(
        json!({"lines": lines.len(), "offset": offset, "more": offset + limit < lines.len(), "text": page}),
    )
}

fn check(f: &File) -> Result<Value, String> {
    if f.meta.is_empty() {
        return Err("no metadata: give `game`, the game's folders, to check against".into());
    }
    let text = f.data.to_text_named(&f.meta);
    let lines: Vec<&str> = text.lines().collect();
    let parsed = w3abdhqtu_rs::from_text(&text, &f.meta);
    // The object a line belongs to: the section above it.
    let object_at = |line: usize| {
        lines[..line.min(lines.len())].iter().rev().find_map(|l| {
            l.strip_prefix('[')
                .and_then(|l| l.split_once(']'))
                .map(|(id, _)| id.to_string())
        })
    };
    let problems: Vec<Value> = w3abdhqtu_rs::check(&parsed, &f.meta)
        .iter()
        .map(|d| {
            let line = lines.get(d.line - 1).copied().unwrap_or("");
            let key = line.split('=').next().unwrap_or("");
            json!({
                "object": object_at(d.line),
                "key": key,
                "level": d.level.as_str(),
                "rule": d.rule,
                "message": d.message,
            })
        })
        .collect();
    Ok(json!({"total": problems.len(), "problems": problems}))
}

/// The value to store for `v` in a field of `meta` (`None`: unknown).
fn to_value(v: &Value, field: Option<&FieldMeta>) -> Result<w3abdhqtu_rs::Value, String> {
    let storage = field.and_then(|f| f.storage);
    let number = |n: f64| -> Result<w3abdhqtu_rs::Value, String> {
        Ok(match storage {
            Some(Storage::Real) => w3abdhqtu_rs::Value::Real(n as f32),
            Some(Storage::Unreal) => w3abdhqtu_rs::Value::Unreal(n as f32),
            Some(Storage::Str) => {
                return Err("the field holds text: give the value as a string".into());
            }
            _ if n.fract() == 0.0 && n.abs() <= f64::from(i32::MAX) => {
                w3abdhqtu_rs::Value::Int(n as i32)
            }
            Some(Storage::Int) => return Err("the field holds a whole number".into()),
            None => w3abdhqtu_rs::Value::Real(n as f32),
        })
    };
    match v {
        Value::Number(n) => number(n.as_f64().ok_or("not a number")?),
        Value::String(s) => match storage {
            Some(Storage::Int | Storage::Real | Storage::Unreal) => number(
                s.trim()
                    .parse::<f64>()
                    .map_err(|_| format!("the field holds a number, not '{s}'"))?,
            ),
            _ => Ok(w3abdhqtu_rs::Value::Str(s.clone())),
        },
        _ => Err("`value` is a string, a number or null".into()),
    }
}

fn set(args: &Value) -> Result<Value, String> {
    let (path, mut data) = open_disk(args)?;
    let meta = metadata(args, data.kind);
    let id = parse_id(string(args, "object")?)?;
    // The field by its name in the game's tables (`Cool`, `DataA`), for
    // this object, or by its id.
    let base = find(&data, id).map(|(custom, i)| {
        if custom {
            data.customs[i].base
        } else {
            data.originals[i].base
        }
    });
    let field_arg = string(args, "field")?;
    let field = match base.and_then(|b| meta.keys(data.kind, b).field(field_arg)) {
        Some(field) => field,
        None => parse_id(field_arg)?,
    };
    let info = meta.get(field);
    if !meta.is_empty() && info.is_none() {
        return Err(format!(
            "{} isn't a field of {} in this game's metadata",
            id_text(field),
            data.kind.title()
        ));
    }
    let levels = data.kind.has_levels(data.version);
    let level = args.get("level").and_then(Value::as_u64).map(|l| l as u32);
    let column = args.get("column").and_then(Value::as_u64).map(|c| c as u32);
    let (level, column) = if levels {
        let has_levels = info.is_some_and(|f| f.levels);
        let level = match level {
            Some(l) => l,
            None if has_levels => {
                return Err(format!(
                    "{} has a value per level: give `level`",
                    id_text(field)
                ));
            }
            None => 0,
        };
        let own = if data.kind == Kind::Ability {
            info.map_or(0, |f| f.column)
        } else {
            0
        };
        (level, column.unwrap_or(own))
    } else {
        if level.is_some_and(|l| l != 0) || column.is_some_and(|c| c != 0) {
            return Err(format!(
                "fields of {} have no level or data column",
                data.kind.title()
            ));
        }
        (0, 0)
    };
    let (custom, i) = find(&data, id).ok_or(format!("no object {} in the file", id_text(id)))?;
    let object = if custom {
        &mut data.customs[i]
    } else {
        &mut data.originals[i]
    };
    if object.sets.is_empty() {
        object.sets.push(Set {
            flags: 0,
            mods: Vec::new(),
        });
    }
    let at = |m: &Mod| m.field == field && m.level == level && m.column == column;
    let before = object.mods().find(|m| at(m)).map(|m| value_json(&m.value));
    let value = match args.get("value") {
        None | Some(Value::Null) => None,
        Some(v) => Some(to_value(v, info)?),
    };
    match value {
        Some(value) => {
            let set = object.sets.iter_mut().find(|s| s.mods.iter().any(&at));
            match set {
                Some(s) => {
                    let m = s.mods.iter_mut().find(|m| at(m)).expect("just found");
                    m.value = value;
                }
                None => object.sets.last_mut().expect("made sure").mods.push(Mod {
                    field,
                    level,
                    column,
                    value,
                    end: Rawcode::NONE,
                }),
            }
        }
        None => {
            for s in &mut object.sets {
                s.mods.retain(|m| !at(m));
            }
        }
    }
    let after = object.mods().find(|m| at(m)).map(|m| value_json(&m.value));
    save(&path, &data)?;
    Ok(
        json!({"object": id_text(id), "field": id_text(field), "level": level, "column": column, "before": before, "after": after}),
    )
}

fn new(args: &Value) -> Result<Value, String> {
    let (path, mut data) = open_disk(args)?;
    let id = parse_id(string(args, "id")?)?;
    let base = parse_id(string(args, "base")?)?;
    if find(&data, id).is_some() {
        return Err(format!("{} is already in the file", id_text(id)));
    }
    let sets = if data.version < 3 {
        vec![Set {
            flags: 0,
            mods: Vec::new(),
        }]
    } else {
        Vec::new()
    };
    data.customs.push(Object { base, id, sets });
    save(&path, &data)?;
    Ok(json!({"added": id_text(id), "base": id_text(base)}))
}

fn delete(args: &Value) -> Result<Value, String> {
    let (path, mut data) = open_disk(args)?;
    let id = parse_id(string(args, "object")?)?;
    let (custom, i) = find(&data, id).ok_or(format!("no object {} in the file", id_text(id)))?;
    if custom {
        data.customs.remove(i);
    } else {
        data.originals.remove(i);
    }
    save(&path, &data)?;
    Ok(json!({"removed": id_text(id), "table": if custom { "custom" } else { "original" }}))
}

#[cfg(test)]
#[path = "obj.test.rs"]
mod tests;
