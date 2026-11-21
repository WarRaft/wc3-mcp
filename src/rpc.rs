//! The MCP protocol: JSON-RPC 2.0 messages, one per line.
//!
//! - `initialize` answers with the protocol version the client asked for,
//!   the server's name and that it has tools;
//! - `tools/list` lists them ([`crate::slk::tools`], [`crate::mpq::tools`]);
//! - `tools/call` runs one: its answer is text (JSON for data), and a
//!   failure is a result with `isError`, so the model reads why;
//! - `ping` answers `{}`; notifications (no `id`) get no answer.

use serde_json::{Value, json};

/// The version answered when a client doesn't say which it speaks.
const PROTOCOL_VERSION: &str = "2025-06-18";

/// The answer to one line, if it needs one.
pub fn handle(line: &str) -> Option<String> {
    let msg: Value = match serde_json::from_str(line) {
        Ok(v) => v,
        Err(e) => return Some(error(Value::Null, -32700, &format!("not JSON: {e}")).to_string()),
    };
    let id = msg.get("id").cloned()?;
    let method = msg.get("method").and_then(Value::as_str).unwrap_or("");
    let params = msg.get("params").cloned().unwrap_or(Value::Null);
    let answer = match method {
        "initialize" => result(id, initialize(&params)),
        "ping" => result(id, json!({})),
        "tools/list" => result(id, json!({"tools": tools()})),
        "tools/call" => result(id, call(&params)),
        _ => error(id, -32601, &format!("unknown method {method}")),
    };
    Some(answer.to_string())
}

fn initialize(params: &Value) -> Value {
    let version = params
        .get("protocolVersion")
        .and_then(Value::as_str)
        .unwrap_or(PROTOCOL_VERSION);
    json!({
        "protocolVersion": version,
        "capabilities": {"tools": {}},
        "serverInfo": {"name": "wc3-mcp", "version": env!("CARGO_PKG_VERSION")},
        "instructions": "Warcraft III files. SLK tables (.slk: Units\\UnitBalance.slk, \
            the *MetaData.slk ...): slk_info for the columns, slk_read to query rows, \
            slk_get / slk_set for one cell, slk_lint for problems; a table can be read \
            from inside an MPQ archive or a map (archive=...). MPQ archives and maps \
            (.mpq, .w3x, .w3m): mpq_info, mpq_list (masks like Units\\*.slk), mpq_read \
            (text, or hex for binary), mpq_extract to a folder, mpq_verify; and to change \
            them mpq_create, mpq_add, mpq_remove, mpq_rename. An archive without a \
            (listfile) needs names from listfiles=[...] to find its files.",
    })
}

fn call(params: &Value) -> Value {
    let name = params.get("name").and_then(Value::as_str).unwrap_or("");
    let args = params.get("arguments").cloned().unwrap_or(json!({}));
    let answer = crate::mpq::call(name, &args).unwrap_or_else(|| crate::slk::call(name, &args));
    match answer {
        Ok(v) => {
            let text = match v {
                Value::String(s) => s,
                // Compact: big reads cost the model fewer tokens.
                other => other.to_string(),
            };
            json!({"content": [{"type": "text", "text": text}], "isError": false})
        }
        Err(e) => json!({"content": [{"type": "text", "text": e}], "isError": true}),
    }
}

/// Every tool: the SLK ones, then the MPQ ones.
fn tools() -> Value {
    let mut all = crate::slk::tools().as_array().cloned().unwrap_or_default();
    all.extend(crate::mpq::tools().as_array().cloned().unwrap_or_default());
    Value::Array(all)
}

fn result(id: Value, result: Value) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "result": result})
}

fn error(id: Value, code: i64, message: &str) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message}})
}

#[cfg(test)]
#[path = "rpc.test.rs"]
mod tests;
