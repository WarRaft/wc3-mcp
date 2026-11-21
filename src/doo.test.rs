use super::*;

fn temp(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("wc3-mcp-doo-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn run(name: &str, args: Value) -> Result<Value, String> {
    call(name, &args).expect("a tool of this module")
}

#[test]
fn list_get_set_new_delete() {
    let dir = temp("all");
    let path = dir.join("war3mapUnits.doo");
    let units = doo_rs::Units {
        version: 8,
        subversion: 11,
        layout: Layout::Classic,
        units: vec![
            doo_rs::Unit::new(doo_rs::Rawcode(*b"hfoo"), 0.0, 0.0, 0),
            doo_rs::Unit::new(doo_rs::Rawcode(*b"ogru"), 500.0, 500.0, 1),
        ],
    };
    std::fs::write(&path, units.to_bytes().unwrap()).unwrap();
    let l = run("doo_list", json!({"path": path, "player": 1})).unwrap();
    assert_eq!((l["total"].clone(), l["matched"].clone(), l["things"][0]["index"].clone()), (json!(2), json!(1), json!(1)));
    assert_eq!(l["things"][0]["id"], "ogru");
    let l = run("doo_list", json!({"path": path, "right": 100, "id": ["hfoo"]})).unwrap();
    assert_eq!(l["matched"], 1);

    let text = run("doo_get", json!({"path": path, "index": [1]})).unwrap();
    let text = text.as_str().unwrap();
    let section = &text[text.find("[unit]").unwrap()..];
    run("doo_set", json!({"path": path, "index": 0, "text": section.replace("player=1", "player=3")})).unwrap();
    run("doo_new", json!({"path": path, "text": "[unit]\nid=hpea\nx=1\ny=2\nnumber=9\n"})).unwrap();
    let back = doo_rs::parse_units(&std::fs::read(&path).unwrap(), Layout::Classic).unwrap();
    assert_eq!(back.units.iter().map(|u| (u.placement.id.to_string(), u.player)).collect::<Vec<_>>(), [("ogru".into(), 3), ("ogru".into(), 1), ("hpea".into(), 0)]);
    run("doo_delete", json!({"path": path, "index": [0, 2]})).unwrap();
    assert_eq!(doo_rs::parse_units(&std::fs::read(&path).unwrap(), Layout::Classic).unwrap().units.len(), 1);

    // Errors keep the lines of the text given.
    let err = run("doo_new", json!({"path": path, "text": "[unit]\nid=hpea\nhp=x\n"})).unwrap_err();
    assert!(err.starts_with("nothing written:\nline 3: `hp`"), "{err}");
    assert!(run("doo_set", json!({"path": path, "index": 5, "text": "[unit]\n"})).unwrap_err().contains("no index 5"));
}
