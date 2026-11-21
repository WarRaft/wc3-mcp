//! Where a file's text comes from: a file on disk, or a file inside an
//! MPQ archive or a map (`.mpq`, `.w3x`, `.w3m`), read with `storm-rs`.
//! Text is read as UTF-8, invalid bytes replaced.
//!
//! Also the game's files, for metadata: [`game`].

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

/// Reads a game file (`Units\\AbilityMetaData.slk`) from the game folders
/// `folders`, as Warcraft III does: each folder's archives in the game's
/// order, the folder's own files over them, a later folder over an earlier
/// one (`storm_rs::archive::Layers`). `None` for a file none has.
pub fn game(folders: &[String]) -> impl Fn(&str) -> Option<String> {
    let folders: Vec<std::path::PathBuf> = folders.iter().map(std::path::PathBuf::from).collect();
    let (layers, _) = storm_rs::archive::Layers::open(&folders, &storm_rs::archive::WAR3_ARCHIVES);
    move |name| {
        layers
            .read(name)
            .map(|(bytes, _)| String::from_utf8_lossy(&bytes).into_owned())
    }
}
