use super::*;

fn temp(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("wc3-mcp-w3r-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn run(name: &str, args: Value) -> Result<Value, String> {
    call(name, &args).expect("a region tool")
}

#[test]
fn new_set_list_delete() {
    let dir = temp("all");
    let path = dir.join("war3map.w3r");
    let rect = json!({"left": -256, "bottom": -256, "right": 256, "top": 256.5});
    let added = run("w3r_new", json!({"path": path, "values": rect})).unwrap();
    assert_eq!(added["added"]["name"], "Region 000");
    assert_eq!(added["added"]["variable"], "gg_rct_Region_000");
    assert_eq!(added["added"]["top"], 256.5);
    let mut v = rect.clone();
    v["name"] = json!("Boss Room");
    v["weather"] = json!("RAhr");
    run("w3r_new", json!({"path": path, "values": v})).unwrap();

    let set = run("w3r_set", json!({"path": path, "region": "Boss Room", "values": {"red": 255, "green": 0, "blue": 0, "sound": "gg_snd_Rain"}})).unwrap();
    assert_eq!(set["before"]["green"], 255);
    assert_eq!(set["after"]["green"], 0);
    let set = run("w3r_set", json!({"path": path, "region": 1, "values": {"weather": ""}})).unwrap();
    assert_eq!(set["after"]["weather"], "");

    let list = run("w3r_list", json!({"path": path})).unwrap();
    assert_eq!(list["count"], 2);
    assert_eq!(list["regions"][1]["index"], 1);
    assert_eq!(list["regions"][1]["sound"], "gg_snd_Rain");

    // Checked before anything is written.
    let before = std::fs::read(&path).unwrap();
    for (tool, args, want) in [
        ("w3r_set", json!({"path": path, "region": 0, "values": {"name": "Boss Room"}}), "named \"Boss Room\" too"),
        ("w3r_set", json!({"path": path, "region": 0, "values": {"index": 1}}), "has index 1 too"),
        ("w3r_set", json!({"path": path, "region": 0, "values": {"left": 1000}}), "past right"),
        ("w3r_set", json!({"path": path, "region": 0, "values": {"weather": "RA"}}), "isn't an id"),
        ("w3r_set", json!({"path": path, "region": 0, "values": {"alpha": 256}}), "0 to 255"),
        ("w3r_set", json!({"path": path, "region": 9, "values": {}}), "no region with index 9"),
        ("w3r_new", json!({"path": path, "values": {"left": 0}}), "needs bottom, right, top"),
    ] {
        let err = run(tool, args).unwrap_err();
        assert!(err.contains(want), "{err} / {want}");
    }
    assert_eq!(std::fs::read(&path).unwrap(), before);

    let removed = run("w3r_delete", json!({"path": path, "region": "Region 000"})).unwrap();
    assert_eq!(removed["count"], 1);
    // The next index stays past the largest.
    let added = run("w3r_new", json!({"path": path, "values": rect})).unwrap();
    assert_eq!(added["added"]["name"], "Region 002");
}
