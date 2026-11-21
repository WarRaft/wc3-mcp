use super::*;

fn temp(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("wc3-mcp-w3c-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn run(name: &str, args: Value) -> Result<Value, String> {
    call(name, &args).expect("a camera tool")
}

#[test]
fn new_set_list_delete_in_1_27a_layout() {
    let dir = temp("classic");
    let path = dir.join("war3map.w3c");
    let added = run(
        "w3c_new",
        json!({"path": path, "values": {"name": "Intro Shot", "target_x": 512.0}}),
    )
    .unwrap();
    assert_eq!(added["layout_from"], "no war3map.w3i: 1.27a");
    assert_eq!(added["added"]["variable"], "gg_cam_Intro_Shot");
    assert_eq!(added["added"]["distance"], 1650.0);
    assert!(added["added"].get("local_yaw").is_none());
    run("w3c_new", json!({"path": path, "values": {}})).unwrap();

    let set = run(
        "w3c_set",
        json!({"path": path, "camera": "Intro Shot", "values": {"far_z": 3000.0}}),
    )
    .unwrap();
    assert_eq!(
        (
            set["before"]["far_z"].clone(),
            set["after"]["far_z"].clone()
        ),
        (json!(5000.0), json!(3000.0))
    );
    let err = run(
        "w3c_set",
        json!({"path": path, "camera": 0, "values": {"local_yaw": 1.0}}),
    )
    .unwrap_err();
    assert!(err.contains("1.31"), "{err}");
    assert!(run("w3c_set", json!({"path": path, "camera": 5, "values": {}})).is_err());

    let list = run("w3c_list", json!({"path": path})).unwrap();
    assert_eq!(list["count"], 2);
    assert_eq!(list["cameras"][1]["name"], "Camera 002");
    assert_eq!(list["cameras"][0]["far_z"], 3000.0);

    run("w3c_delete", json!({"path": path, "camera": 0})).unwrap();
    assert_eq!(
        std::fs::read(&path).unwrap().len(),
        8 + 40 + "Camera 002".len() + 1
    );
}

#[test]
fn layout_from_the_w3i_next_to_it() {
    let dir = temp("local");
    let path = dir.join("war3map.w3c");
    std::fs::write(dir.join("war3map.w3i"), 31u32.to_le_bytes()).unwrap();
    let added = run(
        "w3c_new",
        json!({"path": path, "values": {"local_yaw": 15.0}}),
    )
    .unwrap();
    assert_eq!(
        (added["layout"].clone(), added["layout_from"].clone()),
        (json!("1.31 and later"), json!("war3map.w3i, version 31"))
    );
    assert_eq!(
        std::fs::read(&path).unwrap().len(),
        8 + 52 + "Camera 001".len() + 1
    );
    assert_eq!(
        run("w3c_list", json!({"path": path})).unwrap()["cameras"][0]["local_yaw"],
        15.0
    );
    // Read in the older layout by hand, it doesn't fit.
    let err = run("w3c_list", json!({"path": path, "w3i_version": 25})).unwrap_err();
    assert!(err.contains("Reign of Chaos to 1.30"), "{err}");
}
