use super::*;

fn run(name: &str, args: Value) -> Result<Value, String> {
    call(name, &args).expect("a txt2 tool")
}

const TEXT: &str = "== Intro ==\nhello\n((кат Files\n\na|b\nc|d\n))\n== Next ==\n```cpp\nx\n```\n";

#[test]
fn outline_get_check_html() {
    let o = run("txt2_outline", json!({"text": TEXT})).unwrap();
    let parts = o["outline"].as_array().unwrap();
    assert_eq!(parts.iter().map(|p| p["kind"].as_str().unwrap()).collect::<Vec<_>>(), ["heading", "spoiler", "heading", "code"]);
    assert_eq!((parts[1]["line"].clone(), parts[1]["end"].clone(), parts[1]["title"].clone()), (json!(3), json!(7), json!("Files")));
    assert_eq!(parts[1]["parts"][0]["kind"], "table");
    assert_eq!(parts[3]["lang"], "cpp");

    let s = run("txt2_get", json!({"text": TEXT, "title": "files"})).unwrap();
    assert_eq!(s["text"], "((кат Files\n\na|b\nc|d\n))");
    // A heading runs to the next one.
    let h = run("txt2_get", json!({"text": TEXT, "title": "intro"})).unwrap();
    assert_eq!((h["line"].clone(), h["end"].clone()), (json!(1), json!(7)));
    assert_eq!(run("txt2_get", json!({"text": TEXT, "line": 9})).unwrap()["text"], "```cpp\nx\n```");
    assert!(run("txt2_get", json!({"text": TEXT, "title": "nope"})).unwrap_err().contains("no part titled"));

    assert_eq!(run("txt2_check", json!({"text": TEXT})).unwrap()["ok"], true);
    let bad = run("txt2_check", json!({"text": ">> x\nopen\n"})).unwrap();
    assert_eq!(bad["errors"][0]["line"], 1);

    let html = run("txt2_html", json!({"text": "*b*"})).unwrap();
    assert_eq!(html, "<p data-line=\"1\"><b>b</b></p>\n");
    let out = std::env::temp_dir().join(format!("wc3-mcp-txt2-{}.html", std::process::id()));
    run("txt2_html", json!({"text": "*b*", "page": true, "out": out})).unwrap();
    assert!(std::fs::read_to_string(&out).unwrap().starts_with("<!doctype html>"));
    let _ = std::fs::remove_file(out);
    assert!(run("txt2_syntax", json!({})).unwrap().as_str().unwrap().starts_with("## The format"));
}
