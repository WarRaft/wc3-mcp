use super::*;

fn temp(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("wc3-mcp-mmp-{name}-{}", std::process::id()));
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
    let path = dir.join("war3map.mmp");
    let icons = mmp_rs::MinimapIcons { version: 0, icons: vec![mmp_rs::Icon { kind: 0, x: 10, y: 20, red: 255, green: 255, blue: 0, alpha: 255 }] };
    std::fs::write(&path, icons.to_bytes().unwrap()).unwrap();
    let l = run("mmp_list", json!({"path": path})).unwrap();
    assert_eq!((l["count"].clone(), l["icons"][0]["kind_name"].clone()), (json!(1), json!(mmp_rs::kind_name(0))));
    let text = run("mmp_text", json!({"path": path})).unwrap().as_str().unwrap().replace("x=10", "x=11");
    run("mmp_write", json!({"path": path, "text": text})).unwrap();
    assert_eq!(mmp_rs::parse(&std::fs::read(&path).unwrap()).unwrap().icons[0].x, 11);
}
