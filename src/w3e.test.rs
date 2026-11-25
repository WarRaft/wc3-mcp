use super::*;
use w3e_rs::Rawcode;

fn temp(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("wc3-mcp-w3e-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn run(name: &str, args: Value) -> Result<Value, String> {
    call(name, &args).expect("a terrain tool")
}

/// A 4x3 Lordaeron Summer terrain, flat dirt.
fn terrain() -> Terrain {
    let point = Point {
        height: ZERO_HEIGHT,
        water: 0x2000 - 89,
        flags: 0,
        variation: 0,
        layer: 0xF2,
    };
    Terrain {
        version: 11,
        tileset: b'L',
        custom_tilesets: 0,
        ground: vec![Rawcode(*b"Ldrt"), Rawcode(*b"Lgrs")],
        cliffs: vec![Rawcode(*b"CLdi")],
        width: 4,
        height: 3,
        offset_x: -256.0,
        offset_y: -256.0,
        points: vec![point; 12],
    }
}

#[test]
fn info_points_set() {
    let dir = temp("all");
    let path = dir.join("war3map.w3e");
    std::fs::write(&path, terrain().to_bytes().unwrap()).unwrap();

    let info = run("w3e_info", json!({"path": path})).unwrap();
    // No game folders: no name.
    assert_eq!(info["tileset_name"], Value::Null);
    let game = dir.join("game");
    std::fs::create_dir_all(game.join("UI")).unwrap();
    std::fs::write(game.join("UI/WorldEditData.txt"), "[TileSets]\nL=WESTRING_LOCALE_L,x\n").unwrap();
    std::fs::write(game.join("UI/WorldEditStrings.txt"), "[WorldEditStrings]\nWESTRING_LOCALE_L=Lordaeron Summer\n").unwrap();
    let named = run("w3e_info", json!({"path": path, "game": [game]})).unwrap();
    assert_eq!(named["tileset_name"], "Lordaeron Summer");
    std::fs::create_dir_all(game.join("TerrainArt")).unwrap();
    std::fs::write(game.join("TerrainArt/Terrain.slk"), "ID;PWXL;N;E\nC;X1;Y1;K\"tileID\"\nC;X2;K\"name\"\nC;X1;Y2;K\"Ldrt\"\nC;X2;K\"Dirt\"\nE\n").unwrap();
    let named = run("w3e_info", json!({"path": path, "game": [game]})).unwrap();
    assert_eq!(named["ground"][0]["name"], "Dirt");
    assert_eq!(
        info["ground"][0],
        json!({"index": 0, "code": "Ldrt", "points": 12, "name": null})
    );
    assert_eq!(
        info["bounds"],
        json!({"left": -256.0, "bottom": -256.0, "right": 128.0, "top": 0.0})
    );

    let set = run(
        "w3e_set",
        json!({"path": path, "x": 1, "y": 1, "width": 2, "values": {"texture": "Lgrs", "blight": true, "height": 8292, "layer": 3}}),
    )
    .unwrap();
    assert_eq!(set, json!({"points": 2, "changed": 2}));
    let got = run(
        "w3e_points",
        json!({"path": path, "x": 1, "y": 1, "width": 3}),
    )
    .unwrap();
    let p = &got["points"][0];
    assert_eq!(
        (
            p["texture_code"].clone(),
            p["blight"].clone(),
            p["height_from_zero"].clone()
        ),
        (json!("Lgrs"), json!(true), json!(100))
    );
    assert_eq!(
        (
            p["world_x"].clone(),
            p["world_y"].clone(),
            p["layer"].clone()
        ),
        (json!(-128.0), json!(-128.0), json!(3))
    );
    assert_eq!(got["points"][2]["texture_code"], "Ldrt");
    // Setting the same again changes nothing.
    assert_eq!(
        run(
            "w3e_set",
            json!({"path": path, "x": 1, "y": 1, "values": {"blight": true}})
        )
        .unwrap()["changed"],
        0
    );
    assert_eq!(
        run("w3e_info", json!({"path": path})).unwrap()["ground"][1]["points"],
        2
    );
}

#[test]
fn errors_say_why() {
    let dir = temp("errors");
    let path = dir.join("war3map.w3e");
    std::fs::write(&path, terrain().to_bytes().unwrap()).unwrap();
    let err = |args: Value| run("w3e_set", args).unwrap_err();
    assert!(
        err(json!({"path": path, "x": 3, "y": 0, "width": 2, "values": {}})).contains("4x3 grid")
    );
    assert!(
        err(json!({"path": path, "x": 0, "y": 0, "values": {"texture": "Xxxx"}}))
            .contains("Ldrt, Lgrs")
    );
    assert!(err(json!({"path": path, "x": 0, "y": 0, "values": {"texture": 2}})).contains("has 2"));
    assert!(
        err(json!({"path": path, "x": 0, "y": 0, "values": {"layer": 16}})).contains("at most 15")
    );
    assert!(
        err(json!({"path": path, "x": 0, "y": 0, "values": {"cliff_texture": 1}})).contains("CLdi")
    );
    assert!(
        err(json!({"path": path, "x": 0, "y": 0, "values": {"blight": 1}}))
            .contains("true or false")
    );
    assert!(err(json!({"path": path, "x": 0, "y": 0, "values": {"nope": 1}})).contains("no field"));
    // None of that wrote anything.
    assert_eq!(std::fs::read(&path).unwrap(), terrain().to_bytes().unwrap());
    assert!(
        run(
            "w3e_points",
            json!({"path": path, "x": 0, "y": 0, "width": 4, "height": 3})
        )
        .is_ok()
    );
}

#[test]
fn image_of_the_terrain_with_its_textures() {
    let dir = temp("image");
    let path = dir.join("war3map.w3e");
    std::fs::write(&path, terrain().to_bytes().unwrap()).unwrap();
    // The tables and the texture, a root of its own; a TGA does as a BLP.
    let root = dir.join("root");
    std::fs::create_dir_all(root.join("TerrainArt")).unwrap();
    std::fs::create_dir_all(root.join("T")).unwrap();
    std::fs::write(root.join("TerrainArt/Terrain.slk"), "ID;P\nC;Y1;X1;K\"tileID\"\nC;X2;K\"dir\"\nC;X3;K\"file\"\nC;Y2;X1;K\"Ldrt\"\nC;X2;K\"T\"\nC;X3;K\"Dirt\"\nE\n").unwrap();
    let mut tga = vec![0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 4, 0, 4, 0, 32, 0x28];
    for _ in 0..16 {
        tga.extend_from_slice(&[0, 200, 0, 255]);
    }
    std::fs::write(root.join("T/Dirt.blp"), tga).unwrap();
    let args = json!({"path": path, "game": [], "roots": [root], "scale": 2});
    let out = run("w3e_image", args).unwrap();
    let text = out["mcp_content"][0]["text"].as_str().unwrap();
    assert!(text.starts_with("Terrain, 4x3 points, north up, 2 pixels per point (8x6)."), "{text}");
    // Lgrs isn't in the table: said.
    assert!(text.contains("Lgrs"), "{text}");
    assert_eq!(out["mcp_content"][1]["mimeType"], "image/png");
    // Nothing to draw with: said.
    let none = run("w3e_image", json!({"path": dir.join("nowhere").join("war3map.w3e")}));
    assert!(none.is_err());
}
