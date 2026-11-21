//! Where a file's text comes from: a file on disk, or a file inside an
//! MPQ archive or a map (`.mpq`, `.w3x`, `.w3m`), read with `storm-rs`.
//! Text is read as UTF-8, invalid bytes replaced.

use std::path::Path;

/// The text of `path`, or of the file named `path` inside `archive`.
pub fn read(path: &str, archive: Option<&str>) -> Result<String, String> {
    let bytes = match archive {
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
    };
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

/// Writes `text` to the file `path` on disk.
pub fn write(path: &str, text: &str) -> Result<(), String> {
    if !Path::new(path).is_file() {
        return Err(format!("{path}: not a file on disk"));
    }
    std::fs::write(path, text).map_err(|e| format!("{path}: {e}"))
}
