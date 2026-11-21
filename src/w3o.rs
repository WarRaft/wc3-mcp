//! The object collection tools, on `w3o-rs`: `.w3o`, what World Editor's
//! Object Editor exports (File > Export All Object Data), every kind of
//! object data in one file. The obj_ tools work on one kind's file:
//! w3o_split writes each kind's out, w3o_join puts them back together.

use serde_json::{Value, json};

use crate::source::arg_str;

/// Every tool, for `tools/list`.
pub fn tools() -> Value {
    json!([
        {
            "name": "w3o_info",
            "description": "An object collection (.w3o, World Editor's Export All Object Data): which kinds it has, and how many standard objects changed and custom objects each.",
            "inputSchema": {"type": "object", "properties": {"path": {"type": "string", "description": "The .w3o"}}, "required": ["path"]},
        },
        {
            "name": "w3o_split",
            "description": "Writes each kind of an object collection (.w3o) to `folder` as its own file (war3map.w3u, .w3t, .w3b, .w3d, .w3a, .w3h, .w3q), for the obj_ tools. Existing files there are replaced.",
            "inputSchema": {"type": "object", "properties": {
                "path": {"type": "string", "description": "The .w3o"},
                "folder": {"type": "string", "description": "An existing folder"},
            }, "required": ["path", "folder"]},
        },
        {
            "name": "w3o_join",
            "description": "Writes an object collection (.w3o) from a map's object data: each kind's war3map.w3? in `folder`, or inside `archive` (a map). A kind without its file isn't in the collection. The .w3o is created or replaced.",
            "inputSchema": {"type": "object", "properties": {
                "path": {"type": "string", "description": "The .w3o to write"},
                "folder": {"type": "string", "description": "A folder with war3map.w3u ..."},
                "archive": {"type": "string", "description": "Or a map (.w3x, .w3m) to take them from"},
            }, "required": ["path"]},
        },
    ])
}

/// Runs the tool `name`, if it's one of these.
pub fn call(name: &str, args: &Value) -> Option<Result<Value, String>> {
    Some(match name {
        "w3o_info" => open(args).map(|c| info(&c)),
        "w3o_split" => split(args),
        "w3o_join" => join(args),
        _ => return None,
    })
}

fn open(args: &Value) -> Result<w3o_rs::Collection, String> {
    let path = arg_str(args, "path")?;
    w3o_rs::parse(&std::fs::read(path).map_err(|e| format!("{path}: {e}"))?).map_err(|e| format!("{path}: {e}"))
}

fn info(c: &w3o_rs::Collection) -> Value {
    let kinds: Vec<Value> = c
        .kinds
        .iter()
        .flatten()
        .map(|d| json!({"kind": d.kind.title(), "file": format!("war3map.{}", d.kind.extension()), "changed": d.originals.len(), "custom": d.customs.len()}))
        .collect();
    json!({"version": c.version, "kinds": kinds})
}

fn split(args: &Value) -> Result<Value, String> {
    let c = open(args)?;
    let folder = std::path::Path::new(arg_str(args, "folder")?);
    if !folder.is_dir() {
        return Err(format!("{}: not a folder", folder.display()));
    }
    let mut written = Vec::new();
    for d in c.kinds.iter().flatten() {
        let file = folder.join(format!("war3map.{}", d.kind.extension()));
        std::fs::write(&file, d.to_bytes().map_err(|e| e.to_string())?).map_err(|e| format!("{}: {e}", file.display()))?;
        written.push(file.to_string_lossy().into_owned());
    }
    Ok(json!({"written": written}))
}

fn join(args: &Value) -> Result<Value, String> {
    let path = arg_str(args, "path")?;
    let archive = args.get("archive").and_then(Value::as_str);
    let folder = args.get("folder").and_then(Value::as_str);
    if archive.is_none() == folder.is_none() {
        return Err("give `folder` or `archive`".into());
    }
    let mut c = w3o_rs::Collection::new();
    for kind in w3o_rs::KINDS {
        let name = format!("war3map.{}", kind.extension());
        let bytes = match (folder, archive) {
            (Some(f), _) => std::fs::read(std::path::Path::new(f).join(&name)).ok(),
            (_, a) => crate::source::read_bytes(&name, a).ok(),
        };
        if let Some(bytes) = bytes {
            let data = w3abdhqtu_rs::parse(&bytes, kind).map_err(|e| format!("{name}: {e}"))?;
            c.set(kind, Some(data));
        }
    }
    if c.kinds.iter().all(Option::is_none) {
        return Err("no object data there (war3map.w3u, .w3t, .w3b, .w3d, .w3a, .w3h, .w3q)".into());
    }
    std::fs::write(path, c.to_bytes().map_err(|e| e.to_string())?).map_err(|e| format!("{path}: {e}"))?;
    Ok(json!({"written": path, "kinds": info(&c)["kinds"]}))
}

#[cfg(test)]
#[path = "w3o.test.rs"]
mod tests;
