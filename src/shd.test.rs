use super::*;

fn temp(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("wc3-mcp-shd-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn run(name: &str, args: Value) -> Result<Value, String> {
    call(name, &args).expect("a tool of this module")
}

#[test]
fn a_picture_sized_by_the_terrain() {
    let dir = temp("all");
    let path = dir.join("war3map.shd");
    std::fs::write(&path, [0u8; 16]).unwrap();
    // No terrain beside it: no size.
    assert!(run("shd_image", json!({"path": path})).unwrap_err().contains("war3map.w3e"));
}
