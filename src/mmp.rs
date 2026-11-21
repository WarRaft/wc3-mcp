//! The minimap icon tools, on `mmp-rs`: `war3map.mmp`, the icons World
//! Editor draws on the map's minimap (gold mines, start locations, creep
//! camps ...). A file is read from disk (`path`) or from inside a map
//! (`archive`, `war3map.mmp` by default); it's written to a file on disk.

use serde_json::{Value, json};

use crate::source::{arg_str, archive_schema, file, path_schema, text_errors};

/// Every tool, for `tools/list`.
pub fn tools() -> Value {
    let path = path_schema("war3map.mmp", "The minimap icons");
    json!([
        {
            "name": "mmp_list",
            "description": "A map's minimap icons (war3map.mmp): each one's kind (a number and its name: gold mine, neutral building, start location ...), its place on the minimap (x, y: 0 to 255 from the top left) and color (red, green, blue, alpha).",
            "inputSchema": {"type": "object", "properties": {"path": path, "archive": archive_schema()}},
        },
        {
            "name": "mmp_text",
            "description": "A map's minimap icons as text: [mmp] version=0, then an [icon] section per icon (kind=, x=, y=, color=r,g,b,a). Edit it and give it to mmp_write.",
            "inputSchema": {"type": "object", "properties": {"path": path, "archive": archive_schema()}},
        },
        {
            "name": "mmp_write",
            "description": "Writes a war3map.mmp on disk from its text form (as mmp_text gives it), after checking it: every error, with its line, and nothing written.",
            "inputSchema": {"type": "object", "properties": {
                "path": {"type": "string", "description": "The war3map.mmp on disk"},
                "text": {"type": "string", "description": "The whole text form"},
            }, "required": ["path", "text"]},
        },
    ])
}

/// Runs the tool `name`, if it's one of these.
pub fn call(name: &str, args: &Value) -> Option<Result<Value, String>> {
    Some(match name {
        "mmp_list" => open(args).map(|f| {
            let icons: Vec<Value> = f
                .icons
                .iter()
                .map(|i| json!({"kind": i.kind, "kind_name": mmp_rs::kind_name(i.kind), "x": i.x, "y": i.y, "red": i.red, "green": i.green, "blue": i.blue, "alpha": i.alpha}))
                .collect();
            json!({"count": icons.len(), "icons": icons})
        }),
        "mmp_text" => open(args).map(|f| Value::String(mmp_rs::text::to_text(&f))),
        "mmp_write" => write(args),
        _ => return None,
    })
}

fn open(args: &Value) -> Result<mmp_rs::MinimapIcons, String> {
    let (path, archive) = file(args, "war3map.mmp")?;
    mmp_rs::parse(&crate::source::read_bytes(path, archive)?).map_err(|e| format!("{path}: {e}"))
}

fn write(args: &Value) -> Result<Value, String> {
    let path = arg_str(args, "path")?;
    let parsed = mmp_rs::text::from_text(arg_str(args, "text")?);
    let Some(icons) = parsed.file else {
        return Err(text_errors(parsed.errors.iter().map(|e| (e.line, e.message.as_str()))));
    };
    crate::source::write(path, icons.to_bytes().map_err(|e| format!("nothing written: {e}"))?)?;
    Ok(json!({"written": path, "icons": icons.icons.len()}))
}

#[cfg(test)]
#[path = "mmp.test.rs"]
mod tests;
