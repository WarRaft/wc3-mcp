//! The shadow map tool, on `shd-rs`: `war3map.shd`, where trees and
//! buildings cast shadow on the ground, 16 cells to a terrain cell. The
//! file has no header: its size is the terrain's, from the
//! `war3map.w3e` beside it. Read from disk (`path`) or from inside a map
//! (`archive`, `war3map.shd` by default).

use serde_json::{Value, json};

use crate::source::{archive_schema, file, path_schema};

/// The longest side the picture is drawn at, in pixels.
const MAX_SIDE: u32 = 4096;

/// Every tool, for `tools/list`.
pub fn tools() -> Value {
    json!([
        {
            "name": "shd_image",
            "description": "A picture of a map's shadow map (war3map.shd), a PNG, north up: dark where trees and buildings cast shadow. Its size comes from war3map.w3e beside it (4 x 4 cells per terrain cell). `scale`: pixels per cell (default: so that the picture is at least 512 across, at most 4096).",
            "inputSchema": {"type": "object", "properties": {
                "path": path_schema("war3map.shd", "The shadow map"),
                "archive": archive_schema(),
                "scale": {"type": "integer", "minimum": 1, "maximum": 16},
            }},
        },
    ])
}

/// Runs the tool `name`, if it's one of these.
pub fn call(name: &str, args: &Value) -> Option<Result<Value, String>> {
    Some(match name {
        "shd_image" => image(args),
        _ => return None,
    })
}

fn image(args: &Value) -> Result<Value, String> {
    let (path, archive) = file(args, "war3map.shd")?;
    let bytes = crate::source::read_bytes(path, archive)?;
    let terrain = crate::source::sibling(path, archive, "war3map.w3e")
        .and_then(|b| w3e_rs::parse(&b).ok())
        .ok_or("a shadow map's size is its terrain's, and war3map.w3e beside it can't be read")?;
    let (w, h) = shd_rs::size_for_terrain(terrain.width, terrain.height);
    let shadows = shd_rs::parse(&bytes, w, h).map_err(|e| format!("{path}: {e}"))?;
    let longest = w.max(h).max(1);
    let scale = match args.get("scale").and_then(Value::as_u64) {
        Some(s) => s as u32,
        None => 512u32.div_ceil(longest).max(1),
    }
    .clamp(1, (MAX_SIDE / longest).max(1));
    let rgba = shadows.to_rgba();
    let (pw, ph) = (w * scale, h * scale);
    let mut out = vec![0u8; (pw * ph * 4) as usize];
    for y in 0..ph {
        for x in 0..pw {
            let from = (((y / scale) * w + x / scale) * 4) as usize;
            let to = ((y * pw + x) * 4) as usize;
            out[to..to + 4].copy_from_slice(&rgba[from..from + 4]);
        }
    }
    let shadowed = shadows.cells.iter().filter(|&&c| c != 0).count();
    let text = format!(
        "Shadows, {w}x{h} cells (terrain {}x{}), north up, {scale} pixels per cell: dark where there's shadow ({shadowed} cells, {:.1}%).",
        terrain.width,
        terrain.height,
        100.0 * shadowed as f64 / shadows.cells.len().max(1) as f64
    );
    Ok(json!({"mcp_content": [
        {"type": "text", "text": text},
        {"type": "image", "data": crate::png::base64(&crate::png::encode(pw, ph, &out)), "mimeType": "image/png"},
    ]}))
}

#[cfg(test)]
#[path = "shd.test.rs"]
mod tests;
