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
    assert_eq!(
        info["ground"][0],
        json!({"index": 0, "code": "Ldrt", "points": 12})
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
