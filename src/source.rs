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
