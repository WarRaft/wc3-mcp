//! The MCP protocol: JSON-RPC 2.0 messages, one per line.
//!
//! - `initialize` answers with the protocol version the client asked for,
//!   the server's name and that it has tools;
//! - `tools/list` lists them ([`crate::slk::tools`], [`crate::mpq::tools`],
//!   [`crate::obj::tools`], [`crate::wts::tools`], [`crate::w3c::tools`],
//!   [`crate::w3e::tools`], [`crate::w3i::tools`],
//!   [`crate::w3r::tools`], [`crate::w3s::tools`], [`crate::wpm::tools`], [`crate::imp::tools`],
//!   [`crate::mmp::tools`], [`crate::wct::tools`], [`crate::shd::tools`],
//!   [`crate::doo::tools`], [`crate::wtg::tools`], [`crate::trg::tools`], [`crate::w3o::tools`],
//!   [`crate::txt2::tools`]);
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
        "instructions": "Warcraft III files. Projects (wc3.bni: a map's game folders, roots, code): \
            the tools that need the game's files (names, UI\\TriggerData.txt, metadata) take `game` and `roots`, \
            and without them the settings of the project of the file they work on (the nearest wc3.bni above it, \
            or `project`); project_info shows a project's settings and where the editor's files come from, \
            project_write changes them, project_build builds its map (a [build]: the roots' files and war3map.j made anew). A modified editor's additions (natives in the GUI) go in a folder root as \
            UI\\TriggerData.patch.txt (and TriggerStrings.patch.txt ...), which change the game's file by meaning. \
            A project's code (JASS, vJASS, Zinc, Lua, AngelScript): code_check gives its problems as the editor \
            shows them, jass_rename renames a symbol everywhere it means it (ask it rather than editing names by \
            hand), jass_natives looks up the game's natives and functions, jass_build builds JASS files into one script. \
            SLK tables (.slk: Units\\UnitBalance.slk, \
            the *MetaData.slk ...): slk_info for the columns, slk_read to query rows, \
            slk_get / slk_set for one cell, slk_lint for problems; a table can be read \
            from inside an MPQ archive or a map (archive=...). MPQ archives and maps \
            (.mpq, .w3x, .w3m): mpq_info, mpq_list (masks like Units\\*.slk), mpq_read \
            (text, or hex for binary), mpq_extract to a folder, mpq_verify; and to change \
            them mpq_create, mpq_add, mpq_remove, mpq_rename. A Reforged install's CASC storage (the folder with .build.info): casc_list names its files (War3.w3mod:..., the HD ones War3.w3mod:_HD.w3mod:...), casc_read gives one. An archive without a \
            (listfile) needs names from listfiles=[...] to find its files. Object data \
            (war3map.w3a abilities, .w3b, .w3d, .w3h, .w3q, .w3t items, .w3u units): \
            obj_list, obj_get, obj_text, obj_check, and to change them obj_set, obj_new, \
            obj_delete; the game's files (`game`, or the project's) give field names, types and checks; \
            TRIGSTR_<n> values show as their text from the map's war3map.wts. String tables \
            (war3map.wts): wts_list, wts_get, wts_resolve, wts_lint, and wts_set to change one. \
            Cameras (war3map.w3c): w3c_list, and w3c_set, w3c_new, w3c_delete; the layout \
            (1.31 added a local rotation) comes from the map's war3map.w3i, else 1.27a's. \
            Terrain (war3map.w3e): w3e_info for tilesets and size, w3e_image for a picture from above (PNG), w3e_brush to lay the editor's terrain brushes, w3e_tiles to set its ground tiles (the tile setter), map_resize to make a project's map bigger or smaller, w3e_points for a region \
            of the grid (column, row from the bottom left), w3e_set to change one. \
            Map info (war3map.w3i: name, description, players, forces, tech tree, random tables): \
            w3i_text gives it as text, w3i_write writes that text back after checking it. \
            Regions (war3map.w3r): w3r_list, and w3r_set, w3r_new, w3r_delete; a region by name or index. \
            Sounds (war3map.w3s): w3s_list, and w3s_set, w3s_new, w3s_delete; a sound by its variable or index. \
            Pathing (war3map.wpm): wpm_info, wpm_image for a picture (PNG), wpm_cells for a region, wpm_set to change one. \
            Imported files (war3map.imp): imp_list, imp_text, imp_write. Minimap icons (war3map.mmp): \
            mmp_list, mmp_text, mmp_write. Custom script (war3map.wct): wct_text, wct_write. \
            Shadow map (war3map.shd): shd_image. Doodads and units placed (war3map.doo, \
            war3mapUnits.doo): doo_list with filters, doo_get for the text of some, doo_set, doo_new, \
            doo_delete. GUI triggers (war3map.wtg; UI\\TriggerData.txt from the game's files): wtg_list, \
            wtg_get for a trigger's text, wtg_set, wtg_new, wtg_delete, wtg_text and wtg_write for the \
            whole file. Trigger files of their own (.trg: one trigger each, a folder per category, \
            the globals in _variables.trg; the source a build makes the map's script of): trg_extract takes \
            a map's triggers out into a folder, trg_list shows a project's triggers and variables, trg_read \
            a trigger as World Editor's phrases with each line's place and call (read before changing one), \
            trg_check the problems, trg_jass what it's built into; trg_functions and trg_types look up \
            World Editor's functions (phrase, parameters' types, defaults, a ready line), types and presets \
            to write lines; trg_write writes a file after checking it, trg_rename renames a trigger and its \
            references, trg_variable changes or deletes a variable safely (its uses renamed, what breaks \
            listed first). Object collections (.w3o): w3o_info, w3o_split into each kind's file, w3o_join. \
            Models (.mdx, .mdl): mdx_info for what's in one (geosets, materials, textures, sequences), mdx_text for it as MDL text, from disk, a map or the game's files. \
            txt2, XGM's markup (.txt2 articles): txt2_syntax before writing it, txt2_check, txt2_outline for \
            the parts of a big article with their lines, txt2_get for one part's text, txt2_html for the page.",
    })
}

fn call(params: &Value) -> Value {
    let name = params.get("name").and_then(Value::as_str).unwrap_or("");
    let args = params.get("arguments").cloned().unwrap_or(json!({}));
    let answer = crate::mpq::call(name, &args)
        .or_else(|| crate::casc::call(name, &args))
        .or_else(|| crate::obj::call(name, &args))
        .or_else(|| crate::wts::call(name, &args))
        .or_else(|| crate::w3c::call(name, &args))
        .or_else(|| crate::w3e::call(name, &args))
        .or_else(|| crate::w3i::call(name, &args))
        .or_else(|| crate::w3r::call(name, &args))
        .or_else(|| crate::w3s::call(name, &args))
        .or_else(|| crate::wpm::call(name, &args))
        .or_else(|| crate::imp::call(name, &args))
        .or_else(|| crate::mmp::call(name, &args))
        .or_else(|| crate::wct::call(name, &args))
        .or_else(|| crate::shd::call(name, &args))
        .or_else(|| crate::doo::call(name, &args))
        .or_else(|| crate::wtg::call(name, &args))
        .or_else(|| crate::trg::call(name, &args))
        .or_else(|| crate::w3o::call(name, &args))
        .or_else(|| crate::txt2::call(name, &args))
        .or_else(|| crate::mdx::call(name, &args))
        .or_else(|| crate::project::call(name, &args))
        .or_else(|| crate::code::call(name, &args))
        .or_else(|| crate::nav::call(name, &args))
        .unwrap_or_else(|| crate::slk::call(name, &args));
    match answer {
        // A tool that answers with more than text (a picture): its content
        // as it is.
        Ok(v) if v.get("mcp_content").is_some_and(Value::is_array) => {
            json!({"content": v["mcp_content"], "isError": false})
        }
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

/// Every tool: the SLK ones, the MPQ ones, the object data ones, the
/// string table ones, the camera ones, the terrain ones.
fn tools() -> Value {
    let mut all = crate::slk::tools().as_array().cloned().unwrap_or_default();
    all.extend(crate::mpq::tools().as_array().cloned().unwrap_or_default());
    all.extend(crate::casc::tools().as_array().cloned().unwrap_or_default());
    all.extend(crate::obj::tools().as_array().cloned().unwrap_or_default());
    all.extend(crate::wts::tools().as_array().cloned().unwrap_or_default());
    all.extend(crate::w3c::tools().as_array().cloned().unwrap_or_default());
    all.extend(crate::w3e::tools().as_array().cloned().unwrap_or_default());
    all.extend(crate::w3i::tools().as_array().cloned().unwrap_or_default());
    all.extend(crate::w3r::tools().as_array().cloned().unwrap_or_default());
    all.extend(crate::w3s::tools().as_array().cloned().unwrap_or_default());
    all.extend(crate::wpm::tools().as_array().cloned().unwrap_or_default());
    all.extend(crate::imp::tools().as_array().cloned().unwrap_or_default());
    all.extend(crate::mmp::tools().as_array().cloned().unwrap_or_default());
    all.extend(crate::wct::tools().as_array().cloned().unwrap_or_default());
    all.extend(crate::shd::tools().as_array().cloned().unwrap_or_default());
    all.extend(crate::doo::tools().as_array().cloned().unwrap_or_default());
    all.extend(crate::wtg::tools().as_array().cloned().unwrap_or_default());
    all.extend(crate::trg::tools().as_array().cloned().unwrap_or_default());
    all.extend(crate::w3o::tools().as_array().cloned().unwrap_or_default());
    all.extend(crate::txt2::tools().as_array().cloned().unwrap_or_default());
    all.extend(crate::mdx::tools().as_array().cloned().unwrap_or_default());
    all.extend(crate::project::tools().as_array().cloned().unwrap_or_default());
    all.extend(crate::code::tools().as_array().cloned().unwrap_or_default());
    all.extend(crate::nav::tools().as_array().cloned().unwrap_or_default());
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
