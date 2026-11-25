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
    // Shorter names: Helper is `a`; main stays.
    let r = call("jass_build", &json!({"files": [dir.join("src/a.j")], "uglify": true})).unwrap().unwrap();
    assert!(r["script"].as_str().unwrap().contains("function a takes nothing returns nothing\n    local integer b = 1\n"), "{r}");
    assert!(r["script"].as_str().unwrap().contains("function main takes nothing returns nothing\n    call a()\n"), "{r}");
    write(&dir.join("bad/c.j"), "constant string S = N()\nfunction N returns string\n    return \"\"\nendfunction\n");
    let r = call("jass_build", &json!({"dir": dir.join("bad")})).unwrap().unwrap();
    assert_eq!(r["built"], false);
    assert_eq!(r["errors"][0]["line"], 1);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn rename_across_the_project() {
    let dir = temp("rename");
    let proj = project(&dir);
    write(&proj.join("c.j"), "function G\n    local integer n = 1\n    Helper(n)\nendfunction\nfunction H\n    local integer n = 2\nendfunction\n");
    let run = |a: Value| call("jass_rename", &a).expect("a tool of this module");
    // A dry run: what would change, nothing written.
    let r = run(json!({"path": proj, "name": "Helper", "new_name": "Assist", "dry_run": true})).unwrap();
    assert_eq!((r["renamed"].as_u64(), r["written"].as_bool()), (Some(3), Some(false)), "{r}");
    assert!(std::fs::read_to_string(proj.join("a.j")).unwrap().contains("function Helper"));
    // Renamed: in every file.
    let r = run(json!({"path": proj, "name": "Helper", "new_name": "Assist"})).unwrap();
    assert_eq!(r["files"].as_array().unwrap().len(), 3, "{r}");
    assert!(std::fs::read_to_string(proj.join("a.j")).unwrap().starts_with("function Assist takes integer n"));
    assert!(std::fs::read_to_string(proj.join("b.j")).unwrap().contains("    Assist(1, 2)\n"));
    assert!(std::fs::read_to_string(proj.join("c.j")).unwrap().contains("    Assist(n)\n"));
    // Two locals named alike: which one, by file and line; the other stays.
    let err = run(json!({"path": proj, "name": "n", "new_name": "count", "file": proj.join("c.j")})).unwrap_err();
    assert!(err.contains("declared 2 times") && err.contains("c.j:2") && err.contains("c.j:6"), "{err}");
    run(json!({"path": proj, "name": "n", "new_name": "count", "file": proj.join("c.j"), "line": 2})).unwrap();
    assert_eq!(
        std::fs::read_to_string(proj.join("c.j")).unwrap(),
        "function G\n    local integer count = 1\n    Assist(count)\nendfunction\nfunction H\n    local integer n = 2\nendfunction\n"
    );
    // Refused, nothing written: a native, a name taken, a name not declared.
    assert_eq!(run(json!({"path": proj, "name": "KillUnit", "new_name": "Kill"})).unwrap_err(), "`KillUnit` is the game's (common.j): it can't be renamed");
    assert!(run(json!({"path": proj, "name": "Nowhere", "new_name": "Somewhere"})).unwrap_err().starts_with("nothing named `Nowhere` is declared"));
    let err = run(json!({"path": proj, "name": "G", "new_name": "H"})).unwrap_err();
    assert!(err.starts_with("nothing renamed: `H` is already declared"), "{err}");
    assert!(run(json!({"path": proj, "name": "F", "new_name": "1x"})).unwrap_err().starts_with("nothing renamed:"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn rename_inside_a_maps_custom_script() {
    let dir = temp("rename-map");
    write(&dir.join("game/Scripts/common.j"), "type unit extends handle\n");
    // The map: its custom script declares Shared and calls it.
    let src = dir.join("src");
    let custom = wct_rs::CustomText {
        magic: false,
        version: 1,
        comment: vec![],
        header: wct_rs::Code::new(b"function Shared takes nothing returns nothing\nendfunction\n"),
        triggers: vec![wct_rs::Code::new(b"function Trig_A takes nothing returns nothing\n    call Shared()\nendfunction\nfunction InitTrig_A takes nothing returns nothing\nendfunction\n")],
    };
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(src.join("war3map.wct"), custom.to_bytes().unwrap()).unwrap();
    // World Editor's script made of the triggers beside it: a variable's
    // global, and a copy of the custom script.
    std::fs::write(src.join("war3map.j"), "globals\n    integer udg_Count = 0\nendglobals\nfunction Shared takes nothing returns nothing\nendfunction\nfunction main takes nothing returns nothing\n    set udg_Count = 1\nendfunction\n").unwrap();
    std::fs::write(src.join("war3map.wtg"), b"WTG!").unwrap();
    let map = dir.join("proj/map.w3x");
    std::fs::create_dir_all(map.parent().unwrap()).unwrap();
    crate::mpq::call("mpq_create", &json!({"archive": map, "dir": src})).unwrap().unwrap();
    write(&dir.join("proj/wc3.bni"), &format!("[project]\ngame={}\n[map]\nroot=map.w3x\n[code]\nsources=code\n", dir.join("game").display()));
    write(&dir.join("proj/code/b.j"), "function B takes nothing returns nothing\n    call Shared()\nendfunction\n");

    // Only in the script made of the triggers: theirs, refused.
    let err = call("jass_rename", &json!({"path": dir.join("proj"), "name": "udg_Count", "new_name": "udg_Total"})).unwrap().unwrap_err();
    assert!(err.starts_with("nothing renamed: `udg_Count` is declared only in war3map.j, which World Editor makes"), "{err}");
    let err = call("jass_rename", &json!({"path": dir.join("proj"), "name": "InitTrig_A", "new_name": "InitTrig_B"})).unwrap().unwrap_err();
    assert!(err.starts_with("nothing renamed: `InitTrig_A` is what World Editor's script calls for the trigger `A`"), "{err}");
    // Also there: the custom script's is the one meant (its copy follows).
    let r = call("jass_rename", &json!({"path": dir.join("proj"), "name": "Shared", "new_name": "Common"})).unwrap().unwrap();
    assert_eq!(r["renamed"], 4, "{r}");
    assert!(std::fs::read_to_string(dir.join("proj/code/b.j")).unwrap().contains("call Common()"));
    // The map's custom script, its bytes again, the names changed.
    let bytes = wc3_server::map::bytes(&map.join("war3map.wct")).unwrap();
    let back = wct_rs::parse(&bytes).unwrap();
    assert_eq!(back.header.text(), b"function Common takes nothing returns nothing\nendfunction\n");
    assert_eq!(back.triggers[0].text(), b"function Trig_A takes nothing returns nothing\n    call Common()\nendfunction\nfunction InitTrig_A takes nothing returns nothing\nendfunction\n");
    let _ = std::fs::remove_dir_all(&dir);
}
