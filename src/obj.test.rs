use super::*;
use w3abdhqtu_rs::Value as V;

fn temp(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("wc3-mcp-obj-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn run(name: &str, args: Value) -> Result<Value, String> {
    call(name, &args).expect("an object data tool")
}

/// A game folder with the abilities' metadata, loose (no archives).
fn game(dir: &std::path::Path) -> String {
    let units = dir.join("game").join("Units");
    std::fs::create_dir_all(&units).unwrap();
    let rows: &[&[&str]] = &[
        &[
            "ID",
            "field",
            "type",
            "minVal",
            "maxVal",
            "repeat",
            "data",
            "useSpecific",
        ],
        &["anam", "Name", "string", "", "", "0", "0", ""],
        &["acdn", "Cooldown", "unreal", "0", "300", "4", "0", ""],
        &["Hbz1", "Waves", "int", "1", "99999", "4", "1", "AHbz"],
        &["Hbz2", "Damage", "unreal", "0", "99999", "4", "2", "AHbz"],
    ];
    let mut slk = String::from("ID;PWXL;N;E\n");
    for (y, row) in rows.iter().enumerate() {
        for (x, cell) in row.iter().enumerate() {
            if !cell.is_empty() {
                slk += &format!("C;X{};Y{};K\"{cell}\"\n", x + 1, y + 1);
            }
        }
    }
    std::fs::write(units.join("AbilityMetaData.slk"), slk + "E\n").unwrap();
    dir.join("game").to_string_lossy().into_owned()
}

/// war3map.w3a: Blizzard changed, and A000 based on it.
fn file(dir: &std::path::Path) -> String {
    let m = |field: &[u8; 4], level, column, value| Mod {
        field: Rawcode(*field),
        level,
        column,
        value,
        end: Rawcode::NONE,
    };
    let data = ObjectData {
        kind: Kind::Ability,
        version: 2,
        originals: vec![Object {
            base: Rawcode(*b"AHbz"),
            id: Rawcode::NONE,
            sets: vec![Set {
                flags: 0,
                mods: vec![m(b"Hbz1", 1, 1, V::Int(4))],
            }],
        }],
        customs: vec![Object {
            base: Rawcode(*b"AHbz"),
            id: Rawcode(*b"A000"),
            sets: vec![Set {
                flags: 0,
                mods: vec![
                    m(b"anam", 0, 0, V::Str("Storm".into())),
                    m(b"Hbz2", 1, 2, V::Unreal(25.0)),
                ],
            }],
        }],
    };
    let path = dir.join("war3map.w3a");
    std::fs::write(&path, data.to_bytes().unwrap()).unwrap();
    path.to_string_lossy().into_owned()
}

#[test]
fn list_get_text() {
    let dir = temp("read");
    let (path, game) = (file(&dir), game(&dir));
    let list = run("obj_list", json!({"path": path})).unwrap();
    assert_eq!(list["total"], 2);
    assert_eq!(
        list["objects"][1],
        json!({"id": "A000", "base": "AHbz", "table": "custom", "name": "Storm", "fields": 2})
    );
    let found = run("obj_list", json!({"path": path, "contains": "storm"})).unwrap();
    assert_eq!(found["total"], 1);
    assert_eq!(
        run("obj_list", json!({"path": path, "table": "original"})).unwrap()["objects"][0]["id"],
        "AHbz"
    );

    let a = run(
        "obj_get",
        json!({"path": path, "object": "A000", "game": [game]}),
    )
    .unwrap();
    assert_eq!(
        a["sets"][0]["fields"][1],
        json!({"key": "Hbz2[1].B", "field": "Hbz2", "name": "Damage", "type": "unreal", "level": 1, "column": 2, "stored": "unreal", "value": 25.0})
    );
    assert!(
        run("obj_get", json!({"path": path, "object": "A999"}))
            .unwrap_err()
            .starts_with("no object A999")
    );

    let text = run("obj_text", json!({"path": path, "game": [game]})).unwrap();
    assert!(
        text["text"]
            .as_str()
            .unwrap()
            .contains("Hbz2[1].B=25.0 // Damage: unreal"),
        "{text}"
    );
    let page = run("obj_text", json!({"path": path, "offset": 1, "limit": 2})).unwrap();
    assert_eq!(
        (page["text"].clone(), page["more"].clone()),
        (json!("[w3a]\nversion=2"), json!(true))
    );
}

#[test]
fn checks_need_the_game() {
    let dir = temp("check");
    let (path, game) = (file(&dir), game(&dir));
    assert!(
        run("obj_check", json!({"path": path}))
            .unwrap_err()
            .starts_with("no metadata")
    );
    // Hbz1 is A's data; set a value past its limits on A000.
    run("obj_set", json!({"path": path, "game": [game], "object": "A000", "field": "Hbz1", "level": 1, "value": 0})).unwrap();
    let c = run("obj_check", json!({"path": path, "game": [game]})).unwrap();
    assert_eq!(c["total"], 1);
    assert_eq!(c["problems"][0]["object"], "A000");
    assert_eq!(c["problems"][0]["key"], "Hbz1[1].A");
    assert_eq!(c["problems"][0]["rule"], "object-value-range");
}

#[test]
fn set_new_delete() {
    let dir = temp("write");
    let (path, game) = (file(&dir), game(&dir));
    let g = json!([game]);
    // A number takes the field's type: acdn is an unreal.
    let r = run(
        "obj_set",
        json!({"path": path, "game": g, "object": "A000", "field": "acdn", "level": 2, "value": 6}),
    )
    .unwrap();
    assert_eq!(
        (r["before"].clone(), r["after"].clone()),
        (json!(null), json!(6.0))
    );
    let r = run("obj_set", json!({"path": path, "game": g, "object": "A000", "field": "Hbz2", "level": 1, "value": "30.5"})).unwrap();
    assert_eq!(
        (r["column"].clone(), r["before"].clone(), r["after"].clone()),
        (json!(2), json!(25.0), json!(30.5))
    );
    let fields = run(
        "obj_get",
        json!({"path": path, "game": g, "object": "A000"}),
    )
    .unwrap()["sets"][0]["fields"]
        .clone();
    assert_eq!(fields[2]["stored"], "unreal");
    assert_eq!(fields[2]["key"], "acdn[2]");
    // Mistakes are said, and nothing is written.
    assert!(
        run(
            "obj_set",
            json!({"path": path, "game": g, "object": "A000", "field": "acdn", "value": 1})
        )
        .unwrap_err()
        .contains("give `level`")
    );
    assert!(
        run(
            "obj_set",
            json!({"path": path, "game": g, "object": "A000", "field": "zzzz", "value": 1})
        )
        .unwrap_err()
        .contains("isn't a field")
    );
    assert!(
        run(
            "obj_set",
            json!({"path": path, "game": g, "object": "A000", "field": "anam", "value": 5})
        )
        .unwrap_err()
        .contains("holds text")
    );
    // Removing a field.
    let r = run(
        "obj_set",
        json!({"path": path, "object": "A000", "field": "anam", "value": null}),
    )
    .unwrap();
    assert_eq!(
        (r["before"].clone(), r["after"].clone()),
        (json!("Storm"), json!(null))
    );

    run(
        "obj_new",
        json!({"path": path, "id": "A001", "base": "AHbz"}),
    )
    .unwrap();
    assert!(
        run(
            "obj_new",
            json!({"path": path, "id": "A001", "base": "AHbz"})
        )
        .unwrap_err()
        .contains("already")
    );
    run("obj_delete", json!({"path": path, "object": "AHbz"})).unwrap();
    let list = run("obj_list", json!({"path": path})).unwrap();
    let ids: Vec<&str> = list["objects"]
        .as_array()
        .unwrap()
        .iter()
        .map(|o| o["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, ["A000", "A001"]);
}

#[test]
fn from_a_map() {
    let dir = temp("map");
    let path = file(&dir);
    let map = dir.join("t.w3x");
    storm_rs::archive::MpqArchiveBuilder::new(&map)
        .add_file("war3map.w3a", std::fs::read(&path).unwrap())
        .build()
        .unwrap();
    let list = run("obj_list", json!({"path": "war3map.w3a", "archive": map})).unwrap();
    assert_eq!(list["total"], 2);
    // Writing is to files on disk.
    assert!(
        run(
            "obj_new",
            json!({"path": "war3map.w3a", "id": "A001", "base": "AHbz"})
        )
        .is_err()
    );
}
