use super::*;

/// A Reforged install (`CASC_GAME`; skipped without it).
#[test]
fn lists_and_reads_the_games_files() {
    let Some(game) = std::env::var("CASC_GAME").ok() else { return };
    let out = call("casc_list", &json!({"game": game, "text": "footman.mdx", "on_disk": true})).unwrap().unwrap();
    assert!(out["total"].as_u64().unwrap() >= 2, "{out}");
    let slk = call("casc_read", &json!({"game": game, "name": "War3.w3mod:Units\\UnitData.slk"})).unwrap().unwrap();
    assert!(slk["text"].as_str().unwrap().starts_with("ID;"));
    let mdx = call("casc_read", &json!({"game": game, "name": "War3.w3mod:Units/Human/Footman/Footman.mdx"})).unwrap().unwrap();
    assert!(mdx["hex"].as_str().unwrap().starts_with("4d444c58"));
    assert!(call("casc_read", &json!({"game": game, "name": "nothing"})).unwrap().is_err());
}

#[test]
fn no_storage_said() {
    let e = call("casc_list", &json!({"game": "/nowhere"})).unwrap().unwrap_err();
    assert!(e.contains(".build.info"), "{e}");
}
