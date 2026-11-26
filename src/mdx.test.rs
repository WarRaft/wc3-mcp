use super::*;
use mdx_rs::{Chunk, Extent, Fixed, Geoset, Model, Sequence, Texture};

fn model() -> Vec<u8> {
    let g = Geoset {
        vertices: vec![[0.0; 3]; 3],
        normals: vec![[0.0, 0.0, 1.0]; 3],
        face_types: vec![4],
        face_groups: vec![3],
        faces: vec![0, 1, 2],
        vertex_groups: vec![0; 3],
        matrix_groups: vec![1],
        matrix_indices: vec![0],
        uv_sets: vec![vec![[0.0; 2]; 3]],
        ..Default::default()
    };
    let stand = Sequence { name: Fixed::new("Stand", 80), start: 0, end: 1000, move_speed: 0.0, flags: 0, rarity: 0.0, sync_point: 0, extent: Extent::default() };
    Model {
        chunks: vec![
            Chunk::Version(800),
            Chunk::Sequences(vec![stand]),
            Chunk::Textures(vec![Texture { replaceable_id: 11, path: Fixed::new("", 260), flags: 0 }]),
            Chunk::Geosets(vec![g]),
            Chunk::Other { tag: *b"ATCH", data: vec![0; 8] },
        ],
    }
    .to_bytes()
    .unwrap()
}

#[test]
fn what_a_model_has_from_disk_or_through_the_roots() {
    let dir = std::env::temp_dir().join(format!("wc3-mcp-mdx-{}", std::process::id()));
    std::fs::create_dir_all(dir.join("Doodads")).unwrap();
    std::fs::write(dir.join("Doodads/Cliff.mdx"), model()).unwrap();
    let on_disk = call("mdx_info", &json!({"path": dir.join("Doodads/Cliff.mdx")})).unwrap().unwrap();
    assert_eq!(on_disk["geosets"][0]["triangles"], 1);
    assert_eq!(on_disk["textures"][0]["replaceable_id"], 11);
    assert_eq!(on_disk["sequences"][0]["name"], "Stand");
    assert_eq!(on_disk["other_chunks"][0]["chunk"], "ATCH");
    assert_eq!(on_disk["nodes"], json!([]));
    // By the game's name, through a root.
    let looked_up = call("mdx_info", &json!({"path": "Doodads\\Cliff.mdx", "game": [], "roots": [dir]})).unwrap().unwrap();
    assert_eq!(looked_up, on_disk);
    let missing = call("mdx_info", &json!({"path": "Doodads\\None.mdx", "game": [], "roots": [dir]})).unwrap();
    assert!(missing.unwrap_err().contains("not on disk"));
    let _ = std::fs::remove_dir_all(dir);
}

/// A model's text form: a `.mdx` not found read as the `.mdl` of its
/// name, its lines taken by offset and limit.
#[test]
fn a_models_text_form() {
    let dir = std::env::temp_dir().join(format!("wc3-mcp-mdl-{}", std::process::id()));
    std::fs::create_dir_all(dir.join("Doodads")).unwrap();
    let mdl = "Version { FormatVersion 800, }\nModel \"Rock\" { BlendTime 150, }\nSequences 1 { Anim \"Stand\" { Interval { 0, 1000 }, } }\nPivotPoints 1 { { 0, 0, 0 }, }\n";
    std::fs::write(dir.join("Doodads/Rock.mdl"), mdl).unwrap();
    let all = call("mdx_text", &json!({"path": "Doodads\\Rock.mdx", "game": [], "roots": [dir]})).unwrap().unwrap();
    let text = all["text"].as_str().unwrap();
    assert!(text.starts_with("Version {\n\tFormatVersion 800,\n}\nModel \"Rock\" {"), "{text}");
    assert!(text.contains("Anim \"Stand\" {"), "{text}");
    let part = call("mdx_text", &json!({"path": "Doodads\\Rock.mdx", "game": [], "roots": [dir], "offset": 1, "limit": 1})).unwrap().unwrap();
    assert_eq!((part["text"].as_str(), part["lines"].clone()), (Some("\tFormatVersion 800,"), all["lines"].clone()));
    let _ = std::fs::remove_dir_all(dir);
}
