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
        "description": "What's in a model (.mdx, or its text form .mdl): version, name, extent; its geosets (vertices, triangles, material, texture coordinate sets, extent), materials and their layers (filter mode, texture, alpha), textures (path, or the game's replaceable one by id: 1 team colour, 2 team glow, 11 cliff ...), sequences (name, frames, looping), its skeleton (bones and helpers: parent, the tracks that move each), its particle emitters (texture, rate, life, speed), its lights (the day and night models' sun), its ribbons, and its other chunks with their sizes. Read from disk, from inside an archive or map, or through the game's files.",
        "inputSchema": {"type": "object", "properties": {
            "path": {"type": "string", "description": "The model: a .mdx (or .mdl) on disk, its name inside `archive`, or its name in the game's files (Units\\Human\\Footman\\Footman.mdx)"},
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

/// A light's kind by its number.
fn light_kind(kind: u32) -> &'static str {
    ["omni", "directional", "ambient"].get(kind as usize).copied().unwrap_or("other")
}

fn extent(e: &mdx_rs::Extent) -> Value {
    json!({"radius": e.radius, "min": e.min, "max": e.max})
}

fn info(args: &Value) -> Result<Value, String> {
    let path = crate::source::arg_str(args, "path")?;
    let m = mdx_rs::parse_file(&read(args)?).map_err(|e| format!("{path}: {e}"))?;
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
        "lights": m.lights().iter().map(|l| json!({
            "name": l.node.name.text(),
            "kind": light_kind(l.kind),
            "color_bgr": l.color,
            "intensity": l.intensity,
            "ambient_color_bgr": l.ambient_color,
            "ambient_intensity": l.ambient_intensity,
            "tracks": l.tracks.iter().map(|(t, _)| String::from_utf8_lossy(t).into_owned()).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
        "ribbons": m.ribbons().iter().map(|r| json!({
            "name": r.node.name.text(),
            "color": r.color,
            "alpha": r.alpha,
            "height_above": r.height_above,
            "height_below": r.height_below,
            "life_span": r.life_span,
            "emission_rate": r.emission_rate,
            "material": r.material_id,
            "gravity": r.gravity,
            "tracks": r.tracks.iter().map(|(t, _)| String::from_utf8_lossy(t).into_owned()).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
        "particle_emitters": m.emitters2().iter().map(|e| json!({
            "name": e.node.name.text(),
            "texture": e.texture_id,
            "emission_rate": e.emission_rate,
            "life_span": e.life_span,
            "speed": e.speed,
            "filter_mode": e.filter_mode,
        })).collect::<Vec<_>>(),
        // The skeleton: each node, its parent and which tracks move it.
        "nodes": m.bones().iter().map(|b| (&b.node, "bone")).chain(m.helpers().iter().map(|h| (h, "helper"))).map(|(n, kind)| json!({
            "kind": kind,
            "name": n.name.text(),
            "id": n.object_id,
            "parent": n.parent(),
            "tracks": n.tracks.iter().map(|t| match t {
                mdx_rs::NodeTrack::Translation(t) => format!("translation ({} keys)", t.keys.len()),
                mdx_rs::NodeTrack::Rotation(t) => format!("rotation ({} keys)", t.keys.len()),
                mdx_rs::NodeTrack::Scaling(t) => format!("scaling ({} keys)", t.keys.len()),
            }).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
        "pivots": m.pivots().len(),
        "other_chunks": others,
    }))
}

#[cfg(test)]
#[path = "mdx.test.rs"]
mod tests;
