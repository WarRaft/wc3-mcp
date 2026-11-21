use super::*;

fn temp(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("wc3-mcp-wpm-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn run(name: &str, args: Value) -> Result<Value, String> {
    call(name, &args).expect("a pathing tool")
}

/// 4x2 cells: the top row unwalkable.
fn file() -> Vec<u8> {
    Pathing { version: 0, width: 4, height: 2, cells: vec![0x40, 0x48, 0x40, 0x40, 0xCA, 0xCA, 0xCA, 0x20] }
        .to_bytes()
        .unwrap()
}

#[test]
fn info_image_cells_set() {
    let dir = temp("all");
    let path = dir.join("war3map.wpm");
    std::fs::write(&path, file()).unwrap();

    let info = run("wpm_info", json!({"path": path})).unwrap();
    assert_eq!((info["width"].clone(), info["height"].clone()), (json!(4), json!(2)));
    assert_eq!(info["flags"][1], json!({"bit": 2, "name": "no walking", "cells": 3, "percent": 37.5}));
    assert!(info.get("bounds").is_none());

    // With the terrain beside it: world positions.
    let t = w3e_rs::Terrain {
        version: 11, tileset: b'L', custom_tilesets: 0, ground: vec![], cliffs: vec![],
        width: 2, height: 2, offset_x: -64.0, offset_y: -32.0,
        points: vec![w3e_rs::Point::default(); 4],
    };
    std::fs::write(dir.join("war3map.w3e"), t.to_bytes().unwrap()).unwrap();
    let info = run("wpm_info", json!({"path": path})).unwrap();
    assert_eq!(info["bounds"], json!({"left": -64.0, "bottom": -32.0, "right": 64.0, "top": 32.0}));
    let cells = run("wpm_cells", json!({"path": path, "x": 3, "y": 1})).unwrap();
    assert_eq!(cells["cells"][0], json!({"x": 3, "y": 1, "flags": 32, "names": ["blight"], "world_x": 32.0, "world_y": 0.0}));

    // A picture: text, then the PNG.
    let image = run("wpm_image", json!({"path": path, "scale": 2})).unwrap();
    let content = image["mcp_content"].as_array().unwrap();
    assert!(content[0]["text"].as_str().unwrap().contains("8x4"), "{content:?}");
    assert_eq!(content[1]["type"], "image");
    assert_eq!(content[1]["mimeType"], "image/png");
    assert!(content[1]["data"].as_str().unwrap().starts_with("iVBORw0KGgo"));
    let flag = run("wpm_image", json!({"path": path, "show": "blight", "x": 2, "width": 2})).unwrap();
    assert!(flag["mcp_content"][0]["text"].as_str().unwrap().contains("white where \"blight\" is set"));
    assert!(run("wpm_image", json!({"path": path, "show": "nope"})).unwrap_err().contains("isn't a flag"));

    // Set and clear, on disk.
    let out = run("wpm_set", json!({"path": path, "x": 0, "y": 0, "width": 2, "set": ["no building"], "clear": [64]})).unwrap();
    assert_eq!(out, json!({"cells": 2, "changed": 2}));
    let back = wpm_rs::parse(&std::fs::read(&path).unwrap()).unwrap();
    assert_eq!(&back.cells[..2], &[0x08, 0x08]);
    assert!(run("wpm_set", json!({"path": path, "x": 0, "y": 0, "set": [2], "clear": ["no walking"]})).unwrap_err().contains("both"));
    assert!(run("wpm_cells", json!({"path": path, "x": 4, "y": 0})).unwrap_err().contains("don't fit"));
}
