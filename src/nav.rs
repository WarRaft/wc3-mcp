//! Finding one's way in a project's code, as the editor does: where a
//! name is declared (`code_definition`), every place it's used
//! (`code_references`), and the declarations of a file or of the whole
//! project (`code_symbols`). The wc3-rs server's symbol index
//! (`wc3_server::index`) over its project model, loaded in process, so a
//! name means what it means in VS Code: scopes, shadowing, struct members
//! and namespaces respected, a map's custom script and the game's scripts
//! the project runs with included.
//!
//! A symbol is given by its name (and, when several declarations have
//! it, the `file` and `line` of the one meant), or by a place in a file
//! (`file`, `line`, `col`): the name there, a use or a declaration.
//! Lines and columns are from 1, columns in characters.

use std::path::{Path, PathBuf};

use parser_rs::{Occurrence, Scope};
use serde_json::{Value, json};
use wc3_server::document::{Document, Language, Position};
use wc3_server::workspace::Workspace;

use crate::code::{relative, workspace};

/// Every tool, for `tools/list`.
pub fn tools() -> Value {
    let target = |extra: Value| {
        let mut props = json!({
            "path": {"type": "string", "description": "The project: its wc3.bni, its folder, or any file in it (the nearest wc3.bni above it)"},
            "name": {"type": "string", "description": "The symbol's name (exact, any kind: function, native, global, local, type, struct, method, library ...); or give `file`, `line` and `col` of a place it's written"},
            "file": {"type": "string", "description": "With `name`: the file of the declaration meant, when several have the name. With `line` and `col`: the file of the place. Relative to the project's folder, or absolute"},
            "line": {"type": "integer", "minimum": 1, "description": "With `name`: the declaration's line in `file`. With `col`: the place's line"},
            "col": {"type": "integer", "minimum": 1, "description": "A column (in characters) on the name at `file`:`line`: the symbol written there, used or declared"},
        });
        if let (Some(p), Some(e)) = (props.as_object_mut(), extra.as_object()) {
            p.extend(e.clone());
        }
        json!({"type": "object", "properties": props, "required": ["path"]})
    };
    json!([
        {
            "name": "code_definition",
            "description": "Where a name of a project's code is declared, as the editor's Go to Definition finds it (scopes, shadowing, struct members, namespaces): each declaration's file, line, column, kind, namespace, its line of code and the comment written above it. Finds the game's natives and Blizzard.j functions too (`game`). Give the name, or the place (`file`, `line`, `col`) where it's written to know what that very name there means (a local shadowing a global, a member of which struct). JASS, vJASS, Zinc, Lua, AngelScript. Lines and columns from 1.",
            "inputSchema": target(json!({})),
        },
        {
            "name": "code_references",
            "description": "Every place a symbol of a project's code is used, its declarations among them, as the editor's Find All References finds them: by meaning, not by text (a local's in its own function only, a struct's member apart from another struct's of the same name, the namespace a name resolves in), in every file of the project, a map's custom script (war3map.wct) and scripts inside maps. A native's or a Blizzard.j function's uses in the project too. Each with its file, line, column, whether it's a declaration, and its line of code; `files` counts them by file. Lines and columns from 1.",
            "inputSchema": target(json!({
                "game": {"type": "boolean", "description": "The uses in the game's scripts (common.j, Blizzard.j ...) too (default false: the project's only)"},
                "offset": {"type": "integer", "minimum": 0, "description": "Places to skip (default 0)"},
                "limit": {"type": "integer", "minimum": 1, "maximum": 5000, "description": "Places to give at most (default 200); `total` counts them all"},
            })),
        },
        {
            "name": "code_symbols",
            "description": "The declarations of a project's code: one file's, as the editor's outline (`file`), or the whole project's whose name has a text in it, any case (`query`; an exact name first), as Go to Symbol in Workspace. Each with its name, kind, namespace (`A::B`, a struct's or library's), file, line, column and its line of code. Not parameters and locals unless `locals`; not the game's scripts unless `game`; not a build's war3map.j unless `generated`. Lines and columns from 1.",
            "inputSchema": {"type": "object", "properties": {
                "path": {"type": "string", "description": "The project: its wc3.bni, its folder, or any file in it (the nearest wc3.bni above it)"},
                "file": {"type": "string", "description": "This file's declarations, in their order (relative to the project's folder, or absolute)"},
                "query": {"type": "string", "description": "A part of the name, any case; every declaration when left out"},
                "kind": {"type": "string", "description": "Only this kind: function, native, global, constant, local, parameter, type, library, struct, interface, module, method, field, namespace, class, enum, enum member"},
                "locals": {"type": "boolean", "description": "Parameters and locals too (default false)"},
                "game": {"type": "boolean", "description": "The game's scripts' too (default false)"},
                "generated": {"type": "boolean", "description": "A build's war3map.j's too (default false: its names are the sources' again)"},
                "offset": {"type": "integer", "minimum": 0, "description": "Declarations to skip (default 0)"},
                "limit": {"type": "integer", "minimum": 1, "maximum": 5000, "description": "Declarations to give at most (default 200); `total` counts them all"},
            }, "required": ["path"]},
        },
    ])
}

/// Runs the tool `name`, if it's one of these.
pub fn call(name: &str, args: &Value) -> Option<Result<Value, String>> {
    Some(match name {
        "code_definition" => definition(args),
        "code_references" => references(args),
        "code_symbols" => symbols(args),
        _ => return None,
    })
}

type Found<'a> = (&'a str, &'a Document, &'a Occurrence);

/// The file `f` names: relative to the project's folder, else to the
/// current folder.
fn file_id(ws: &Workspace, root: &Path, f: &str) -> String {
    let p = PathBuf::from(f);
    if p.is_relative() {
        let under = root.join(&p).to_string_lossy().into_owned();
        if ws.file(&under).is_some() {
            return under;
        }
    }
    std::path::absolute(&p).unwrap_or(p).to_string_lossy().into_owned()
}

/// The editor's position (UTF-16 columns from 0) of a 1-based character
/// column of a 1-based line.
fn position(text: &str, line: usize, col: usize) -> Position {
    let l = text.lines().nth(line.saturating_sub(1)).unwrap_or("");
    let before: String = l.chars().take(col.saturating_sub(1)).collect();
    Position::new(line.saturating_sub(1) as u32, before.encode_utf16().count() as u32)
}

/// The symbol meant: the file to look from and its occurrence there.
fn target<'a>(ws: &'a Workspace, root: &Path, args: &Value) -> Result<(&'a str, &'a Occurrence), String> {
    let file = args.get("file").and_then(Value::as_str).map(|f| file_id(ws, root, f));
    let line = args.get("line").and_then(Value::as_u64).map(|l| l as usize);
    let col = args.get("col").and_then(Value::as_u64).map(|c| c as usize);
    let name = args.get("name").and_then(Value::as_str);

    // A place: the name written there.
    if let Some(col) = col {
        let (Some(file), Some(line)) = (&file, line) else {
            return Err("`col` goes with `file` and `line`".into());
        };
        let (id, doc, _) = ws.all_files().find(|(id, _, _)| id == file).ok_or(format!("{}: not a file of the project's code", relative(root, file)))?;
        let o = wc3_server::index::at(ws, id, position(&doc.text, line, col)).ok_or(format!("{}:{line}:{col}: no name there", relative(root, file)))?;
        if name.is_some_and(|n| n != o.name) {
            return Err(format!("{}:{line}:{col} is `{}`, not `{}`", relative(root, file), o.name, name.unwrap_or_default()));
        }
        return Ok((id, o));
    }

    let name = name.ok_or("give `name`, or `file`, `line` and `col` of a place")?;
    // The project's declarations of the name, in `file` and at `line`.
    let mut decls: Vec<Found> = ws
        .all_files()
        .filter(|(id, _, _)| !ws.is_game(id))
        .filter(|(id, _, _)| file.as_deref().is_none_or(|f| *id == f))
        .flat_map(|(id, doc, occ)| occ.iter().filter(|o| o.declaration && o.name == name && line.is_none_or(|l| o.line == l)).map(move |o| (id, doc, o)))
        .collect();
    decls.sort_by_key(|(id, _, o)| (*id, o.line, o.col));
    // A build's copy (a map's war3map.j) isn't meant when a source has it.
    if file.is_none() && decls.iter().any(|d| !ws.is_generated(d.0)) {
        decls.retain(|d| !ws.is_generated(d.0));
    }
    // Declarations of one symbol (a method's overloads, a struct's
    // members declared twice) are one.
    let mut symbols: Vec<Found> = Vec::new();
    for d in decls {
        let same = symbols.iter().any(|s| wc3_server::index::matches(ws, s.0, s.2).iter().any(|m| m.0 == d.0 && m.2.line == d.2.line && m.2.col == d.2.col));
        if !same {
            symbols.push(d);
        }
    }
    match symbols.as_slice() {
        [(id, _, o)] => return Ok((id, o)),
        [] => {}
        many => {
            let places: Vec<String> = many.iter().map(|(id, _, o)| format!("{}:{} ({}{})", relative(root, id), o.line, o.kind.as_str(), namespaced(o))).collect();
            return Err(format!("`{name}` names {} symbols: give `file` (and `line`) of the declaration meant: {}", many.len(), places.join(", ")));
        }
    }
    if file.is_some() {
        return Err(format!("nothing named `{name}` is declared in {}{}", relative(root, file.as_deref().unwrap_or_default()), line.map_or(String::new(), |l| format!(" at line {l}"))));
    }
    // The game's: looked up from a use in the project, which sees the
    // game's scripts it runs with; else from its own script.
    let used = ws
        .all_files()
        .filter(|(id, _, _)| !ws.is_game(id) && !ws.is_generated(id))
        .flat_map(|(id, _, occ)| occ.iter().filter(|o| !o.declaration && o.name == name && o.scope == Scope::Project).map(move |o| (id, o)))
        .min_by_key(|(id, o)| (*id, o.line, o.col));
    if let Some((id, o)) = used
        && wc3_server::index::matches(ws, id, o).iter().any(|m| m.2.declaration && ws.is_game(m.0))
    {
        return Ok((id, o));
    }
    let game = ws
        .all_files()
        .filter(|(id, _, _)| ws.is_game(id))
        .flat_map(|(id, _, occ)| occ.iter().filter(|o| o.declaration && o.name == name).map(move |o| (id, o)))
        .min_by_key(|(id, o)| (*id, o.line));
    game.ok_or(format!("nothing named `{name}` is declared in the project or the game's scripts it runs with (code_check shows where it's used undeclared; code_symbols finds names by a part)"))
}

/// `, in A::B` for a declaration in a namespace.
fn namespaced(o: &Occurrence) -> String {
    if o.declaration && !o.namespace.is_empty() { format!(", in {}", o.namespace) } else { String::new() }
}

/// One place, as the tools give it.
fn place(ws: &Workspace, root: &Path, (id, doc, o): Found) -> Value {
    let code = doc.text.lines().nth(o.line - 1).unwrap_or("").trim();
    let mut v = json!({"file": relative(root, id), "line": o.line, "col": o.col, "end_col": o.end_col, "kind": o.kind.as_str(), "code": code});
    if o.declaration && !o.namespace.is_empty() {
        v["namespace"] = json!(o.namespace);
    }
    if ws.is_game(id) {
        v["game"] = json!(true);
    }
    if ws.is_generated(id) {
        v["generated"] = json!(true);
    }
    v
}

fn definition(args: &Value) -> Result<Value, String> {
    let (ws, root) = workspace(args)?;
    let (id, o) = target(&ws, &root, args)?;
    ws.note_context(id);
    let decls: Vec<Value> = wc3_server::index::matches(&ws, id, o)
        .into_iter()
        .filter(|m| m.2.declaration)
        .map(|m| {
            let mut v = place(&ws, &root, m);
            // The comment above it: `//` lines (Lua writes `--`).
            if m.1.language != Language::Lua {
                let comment = jass_rs::names::comment_of(&m.1.text, m.2.line);
                if !comment.is_empty() {
                    v["comment"] = json!(comment);
                }
            }
            v
        })
        .collect();
    let mut out = json!({"name": o.name});
    if decls.is_empty() {
        out["note"] = json!("declared nowhere the place sees");
    }
    out["declarations"] = json!(decls);
    Ok(out)
}

fn references(args: &Value) -> Result<Value, String> {
    let game = args.get("game").and_then(Value::as_bool).unwrap_or(false);
    let offset = args.get("offset").and_then(Value::as_u64).unwrap_or(0) as usize;
    let limit = args.get("limit").and_then(Value::as_u64).unwrap_or(200) as usize;
    let (ws, root) = workspace(args)?;
    let (id, o) = target(&ws, &root, args)?;
    ws.note_context(id);
    let found: Vec<Found> = wc3_server::index::matches(&ws, id, o).into_iter().filter(|m| game || !ws.is_game(m.0)).collect();
    let mut files: Vec<(String, usize)> = Vec::new();
    for (fid, _, _) in &found {
        let shown = relative(&root, fid);
        match files.iter_mut().find(|(f, _)| *f == shown) {
            Some((_, n)) => *n += 1,
            None => files.push((shown, 1)),
        }
    }
    let declarations = found.iter().filter(|m| m.2.declaration).count();
    let total = found.len();
    let places: Vec<Value> = found
        .into_iter()
        .skip(offset)
        .take(limit)
        .map(|m| {
            let mut v = place(&ws, &root, m);
            if m.2.declaration {
                v["declaration"] = json!(true);
            }
            v
        })
        .collect();
    let files: Vec<Value> = files.into_iter().map(|(f, n)| json!({"file": f, "count": n})).collect();
    let mut out = json!({"name": o.name, "total": total, "declarations": declarations, "files": files, "places": places});
    // Declared more than once, one symbol all the same (struct members
    // are matched by their name, as the editor matches them: what a
    // value's struct is isn't known): the uses of them all.
    if declarations > 1 {
        out["note"] = json!(format!("`{}` is declared {declarations} times and matched by its name: these are the uses of every one", o.name));
    }
    Ok(out)
}

fn symbols(args: &Value) -> Result<Value, String> {
    let query = args.get("query").and_then(Value::as_str).map(str::to_lowercase);
    let kind = args.get("kind").and_then(Value::as_str);
    let locals = args.get("locals").and_then(Value::as_bool).unwrap_or(false);
    let game = args.get("game").and_then(Value::as_bool).unwrap_or(false);
    let generated = args.get("generated").and_then(Value::as_bool).unwrap_or(false);
    let offset = args.get("offset").and_then(Value::as_u64).unwrap_or(0) as usize;
    let limit = args.get("limit").and_then(Value::as_u64).unwrap_or(200) as usize;
    let (ws, root) = workspace(args)?;
    let file = args.get("file").and_then(Value::as_str).map(|f| file_id(&ws, &root, f));
    if let Some(f) = &file
        && ws.file(f).is_none()
    {
        return Err(format!("{}: not a file of the project's code", relative(&root, f)));
    }
    let mut found: Vec<Found> = ws
        .all_files()
        .filter(|(id, _, _)| match &file {
            Some(f) => id == f,
            None => (game || !ws.is_game(id)) && (generated || !ws.is_generated(id)),
        })
        .flat_map(|(id, doc, occ)| occ.iter().map(move |o| (id, doc, o)))
        .filter(|(_, _, o)| o.declaration && (locals || !o.kind.is_local()))
        .filter(|(_, _, o)| kind.is_none_or(|k| o.kind.as_str() == k))
        .filter(|(_, _, o)| query.as_deref().is_none_or(|q| o.name.to_lowercase().contains(q)))
        .collect();
    if file.is_some() {
        found.sort_by_key(|(_, _, o)| (o.line, o.col));
    } else {
        // The exact name first, then by name, file and place.
        let exact = |o: &Occurrence| query.as_deref().is_none_or(|q| o.name.to_lowercase() != q);
        found.sort_by(|a, b| (exact(a.2), &a.2.name, ws.is_game(a.0), a.0, a.2.line).cmp(&(exact(b.2), &b.2.name, ws.is_game(b.0), b.0, b.2.line)));
    }
    // A name declared again in one place (overloads) is one.
    found.dedup_by(|a, b| a.0 == b.0 && a.2.line == b.2.line && a.2.col == b.2.col);
    let total = found.len();
    let items: Vec<Value> = found.into_iter().skip(offset).take(limit).map(|m| {
        let mut v = place(&ws, &root, m);
        v["name"] = json!(m.2.name);
        v
    }).collect();
    Ok(json!({"total": total, "items": items}))
}

#[cfg(test)]
#[path = "nav.test.rs"]
mod tests;
