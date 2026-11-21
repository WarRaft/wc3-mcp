use super::*;

fn temp(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("wc3-mcp-w3o-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn run(name: &str, args: Value) -> Result<Value, String> {
    call(name, &args).expect("a tool of this module")
}

#[test]
fn join_info_split() {
    let dir = temp("all");
    let units = w3abdhqtu_rs::ObjectData { kind: w3abdhqtu_rs::Kind::Unit, version: 2, originals: vec![], customs: vec![] };
    std::fs::write(dir.join("war3map.w3u"), units.to_bytes().unwrap()).unwrap();
    let w3o = dir.join("all.w3o");
    let joined = run("w3o_join", json!({"path": w3o, "folder": dir})).unwrap();
    assert_eq!(joined["kinds"][0]["file"], "war3map.w3u");
    assert_eq!(run("w3o_info", json!({"path": w3o})).unwrap()["kinds"].as_array().unwrap().len(), 1);
    let out = dir.join("out");
    std::fs::create_dir_all(&out).unwrap();
    run("w3o_split", json!({"path": w3o, "folder": out})).unwrap();
    assert_eq!(std::fs::read(out.join("war3map.w3u")).unwrap(), units.to_bytes().unwrap());
    assert!(run("w3o_join", json!({"path": w3o, "folder": out.join("none")})).is_err());
}
