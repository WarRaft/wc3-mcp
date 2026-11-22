use super::*;

fn temp(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("wc3-mcp-code-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn write(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

fn run(args: Value) -> Result<Value, String> {
    call("code_check", &args).expect("a tool of this module")
}

/// A project over a game folder whose `Scripts/common.j` lies loose in it.
fn project(dir: &Path) -> PathBuf {
    write(&dir.join("game/Scripts/common.j"), "type unit extends handle\nnative KillUnit takes unit u returns nothing\n");
    write(&dir.join("proj/wc3.bni"), &format!("[project]\ngame={}\n[code]\nsources=.\n", dir.join("game").display()));
    write(&dir.join("proj/a.j"), "function Helper takes integer n returns nothing\nendfunction\n");
    write(&dir.join("proj/b.j"), "function F\n    Helper(1, 2)\n    Missing()\n    KillUnit(null)\n    local integer unused\nendfunction\n");
    dir.join("proj")
}

#[test]
fn problems_between_files() {
    let dir = temp("between");
    let proj = project(&dir);
    let r = run(json!({"path": proj})).unwrap();
    // Read: wc3.bni, a.j, b.j and the game's common.j.
    assert_eq!(r["files"], 4, "{r}");
    let got: Vec<(String, u64, String, String)> = r["problems"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| (p["file"].as_str().unwrap().into(), p["line"].as_u64().unwrap(), p["rule"].as_str().unwrap().into(), p["level"].as_str().unwrap().into()))
        .collect();
    assert_eq!(
        got,
        [
            ("b.j".into(), 2, "argument-count".into(), "error".into()),
            ("b.j".into(), 3, "unknown-name".into(), "error".into()),
            ("b.j".into(), 5, "unused-local".into(), "warning".into()),
        ]
    );
    assert_eq!(r["problems"][1]["message"], "the function 'Missing' is declared nowhere");
    // A call as a statement: its line.
    assert_eq!((r["problems"][1]["col"].as_u64(), r["problems"][1]["end_col"].as_u64()), (Some(1), Some(14)));

    // Errors only, one file, a limit; a file of the project finds it too.
    let r = run(json!({"path": proj.join("a.j"), "file": proj.join("b.j"), "errors": true, "limit": 1})).unwrap();
    assert_eq!((r["total"].as_u64(), r["problems"].as_array().unwrap().len()), (Some(2), 1));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn no_project() {
    let dir = temp("none");
    let e = run(json!({"path": dir})).unwrap_err();
    assert!(e.contains("no wc3.bni"), "{e}");
    let _ = std::fs::remove_dir_all(&dir);
}
