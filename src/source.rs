//! Where a file's text comes from: a file on disk, or a file inside an
//! MPQ archive or a map (`.mpq`, `.w3x`, `.w3m`), read with `storm-rs`.
//! Text is read as UTF-8, invalid bytes replaced.

use std::path::Path;

/// The text of `path`, or of the file named `path` inside `archive`.
pub fn read(path: &str, archive: Option<&str>) -> Result<String, String> {
    let bytes = read_bytes(path, archive)?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

/// The bytes of `path`, or of the file named `path` inside `archive`.
pub fn read_bytes(path: &str, archive: Option<&str>) -> Result<Vec<u8>, String> {
    Ok(match archive {
        None => std::fs::read(path).map_err(|e| format!("{path}: {e}"))?,
        Some(archive) => {
            let mpq = storm_rs::MpqArchive::open(archive).map_err(|e| format!("{archive}: {e}"))?;
            let name = storm_rs::archive::mpq_name(path);
            if !mpq.has_file(&name) {
                return Err(format!("{archive}: no {name} in it"));
            }
            mpq.read_file(&name)
                .map_err(|e| format!("{archive}: {name}: {e}"))?
        }
    })
}

/// Writes `data` to the file `path` on disk, which must exist.
pub fn write(path: &str, data: impl AsRef<[u8]>) -> Result<(), String> {
    if !Path::new(path).is_file() {
        return Err(format!("{path}: not a file on disk"));
    }
    std::fs::write(path, data).map_err(|e| format!("{path}: {e}"))
}

/// Where the game's files are looked up, from `args.game` (the game's
/// folders, lowest first, as a project's `game=`) and `args.roots` (the
/// map's roots, folders or maps, as `[map] root=`): the same layers the
/// editor reads, the last that has a file wins.
pub fn lookup(args: &serde_json::Value) -> wc3_server::game::Lookup {
    let paths = |key: &str| -> Vec<std::path::PathBuf> {
        args.get(key)
            .and_then(serde_json::Value::as_array)
            .map(|l| l.iter().filter_map(serde_json::Value::as_str).map(std::path::PathBuf::from).collect())
            .unwrap_or_default()
    };
    wc3_server::game::Lookup {
        game: paths("game"),
        roots: paths("roots"),
    }
}

/// The schema of `roots`, for the tools that take `game`.
pub fn roots_schema() -> serde_json::Value {
    serde_json::json!({"type": "array", "items": {"type": "string"}, "description": "The map's roots over the game's folders, lowest first (as a project's [map] root=): folders or maps whose files replace the game's"})
}

/// The schema of `game`, for the tools that show the game's names.
pub fn game_schema() -> serde_json::Value {
    serde_json::json!({"type": "array", "items": {"type": "string"}, "description": "The game's folders, lowest first (as a project's game=): names come from their files (UI\\WorldEditStrings.txt ...), in the game's language"})
}

/// The names the game's files give, through `args.game` and
/// `args.roots` (none without them).
pub fn names(args: &serde_json::Value) -> wc3_server::names::Names {
    wc3_server::names::names(&lookup(args))
}

/// The file a tool reads: `args.path`, or `default` inside `args.archive`
/// when there's no path; and the archive.
pub fn file<'a>(args: &'a serde_json::Value, default: &'a str) -> Result<(&'a str, Option<&'a str>), String> {
    let archive = args.get("archive").and_then(serde_json::Value::as_str);
    match (args.get("path").and_then(serde_json::Value::as_str), archive) {
        (Some(p), _) => Ok((p, archive)),
        (None, Some(_)) => Ok((default, archive)),
        (None, None) => Err("`path` is missing".into()),
    }
}

/// The bytes of the file named `name` beside `path` (the same folder, or
/// the same folder inside `archive`).
pub fn sibling(path: &str, archive: Option<&str>, name: &str) -> Option<Vec<u8>> {
    let beside = match archive {
        Some(_) => match path.rfind(['\\', '/']) {
            Some(i) => format!("{}{name}", &path[..=i]),
            None => name.to_string(),
        },
        None => Path::new(path).with_file_name(name).to_string_lossy().into_owned(),
    };
    read_bytes(&beside, archive).ok()
}

/// The errors of a text form, as one message.
pub fn text_errors<'a>(errors: impl IntoIterator<Item = (usize, &'a str)>) -> String {
    let lines: Vec<String> = errors.into_iter().map(|(l, m)| format!("line {l}: {m}")).collect();
    format!("nothing written:\n{}", lines.join("\n"))
}

/// The text argument `key`.
pub fn arg_str<'a>(args: &'a serde_json::Value, key: &str) -> Result<&'a str, String> {
    args.get(key).and_then(serde_json::Value::as_str).ok_or(format!("`{key}` is missing"))
}

/// The schemas of `path` (for `default`) and `archive`.
pub fn path_schema(default: &str, what: &str) -> serde_json::Value {
    serde_json::json!({"type": "string", "description": format!("{what}: {default} on disk, or with `archive` its name inside ({default} when left out)")})
}

pub fn archive_schema() -> serde_json::Value {
    serde_json::json!({"type": "string", "description": "A map (.w3x, .w3m) to read the file from"})
}
