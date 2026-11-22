//! The project's code checked as the editor checks it: the server's
//! project model (`wc3_server::workspace`) loaded for the project, in
//! process, so what comes back is what VS Code shows. Each file's parse
//! errors and lint, and its names checked against every file of its
//! project and the game's scripts it runs with (`unknown-name`,
//! `argument-count`): an error between files shows up here. And what
//! the game's scripts declare (`jass_natives`), with the comments written
//! above it.

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
        {
            "name": "jass_natives",
            "description": "What the game's scripts declare, as a project sees them (common.j, Blizzard.j, common.ai through its game= folders and map roots, the last found winning: UjAPI's common.j over the game's): natives, functions and globals, each with its declaration, the script and line it's in, and the comment written above it. Search by name (any part, any case); an exact name first. Use it to find the native for a job, or what one takes.",
            "inputSchema": {"type": "object", "properties": {
                "path": {"type": "string", "description": "Any file or folder of the project (the nearest wc3.bni above it)"},
                "game": {"type": "array", "items": {"type": "string"}, "description": "The game's folders, lowest first, in place of the project's"},
                "roots": {"type": "array", "items": {"type": "string"}, "description": "The map's roots, lowest first, in place of the project's"},
                "name": {"type": "string", "description": "A part of the name to look for, any case; all when left out"},
                "comment": {"type": "string", "description": "A part of the comment to look for, any case"},
                "kind": {"type": "string", "enum": ["native", "function", "global", "all"], "description": "What to list (default all)"},
                "script": {"type": "string", "enum": ["common.j", "Blizzard.j", "common.ai"], "description": "Only this script's"},
                "offset": {"type": "integer", "minimum": 0, "description": "Items to skip (default 0)"},
                "limit": {"type": "integer", "minimum": 1, "maximum": 2000, "description": "Items to give at most (default 100); `total` counts them all"},
            }},
        },
    ])
}

/// Runs the tool `name`, if it's one of these.
pub fn call(name: &str, args: &Value) -> Option<Result<Value, String>> {
    Some(match name {
        "code_check" => check(args),
        "jass_natives" => natives(args),
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

/// A declaration of the game's scripts.
struct Item {
    name: String,
    kind: &'static str,
    script: &'static str,
    line: usize,
    declaration: String,
    comment: String,
}

/// A signature as JASS writes it: `takes unit u, real x returns nothing`.
fn signature(sig: &jass_rs::ast::FunctionSig) -> String {
    let takes = if sig.params.is_empty() {
        "nothing".to_string()
    } else {
        sig.params.iter().map(|p| format!("{} {}", p.type_name, p.name)).collect::<Vec<_>>().join(", ")
    };
    format!("{} takes {takes} returns {}", sig.name, sig.returns.as_deref().unwrap_or("nothing"))
}

fn natives(args: &Value) -> Result<Value, String> {
    use jass_rs::ast::TopLevel;
    let lookup = crate::source::lookup(args);
    if lookup.is_empty() {
        return Err("no game folders: give `path` in a project with game=, or `game`".into());
    }
    let kind = args.get("kind").and_then(Value::as_str).unwrap_or("all");
    let only_script = args.get("script").and_then(Value::as_str);
    let name = args.get("name").and_then(Value::as_str).map(str::to_lowercase);
    let about = args.get("comment").and_then(Value::as_str).map(str::to_lowercase);
    let offset = args.get("offset").and_then(Value::as_u64).unwrap_or(0) as usize;
    let limit = args.get("limit").and_then(Value::as_u64).unwrap_or(100) as usize;

    let mut items = Vec::new();
    let mut found = Vec::new();
    for (path, doc) in wc3_server::game::scripts(&lookup) {
        let script = match wc3_server::game::Script::of(&path) {
            Some(wc3_server::game::Script::Common) => "common.j",
            Some(wc3_server::game::Script::Blizzard) => "Blizzard.j",
            Some(wc3_server::game::Script::Ai) => "common.ai",
            None => continue,
        };
        found.push(json!({"script": script, "path": path}));
        if only_script.is_some_and(|s| !s.eq_ignore_ascii_case(script)) {
            continue;
        }
        let program = jass_rs::parse(&doc.text).map_err(|e| format!("{}: line {}: {}", path.display(), e.line, e.message))?;
        let mut add = |name: &str, kind: &'static str, line: usize, declaration: String| {
            items.push(Item {
                name: name.to_string(),
                kind,
                script,
                line,
                declaration,
                comment: jass_rs::names::comment_of(&doc.text, line),
            });
        };
        for item in &program.items {
            match item {
                TopLevel::Native(sig) => {
                    let constant = if sig.is_constant { "constant " } else { "" };
                    add(&sig.name, "native", sig.line, format!("{constant}native {}", signature(sig)));
                }
                TopLevel::Function(f) => {
                    let constant = if f.sig.is_constant { "constant " } else { "" };
                    add(&f.sig.name, "function", f.sig.line, format!("{constant}function {}", signature(&f.sig)));
                }
                TopLevel::Globals(decls) => {
                    for g in decls {
                        let constant = if g.is_constant { "constant " } else { "" };
                        let array = if g.is_array { " array" } else { "" };
                        add(&g.name, "global", g.line, format!("{constant}{}{array} {}", g.type_name, g.name));
                    }
                }
                _ => {}
            }
        }
    }
    let mut items: Vec<Item> = items
        .into_iter()
        .filter(|i| kind == "all" || i.kind == kind)
        .filter(|i| name.as_deref().is_none_or(|n| i.name.to_lowercase().contains(n)))
        .filter(|i| about.as_deref().is_none_or(|c| i.comment.to_lowercase().contains(c)))
        .collect();
    // The exact name first.
    if let Some(n) = &name {
        items.sort_by_key(|i| i.name.to_lowercase() != *n);
    }
    let total = items.len();
    let shown: Vec<Value> = items
        .iter()
        .skip(offset)
        .take(limit)
        .map(|i| {
            let mut v = json!({"name": i.name, "kind": i.kind, "declaration": i.declaration, "script": i.script, "line": i.line});
            if !i.comment.is_empty() {
                v["comment"] = json!(i.comment);
            }
            v
        })
        .collect();
    Ok(json!({"scripts": found, "total": total, "items": shown}))
}

/// `id` against the project's folder, when it's inside it.
fn relative(root: &Path, id: &str) -> String {
    Path::new(id).strip_prefix(root).map_or(id.to_string(), |p| p.to_string_lossy().into_owned())
}

#[cfg(test)]
#[path = "code.test.rs"]
mod tests;
