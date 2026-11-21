use super::*;

fn temp(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("wc3-mcp-w3i-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn run(name: &str, args: Value) -> Result<Value, String> {
    call(name, &args).expect("a map info tool")
}

#[test]
fn text_and_write() {
    let dir = temp("all");
    let path = dir.join("war3map.w3i");
    let p = path.to_str().unwrap();
    let mut info = w3i_rs::Info::new(25);
    info.name = "Old".into();
    info.players.push(w3i_rs::Player { kind: 1, race: 1, ..Default::default() });
    std::fs::write(&path, info.to_bytes().unwrap()).unwrap();

    let text = run("w3i_text", json!({"path": p})).unwrap();
    let text = text.as_str().unwrap();
    assert!(text.contains("name=\"Old\"\n"), "{text}");
    let new = text.replace("name=\"Old\"", "name=\"New\"").replace("race=1", "race=2");
    let out = run("w3i_write", json!({"path": p, "text": new})).unwrap();
    assert_eq!(out["players"], 1);
    let back = w3i_rs::parse(&std::fs::read(&path).unwrap()).unwrap();
    assert_eq!((back.name.as_str(), back.players[0].race), ("New", 2));

    let before = std::fs::read(&path).unwrap();
    let err = run("w3i_write", json!({"path": p, "text": "[w3i]\nversion=25\nscript=1\nfoo=2\n"})).unwrap_err();
    assert!(err.starts_with("nothing written:\nline 3: `script`"), "{err}");
    assert!(err.contains("line 4: no field `foo`"), "{err}");
    assert_eq!(std::fs::read(&path).unwrap(), before);
    let err = run("w3i_write", json!({"path": dir.join("none.w3i").to_str().unwrap(), "text": text})).unwrap_err();
    assert!(err.contains("not a file on disk"), "{err}");
    assert!(run("w3i_text", json!({})).unwrap_err().contains("`path`"));
}
