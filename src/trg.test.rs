use super::*;

fn temp(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("wc3-mcp-trg-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn run(name: &str, args: Value) -> Result<Value, String> {
    call(name, &args).expect("a tool of this module")
}

const DATA: &str = concat!(
    "[TriggerCategories]\nTC_UNIT=WESTRING_TRIGCAT_UNIT,ReplaceableTextures\\WorldEditUI\\Actions-Unit\n",
    "[TriggerTypes]\nplayer=0,1,1,WESTRING_TRIGTYPE_player\nunitcode=0,1,1,WESTRING_TRIGTYPE_unitcode,integer\n",
    "integer=1,1,1,WESTRING_TRIGTYPE_integer\ntrigger=1,1,1,WESTRING_TRIGTYPE_trigger\n",
    "[TriggerParams]\nPlayer00=0,player,Player(0),WESTRING_PLAYER_00\nPlayer01=0,player,Player(1),WESTRING_PLAYER_01\n",
    "[TriggerEvents]\nMapInitializationEvent=0,nothing\n",
    "[TriggerActions]\nCreateNUnitsAtLoc=0,integer,unitcode,player\n_CreateNUnitsAtLoc_Defaults=1,hfoo,Player00\n_CreateNUnitsAtLoc_Category=TC_UNIT\n",
    "SetVariable=0,AnyGlobal,Null\nTriggerExecute=0,trigger\nCustomScriptCode=0,scriptcode\nDoNothing=0,nothing\n",
);
const STRINGS: &str = concat!(
    "[TriggerEventStrings]\nMapInitializationEvent=\"Map initialization\"\nMapInitializationEvent=\"Map initialization\"\n",
    "[TriggerActionStrings]\nCreateNUnitsAtLoc=\"Create Units\"\nCreateNUnitsAtLoc=\"Create \",~Number,\" \",~Unit,\" for \",~Player\n",
    "CreateNUnitsAtLocHint=\"Creates units.\"\n",
    "SetVariable=\"Set Variable\"\nSetVariable=\"Set \",~Variable,\" = \",~Value\n",
    "TriggerExecute=\"Run Trigger\"\nTriggerExecute=\"Run \",~Trigger\n",
);

/// A game folder with the editor's files, and a map's war3map.wtg of
/// `text`.
fn setup(name: &str, text: &str) -> (PathBuf, Value, PathBuf) {
    let dir = temp(name);
    let game = dir.join("game");
    std::fs::create_dir_all(game.join("UI")).unwrap();
    std::fs::write(game.join("UI/TriggerData.txt"), DATA).unwrap();
    std::fs::write(game.join("UI/TriggerStrings.txt"), STRINGS).unwrap();
    let wtg = dir.join("war3map.wtg");
    let data = wtg_rs::TriggerData::parse(DATA);
    let parsed = wtg_rs::text::from_text(text, Some(&data));
    assert_eq!(parsed.errors, [], "{text}");
    std::fs::write(&wtg, parsed.file.unwrap().to_bytes().unwrap()).unwrap();
    (dir, json!([game]), wtg)
}

const MAP: &str = concat!(
    "[wtg]\nversion=7\ngame=2\n\n",
    "[category]\nindex=0\nname=\"Init\"\n\n",
    "[variable]\nname=\"who\"\ntype=\"player\"\nsize=1\n\n",
    "[trigger]\nname=\"Шляпа\"\ndescription=\"Spawns\"\ncategory=0\n",
    "event=MapInitializationEvent()\n",
    "action=CreateNUnitsAtLoc(\"1\", \"hfoo\", $who)\n",
    "action=SetVariable($who, Player01)\n\n",
    "[trigger]\nname=\"Run it\"\ndescription=\"\"\ncategory=0\n",
    "action=TriggerExecute($gg_trg___________u)\n",
    "action=CustomScriptCode(\"call TriggerExecute(gg_trg___________u) // gg_trg___________u\")\n",
);

#[test]
fn extract_list_read_check_jass() {
    let (dir, game, wtg) = setup("read", MAP);
    let out = dir.join("triggers");
    let e = run("trg_extract", json!({"path": wtg, "dir": out, "game": game})).unwrap();
    assert_eq!(e["files"], json!(["_variables.trg", "000_Init/000_Шляпа.trg", "000_Init/001_Run it.trg"]));
    // Not again, unless replace.
    assert!(run("trg_extract", json!({"path": wtg, "dir": out, "game": game})).unwrap_err().contains("nothing written"));
    run("trg_extract", json!({"path": wtg, "dir": out, "game": game, "replace": true})).unwrap();

    let l = run("trg_list", json!({"dir": out, "game": game})).unwrap();
    assert_eq!(l["triggers"][0]["name"], "Шляпа");
    assert_eq!(l["triggers"][0]["script"], "gg_trg___________u");
    assert_eq!((l["triggers"][0]["events"].clone(), l["triggers"][0]["actions"].clone()), (json!(1), json!(2)));
    assert_eq!(l["triggers"][1]["problems"], 0, "{l}");
    assert_eq!(l["variables"][0], json!({"file": "_variables.trg", "name": "who", "type": "player", "array": false, "scope": "global"}));

    let hat = out.join("000_Init/000_Шляпа.trg");
    let r = run("trg_read", json!({"file": hat, "game": game})).unwrap();
    let r = r.as_str().unwrap();
    assert!(r.starts_with("Trigger \"Шляпа\" (gg_trg___________u), file 000_Шляпа.trg\nComment: Spawns\n"), "{r}");
    assert!(r.contains("L4 Map initialization\n       = MapInitializationEvent()\n"), "{r}");
    assert!(r.contains("L5 WESTRING_TRIGCAT_UNIT - Create [1] [hfoo] for [who]\n       = CreateNUnitsAtLoc(\"1\", \"hfoo\", $who)\n"), "{r}");
    assert!(!r.contains("Problems"), "{r}");

    // A problem: a variable no file declares.
    std::fs::write(&hat, std::fs::read_to_string(&hat).unwrap().replace("$who)", "$nobody)")).unwrap();
    let c = run("trg_check", json!({"dir": out, "game": game})).unwrap();
    assert_eq!(c["problems"], json!([{"file": "000_Init/000_Шляпа.trg", "line": 5, "message": "the variable `nobody` isn't declared in the project's trigger files ([variable])"}]), "{c}");
    let r = run("trg_read", json!({"file": hat, "game": game, "calls": false})).unwrap();
    assert!(r.as_str().unwrap().contains("Create [1] [hfoo] for [nobody?]\n") && r.as_str().unwrap().contains("Problems\n  line 5:"), "{r}");

    let j = run("trg_jass", json!({"file": out.join("000_Init/001_Run it.trg"), "game": game})).unwrap();
    assert!(j.as_str().unwrap().contains("function Trig_Run_it_Actions"), "{j}");
}

#[test]
fn functions_and_types() {
    let (_, game, _) = setup("words", "[wtg]\nversion=7\ngame=2\n");
    let f = run("trg_functions", json!({"game": game, "query": "create"})).unwrap();
    assert_eq!(f["total"], 1);
    assert_eq!(f["functions"][0], json!({"name": "CreateNUnitsAtLoc", "kind": "action", "title": "Create Units", "category": "WESTRING_TRIGCAT_UNIT"}));
    let f = run("trg_functions", json!({"game": game, "name": "CreateNUnitsAtLoc"})).unwrap();
    assert_eq!(f[0]["phrase"], "Create {0} {1} for {2}");
    assert_eq!(f[0]["line"], "action=CreateNUnitsAtLoc(\"1\", \"hfoo\", Player00)");
    assert_eq!(f[0]["params"][2], json!({"index": 2, "name": "Player", "type": "player", "type_name": "WESTRING_TRIGTYPE_player", "default": "Player00", "presets": 2, "value": false}));
    assert_eq!(f[0]["hint"], "Creates units.");
    assert!(run("trg_functions", json!({"game": game, "kind": "x"})).unwrap_err().contains("no kind"));

    let t = run("trg_types", json!({"game": game})).unwrap();
    assert!(t.as_array().unwrap().iter().any(|x| x["type"] == "unitcode" && x["base"] == "integer" && x["value"] == true), "{t}");
    let t = run("trg_types", json!({"game": game, "type": "player", "query": "01"})).unwrap();
    assert_eq!(t["presets"], json!([{"preset": "Player01", "display": "WESTRING_PLAYER_01", "script": "Player(1)"}]));
}

#[test]
fn write_rename_and_variables() {
    let (dir, game, wtg) = setup("change", MAP);
    let out = dir.join("triggers");
    run("trg_extract", json!({"path": wtg, "dir": out, "game": game})).unwrap();
    let hat = out.join("000_Init/000_Шляпа.trg");
    let runner = out.join("000_Init/001_Run it.trg");

    // Written after checking; a function with the wrong parameters isn't.
    let bad = run("trg_write", json!({"file": out.join("000_Init/002_New.trg"), "game": game, "text": "[trigger]\naction=DoNothing(\"1\")\n"})).unwrap_err();
    assert!(bad.contains("nothing written") && bad.contains("line 2"), "{bad}");
    let w = run("trg_write", json!({"file": out.join("000_Init/002_New.trg"), "game": game, "text": "[trigger]\naction=SetVariable($ghost, Player00)\n"})).unwrap();
    assert_eq!(w["problems"][0]["line"], 2, "{w}");

    // Renamed: the references follow, the comment in the custom script doesn't.
    let r = run("trg_rename", json!({"file": hat, "name": "Hat", "game": game})).unwrap();
    assert_eq!((r["from"].clone(), r["to"].clone()), (json!("gg_trg___________u"), json!("gg_trg_Hat")));
    let t = std::fs::read_to_string(&runner).unwrap();
    assert!(t.contains("action=TriggerExecute($gg_trg_Hat)\n"), "{t}");
    assert!(t.contains("call TriggerExecute(gg_trg_Hat) // gg_trg___________u"), "{t}");
    assert!(std::fs::read_to_string(&hat).unwrap().contains("name=\"Hat\""));
    // Not to another's name.
    assert!(run("trg_rename", json!({"file": hat, "name": "Run it", "game": game})).unwrap_err().contains("names 001_Run it.trg gg_trg_Run_it too"));

    // A variable renamed: its uses too, in every file.
    let vars = out.join("_variables.trg");
    let v = run("trg_variable", json!({"file": vars, "variable": "who", "name": "owner", "game": game})).unwrap();
    assert_eq!((v["done"].clone(), v["renamed"].clone()), (json!(true), json!(2)), "{v}");
    assert!(std::fs::read_to_string(&hat).unwrap().contains("CreateNUnitsAtLoc(\"1\", \"hfoo\", $owner)"));
    // Deleting a used one: listed, nothing done without confirm.
    let v = run("trg_variable", json!({"file": vars, "variable": "owner", "delete": true, "game": game})).unwrap();
    assert_eq!(v["done"], false);
    assert_eq!(v["places"].as_array().unwrap().len(), 2, "{v}");
    let v = run("trg_variable", json!({"file": vars, "variable": "owner", "delete": true, "confirm": true, "game": game})).unwrap();
    assert_eq!(v["done"], true);
    assert!(!std::fs::read_to_string(&vars).unwrap().contains("owner"));
}
