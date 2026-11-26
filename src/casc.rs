//! The game's files in a Reforged install's CASC storage (`casc-rs`):
//! `casc_list` names them, `casc_read` gives one (text, or its size and
//! the hex of its first bytes, or written to a file). A storage opened
//! once is kept.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

use serde_json::{Value, json};

/// Text given at most at once.
const MAX_TEXT: usize = 200_000;

pub fn tools() -> Value {
    let game = json!({"type": "string", "description": "The Reforged game's folder (the one with .build.info)"});
    json!([
        {
            "name": "casc_list",
            "description": "The files of a Reforged install's CASC storage, by the names its root gives them: `War3.w3mod:Units/Human/Footman/Footman.mdx` the game's, a mod's under it (`War3.w3mod:_HD.w3mod:` the HD ones, `War3.w3mod:_Locales/ruRU.w3mod:`, `War3.w3mod:_Tilesets/L.w3mod:` ...); `on_disk` whether its data is installed (a locale not installed isn't). Those with `text` in their name (any case); paged.",
            "inputSchema": {"type": "object", "properties": {
                "game": game,
                "text": {"type": "string"},
                "on_disk": {"type": "boolean", "description": "Only those installed"},
                "offset": {"type": "integer", "minimum": 0},
                "limit": {"type": "integer", "minimum": 1, "maximum": 5000},
            }, "required": ["game"]},
        },
        {
            "name": "casc_read",
            "description": "One file of a Reforged install's CASC storage by its name (any case, \\ or /): text as text (from `offset`, at most 200,000 bytes at once), anything else as its size and the hex of up to 256 bytes; with `out`, written to that file on disk.",
            "inputSchema": {"type": "object", "properties": {
                "game": game,
                "name": {"type": "string"},
                "offset": {"type": "integer", "minimum": 0},
                "out": {"type": "string", "description": "A file on disk to write it to"},
            }, "required": ["game", "name"]},
        },
    ])
}

pub fn call(name: &str, args: &Value) -> Option<Result<Value, String>> {
    Some(match name {
        "casc_list" => storage(args).map(|s| list(&s, args)),
        "casc_read" => storage(args).and_then(|s| read(&s, args)),
        _ => return None,
    })
}

/// The storage of `game`, opened once.
fn storage(args: &Value) -> Result<Arc<casc_rs::Storage>, String> {
    static OPEN: OnceLock<Mutex<HashMap<PathBuf, Arc<casc_rs::Storage>>>> = OnceLock::new();
    let game = PathBuf::from(args.get("game").and_then(Value::as_str).ok_or("`game` is missing")?);
    let mut open = OPEN.get_or_init(Default::default).lock().map_err(|_| "the storages' lock is poisoned".to_string())?;
    if let Some(s) = open.get(&game) {
        return Ok(s.clone());
    }
    let s = Arc::new(casc_rs::Storage::open(Path::new(&game)).map_err(|e| e.to_string())?);
    open.insert(game, s.clone());
    Ok(s)
}

fn list(s: &casc_rs::Storage, args: &Value) -> Value {
    let text = args.get("text").and_then(Value::as_str).map(str::to_ascii_lowercase);
    let on_disk = args.get("on_disk").and_then(Value::as_bool).unwrap_or(false);
    let offset = args.get("offset").and_then(Value::as_u64).unwrap_or(0) as usize;
    let limit = args.get("limit").and_then(Value::as_u64).unwrap_or(500) as usize;
    let all: Vec<(&str, bool)> = s
        .entries
        .iter()
        .filter(|e| text.as_ref().is_none_or(|t| e.name.to_ascii_lowercase().contains(t)))
        .map(|e| (e.name.as_str(), s.has(&e.name)))
        .filter(|(_, has)| !on_disk || *has)
        .collect();
    let files: Vec<Value> = all.iter().skip(offset).take(limit).map(|(n, has)| json!({"name": n, "on_disk": has})).collect();
    json!({"build": s.build, "total": all.len(), "offset": offset, "files": files})
}

fn read(s: &casc_rs::Storage, args: &Value) -> Result<Value, String> {
    let name = args.get("name").and_then(Value::as_str).ok_or("`name` is missing")?;
    let data = s.read(name).map_err(|e| e.to_string())?;
    if let Some(out) = args.get("out").and_then(Value::as_str) {
        std::fs::write(out, &data).map_err(|e| format!("{out}: {e}"))?;
        return Ok(json!({"name": name, "size": data.len(), "written": out}));
    }
    let offset = (args.get("offset").and_then(Value::as_u64).unwrap_or(0) as usize).min(data.len());
    match std::str::from_utf8(&data) {
        Ok(text) => {
            let mut start = offset;
            while !text.is_char_boundary(start) {
                start -= 1;
            }
            let mut end = (start + MAX_TEXT).min(text.len());
            while !text.is_char_boundary(end) {
                end -= 1;
            }
            Ok(json!({"name": name, "size": data.len(), "offset": start, "more": end < text.len(), "text": &text[start..end]}))
        }
        Err(_) => {
            let hex: String = data[offset..].iter().take(256).map(|b| format!("{b:02x}")).collect();
            Ok(json!({"name": name, "size": data.len(), "offset": offset, "hex": hex}))
        }
    }
}

#[cfg(test)]
#[path = "casc.test.rs"]
mod tests;
