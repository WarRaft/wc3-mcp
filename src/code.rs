//! The project's code checked as the editor checks it: the server's
//! project model (`wc3_server::workspace`) loaded for the project, in
//! process, so what comes back is what VS Code shows. Each file's parse
//! errors and lint, and its names checked against every file of its
//! project and the game's scripts it runs with (`unknown-name`,
//! `argument-count`): an error between files shows up here.

use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use crate::source::arg_str;
use wc3_server::protocol::ServerMsg;
use wc3_server::workspace::Workspace;

/// Every tool, for `tools/list`.
pub fn tools() -> Value {
    json!([
        {
            "name": "code_check",
            "description": "Every problem of a project's code, as the editor shows it: parse errors, lint (with the project's [lint] settings), and each JASS name and type checked against the whole project and the game's scripts it runs with (common.j, Blizzard.j, through game= and the map's roots): a function, variable or type declared nowhere, a call with the wrong number of arguments, a value of the wrong type (argument, set, return, condition, operator), an array without its index, a constant set; also between files. JASS, Zinc, Lua, AngelScript. Names are checked only when the game's common.j is found (give the project game=). Lines and columns from 1.",
            "inputSchema": {"type": "object", "properties": {
                "path": {"type": "string", "description": "The project: its wc3.bni, its folder, or any file in it (the nearest wc3.bni above it)"},
                "file": {"type": "string", "description": "Only this file's problems (the whole project is read all the same)"},
                "errors": {"type": "boolean", "description": "Errors only, no warnings (default false)"},
                "limit": {"type": "integer", "minimum": 1, "maximum": 5000, "description": "Problems to give at most (default 300); `total` counts them all"},
            }, "required": ["path"]},
        },
    ])
}

/// Runs the tool `name`, if it's one of these.
pub fn call(name: &str, args: &Value) -> Option<Result<Value, String>> {
    Some(match name {
        "code_check" => check(args),
        _ => return None,
    })
}

fn check(args: &Value) -> Result<Value, String> {
    let at = PathBuf::from(arg_str(args, "path")?);
    let at = std::path::absolute(&at).unwrap_or(at);
    let (_, manifest, _) = crate::source::load_project(&at)
        .ok_or(format!("{}: no {} in it or above it (project_write makes one)", at.display(), wc3_server::project::MANIFEST))?;
    let root = manifest.parent().ok_or("a manifest without a folder")?.to_path_buf();
    let only = args.get("file").and_then(Value::as_str).map(|f| {
        let p = PathBuf::from(f);
        std::path::absolute(&p).unwrap_or(p)
    });
    let errors_only = args.get("errors").and_then(Value::as_bool).unwrap_or(false);
    let limit = args.get("limit").and_then(Value::as_u64).unwrap_or(300) as usize;

    let mut ws = Workspace::default();
    ws.set_roots(vec![root.clone()]);
    let mut files = 0;
    let mut problems = Vec::new();
    for msg in ws.take_reports() {
        let ServerMsg::File { id, diagnostics, .. } = msg else { continue };
        files += 1;
        if only.as_deref().is_some_and(|f| Path::new(&id) != f) {
            continue;
        }
        let shown = relative(&root, &id);
        for d in diagnostics.iter().filter(|d| !errors_only || d.level == parser_rs::Level::Error) {
            problems.push(json!({
                "file": shown,
                "line": d.range.start.line + 1,
                "col": d.range.start.character + 1,
                "end_line": d.range.end.line + 1,
                "end_col": d.range.end.character + 1,
                "level": d.level.as_str(),
                "rule": d.rule,
                "message": d.message,
            }));
        }
    }
    let total = problems.len();
    problems.truncate(limit);
    Ok(json!({"project": root, "files": files, "total": total, "problems": problems}))
}

/// `id` against the project's folder, when it's inside it.
fn relative(root: &Path, id: &str) -> String {
    Path::new(id).strip_prefix(root).map_or(id.to_string(), |p| p.to_string_lossy().into_owned())
}

#[cfg(test)]
#[path = "code.test.rs"]
mod tests;
