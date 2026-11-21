use super::*;

const SHEET: &str = "ID;PWXL;N;E\nB;Y3;X3\nC;Y1;X1;K\"unitID\"\nC;X2;K\"hp\"\nC;X3;K\"flying\"\nC;Y2;X1;K\"hfoo\"\nC;X2;K420\nC;X3;KFALSE\nC;Y3;X1;K\"hkni\"\nC;X2;K835.5;ER[-1]C*2\nE\n";

fn file(name: &str, text: &str) -> String {
    let dir = std::env::temp_dir().join(format!("wc3-mcp-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("t.slk");
    std::fs::write(&path, text).unwrap();
    path.to_string_lossy().into_owned()
}

#[test]
fn info_and_read() {
    let path = file("read", SHEET);
    let info = call("slk_info", &json!({"path": path})).unwrap();
    assert_eq!(
        (
            info["rows"].clone(),
            info["columns"].clone(),
            info["problems"].clone()
        ),
        (json!(3), json!(3), json!(0))
    );
    assert_eq!(
        info["column_names"][1],
        json!({"column": 2, "letter": "B", "name": "hp"})
    );

    let all = call("slk_read", &json!({"path": path})).unwrap();
    assert_eq!(all["total"], 2);
    assert_eq!(
        all["rows"][0],
        json!({"row": 2, "unitID": "hfoo", "hp": 420, "flying": false})
    );
    // Keys in the table's order.
    assert_eq!(
        all["rows"][0].to_string(),
        r#"{"row":2,"unitID":"hfoo","hp":420,"flying":false}"#
    );
    let one = call(
        "slk_read",
        &json!({"path": path, "where": {"unitID": "hkni"}, "columns": ["hp", "A"]}),
    )
    .unwrap();
    assert_eq!(
        one["rows"],
        json!([{"row": 3, "hp": 835.5, "unitID": "hkni"}])
    );
    let found = call(
        "slk_read",
        &json!({"path": path, "contains": "FOO", "limit": 1}),
    )
    .unwrap();
    assert_eq!(
        (found["total"].clone(), found["rows"][0]["row"].clone()),
        (json!(1), json!(2))
    );
    let paged = call("slk_read", &json!({"path": path, "offset": 1})).unwrap();
    assert_eq!(
        (
            paged["total"].clone(),
            paged["rows"].as_array().unwrap().len()
        ),
        (json!(2), 1)
    );
    assert!(
        call("slk_read", &json!({"path": path, "columns": ["nope"]}))
            .unwrap_err()
            .starts_with("no column 'nope'")
    );
}

#[test]
fn get_and_set() {
    let path = file("set", SHEET);
    let cell = call("slk_get", &json!({"path": path, "row": 3, "column": "hp"})).unwrap();
    assert_eq!(cell["value"], 835.5);
    assert_eq!(cell["formula"], "R[-1]C*2");
    assert_eq!(cell["kind"], "number");
    let set = call(
        "slk_set",
        &json!({"path": path, "row": 2, "column": 2, "value": 500}),
    )
    .unwrap();
    assert_eq!(
        (
            set["before"].clone(),
            set["after"].clone(),
            set["changed"].clone()
        ),
        (json!("420"), json!("500"), json!(true))
    );
    // A new row, then a cleared cell.
    call(
        "slk_set",
        &json!({"path": path, "row": 4, "column": "unitID", "value": "hpea"}),
    )
    .unwrap();
    call(
        "slk_set",
        &json!({"path": path, "row": 2, "column": "C", "value": null}),
    )
    .unwrap();
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.contains("C;X2;K500\n"), "{text}");
    assert!(text.contains("B;Y4;X3\n"), "{text}");
    assert!(!text.contains("KFALSE"), "{text}");
    let lint = call("slk_lint", &json!({"path": path})).unwrap();
    assert_eq!(lint, json!({"errors": [], "warnings": []}));
}

#[test]
fn from_an_archive() {
    let dir = std::env::temp_dir().join(format!("wc3-mcp-mpq-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let mpq = dir.join("t.w3x");
    storm_rs::archive::MpqArchiveBuilder::new(&mpq)
        .add_file("Units\\UnitBalance.slk", SHEET.as_bytes().to_vec())
        .build()
        .unwrap();
    let mpq = mpq.to_string_lossy();
    let info = call(
        "slk_info",
        &json!({"path": "Units/UnitBalance.slk", "archive": mpq}),
    )
    .unwrap();
    assert_eq!(info["rows"], 3);
    assert!(
        call("slk_info", &json!({"path": "nope.slk", "archive": mpq}))
            .unwrap_err()
            .ends_with("no nope.slk in it")
    );
    // Writing goes to files on disk only.
    assert!(
        call(
            "slk_set",
            &json!({"path": "Units/UnitBalance.slk", "row": 1, "column": 1, "value": "x"})
        )
        .is_err()
    );
}
