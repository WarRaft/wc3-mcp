use super::*;

/// A fresh folder for one test.
fn temp(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("wc3-mcp-mpq-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn run(name: &str, args: Value) -> Result<Value, String> {
    call(name, &args).expect("an MPQ tool")
}

/// An archive with a script, a table, a binary file and, without a
/// listfile, a file no name is known for.
fn archive(dir: &Path, listfile: bool) -> String {
    let path = dir.join("map.w3x");
    MpqArchiveBuilder::new(&path)
        .options(MpqCreateOptions::v1().create_listfile(listfile))
        .add_file(
            "war3map.j",
            b"function main takes nothing returns nothing\nendfunction\n".to_vec(),
        )
        .add_file(
            "Units\\UnitBalance.slk",
            b"ID;P\nC;Y1;X1;K\"id\"\nE\n".to_vec(),
        )
        .add_file("war3map.w3e", vec![0x57, 0x33, 0x45, 0x21, 0, 0xff])
        .build()
        .unwrap();
    path.to_string_lossy().into_owned()
}

#[test]
fn info_and_list() {
    let dir = temp("list");
    let a = archive(&dir, true);
    let info = run("mpq_info", json!({"archive": a})).unwrap();
    assert_eq!(
        (
            info["files"].clone(),
            info["named"].clone(),
            info["listfile"].clone()
        ),
        (json!(4), json!(4), json!(true))
    );
    let list = run("mpq_list", json!({"archive": a, "masks": ["war3map.*"]})).unwrap();
    assert_eq!(list["total"], 2);
    assert_eq!(list["files"][0]["name"], "war3map.j");
    assert_eq!(list["files"][0]["flags"], json!(["compress"]));
    let page = run("mpq_list", json!({"archive": a, "offset": 1, "limit": 1})).unwrap();
    assert_eq!(
        (
            page["total"].clone(),
            page["files"].as_array().unwrap().len()
        ),
        (json!(4), 1)
    );
    assert!(
        run("mpq_info", json!({"archive": "/nonexistent.w3x"}))
            .unwrap_err()
            .starts_with("/nonexistent.w3x: ")
    );
    // Not an MPQ tool.
    assert!(call("slk_info", &json!({})).is_none());
}

#[test]
fn unnamed_files_and_listfiles() {
    let dir = temp("unnamed");
    let a = archive(&dir, false);
    let list = run("mpq_list", json!({"archive": a})).unwrap();
    assert_eq!(
        (list["total"].clone(), list["unnamed"].clone()),
        (json!(0), json!(3))
    );
    let names = dir.join("names.txt");
    std::fs::write(&names, "war3map.j\r\nUnits\\UnitBalance.slk\n").unwrap();
    let list = run("mpq_list", json!({"archive": a, "listfiles": [names]})).unwrap();
    assert_eq!(
        (list["total"].clone(), list["unnamed"].clone()),
        (json!(2), json!(1))
    );
}

#[test]
fn read_text_and_binary() {
    let dir = temp("read");
    let a = archive(&dir, true);
    let j = run("mpq_read", json!({"archive": a, "name": "war3map.j"})).unwrap();
    assert!(j["text"].as_str().unwrap().starts_with("function main"));
    assert_eq!(j["more"], false);
    let part = run(
        "mpq_read",
        json!({"archive": a, "name": "war3map.j", "offset": 9, "length": 4}),
    )
    .unwrap();
    assert_eq!(
        (part["text"].clone(), part["more"].clone()),
        (json!("main"), json!(true))
    );
    let bin = run("mpq_read", json!({"archive": a, "name": "war3map.w3e"})).unwrap();
    assert_eq!(
        (
            bin["binary"].clone(),
            bin["size"].clone(),
            bin["hex"].clone()
        ),
        (json!(true), json!(6), json!(["57 33 45 21 00 ff"]))
    );
    assert!(
        run("mpq_read", json!({"archive": a, "name": "nope.j"}))
            .unwrap_err()
            .ends_with("no nope.j in it")
    );
}

#[test]
fn extract_and_verify() {
    let dir = temp("extract");
    let a = archive(&dir, true);
    let out = dir.join("out");
    let r = run(
        "mpq_extract",
        json!({"archive": a, "dir": out, "masks": ["Units/*"]}),
    )
    .unwrap();
    assert_eq!(
        (r["extracted"].clone(), r["failed"].clone()),
        (json!(1), json!([]))
    );
    assert!(out.join("Units").join("UnitBalance.slk").is_file());
    let v = run("mpq_verify", json!({"archive": a})).unwrap();
    assert_eq!(v["bad"], json!([]));
}

#[test]
fn create_and_change() {
    let dir = temp("change");
    let src = dir.join("src");
    std::fs::create_dir_all(src.join("Units")).unwrap();
    std::fs::write(src.join("war3map.j"), "x").unwrap();
    std::fs::write(src.join("Units").join("a.slk"), "ID;P\nE\n").unwrap();
    let a = dir.join("new.w3x").to_string_lossy().into_owned();
    assert_eq!(
        run("mpq_create", json!({"archive": a, "dir": src})).unwrap()["files"],
        2
    );
    assert!(
        run("mpq_create", json!({"archive": a, "dir": src}))
            .unwrap_err()
            .ends_with("already exists")
    );

    run(
        "mpq_add",
        json!({"archive": a, "name": "Scripts/b.j", "text": "bee"}),
    )
    .unwrap();
    run(
        "mpq_rename",
        json!({"archive": a, "from": "war3map.j", "to": "c.j"}),
    )
    .unwrap();
    run(
        "mpq_remove",
        json!({"archive": a, "names": ["Units\\a.slk"]}),
    )
    .unwrap();
    assert!(run("mpq_add", json!({"archive": a, "name": "x.j"})).is_err());
    let names: Vec<String> = run("mpq_list", json!({"archive": a, "masks": ["*.j", "*.slk"]}))
        .unwrap()["files"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["name"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(names, ["c.j", "Scripts\\b.j"]);
    assert_eq!(
        run("mpq_read", json!({"archive": a, "name": "Scripts\\b.j"})).unwrap()["text"],
        "bee"
    );
}
