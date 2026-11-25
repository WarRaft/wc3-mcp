//! The model tool, on `mdx-rs`: what's in a `.mdx`. A model is read from
//! disk (`path`), from inside an archive or a map (`path` inside
//! `archive`), or, when neither has it, through the game's folders and
//! the map's roots (`game`, `roots`, by default the project's), the way
//! the game finds it (`Units\Human\Footman\Footman.mdx`).

use serde_json::{Value, json};

/// Every tool, for `tools/list`.
pub fn tools() -> Value {
    json!([{
        "name": "mdx_info",
        "description": "What's in a model (.mdx): version, name, extent; its geosets (vertices, triangles, material, texture coordinate sets, extent), materials and their layers (filter mode, texture, alpha), textures (path, or the game's replaceable one by id: 1 team colour, 2 team glow, 11 cliff ...), sequences (name, frames, looping), and its other chunks with their sizes. Read from disk, from inside an archive or map, or through the game's files.",
        "inputSchema": {"type": "object", "properties": {
            "path": {"type": "string", "description": "The model: a .mdx on disk, its name inside `archive`, or its name in the game's files (Units\\Human\\Footman\\Footman.mdx)"},
            "archive": {"type": "string", "description": "An MPQ archive or a map (.mpq, .w3x, .w3m) to read `path` from"},
            "game": crate::source::game_schema(),
            "roots": crate::source::roots_schema(),
            "project": crate::source::project_schema(),
        }, "required": ["path"]},
    }])
}

/// Runs the tool `name`, if it's this one.
pub fn call(name: &str, args: &Value) -> Option<Result<Value, String>> {
    (name == "mdx_info").then(|| info(args))
}

fn read(args: &Value) -> Result<Vec<u8>, String> {
    let path = crate::source::arg_str(args, "path")?;
    let archive = args.get("archive").and_then(Value::as_str);
    if archive.is_some() || std::path::Path::new(path).is_file() {
        return crate::source::read_bytes(path, archive);
    }
    let lookup = crate::source::lookup(args);
    let (game, _) = lookup.open();
    game.read(path)
        .map(|(bytes, _)| bytes)
        .ok_or_else(|| format!("{path}: not on disk, nor in {}", crate::source::layers_named(args)))
}

fn extent(e: &mdx_rs::Extent) -> Value {
    json!({"radius": e.radius, "min": e.min, "max": e.max})
}

fn info(args: &Value) -> Result<Value, String> {
    let path = crate::source::arg_str(args, "path")?;
    let m = mdx_rs::parse(&read(args)?).map_err(|e| format!("{path}: {e}"))?;
    let geosets: Vec<Value> = m
        .geosets()
        .iter()
        .map(|g| {
            json!({
                "vertices": g.vertices.len(),
                "triangles": g.faces.len() / 3,
                "material": g.material_id,
                "uv_sets": g.uv_sets.len(),
                "extent": extent(&g.extent),
            })
        })
        .collect();
    let materials: Vec<Value> = m
        .materials()
        .iter()
        .map(|mat| {
            json!({
                "priority_plane": mat.priority_plane,
                "flags": mat.flags,
                "shader": mat.shader.as_ref().map(|s| s.text()),
                "layers": mat.layers.iter().map(|l| json!({"filter_mode": l.filter_mode, "shading": l.shading, "texture": l.texture_id, "alpha": l.alpha})).collect::<Vec<_>>(),
            })
        })
        .collect();
    let textures: Vec<Value> = m
        .textures()
        .iter()
        .map(|t| json!({"path": t.path.text(), "replaceable_id": t.replaceable_id, "flags": t.flags}))
        .collect();
    let sequences: Vec<Value> = m
        .sequences()
        .iter()
        .map(|s| json!({"name": s.name.text(), "start": s.start, "end": s.end, "looping": s.flags & 1 == 0, "rarity": s.rarity, "move_speed": s.move_speed}))
        .collect();
    let others: Vec<Value> = m
        .chunks
        .iter()
        .filter_map(|c| match c {
            mdx_rs::Chunk::Other { tag, data } => Some(json!({"chunk": String::from_utf8_lossy(tag), "bytes": data.len()})),
            _ => None,
        })
        .collect();
    let info = m.info();
    Ok(json!({
        "version": m.version(),
        "name": info.map(|i| i.name.text()),
        "animation_file": info.map(|i| i.animation_file.text()).filter(|s| !s.is_empty()),
        "extent": info.map(|i| extent(&i.extent)),
        "geosets": geosets,
        "materials": materials,
        "textures": textures,
        "sequences": sequences,
        "global_sequences": m.global_sequences(),
        "pivots": m.pivots().len(),
        "other_chunks": others,
    }))
}

#[cfg(test)]
#[path = "mdx.test.rs"]
mod tests;
