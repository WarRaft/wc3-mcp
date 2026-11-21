use super::*;

fn temp(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("wc3-mcp-imp-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn run(name: &str, args: Value) -> Result<Value, String> {
    call(name, &args).expect("a tool of this module")
}

#[test]
fn list_text_write() {
    let dir = temp("all");
    let path = dir.join("war3map.imp");
    let list = imp_rs::Imports { version: 1, files: vec![
        imp_rs::Import { flags: 13, path: "Airlock.blp".into() },
        imp_rs::Import { flags: 0, path: "Units\\Acolyte01.mp3".into() },
    ] };
    std::fs::write(&path, list.to_bytes().unwrap()).unwrap();
    let l = run("imp_list", json!({"path": path})).unwrap();
    assert_eq!(l["count"], 2);
    assert_eq!(l["files"][0]["in_map"], "Airlock.blp");
    assert_eq!(l["files"][1]["in_map"], "war3mapImported\\Acolyte01.mp3");
    let text = run("imp_text", json!({"path": path})).unwrap();
    let text = text.as_str().unwrap().replace("Airlock", "Hatch");
    assert_eq!(run("imp_write", json!({"path": path, "text": text})).unwrap()["files"], 2);
    assert_eq!(imp_rs::parse(&std::fs::read(&path).unwrap()).unwrap().files[0].path, "Hatch.blp");
    let err = run("imp_write", json!({"path": path, "text": "[imp]\nversion=1\nwhat=1\n"})).unwrap_err();
    assert!(err.starts_with("nothing written:\nline 3:"), "{err}");
}
