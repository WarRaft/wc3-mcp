use super::*;

const TABLE: &str = "\u{FEFF}STRING 1\r\n// Units: h000 (Knight), Name (unam)\r\n{\r\nKnight\r\n}\r\n\r\nSTRING 2\r\n{\r\nA |cffffcc00holy|r warrior.\r\n}\r\n";

fn temp(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("wc3-mcp-wts-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn run(name: &str, args: Value) -> Result<Value, String> {
    call(name, &args).expect("a string table tool")
}

#[test]
fn list_get_resolve_lint() {
    let dir = temp("read");
    let path = dir.join("war3map.wts");
    std::fs::write(&path, TABLE).unwrap();
    let list = run("wts_list", json!({"path": path})).unwrap();
    assert_eq!(list["total"], 2);
    assert_eq!(
        list["strings"][0],
        json!({"id": 1, "comments": ["// Units: h000 (Knight), Name (unam)"], "text": "Knight", "length": 6})
    );
    assert_eq!(
        run("wts_list", json!({"path": path, "contains": "h000"})).unwrap()["total"],
        1
    );
    let got = run("wts_get", json!({"path": path, "ids": [2, 9]})).unwrap();
    assert_eq!(got["strings"][0]["text"], "A |cffffcc00holy|r warrior.");
    assert_eq!(got["strings"][1], json!({"id": 9, "missing": true}));
    assert_eq!(
        run(
            "wts_resolve",
            json!({"path": path, "text": "TRIGSTR_001 TRIGSTR_7"})
        )
        .unwrap(),
        "Knight TRIGSTR_7"
    );
    assert_eq!(
        run("wts_lint", json!({"path": path})).unwrap(),
        json!({"strings": 2, "errors": [], "warnings": []})
    );
}

#[test]
fn set_keeps_the_bom_and_the_rest() {
    let dir = temp("set");
    let path = dir.join("war3map.wts");
    std::fs::write(&path, TABLE).unwrap();
    let r = run(
        "wts_set",
        json!({"path": path, "id": 1, "text": "Paladin\nof light"}),
    )
    .unwrap();
    assert_eq!(
        (r["before"].clone(), r["after"].clone()),
        (json!("Knight"), json!("Paladin\nof light"))
    );
    let file = std::fs::read_to_string(&path).unwrap();
    assert_eq!(file, TABLE.replace("Knight\r\n", "Paladin\r\nof light\r\n"));
    run("wts_set", json!({"path": path, "id": 5, "text": "new"})).unwrap();
    run("wts_set", json!({"path": path, "id": 2, "text": null})).unwrap();
    let file = std::fs::read_to_string(&path).unwrap();
    assert!(file.starts_with('\u{FEFF}'));
    assert!(
        file.ends_with("}\r\n\r\nSTRING 5\r\n{\r\nnew\r\n}\r\n"),
        "{file:?}"
    );
    assert!(!file.contains("holy"));
    assert!(run("wts_set", json!({"path": path, "id": 1, "text": "a\n}\nb"})).is_err());
}

#[test]
fn from_a_map_and_for_object_data() {
    let dir = temp("map");
    let map = dir.join("t.w3x");
    storm_rs::archive::MpqArchiveBuilder::new(&map)
        .add_file("war3map.wts", TABLE.as_bytes().to_vec())
        .build()
        .unwrap();
    // war3map.wts by default.
    assert_eq!(
        run("wts_list", json!({"archive": map})).unwrap()["total"],
        2
    );
    assert_eq!(
        table_for(&json!({"archive": map}))
            .unwrap()
            .lookup("TRIGSTR_1"),
        Some("Knight")
    );
    assert!(table_for(&json!({"path": "x.w3u"})).is_none());
}
