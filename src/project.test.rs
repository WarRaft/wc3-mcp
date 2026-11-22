use super::*;

fn temp(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("wc3-mcp-project-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn run(name: &str, args: Value) -> Result<Value, String> {
    call(name, &args).expect("a tool of this module")
}

fn write(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

const DATA: &str = "[TriggerEvents]\nMapInitializationEvent=0,nothing\n[TriggerActions]\nDoNothing=0,nothing\n";

/// A project: a game folder with the editor's data, a folder root with a
/// patch adding a native, a trigger file calling it.
fn project(dir: &Path) -> PathBuf {
    write(&dir.join("game/UI/TriggerData.txt"), DATA);
    write(&dir.join("proj/jngp/UI/TriggerData.patch.txt"), "[TriggerActions]\nPauseGame=0,boolean\n");
    write(&dir.join("proj/wc3.bni"), &format!("[project]\ngame={}\n\n[map]\nroot=jngp\nroot=missing\n", dir.join("game").display()));
    let text = "[wtg]\nversion=7\ngame=2\n\n[category]\nindex=0\nname=\"Init\"\n\n[trigger]\nname=\"A\"\ndescription=\"\"\ncategory=0\nevent=MapInitializationEvent()\naction=PauseGame(true)\n";
    let data = wtg_rs::TriggerData::parse(&format!("{DATA}[TriggerActions]\nPauseGame=0,boolean\n"));
    let path = dir.join("proj/map/war3map.wtg");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, wtg_rs::text::from_text(text, Some(&data)).file.unwrap().to_bytes().unwrap()).unwrap();
    path
}

#[test]
fn info_shows_the_settings_and_the_editors_files() {
    let dir = temp("info");
    project(&dir);
    let i = run("project_info", json!({"path": dir.join("proj/map/war3map.wtg")})).unwrap();
    assert_eq!(i["manifest"], json!(dir.join("proj/wc3.bni")));
    assert_eq!(i["game"][0]["kind"], "folder");
    assert_eq!(i["roots"][0], json!({"path": dir.join("proj/jngp"), "kind": "folder"}));
    assert_eq!(i["roots"][1]["kind"], "missing");
    let data = &i["editor_files"].as_array().unwrap().iter().find(|f| f["file"] == "UI\\TriggerData.txt").unwrap();
    assert_eq!(data["from"], json!(dir.join("game/UI/TriggerData.txt")));
    assert_eq!(data["patch_from"], json!(dir.join("proj/jngp/UI/TriggerData.patch.txt")));
    assert!(i["text"].as_str().unwrap().starts_with("[project]"));

    let none = run("project_info", json!({"path": dir.join("game")})).unwrap_err();
    assert!(none.contains("no wc3.bni"), "{none}");
}

#[test]
fn write_checks_before_writing() {
    let dir = temp("write");
    let bad = run("project_write", json!({"path": dir, "text": "[project]\nsources=src\n"})).unwrap_err();
    assert!(bad.starts_with("nothing written:\nline 2:"), "{bad}");
    assert!(!dir.join("wc3.bni").exists());
    assert!(run("project_write", json!({"path": dir.join("x.txt"), "text": ""})).unwrap_err().contains("wc3.bni"));
    assert!(run("project_write", json!({"path": dir.join("none/wc3.bni"), "text": ""})).unwrap_err().contains("no such folder"));

    let ok = run("project_write", json!({"path": dir, "text": "[map]\nroot=folder\n"})).unwrap();
    assert_eq!(ok["written"], json!(dir.join("wc3.bni")));
    assert_eq!(std::fs::read_to_string(dir.join("wc3.bni")).unwrap(), "[map]\nroot=folder\n");
}

#[test]
fn other_tools_take_the_projects_layers() {
    let dir = temp("layers");
    let wtg = project(&dir);
    // No `game`, no `roots`: the project's, the patch with them.
    let l = crate::wtg::call("wtg_list", &json!({"path": wtg})).unwrap().unwrap();
    assert_eq!(l["triggers"][0]["actions"], 1);
    // `project` names it for a file outside it.
    let outside = dir.join("war3map.wtg");
    std::fs::copy(&wtg, &outside).unwrap();
    let e = crate::wtg::call("wtg_list", &json!({"path": outside})).unwrap().unwrap_err();
    assert!(e.contains("no wc3.bni of a project above"), "{e}");
    assert!(crate::wtg::call("wtg_list", &json!({"path": outside, "project": dir.join("proj")})).unwrap().is_ok());
    // Given `roots` win: without the patch, the native is unknown.
    let e = crate::wtg::call("wtg_list", &json!({"path": wtg, "roots": []})).unwrap().unwrap_err();
    assert!(e.contains("PauseGame"), "{e}");
    // A missing file is said first.
    let e = crate::wtg::call("wtg_list", &json!({"path": dir.join("none.wtg")})).unwrap().unwrap_err();
    assert!(e.contains("none.wtg"), "{e}");
}
