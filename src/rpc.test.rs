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
    let want = "slk_info slk_read slk_get slk_set slk_lint \
                mpq_info mpq_list mpq_read mpq_extract mpq_verify mpq_create mpq_add mpq_remove mpq_rename \
                obj_list obj_get obj_text obj_check obj_set obj_new obj_delete \
                wts_list wts_get wts_resolve wts_set wts_lint \
                w3c_list w3c_set w3c_new w3c_delete \
                w3e_info w3e_points w3e_set w3i_text w3i_write w3r_list w3r_set w3r_new w3r_delete w3s_list w3s_set w3s_new w3s_delete wpm_info wpm_image wpm_cells wpm_set \
                imp_list imp_text imp_write mmp_list mmp_text mmp_write wct_text wct_write shd_image \
                doo_list doo_get doo_set doo_new doo_delete wtg_list wtg_get wtg_set wtg_new wtg_delete wtg_text wtg_write \
                trg_extract trg_list trg_read trg_check trg_jass trg_functions trg_types trg_write trg_rename trg_variable \
                w3o_info w3o_split w3o_join txt2_syntax txt2_check txt2_outline txt2_get txt2_html project_info project_write code_check";
    assert_eq!(names, want.split_whitespace().collect::<Vec<_>>());
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
