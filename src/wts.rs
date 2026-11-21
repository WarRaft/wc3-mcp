//! The string table tools, on `wts-rs`: `war3map.wts` (and
//! `war3campaign.wts`), the text a map's object data, triggers and info
//! name as `TRIGSTR_<n>`. A table is read from disk (`path`) or from inside
//! a map (`path` inside `archive`, `war3map.wts` by default); changes are
//! written to files on disk, keeping their byte-order mark and line breaks.

use serde_json::{Value, json};
use wts_rs::{Document, split_bom};

/// The most text `wts_list` shows of each string.
const PREVIEW: usize = 200;

/// Every tool, for `tools/list`.
pub fn tools() -> Value {
    let path = json!({"type": "string", "description": "The string table: war3map.wts on disk, or with `archive` its name inside (war3map.wts when left out)"});
    let archive = json!({"type": "string", "description": "A map or archive (.w3x, .w3m, .mpq) to read the table from"});
    json!([
        {
            "name": "wts_list",
            "description": "The strings of a map's string table (war3map.wts): number, the comments World Editor writes (what uses it), and the text (the first 200 characters; `length` is the whole). Filter by text in the comments or text, page with offset / limit.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": path,
                    "archive": archive,
                    "contains": {"type": "string", "description": "Keep strings whose text or comments contain this, in any case"},
                    "offset": {"type": "integer", "minimum": 0},
                    "limit": {"type": "integer", "minimum": 1, "maximum": 5000, "description": "Strings to give at most (default 200)"},
                },
            },
        },
        {
            "name": "wts_get",
            "description": "Strings of a map's string table by number, whole: their comments and text.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": path,
                    "archive": archive,
                    "ids": {"type": "array", "items": {"type": "integer", "minimum": 0}, "description": "String numbers: 3 for TRIGSTR_003"},
                },
                "required": ["ids"],
            },
        },
        {
            "name": "wts_resolve",
            "description": "Text with every TRIGSTR_<n> that a map's string table has replaced by that string's text.",
            "inputSchema": {
                "type": "object",
                "properties": {"path": path, "archive": archive, "text": {"type": "string"}},
                "required": ["text"],
            },
        },
        {
            "name": "wts_set",
            "description": "Writes one string of a string table on disk (war3map.wts), or adds it, or removes it (text null), changing only that string's lines. A line that is only `}` can't be in a string.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "The war3map.wts on disk"},
                    "id": {"type": "integer", "minimum": 0},
                    "text": {"type": ["string", "null"], "description": "The new text (\\n between lines); null removes the string"},
                },
                "required": ["path", "id", "text"],
            },
        },
        {
            "name": "wts_lint",
            "description": "A string table's problems: parse errors (line, column) and lint warnings (a number defined twice).",
            "inputSchema": {"type": "object", "properties": {"path": path, "archive": archive}},
        },
    ])
}

/// Runs the tool `name`, if it's one of these; an error is a message for
/// the model.
pub fn call(name: &str, args: &Value) -> Option<Result<Value, String>> {
    Some(match name {
        "wts_list" => open(args).and_then(|d| list(&d, args)),
        "wts_get" => open(args).and_then(|d| get(&d, args)),
        "wts_resolve" => open(args).and_then(|d| resolve(&d, args)),
        "wts_set" => set(args),
        "wts_lint" => open(args).map(|d| lint(&d)),
        _ => return None,
    })
}

/// The table at `args.path`, or `war3map.wts` (or `args.path`) inside
/// `args.archive`.
fn open(args: &Value) -> Result<Document, String> {
    let archive = args.get("archive").and_then(Value::as_str);
    let path = match (args.get("path").and_then(Value::as_str), archive) {
        (Some(p), _) => p,
        (None, Some(_)) => "war3map.wts",
        (None, None) => return Err("`path` is missing".into()),
    };
    let text = crate::source::read(path, archive)?;
    Ok(wts_rs::parse(split_bom(&text).0))
}

/// The map's string table, for resolving `TRIGSTR_<n>` elsewhere: inside
/// `archive`, or at `strings` on disk; `None` when there's none.
pub fn table_for(args: &Value) -> Option<Document> {
    if let Some(strings) = args.get("strings").and_then(Value::as_str) {
        return open(&json!({"path": strings})).ok();
    }
    let archive = args.get("archive").and_then(Value::as_str)?;
    open(&json!({"archive": archive})).ok()
}

fn entry_json(e: &wts_rs::Entry, preview: bool) -> Value {
    let text = if preview && e.text.chars().count() > PREVIEW {
        e.text.chars().take(PREVIEW).collect::<String>() + "…"
    } else {
        e.text.clone()
    };
    json!({"id": e.id, "comments": e.comments, "text": text, "length": e.text.chars().count()})
}

fn list(doc: &Document, args: &Value) -> Result<Value, String> {
    let needle = args
        .get("contains")
        .and_then(Value::as_str)
        .map(str::to_lowercase);
    let offset = args.get("offset").and_then(Value::as_u64).unwrap_or(0) as usize;
    let limit = args
        .get("limit")
        .and_then(Value::as_u64)
        .unwrap_or(200)
        .clamp(1, 5000) as usize;
    let matching: Vec<&wts_rs::Entry> = doc
        .strings
        .iter()
        .filter(|e| {
            needle.as_ref().is_none_or(|n| {
                e.text.to_lowercase().contains(n)
                    || e.comments.iter().any(|c| c.to_lowercase().contains(n))
            })
        })
        .collect();
    let strings: Vec<Value> = matching
        .iter()
        .skip(offset)
        .take(limit)
        .map(|e| entry_json(e, true))
        .collect();
    Ok(json!({"total": matching.len(), "offset": offset, "strings": strings}))
}

fn get(doc: &Document, args: &Value) -> Result<Value, String> {
    let ids = args
        .get("ids")
        .and_then(Value::as_array)
        .ok_or("`ids` is missing")?;
    let found: Vec<Value> = ids
        .iter()
        .map(|id| {
            let n = id.as_u64().and_then(|n| u32::try_from(n).ok());
            match n.and_then(|n| doc.get(n)) {
                Some(e) => entry_json(e, false),
                None => json!({"id": id, "missing": true}),
            }
        })
        .collect();
    Ok(json!({"strings": found}))
}

fn resolve(doc: &Document, args: &Value) -> Result<Value, String> {
    let text = args
        .get("text")
        .and_then(Value::as_str)
        .ok_or("`text` is missing")?;
    Ok(json!(doc.resolve(text)))
}

fn set(args: &Value) -> Result<Value, String> {
    let path = args
        .get("path")
        .and_then(Value::as_str)
        .ok_or("`path` is missing")?;
    let id = args
        .get("id")
        .and_then(Value::as_u64)
        .and_then(|n| u32::try_from(n).ok())
        .ok_or("`id` is a number")?;
    let file = crate::source::read(path, None)?;
    let (src, bom) = split_bom(&file);
    let doc = wts_rs::parse(src);
    let before = doc.get(id).map(|e| e.text.clone());
    let edits = match args.get("text") {
        None | Some(Value::Null) => wts_rs::remove_string(&doc, id)?,
        Some(Value::String(text)) => wts_rs::set_string(&doc, id, text)?,
        Some(_) => return Err("`text` is a string or null".into()),
    };
    let out = wts_rs::apply(src, &edits);
    let after = wts_rs::parse(&out).get(id).map(|e| e.text.clone());
    if !edits.is_empty() {
        let out = if bom { format!("\u{FEFF}{out}") } else { out };
        crate::source::write(path, out)?;
    }
    Ok(json!({"id": id, "before": before, "after": after, "changed": !edits.is_empty()}))
}

fn lint(doc: &Document) -> Value {
    let errors: Vec<Value> = doc
        .errors
        .iter()
        .map(|e| json!({"line": e.line, "column": e.col, "message": e.message}))
        .collect();
    let warnings: Vec<Value> = wts_rs::lint(doc)
        .iter()
        .map(|d| json!({"line": d.line, "rule": d.rule, "message": d.message}))
        .collect();
    json!({"strings": doc.strings.len(), "errors": errors, "warnings": warnings})
}

#[cfg(test)]
#[path = "wts.test.rs"]
mod tests;
