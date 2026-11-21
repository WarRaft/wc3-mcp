//! The import list tools, on `imp-rs`: `war3map.imp`, the files a map
//! has imported (World Editor's Import Manager). A file is read from
//! disk (`path`) or from inside a map (`archive`, `war3map.imp` by
//! default); it's written to a file on disk.

use serde_json::{Value, json};

use crate::source::{arg_str, archive_schema, file, path_schema, text_errors};

/// Every tool, for `tools/list`.
pub fn tools() -> Value {
    let path = path_schema("war3map.imp", "The import list");
    json!([
        {
            "name": "imp_list",
            "description": "A map's imported files (war3map.imp): each one's path as stored, where it is in the map (war3map\\Imported\\ and its name unless it has a custom path), and its flags by name.",
            "inputSchema": {"type": "object", "properties": {"path": path, "archive": archive_schema()}},
        },
        {
            "name": "imp_text",
            "description": "A map's import list as text: [imp] version=, then a file=<flags>,\"path\" line per file (version 0: file=\"path\"). Edit it and give it to imp_write. The list only names the files; the files themselves are in the map (mpq_add).",
            "inputSchema": {"type": "object", "properties": {"path": path, "archive": archive_schema()}},
        },
        {
            "name": "imp_write",
            "description": "Writes a war3map.imp on disk from its text form (as imp_text gives it), after checking it: every error, with its line, and nothing written.",
            "inputSchema": {"type": "object", "properties": {
                "path": {"type": "string", "description": "The war3map.imp on disk"},
                "text": {"type": "string", "description": "The whole text form"},
            }, "required": ["path", "text"]},
        },
    ])
}

/// Runs the tool `name`, if it's one of these.
pub fn call(name: &str, args: &Value) -> Option<Result<Value, String>> {
    Some(match name {
        "imp_list" => open(args).map(|f| {
            let files: Vec<Value> = f
                .files
                .iter()
                .map(|i| {
                    let flags: Vec<&str> = imp_rs::FLAGS.iter().filter(|(b, _)| i.flags & b != 0).map(|(_, n)| *n).collect();
                    json!({"path": i.path, "in_map": i.path_in_map(), "flags": i.flags, "flag_names": flags})
                })
                .collect();
            json!({"version": f.version, "count": files.len(), "files": files})
        }),
        "imp_text" => open(args).map(|f| Value::String(imp_rs::text::to_text(&f))),
        "imp_write" => write(args),
        _ => return None,
    })
}

fn open(args: &Value) -> Result<imp_rs::Imports, String> {
    let (path, archive) = file(args, "war3map.imp")?;
    imp_rs::parse(&crate::source::read_bytes(path, archive)?).map_err(|e| format!("{path}: {e}"))
}

fn write(args: &Value) -> Result<Value, String> {
    let path = arg_str(args, "path")?;
    let parsed = imp_rs::text::from_text(arg_str(args, "text")?);
    let Some(list) = parsed.list else {
        return Err(text_errors(parsed.errors.iter().map(|e| (e.line, e.message.as_str()))));
    };
    crate::source::write(path, list.to_bytes().map_err(|e| format!("nothing written: {e}"))?)?;
    Ok(json!({"written": path, "files": list.files.len()}))
}

#[cfg(test)]
#[path = "imp.test.rs"]
mod tests;
