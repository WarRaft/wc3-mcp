use super::*;

fn temp(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("wc3-mcp-nav-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn write(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

fn run(tool: &str, args: Value) -> Result<Value, String> {
    call(tool, &args).expect("a tool of this module")
}

/// A project of two files over a game folder with a `common.j`: a global
/// `count` shadowed by a local of `F`, two structs with a member `x`.
fn project(dir: &Path) -> PathBuf {
    write(&dir.join("game/Scripts/common.j"), "type unit extends handle\n// Kills it at once\nnative KillUnit takes unit u returns nothing\n");
    write(&dir.join("proj/wc3.bni"), &format!("[project]\ngame={}\n[code]\nsources=.\n", dir.join("game").display()));
    write(
        &dir.join("proj/a.j"),
        "globals\n    integer count = 0\nendglobals\n// Adds one\nfunction Helper takes integer n returns nothing\n    set count = count + n\nendfunction\n",
    );
    write(
        &dir.join("proj/b.j"),
        "function F takes nothing returns nothing\n    local integer count = 1\n    call Helper(count)\n    call KillUnit(null)\nendfunction\nfunction G takes nothing returns nothing\n    call Helper(count)\n    call KillUnit(null)\nendfunction\n",
    );
    dir.join("proj")
}

fn lines(places: &Value) -> Vec<(String, u64)> {
    places.as_array().unwrap().iter().map(|p| (p["file"].as_str().unwrap().to_string(), p["line"].as_u64().unwrap())).collect()
}

#[test]
fn definition_by_name_and_by_place() {
    let dir = temp("definition");
    let proj = project(&dir);
    let r = run("code_definition", json!({"path": proj, "name": "Helper"})).unwrap();
    assert_eq!(r["declarations"].as_array().unwrap().len(), 1, "{r}");
    let d = &r["declarations"][0];
    assert_eq!((d["file"].as_str(), d["line"].as_u64(), d["kind"].as_str(), d["comment"].as_str()), (Some("a.j"), Some(5), Some("function"), Some("Adds one")));
    assert_eq!(d["code"], "function Helper takes integer n returns nothing");

    // `count` in F is its local; in G, the global.
    let r = run("code_definition", json!({"path": proj, "file": "b.j", "line": 3, "col": 17})).unwrap();
    assert_eq!(lines(&r["declarations"]), [("b.j".to_string(), 2)], "{r}");
    let r = run("code_definition", json!({"path": proj, "file": "b.j", "line": 7, "col": 17})).unwrap();
    assert_eq!(lines(&r["declarations"]), [("a.j".to_string(), 2)], "{r}");

    // A native: the game's, with its comment.
    let r = run("code_definition", json!({"path": proj, "name": "KillUnit"})).unwrap();
    assert_eq!((r["declarations"][0]["game"].as_bool(), r["declarations"][0]["comment"].as_str()), (Some(true), Some("Kills it at once")), "{r}");

    // Two symbols of the name: which is asked.
    let e = run("code_definition", json!({"path": proj, "name": "count"})).unwrap_err();
    assert!(e.contains("names 2 symbols") && e.contains("a.j:2") && e.contains("b.j:2"), "{e}");
    let r = run("code_definition", json!({"path": proj, "name": "count", "file": "b.j"})).unwrap();
    assert_eq!(r["declarations"][0]["kind"], "local");

    let e = run("code_definition", json!({"path": proj, "name": "Nothing"})).unwrap_err();
    assert!(e.contains("nothing named `Nothing`"), "{e}");
    let e = run("code_definition", json!({"path": proj, "file": "b.j", "line": 1, "col": 1})).unwrap_err();
    assert!(e.contains("no name there"), "{e}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn references_by_meaning() {
    let dir = temp("references");
    let proj = project(&dir);
    // The global: its declaration, its two uses in Helper, G's; not F's local.
    let r = run("code_references", json!({"path": proj, "name": "count", "file": "a.j"})).unwrap();
    assert_eq!(lines(&r["places"]), [("a.j".into(), 2), ("a.j".into(), 6), ("a.j".into(), 6), ("b.j".into(), 7)], "{r}");
    assert_eq!((r["total"].as_u64(), r["declarations"].as_u64()), (Some(4), Some(1)));
    assert_eq!(r["places"][0]["declaration"], true);
    assert_eq!(r["files"], json!([{"file": "a.j", "count": 3}, {"file": "b.j", "count": 1}]));

    // The local, from its use.
    let r = run("code_references", json!({"path": proj, "file": "b.j", "line": 3, "col": 17})).unwrap();
    assert_eq!(lines(&r["places"]), [("b.j".into(), 2), ("b.j".into(), 3)], "{r}");

    // A native's uses in the project, not its declaration in common.j...
    let r = run("code_references", json!({"path": proj, "name": "KillUnit"})).unwrap();
    assert_eq!(lines(&r["places"]), [("b.j".into(), 4), ("b.j".into(), 8)], "{r}");
    // ... unless the game's scripts are asked for.
    let r = run("code_references", json!({"path": proj, "name": "KillUnit", "game": true, "limit": 1})).unwrap();
    assert_eq!((r["total"].as_u64(), r["places"].as_array().unwrap().len()), (Some(3), 1), "{r}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn members_by_name() {
    let dir = temp("members");
    write(&dir.join("proj/wc3.bni"), "[project]\n[code]\nsources=.\n[jass]\ndialect=vjass\n");
    write(
        &dir.join("proj/s.j"),
        "struct A\n    integer x\nendstruct\nstruct B\n    integer x\nendstruct\nfunction F takes A a returns integer\n    return a.x\nendfunction\n",
    );
    let proj = dir.join("proj");
    // Members are matched by name, as the editor matches them: both
    // structs' `x`, and the use, with a note saying so.
    let r = run("code_references", json!({"path": proj, "name": "x"})).unwrap();
    assert_eq!(lines(&r["places"]), [("s.j".into(), 2), ("s.j".into(), 5), ("s.j".into(), 8)], "{r}");
    assert!(r["note"].as_str().is_some_and(|n| n.contains("declared 2 times")), "{r}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn outline_and_search() {
    let dir = temp("symbols");
    let proj = project(&dir);
    let r = run("code_symbols", json!({"path": proj, "file": "a.j"})).unwrap();
    let names: Vec<&str> = r["items"].as_array().unwrap().iter().map(|i| i["name"].as_str().unwrap()).collect();
    assert_eq!(names, ["count", "Helper"], "{r}");
    let r = run("code_symbols", json!({"path": proj, "file": "a.j", "locals": true})).unwrap();
    assert_eq!(r["total"], 3, "{r}");

    // Across the project: functions with "e" in them, the exact one first.
    let r = run("code_symbols", json!({"path": proj, "query": "g", "kind": "function"})).unwrap();
    let names: Vec<&str> = r["items"].as_array().unwrap().iter().map(|i| i["name"].as_str().unwrap()).collect();
    assert_eq!(names, ["G"], "{r}");
    let r = run("code_symbols", json!({"path": proj, "query": "kill"})).unwrap();
    assert_eq!(r["total"], 0, "{r}");
    let r = run("code_symbols", json!({"path": proj, "query": "kill", "game": true})).unwrap();
    assert_eq!((r["items"][0]["name"].as_str(), r["items"][0]["game"].as_bool()), (Some("KillUnit"), Some(true)), "{r}");

    let e = run("code_symbols", json!({"path": proj, "file": "none.j"})).unwrap_err();
    assert!(e.contains("not a file"), "{e}");
    let _ = std::fs::remove_dir_all(&dir);
}
