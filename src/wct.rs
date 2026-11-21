//! The custom script tools, on `wct-rs`: `war3map.wct`, the code World
//! Editor's trigger editor keeps as text (the map's custom script and
//! each trigger converted to custom text). A file is read from disk
//! (`path`) or from inside a map (`archive`, `war3map.wct` by default);
//! it's written to a file on disk.

use serde_json::{Value, json};

use crate::source::{arg_str, archive_schema, file, path_schema};
use wct_rs::text::Script;

/// Every tool, for `tools/list`.
pub fn tools() -> Value {
    let path = path_schema("war3map.wct", "The custom script");
    let script = json!({"type": "string", "enum": ["jass", "lua"], "description": "The map's script language, for the marker comments (// or --); jass when left out"});
    json!([
        {
            "name": "wct_text",
            "description": "A map's custom script (war3map.wct) as code: the map's header code and each trigger's custom text, one after another, each after a `// @wct` marker line (`// @wct header`, `// @wct trigger`; `// @wct trigger none` for a trigger without code: most aren't custom text). Triggers are in the trigger editor's order (war3map.wtg's). Edit the code and give it to wct_write.",
            "inputSchema": {"type": "object", "properties": {"path": path, "archive": archive_schema(), "script": script}},
        },
        {
            "name": "wct_write",
            "description": "Writes a war3map.wct on disk from its text (as wct_text gives it), after checking the markers: every error, with its line, and nothing written. Keep the markers: there's one per trigger of war3map.wtg, in its order.",
            "inputSchema": {"type": "object", "properties": {
                "path": {"type": "string", "description": "The war3map.wct on disk"},
                "text": {"type": "string", "description": "The whole text"},
            }, "required": ["path", "text"]},
        },
    ])
}

/// Runs the tool `name`, if it's one of these.
pub fn call(name: &str, args: &Value) -> Option<Result<Value, String>> {
    Some(match name {
        "wct_text" => text(args),
        "wct_write" => write(args),
        _ => return None,
    })
}

fn text(args: &Value) -> Result<Value, String> {
    let (path, archive) = file(args, "war3map.wct")?;
    let script = match args.get("script").and_then(Value::as_str) {
        None | Some("jass") => Script::Jass,
        Some("lua") => Script::Lua,
        Some(s) => return Err(format!("`script` is jass or lua, not {s:?}")),
    };
    let f = wct_rs::parse(&crate::source::read_bytes(path, archive)?).map_err(|e| format!("{path}: {e}"))?;
    wct_rs::text::to_text(&f, script).map(Value::String).map_err(|e| format!("{path}: {e}"))
}

fn write(args: &Value) -> Result<Value, String> {
    let path = arg_str(args, "path")?;
    let f = wct_rs::text::from_text(arg_str(args, "text")?)
        .map_err(|errors| crate::source::text_errors(errors.iter().map(|e| (e.line, e.message.as_str()))))?;
    crate::source::write(path, f.to_bytes().map_err(|e| format!("nothing written: {e}"))?)?;
    Ok(json!({"written": path, "triggers": f.triggers.len()}))
}

#[cfg(test)]
#[path = "wct.test.rs"]
mod tests;
