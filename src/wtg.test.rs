use super::*;

fn temp(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("wc3-mcp-wtg-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn run(name: &str, args: Value) -> Result<Value, String> {
    call(name, &args).expect("a tool of this module")
}

const DATA: &str = "[TriggerEvents]\nMapInitializationEvent=0,nothing\n[TriggerActions]\nDoNothing=0,nothing\nTriggerSleepAction=0,real\n";

#[test]
fn list_get_set_new_delete_with_the_custom_script() {
    let dir = temp("all");
    let game = dir.join("game");
    std::fs::create_dir_all(game.join("UI")).unwrap();
    std::fs::write(game.join("UI/TriggerData.txt"), DATA).unwrap();
    let path = dir.join("war3map.wtg");
    let text = "[wtg]\nversion=7\ngame=2\n\n[category]\nindex=0\nname=\"Init\"\n\n[trigger]\nname=\"A\"\ndescription=\"\"\ncategory=0\nevent=MapInitializationEvent()\naction=DoNothing()\n";
    std::fs::write(&path, wtg_rs::text::from_text(text, None).file.unwrap().to_bytes().unwrap()).unwrap();
    let wct = dir.join("war3map.wct");
    let custom = wct_rs::CustomText { magic: false, version: 1, comment: vec![], header: wct_rs::Code::new(b""), triggers: vec![wct_rs::Code(None)] };
    std::fs::write(&wct, custom.to_bytes().unwrap()).unwrap();
    let g = json!([game]);

    assert!(run("wtg_list", json!({"path": path})).unwrap_err().contains("UI\\TriggerData.txt"));
    let l = run("wtg_list", json!({"path": path, "game": g})).unwrap();
    assert_eq!((l["triggers"][0]["name"].clone(), l["triggers"][0]["actions"].clone()), (json!("A"), json!(1)));
    assert_eq!(l["triggers"][0]["category_name"], "Init");

    let t = run("wtg_get", json!({"path": path, "game": g, "trigger": ["A"]})).unwrap();
    let t = t.as_str().unwrap();
    let section = &t[t.find("[trigger]").unwrap()..];
    run("wtg_set", json!({"path": path, "game": g, "trigger": 0, "text": section.replace("DoNothing()", "TriggerSleepAction(\"2.00\")")})).unwrap();
    let added = run("wtg_new", json!({"path": path, "game": g, "text": "[trigger]\nname=\"B\"\naction=DoNothing()\n"})).unwrap();
    assert_eq!(added["custom_script"]["pieces"], 2);
    let whole = run("wtg_text", json!({"path": path, "game": g})).unwrap();
    assert!(whole.as_str().unwrap().contains("action=TriggerSleepAction(\"2.00\")\n"), "{whole}");
    run("wtg_delete", json!({"path": path, "game": g, "trigger": [0]})).unwrap();
    assert_eq!(wct_rs::parse(&std::fs::read(&wct).unwrap()).unwrap().triggers.len(), 1);
    let l = run("wtg_list", json!({"path": path, "game": g})).unwrap();
    assert_eq!(l["triggers"][0]["name"], "B");

    let err = run("wtg_set", json!({"path": path, "game": g, "trigger": "B", "text": "[trigger]\nname=\"B\"\naction=DoNothing(1)\n"})).unwrap_err();
    assert!(err.starts_with("nothing written:\nline 3:"), "{err}");
}

#[test]
fn a_131_file_kept_of_its_layout() {
    let dir = temp("tree");
    let game = dir.join("game");
    std::fs::create_dir_all(game.join("UI")).unwrap();
    std::fs::write(game.join("UI/TriggerData.txt"), DATA).unwrap();
    let path = dir.join("war3map.wtg");
    // 1.31's: a category, a comment, a trigger; ids of their types.
    let text = "[wtg]\nversion=7\ngame=2\ntree=1\nmap=\"map.w3x\"\nmap_element=1,0,0,1,0\n\n\
        [category]\nindex=33554432\nname=\"Init\"\nopen=1\n\n\
        [trigger]\nname=\"Read me\"\ndescription=\"\"\ncategory=33554432\nid=67108864\ncomment=1\n\n\
        [trigger]\nname=\"A\"\ndescription=\"\"\ncategory=33554432\nid=50331648\nevent=MapInitializationEvent()\naction=DoNothing()\n";
    let p = wtg_rs::text::from_text(text, None);
    assert_eq!(p.errors, []);
    std::fs::write(&path, p.to_file().unwrap().to_bytes().unwrap()).unwrap();
    // Its custom script: a piece for the trigger only (1.31's: none for a comment).
    let wct = dir.join("war3map.wct");
    let custom = wct_rs::CustomText { magic: true, version: 1, comment: vec![], header: wct_rs::Code::new(b""), triggers: vec![wct_rs::Code::new(b"// A")] };
    std::fs::write(&wct, custom.to_bytes().unwrap()).unwrap();
    let g = json!([game]);

    // As text: the tree's text, what was read.
    let whole = run("wtg_text", json!({"path": path, "game": g})).unwrap();
    assert_eq!(whole.as_str().unwrap(), wtg_rs::text::to_text_any(&p.to_file().unwrap()).unwrap());
    // A trigger new, the comment deleted: still 1.31's, the ids kept, the
    // pieces following (none was the comment's).
    run("wtg_new", json!({"path": path, "game": g, "text": "[trigger]\nname=\"B\"\ncategory=33554432\naction=DoNothing()\n"})).unwrap();
    assert_eq!(wct_rs::parse(&std::fs::read(&wct).unwrap()).unwrap().triggers.len(), 2);
    run("wtg_delete", json!({"path": path, "game": g, "trigger": ["Read me"]})).unwrap();
    let pieces = wct_rs::parse(&std::fs::read(&wct).unwrap()).unwrap().triggers;
    assert_eq!(pieces.len(), 2);
    assert_eq!(pieces[0].text(), b"// A");
    let data = wtg_rs::TriggerData::parse(DATA);
    let wtg_rs::File::Tree(t) = wtg_rs::parse_any(&std::fs::read(&path).unwrap(), &data).unwrap() else { panic!("written as classic") };
    let ids: Vec<(u32, u32)> = t.elements.iter().filter_map(|e| if let wtg_rs::Element::Trigger { kind, id, .. } = e { Some((*kind, *id)) } else { None }).collect();
    assert_eq!(ids, [(8, 0x0300_0000), (8, 0x0300_0001)]);
}
