//! The MPQ tools, on `storm-rs`: archives (`.mpq`) and maps (`.w3x`,
//! `.w3m`), to look into and change.
//!
//! Names inside an archive use `\` (`Units\UnitBalance.slk`); `/` works
//! too. Masks are StormLib's: `*` and `?`, in any case, over the whole
//! name (`Units\*.slk`, `war3map.*`). An archive without a `(listfile)`
//! (most protected maps) only lets you find a file you can name:
//! `listfiles` gives more names, from files on disk with one per line.

use std::path::Path;

use serde_json::{Value, json};
use storm_rs::archive::create::MpqCreateOptions;
use storm_rs::archive::{MpqArchiveBuilder, extract_path, flag_names, mpq_name};
use storm_rs::verify::flags::{VERIFY_FILE_CRC, VERIFY_FILE_MD5, VERIFY_SECTOR_CRC};
use storm_rs::verify::verify_file;
use storm_rs::{AddFileOptions, FileInfo, MpqArchive, MpqArchiveWriter};

/// The most text `mpq_read` gives at once, in bytes.
const MAX_TEXT: usize = 200_000;
/// The most bytes `mpq_read` shows of a binary file, as hex.
const MAX_HEX: usize = 4096;

/// Every tool, for `tools/list`.
pub fn tools() -> Value {
    let archive =
        json!({"type": "string", "description": "The archive or map on disk: .mpq, .w3x, .w3m"});
    let listfiles = json!({"type": "array", "items": {"type": "string"}, "description": "Listfiles on disk (a name per line) with names the archive's own (listfile) lacks"});
    let masks = json!({"type": "array", "items": {"type": "string"}, "description": "Masks over the whole name: * and ?, any case (Units\\*.slk, war3map.*); all files when left out"});
    json!([
        {
            "name": "mpq_info",
            "description": "An MPQ archive's (or map's) header and tables: format version, sizes, how many files it has, how many of them have a known name, and whether it has a (listfile), (attributes) and (signature).",
            "inputSchema": {"type": "object", "properties": {"archive": archive, "listfiles": listfiles}, "required": ["archive"]},
        },
        {
            "name": "mpq_list",
            "description": "The files of an MPQ archive or map with a known name, sorted: name, size, compressed size, block flags. Filter with masks, page with offset / limit. `unnamed` is how many files have no known name: give `listfiles` to name them.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "archive": archive,
                    "masks": masks,
                    "listfiles": listfiles,
                    "offset": {"type": "integer", "minimum": 0, "description": "Files to skip (default 0)"},
                    "limit": {"type": "integer", "minimum": 1, "maximum": 5000, "description": "Files to give at most (default 500)"},
                },
                "required": ["archive"],
            },
        },
        {
            "name": "mpq_read",
            "description": "One file of an MPQ archive or map. Text (UTF-8) comes as text, from `offset` (bytes), at most 200,000 bytes at a time; anything else as its size and the hex of up to 4096 bytes from `offset`. For a SLK table use slk_read with `archive`; for a whole file on disk use mpq_extract.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "archive": archive,
                    "name": {"type": "string", "description": "The file's name inside: war3map.j, Units\\UnitBalance.slk"},
                    "offset": {"type": "integer", "minimum": 0},
                    "length": {"type": "integer", "minimum": 1, "description": "Bytes to give at most"},
                },
                "required": ["archive", "name"],
            },
        },
        {
            "name": "mpq_extract",
            "description": "Writes files of an MPQ archive or map into a folder on disk, the name's `\\` parts as folders. A name that would leave the folder (..\\x) is skipped and reported.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "archive": archive,
                    "dir": {"type": "string", "description": "The folder to write into; made if missing"},
                    "masks": masks,
                    "listfiles": listfiles,
                },
                "required": ["archive", "dir"],
            },
        },
        {
            "name": "mpq_verify",
            "description": "Checks files of an MPQ archive or map against the CRC32 and MD5 its (attributes) keep, and their sector CRCs: the files that fail and why.",
            "inputSchema": {"type": "object", "properties": {"archive": archive, "masks": masks, "listfiles": listfiles}, "required": ["archive"]},
        },
        {
            "name": "mpq_create",
            "description": "Makes a new MPQ archive (or map) of every file under a folder on disk, each named by its path relative to the folder, with a (listfile) and (attributes). Refuses to overwrite an existing file.",
            "inputSchema": {
                "type": "object",
                "properties": {"archive": archive, "dir": {"type": "string", "description": "The folder whose files go in"}},
                "required": ["archive", "dir"],
            },
        },
        {
            "name": "mpq_add",
            "description": "Adds a file to an MPQ archive or map, or replaces it: from a file on disk (`file`) or from text (`text`), compressed.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "archive": archive,
                    "name": {"type": "string", "description": "Its name inside: war3map.j, Units\\UnitBalance.slk"},
                    "file": {"type": "string", "description": "A file on disk to add"},
                    "text": {"type": "string", "description": "Text to add, as UTF-8"},
                },
                "required": ["archive", "name"],
            },
        },
        {
            "name": "mpq_remove",
            "description": "Removes files from an MPQ archive or map, by name.",
            "inputSchema": {
                "type": "object",
                "properties": {"archive": archive, "names": {"type": "array", "items": {"type": "string"}}},
                "required": ["archive", "names"],
            },
        },
        {
            "name": "mpq_rename",
            "description": "Renames a file inside an MPQ archive or map.",
            "inputSchema": {
                "type": "object",
                "properties": {"archive": archive, "from": {"type": "string"}, "to": {"type": "string"}},
                "required": ["archive", "from", "to"],
            },
        },
    ])
}

/// Runs the tool `name`, if it's one of these; an error is a message for
/// the model.
pub fn call(name: &str, args: &Value) -> Option<Result<Value, String>> {
    Some(match name {
        "mpq_info" => open(args).and_then(|a| info(&a)),
        "mpq_list" => open(args).and_then(|a| list(&a, args)),
        "mpq_read" => open(args).and_then(|a| read(&a, args)),
        "mpq_extract" => open(args).and_then(|a| extract(&a, args)),
        "mpq_verify" => open(args).and_then(|a| verify(&a, args)),
        "mpq_create" => create(args),
        "mpq_add" => add(args),
        "mpq_remove" => remove(args),
        "mpq_rename" => rename(args),
        _ => return None,
    })
}

fn string<'a>(args: &'a Value, key: &str) -> Result<&'a str, String> {
    args.get(key)
        .and_then(Value::as_str)
        .ok_or(format!("`{key}` is missing"))
}

fn strings(args: &Value, key: &str) -> Vec<String> {
    args.get(key)
        .and_then(Value::as_array)
        .map(|list| {
            list.iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

/// The archive, with the names of the given listfiles.
fn open(args: &Value) -> Result<MpqArchive, String> {
    let path = string(args, "archive")?;
    let mut archive = MpqArchive::open(path).map_err(|e| format!("{path}: {e}"))?;
    for listfile in strings(args, "listfiles") {
        archive
            .add_external_listfile(&listfile)
            .map_err(|e| format!("{listfile}: {e}"))?;
    }
    Ok(archive)
}

fn file_json(f: &FileInfo) -> Value {
    json!({"name": f.name, "size": f.file_size, "compressed": f.compressed_size, "flags": flag_names(f.flags)})
}

fn info(a: &MpqArchive) -> Result<Value, String> {
    let named = a.named_files().len();
    let unnamed = a.unnamed_count();
    let h = &a.header;
    Ok(json!({
        "format_version": h.format_version + 1,
        "offset": a.mpq_offset,
        "archive_size": h.archive_size,
        "sector_size": a.sector_size,
        "hash_table_size": h.hash_table_size,
        "block_table_size": h.block_table_size,
        "files": named + unnamed,
        "named": named,
        "unnamed": unnamed,
        "listfile": a.has_file("(listfile)"),
        "attributes": a.has_file("(attributes)"),
        "signature": a.has_file("(signature)"),
    }))
}

fn list(a: &MpqArchive, args: &Value) -> Result<Value, String> {
    let found = a.find_files(&strings(args, "masks"));
    let offset = args.get("offset").and_then(Value::as_u64).unwrap_or(0) as usize;
    let limit = args
        .get("limit")
        .and_then(Value::as_u64)
        .unwrap_or(500)
        .clamp(1, 5000) as usize;
    let files: Vec<Value> = found
        .iter()
        .skip(offset)
        .take(limit)
        .map(file_json)
        .collect();
    Ok(
        json!({"total": found.len(), "offset": offset, "files": files, "unnamed": a.unnamed_count()}),
    )
}

fn read(a: &MpqArchive, args: &Value) -> Result<Value, String> {
    let name = mpq_name(string(args, "name")?);
    if !a.has_file(&name) {
        return Err(format!("{}: no {name} in it", a.path));
    }
    let data = a.read_file(&name).map_err(|e| format!("{name}: {e}"))?;
    let offset = (args.get("offset").and_then(Value::as_u64).unwrap_or(0) as usize).min(data.len());
    let rest = &data[offset..];
    match std::str::from_utf8(&data) {
        Ok(_) => {
            let want = args
                .get("length")
                .and_then(Value::as_u64)
                .map_or(MAX_TEXT, |l| l as usize)
                .min(MAX_TEXT);
            // Stop on a character boundary.
            let mut end = want.min(rest.len());
            while end < rest.len() && !is_boundary(rest, end) {
                end -= 1;
            }
            let start = (0..=offset.min(data.len()))
                .rev()
                .find(|&i| is_boundary(&data, i))
                .unwrap_or(0);
            let text = String::from_utf8_lossy(&data[start..offset + end]).into_owned();
            Ok(
                json!({"name": name, "size": data.len(), "offset": start, "length": text.len(), "more": offset + end < data.len(), "text": text}),
            )
        }
        Err(_) => {
            let want = args
                .get("length")
                .and_then(Value::as_u64)
                .map_or(MAX_HEX, |l| l as usize)
                .min(MAX_HEX);
            let bytes = &rest[..want.min(rest.len())];
            let hex: Vec<String> = bytes
                .chunks(16)
                .map(|row| {
                    row.iter()
                        .map(|b| format!("{b:02x}"))
                        .collect::<Vec<_>>()
                        .join(" ")
                })
                .collect();
            Ok(
                json!({"name": name, "size": data.len(), "binary": true, "offset": offset, "length": bytes.len(), "more": offset + bytes.len() < data.len(), "hex": hex}),
            )
        }
    }
}

/// Is byte `i` the start of a UTF-8 character (or the end)?
fn is_boundary(bytes: &[u8], i: usize) -> bool {
    i >= bytes.len() || (bytes[i] as i8) >= -0x40
}

fn extract(a: &MpqArchive, args: &Value) -> Result<Value, String> {
    let dir = Path::new(string(args, "dir")?);
    let found = a.find_files(&strings(args, "masks"));
    let mut written = 0;
    let mut failed = Vec::new();
    for f in &found {
        let Some(path) = extract_path(dir, &f.name) else {
            failed.push(json!({"name": f.name, "error": "the name leaves the folder"}));
            continue;
        };
        let result = a
            .read_file(&f.name)
            .map_err(|e| e.to_string())
            .and_then(|data| {
                std::fs::create_dir_all(path.parent().unwrap_or(dir)).map_err(|e| e.to_string())?;
                std::fs::write(&path, data).map_err(|e| e.to_string())
            });
        match result {
            Ok(()) => written += 1,
            Err(e) => failed.push(json!({"name": f.name, "error": e})),
        }
    }
    Ok(json!({"dir": dir, "extracted": written, "failed": failed, "unnamed": a.unnamed_count()}))
}

fn verify(a: &MpqArchive, args: &Value) -> Result<Value, String> {
    let found = a.find_files(&strings(args, "masks"));
    let bad: Vec<Value> = found
        .iter()
        .map(|f| {
            (
                f,
                verify_file(
                    a,
                    &f.name,
                    VERIFY_FILE_CRC | VERIFY_FILE_MD5 | VERIFY_SECTOR_CRC,
                ),
            )
        })
        .filter(|(_, r)| !r.is_ok())
        .map(|(f, r)| json!({"name": f.name, "result": r.error_string()}))
        .collect();
    Ok(json!({"checked": found.len(), "bad": bad}))
}

fn create(args: &Value) -> Result<Value, String> {
    let archive = string(args, "archive")?;
    let dir = Path::new(string(args, "dir")?);
    if Path::new(archive).exists() {
        return Err(format!("{archive}: already exists"));
    }
    let count = storm_rs::archive::folder_files(dir)
        .map_err(|e| format!("{}: {e}", dir.display()))?
        .len();
    MpqArchiveBuilder::new(archive)
        .options(MpqCreateOptions::v1().create_attributes(true))
        .add_folder(dir)
        .map_err(|e| format!("{}: {e}", dir.display()))?
        .build()
        .map_err(|e| format!("{archive}: {e}"))?;
    Ok(json!({"archive": archive, "files": count}))
}

/// Opens `archive` for changing, lets `change` do it, and writes it back.
fn change(
    args: &Value,
    change: impl FnOnce(&mut MpqArchiveWriter) -> Result<(), String>,
) -> Result<(), String> {
    let archive = string(args, "archive")?;
    let mut writer = MpqArchiveWriter::open(archive).map_err(|e| format!("{archive}: {e}"))?;
    change(&mut writer)?;
    writer.finish().map_err(|e| format!("{archive}: {e}"))
}

fn add(args: &Value) -> Result<Value, String> {
    let name = mpq_name(string(args, "name")?);
    let data = match (
        args.get("file").and_then(Value::as_str),
        args.get("text").and_then(Value::as_str),
    ) {
        (Some(file), None) => std::fs::read(file).map_err(|e| format!("{file}: {e}"))?,
        (None, Some(text)) => text.as_bytes().to_vec(),
        _ => {
            return Err(
                "give the content as `file` (a file on disk) or `text`, one of them".into(),
            );
        }
    };
    let size = data.len();
    change(args, |w| {
        w.add_file(&name, &data, &AddFileOptions::default())
            .map_err(|e| format!("{name}: {e}"))
    })?;
    Ok(json!({"added": name, "size": size}))
}

fn remove(args: &Value) -> Result<Value, String> {
    let names: Vec<String> = strings(args, "names").iter().map(|n| mpq_name(n)).collect();
    if names.is_empty() {
        return Err("`names` is empty".into());
    }
    change(args, |w| {
        names
            .iter()
            .try_for_each(|n| w.remove_file(n).map_err(|e| format!("{n}: {e}")))
    })?;
    Ok(json!({"removed": names}))
}

fn rename(args: &Value) -> Result<Value, String> {
    let (from, to) = (
        mpq_name(string(args, "from")?),
        mpq_name(string(args, "to")?),
    );
    change(args, |w| {
        w.rename_file(&from, &to)
            .map_err(|e| format!("{from}: {e}"))
    })?;
    Ok(json!({"renamed": from, "to": to}))
}

#[cfg(test)]
#[path = "mpq.test.rs"]
mod tests;
