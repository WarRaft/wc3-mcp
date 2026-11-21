use super::*;

fn temp(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("wc3-mcp-w3s-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn run(name: &str, args: Value) -> Result<Value, String> {
    call(name, &args).expect("a sound tool")
}

#[test]
fn new_set_list_delete() {
    let dir = temp("all");
    let path = dir.join("war3map.w3s");
    let added = run("w3s_new", json!({"path": path, "values": {"name": "gg_snd_Rain", "file": "Sound\\Ambient\\Rain.wav"}})).unwrap();
    assert_eq!(added["added"]["volume"], "default");
    assert_eq!(added["added"]["fade_in"], 10);
    assert_eq!(added["added"]["flag_names"], json!(["3D sound", "stop when out of range"]));
    run("w3s_new", json!({"path": path, "values": {"name": "gg_snd_B", "file": "b.wav", "pitch": 1.04, "flags": 1}})).unwrap();

    let set = run("w3s_set", json!({"path": path, "sound": "gg_snd_Rain", "values": {"volume": 127, "pitch": 0.5, "eax": "SpellsEAX"}})).unwrap();
    assert_eq!(set["before"]["volume"], "default");
    assert_eq!((set["after"]["volume"].clone(), set["after"]["pitch"].clone()), (json!(127), json!(0.5)));
    let set = run("w3s_set", json!({"path": path, "sound": 0, "values": {"volume": "default"}})).unwrap();
    assert_eq!(set["after"]["volume"], "default");

    let list = run("w3s_list", json!({"path": path})).unwrap();
    assert_eq!(list["count"], 2);
    assert_eq!(list["sounds"][1]["pitch"], 1.04);
    assert_eq!(list["sounds"][0]["eax"], "SpellsEAX");
    assert_eq!(list["sounds"][0]["channel_name"], "default");
    let set = run("w3s_set", json!({"path": path, "sound": 1, "values": {"channel": 5, "flags": 17}})).unwrap();
    assert_eq!(set["after"]["channel_name"], "Combat");
    assert_eq!(set["after"]["flag_names"], json!(["looping", "imported"]));

    let before = std::fs::read(&path).unwrap();
    for (tool, args, want) in [
        ("w3s_set", json!({"path": path, "sound": 0, "values": {"name": "gg_snd_B"}}), "is \"gg_snd_B\" too"),
        ("w3s_set", json!({"path": path, "sound": 0, "values": {"volume": 1.5}}), "whole number"),
        ("w3s_set", json!({"path": path, "sound": 0, "values": {"loud": 1}}), "no field `loud`"),
        ("w3s_set", json!({"path": path, "sound": 5, "values": {}}), "no sound 5"),
        ("w3s_new", json!({"path": path, "values": {"name": "gg_snd_C"}}), "needs file"),
    ] {
        let err = run(tool, args).unwrap_err();
        assert!(err.contains(want), "{err} / {want}");
    }
    assert_eq!(std::fs::read(&path).unwrap(), before);

    // The game's words with `game`.
    let game = dir.join("game");
    std::fs::create_dir_all(game.join("UI")).unwrap();
    std::fs::write(game.join("UI/WorldEditData.txt"), "[SoundChannels]\n05=1,WESTRING_SOUNDCHANNEL_COMBAT\n").unwrap();
    std::fs::write(game.join("UI/WorldEditStrings.txt"), "[WorldEditStrings]\nWESTRING_SOUNDCHANNEL_COMBAT=Бой\nWESTRING_SNDPROPS_LOOPING=&Повтор\n").unwrap();
    let named = run("w3s_list", json!({"path": path, "game": [game]})).unwrap();
    assert_eq!(named["sounds"][1]["channel_name"], "Бой");
    assert_eq!(named["sounds"][1]["flag_names"], json!(["Повтор", "imported"]));

    let removed = run("w3s_delete", json!({"path": path, "sound": "gg_snd_Rain"})).unwrap();
    assert_eq!(removed["count"], 1);
}
