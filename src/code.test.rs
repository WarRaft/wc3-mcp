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

#[test]
fn natives_with_their_comments() {
    let dir = temp("natives");
    write(&dir.join("game/Scripts/common.j"), "type unit extends handle\n// Kills it at once\nnative KillUnit takes unit u returns nothing\nconstant native GetTriggerUnit takes nothing returns unit\n");
    write(&dir.join("game/Scripts/Blizzard.j"), "globals\n    constant integer bj_MAX_PLAYERS = 12 // players\nendglobals\nfunction KillUnitLater takes unit u, real wait returns nothing\nendfunction\n");
    write(&dir.join("ujapi/Scripts/common.j"), "type unit extends handle\n// UjAPI's\nnative KillUnit takes unit whichUnit returns nothing\n");
    write(&dir.join("proj/wc3.bni"), &format!("[project]\ngame={}\n", dir.join("game").display()));
    let n = |args: Value| call("jass_natives", &args).unwrap().unwrap();

    let r = n(json!({"path": dir.join("proj"), "name": "killunit"}));
    assert_eq!(r["total"], 2, "{r}");
    assert_eq!(r["items"][0], json!({"name": "KillUnit", "kind": "native", "declaration": "native KillUnit takes unit u returns nothing", "script": "common.j", "line": 3, "comment": "Kills it at once"}));
    assert_eq!(r["items"][1]["declaration"], "function KillUnitLater takes unit u, real wait returns nothing");
    let r = n(json!({"path": dir.join("proj"), "kind": "global"}));
    assert_eq!(r["items"], json!([{"name": "bj_MAX_PLAYERS", "kind": "global", "declaration": "constant integer bj_MAX_PLAYERS", "script": "Blizzard.j", "line": 2, "comment": "players"}]));
    let r = n(json!({"path": dir.join("proj"), "comment": "at once"}));
    assert_eq!(r["total"], 1);
    let r = n(json!({"path": dir.join("proj"), "script": "common.j", "kind": "native", "limit": 1, "offset": 1}));
    assert_eq!((r["total"].as_u64(), r["items"][0]["declaration"].as_str()), (Some(2), Some("constant native GetTriggerUnit takes nothing returns unit")));
    // UjAPI's common.j over the game's.
    let r = n(json!({"game": [dir.join("game"), dir.join("ujapi")], "name": "KillUnit", "kind": "native"}));
    assert_eq!((r["items"][0]["declaration"].as_str(), r["items"][0]["comment"].as_str()), (Some("native KillUnit takes unit whichUnit returns nothing"), Some("UjAPI's")));
    assert!(call("jass_natives", &json!({"path": dir})).unwrap().is_err());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn builds_jass() {
    let dir = temp("build");
    write(&dir.join("src/b.j"), "function main\n    Helper()\nendfunction\n");
    write(&dir.join("src/a.j"), "function Helper\n    integer i = 1\nendfunction\nfunction main\n    Helper()\nendfunction\n");
    let r = call("jass_build", &json!({"dir": dir.join("src")})).unwrap().unwrap();
    assert_eq!(r["built"], true, "{r}");
    assert_eq!(
        r["script"],
        "globals\nendglobals\n\nfunction Helper takes nothing returns nothing\n    local integer i = 1\nendfunction\n\nfunction main takes nothing returns nothing\n    call Helper()\n    call Helper()\nendfunction\n"
    );
    let out = dir.join("war3map.j");
    let r = call("jass_build", &json!({"files": [dir.join("src/a.j")], "out": out})).unwrap().unwrap();
    assert_eq!((r["built"].as_bool(), r["lines"].as_u64()), (Some(true), Some(10)));
    assert!(std::fs::read_to_string(&out).unwrap().contains("call Helper()"));
    write(&dir.join("bad/c.j"), "constant string S = N()\nfunction N returns string\n    return \"\"\nendfunction\n");
    let r = call("jass_build", &json!({"dir": dir.join("bad")})).unwrap().unwrap();
    assert_eq!(r["built"], false);
    assert_eq!(r["errors"][0]["line"], 1);
    let _ = std::fs::remove_dir_all(&dir);
}
