use super::*;

fn temp(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("wc3-mcp-wct-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn run(name: &str, args: Value) -> Result<Value, String> {
    call(name, &args).expect("a tool of this module")
}

#[test]
fn text_and_write() {
    let dir = temp("all");
    let path = dir.join("war3map.wct");
    let f = wct_rs::CustomText { magic: false, version: 1, comment: b"hi".to_vec(), header: wct_rs::Code::new(b"function A takes nothing returns nothing\r\nendfunction"), triggers: vec![wct_rs::Code(None)] };
    std::fs::write(&path, f.to_bytes().unwrap()).unwrap();
    let text = run("wct_text", json!({"path": path})).unwrap();
    let text = text.as_str().unwrap();
    assert!(text.contains("// @wct header\nfunction A"), "{text}");
    assert!(run("wct_text", json!({"path": path, "script": "lua"})).unwrap().as_str().unwrap().contains("-- @wct trigger none"));
    run("wct_write", json!({"path": path, "text": text.replace("function A", "function B")})).unwrap();
    assert_eq!(wct_rs::parse(&std::fs::read(&path).unwrap()).unwrap().header.text(), b"function B takes nothing returns nothing\r\nendfunction");
}
