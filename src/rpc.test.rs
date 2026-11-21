use super::*;

fn ask(msg: Value) -> Value {
    serde_json::from_str(&handle(&msg.to_string()).expect("an answer")).unwrap()
}

#[test]
fn initialize_and_list() {
    let a = ask(
        json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"protocolVersion": "2025-03-26", "capabilities": {}}}),
    );
    assert_eq!(a["id"], 1);
    assert_eq!(a["result"]["protocolVersion"], "2025-03-26");
    assert_eq!(a["result"]["serverInfo"]["name"], "wc3-mcp");
    assert!(a["result"]["capabilities"]["tools"].is_object());
    let names: Vec<String> =
        ask(json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list"}))["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t["name"].as_str().unwrap().to_string())
            .collect();
    assert_eq!(
        names,
        ["slk_info", "slk_read", "slk_get", "slk_set", "slk_lint"]
    );
}

#[test]
fn notifications_errors_and_failed_calls() {
    assert_eq!(
        handle(&json!({"jsonrpc": "2.0", "method": "notifications/initialized"}).to_string()),
        None
    );
    assert_eq!(
        ask(json!({"jsonrpc": "2.0", "id": "x", "method": "nope"}))["error"]["code"],
        -32601
    );
    assert_eq!(
        ask(json!({"jsonrpc": "2.0", "id": 3, "method": "ping"}))["result"],
        json!({})
    );
    let a: Value = serde_json::from_str(&handle("{oops").unwrap()).unwrap();
    assert_eq!(a["error"]["code"], -32700);
    // A tool that fails answers with isError, for the model to read.
    let a = ask(
        json!({"jsonrpc": "2.0", "id": 4, "method": "tools/call", "params": {"name": "slk_info", "arguments": {"path": "/nonexistent.slk"}}}),
    );
    assert_eq!(a["result"]["isError"], true);
    assert!(
        a["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .starts_with("/nonexistent.slk: ")
    );
}
