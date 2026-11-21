//! The map info tools, on `w3i-rs`: `war3map.w3i`, what World Editor's
//! Scenario menu sets (name, description, players, forces, tech tree,
//! random tables ...). Both work on its text form, the same the wc3-rs
//! editor shows: BNI, a `[w3i]` section, then a section per player,
//! force, upgrade, tech and random table. A file is read from disk
//! (`path`) or from inside a map (`archive`, `war3map.w3i` by default);
//! it's written to a file on disk.

use serde_json::{Value, json};

/// Every tool, for `tools/list`.
pub fn tools() -> Value {
    json!([
        {
            "name": "w3i_text",
            "description": "A map's info (war3map.w3i: name, author, description, camera bounds, flags, loading screen, fog, players, forces, upgrades, techs, random tables) as text: BNI, a [w3i] section, then [player], [force], [upgrade], [tech], [unit_table], [item_table] sections. The comment after a value says what it means. Edit it and give it to w3i_write.",
            "inputSchema": {"type": "object", "properties": {
                "path": {"type": "string", "description": "war3map.w3i on disk, or with `archive` its name inside (war3map.w3i when left out)"},
                "archive": {"type": "string", "description": "A map (.w3x, .w3m) to read the file from"},
            }},
        },
        {
            "name": "w3i_write",
            "description": "Writes a war3map.w3i on disk from its text form (as w3i_text gives it). The text is checked first: every error, with its line, and nothing is written. The version decides which fields there are; a field left out takes its default (0, an empty string, -1 for loading_screen), so change the text w3i_text gave rather than writing a short one.",
            "inputSchema": {"type": "object", "properties": {
                "path": {"type": "string", "description": "The war3map.w3i on disk"},
                "text": {"type": "string", "description": "The whole text form"},
            }, "required": ["path", "text"]},
        },
    ])
}

/// Runs the tool `name`, if it's one of these; an error is a message for
/// the model.
pub fn call(name: &str, args: &Value) -> Option<Result<Value, String>> {
    Some(match name {
        "w3i_text" => text(args),
        "w3i_write" => write(args),
        _ => return None,
    })
}

fn text(args: &Value) -> Result<Value, String> {
    let archive = args.get("archive").and_then(Value::as_str);
    let path = match (args.get("path").and_then(Value::as_str), archive) {
        (Some(p), _) => p,
        (None, Some(_)) => "war3map.w3i",
        (None, None) => return Err("`path` is missing".into()),
    };
    let bytes = crate::source::read_bytes(path, archive)?;
    let info = w3i_rs::parse(&bytes).map_err(|e| format!("{path}: {e}"))?;
    Ok(Value::String(w3i_rs::text::to_text(&info)))
}

fn write(args: &Value) -> Result<Value, String> {
    let path = args.get("path").and_then(Value::as_str).ok_or("`path` is missing")?;
    let text = args.get("text").and_then(Value::as_str).ok_or("`text` is missing")?;
    let parsed = w3i_rs::text::from_text(text);
    let Some(info) = parsed.info else {
        let lines: Vec<String> = parsed
            .errors
            .iter()
            .map(|e| format!("line {}: {}", e.line, e.message))
            .collect();
        return Err(format!("nothing written:\n{}", lines.join("\n")));
    };
    let bytes = info.to_bytes().map_err(|e| format!("nothing written: {e}"))?;
    crate::source::write(path, &bytes)?;
    Ok(json!({
        "written": path,
        "version": info.version,
        "players": info.players.len(),
        "forces": info.forces.len(),
    }))
}

#[cfg(test)]
#[path = "w3i.test.rs"]
mod tests;
