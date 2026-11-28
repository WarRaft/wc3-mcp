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
                "strict": {"type": "array", "items": {"type": "string"}, "description": "Instead of the project's code: these JASS files checked as the game reads a map's script, one file glued in the order given (a builder of one's own; the game has no imports): strict JASS, a function called above its declaration and the rest of the group `strict` errors, names over the game's common.j and Blizzard.j (common.ai when every file is .ai) from the project's game= and roots. Paths relative to the project's folder. Each problem at its own file and line"},
            }, "required": ["path"]},
        },
        {
            "name": "jass_rename",
            "description": "Renames a name of a project's code everywhere it means that symbol, as the editor's rename does: a function, global, local, parameter, type, struct, method, field, library, scope ... in every file of the project (JASS, vJASS, Zinc, Lua, AngelScript; the custom script inside a map too), shadowing and scopes respected. Refused, with why and nothing written, when it can't be done safely: a native or something of the game's scripts, a name declared nowhere in the project, a member declared in several structs, a new name that isn't a name or is already declared there, a map that can't be written. Give the current name; when several declarations have it (two functions' locals), `file` and `line` pick one. `dry_run` shows the edits without writing. Lines and columns from 1.",
            "inputSchema": {"type": "object", "properties": {
                "path": {"type": "string", "description": "The project: its wc3.bni, its folder, or any file in it (the nearest wc3.bni above it)"},
                "name": {"type": "string", "description": "The current name"},
                "new_name": {"type": "string", "description": "The name it gets"},
                "file": {"type": "string", "description": "The file of the declaration meant, when several have the name"},
                "line": {"type": "integer", "minimum": 1, "description": "Its line there, when the file has several"},
                "dry_run": {"type": "boolean", "description": "Only show what would change (default false)"},
            }, "required": ["path", "name", "new_name"]},
        },
        {
            "name": "jass_build",
            "description": "Builds JASS files into one script the game reads (wc3-build), JassHelper's preprocessor run over them all first (textmacro/runtextmacro, novjass, import beside the file): this toolchain's JASS (set, call, local, takes nothing returns nothing left out; locals anywhere; globals at the top level; any number of main and config) made the game's: main and config merged, globals into globals and sorted, functions sorted so none calls one below it (a cycle goes through TriggerEvaluate, as JassHelper's .evaluate), locals to the top. Give the files (in their order) or a folder (its .j files, sorted by name); the script is written to `out`, or given back. Errors with their file and line.",
            "inputSchema": {"type": "object", "properties": {
                "files": {"type": "array", "items": {"type": "string"}, "description": "JASS files, in the order their main and config parts run"},
                "dir": {"type": "string", "description": "A folder: its .j files, sorted by name, in place of `files`"},
                "out": {"type": "string", "description": "Where to write the script; given back when left out"},
                "uglify": {"type": "boolean", "description": "Shorter names: everything the game doesn't need as it is (the map's functions but main and config, globals, types, locals, parameters) renamed a, b ... a1, the most used the shortest; names in ExecuteFunc(\"F\") and TriggerRegisterVariableEvent literals with them (default false)"},
                "inline": {"type": "boolean", "description": "As JassHelper inlines: a one-line function's calls are what its line does, where nothing can change (each parameter read once, in order, calls after them) (default false)"},
                "dead_code": {"type": "boolean", "description": "The functions and globals nothing reachable from main and config uses left out (default false)"},
                "string_hash": {"type": "boolean", "description": "StringHash(\"text\") of an ASCII literal made the number the game gives (its SStrHash2), so the game hashes nothing as it runs (default false)"},
            }},
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
        "jass_build" => build(args),
        "jass_rename" => rename(args),
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
    if let Some(list) = args.get("strict") {
        return check_strict(list, &at, &root, errors_only, limit);
    }

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

/// `code_check`'s `strict`: the files glued in their order, one script as
/// the game reads it (`jass_rs::strict::check`), over the game's scripts
/// of the project's lookup.
fn check_strict(list: &Value, at: &Path, root: &Path, errors_only: bool, limit: usize) -> Result<Value, String> {
    let paths: Vec<PathBuf> = list
        .as_array()
        .ok_or("'strict' is a list of files")?
        .iter()
        .map(|v| v.as_str().map(|f| root.join(f)).ok_or_else(|| "'strict' is a list of files".to_string()))
        .collect::<Result<_, _>>()?;
    if paths.is_empty() {
        return Err("'strict' names no file".into());
    }
    let texts: Vec<String> = paths
        .iter()
        .map(|p| std::fs::read(p).map(wc3_server::document::decode).map_err(|e| format!("{}: {e}", p.display())))
        .collect::<Result<_, _>>()?;
    let (project, _, _) = crate::source::load_project(at).ok_or("no project")?;
    let (game, _) = wc3_server::game::Lookup::of(&project).open();
    let read = |n: &str| game.read(n).map(|(b, _)| b);
    let ai = paths.iter().all(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("ai")));
    let env = wc3_server::build::game_declarations(&read, ai);
    let names_checked = !env.is_empty();
    let env: Vec<&jass_rs::names::Declarations> = env.iter().collect();
    let shown: Vec<String> = paths.iter().map(|p| relative(root, &p.to_string_lossy())).collect();
    let sources: Vec<(&str, &str)> = shown.iter().map(String::as_str).zip(texts.iter().map(String::as_str)).collect();
    let mut problems: Vec<Value> = jass_rs::strict::check(&sources, &env)
        .into_iter()
        .filter(|f| !errors_only || f.diagnostic.level == parser_rs::Level::Error)
        .map(|f| {
            let d = f.diagnostic;
            let (col, end_line, end_col) = d.span.map_or((1, d.line, 1), |s| (s.start_col, s.end_line, s.end_col));
            json!({"file": shown[f.file], "line": d.line, "col": col, "end_line": end_line, "end_col": end_col, "level": d.level.as_str(), "rule": d.rule, "message": d.message})
        })
        .collect();
    let total = problems.len();
    problems.truncate(limit);
    Ok(json!({"project": root, "files": paths.len(), "strict": true, "names_checked": names_checked, "total": total, "problems": problems}))
}

/// The project's workspace, loaded as the editor loads it, and its root.
fn workspace(args: &Value) -> Result<(Workspace, PathBuf), String> {
    let at = PathBuf::from(arg_str(args, "path")?);
    let at = std::path::absolute(&at).unwrap_or(at);
    let (_, manifest, _) = crate::source::load_project(&at)
        .ok_or(format!("{}: no {} in it or above it (project_write makes one)", at.display(), wc3_server::project::MANIFEST))?;
    let root = manifest.parent().ok_or("a manifest without a folder")?.to_path_buf();
    let mut ws = Workspace::default();
    ws.set_roots(vec![root.clone()]);
    ws.take_reports();
    Ok((ws, root))
}

/// The editor's position (UTF-16 columns from 0) of a name's start.
fn position_of(text: &str, o: &parser_rs::Occurrence) -> wc3_server::document::Position {
    let line = text.lines().nth(o.line - 1).unwrap_or("");
    let before: String = line.chars().take(o.col - 1).collect();
    wc3_server::document::Position::new(o.line as u32 - 1, before.encode_utf16().count() as u32)
}

fn rename(args: &Value) -> Result<Value, String> {
    let name = arg_str(args, "name")?;
    let new_name = arg_str(args, "new_name")?;
    let file = args.get("file").and_then(Value::as_str).map(|f| {
        let p = PathBuf::from(f);
        std::path::absolute(&p).unwrap_or(p)
    });
    let line = args.get("line").and_then(Value::as_u64).map(|l| l as usize);
    let dry_run = args.get("dry_run").and_then(Value::as_bool).unwrap_or(false);
    let (ws, root) = workspace(args)?;

    // The declaration meant: the only one of the name, or the one `file`
    // and `line` pick.
    let mut decls: Vec<(&str, &wc3_server::document::Document, &parser_rs::Occurrence)> = ws
        .all_files()
        .filter(|(id, _, _)| !ws.is_game(id))
        .filter(|(id, _, _)| file.as_deref().is_none_or(|f| Path::new(id) == f))
        .flat_map(|(id, doc, occ)| occ.iter().filter(|o| o.declaration && o.name == name && line.is_none_or(|l| o.line == l)).map(move |o| (id, doc, o)))
        .collect();
    // A name declared again in the same file and place (a method's
    // overloads) is one.
    decls.dedup_by(|a, b| a.0 == b.0 && a.2.line == b.2.line && a.2.col == b.2.col);
    // A copy in a map's script (`war3map.j`, an earlier build's) isn't
    // the one meant when the name's source is found too: the source is
    // (the copy, apart, stays till the next build). `file` picks the copy.
    if file.is_none() && decls.iter().any(|d| !ws.is_generated(d.0)) {
        decls.retain(|d| !ws.is_generated(d.0));
    }
    let (id, doc, target) = match decls.as_slice() {
        [] => {
            // The game's own: said so.
            let game = ws.all_files().find(|(id, _, occ)| ws.is_game(id) && occ.iter().any(|o| o.declaration && o.name == name));
            if let Some((id, _, _)) = game {
                let script = Path::new(id).file_name().map_or(id.into(), |n| n.to_string_lossy());
                return Err(format!("`{name}` is the game's ({script}): it can't be renamed"));
            }
            let where_ = file.as_ref().map_or(String::new(), |f| format!(" in {}", relative(&root, &f.to_string_lossy())));
            return Err(format!("nothing named `{name}` is declared{where_} in the project: only the project's own names can be renamed"));
        }
        [one] => *one,
        many => {
            let places: Vec<String> = many.iter().map(|(id, _, o)| format!("{}:{} ({:?})", relative(&root, id), o.line, o.kind)).collect();
            return Err(format!("`{name}` is declared {} times: give `file` (and `line`) of the one meant: {}", many.len(), places.join(", ")));
        }
    };
    let answer = wc3_server::rename::rename(&ws, id, position_of(&doc.text, target), new_name);
    if !answer.text.is_empty() {
        return Err(format!("nothing renamed: {}", answer.text));
    }
    if answer.locations.is_empty() {
        return Ok(json!({"renamed": 0, "files": [], "note": "the name is already that"}));
    }

    // Each file's new text, then its bytes: all made before any is written.
    let mut by_file: Vec<(String, Vec<wc3_server::document::Range>)> = Vec::new();
    for l in &answer.locations {
        match by_file.iter_mut().find(|(f, _)| *f == l.id) {
            Some((_, ranges)) => ranges.push(l.range),
            None => by_file.push((l.id.clone(), vec![l.range])),
        }
    }
    let mut writes: Vec<(PathBuf, Vec<u8>)> = Vec::new();
    let mut report = Vec::new();
    for (id, mut ranges) in by_file {
        let (doc, _) = ws.file(&id).ok_or(format!("{id}: not in the project any more"))?;
        let path = doc.path.clone().ok_or(format!("{id}: a file without a path"))?;
        let mut new = doc.clone();
        ranges.sort_by_key(|r| std::cmp::Reverse((r.start.line, r.start.character)));
        for r in &ranges {
            new.apply(*r, new_name);
        }
        let shown = relative(&root, &path.to_string_lossy());
        let at: Vec<Value> = ranges.iter().rev().map(|r| json!([r.start.line + 1, r.start.character + 1])).collect();
        report.push(json!({"file": shown, "count": ranges.len(), "at": at}));
        let in_map = wc3_server::map::split(&path).is_some();
        let original = if in_map { wc3_server::map::bytes(&path) } else { std::fs::read(&path).ok() };
        let bytes = if wc3_server::customtext::is_custom_text_file(&path) {
            // The custom script is its text form here: back to its bytes.
            wc3_server::customtext::convert(wc3_server::objects::TO_BYTES, &path.to_string_lossy(), new.text.as_bytes()).map_err(|e| format!("nothing renamed: {shown}: {e}"))?
        } else {
            // Text that isn't UTF-8 would come back otherwise.
            if original.as_deref().is_some_and(|b| std::str::from_utf8(b).is_err()) {
                return Err(format!("nothing renamed: {shown} isn't UTF-8 text: writing it would change more than the name"));
            }
            new.text.into_bytes()
        };
        writes.push((path, bytes));
    }
    let total: usize = report.iter().filter_map(|r| r["count"].as_u64()).sum::<u64>() as usize;
    if !dry_run {
        let mut maps = wc3_server::mapfs::MapFs::default();
        for (path, bytes) in writes {
            if wc3_server::map::split(&path).is_some() {
                maps.write(&path, bytes, false, true).map_err(|e| format!("{}: {e:?} (the files before it are renamed)", path.display()))?;
            } else {
                std::fs::write(&path, bytes).map_err(|e| format!("{}: {e} (the files before it are renamed)", path.display()))?;
            }
        }
    }
    Ok(json!({"renamed": total, "written": !dry_run, "from": name, "to": new_name, "files": report}))
}

fn build(args: &Value) -> Result<Value, String> {
    let mut paths: Vec<PathBuf> = match (args.get("files").and_then(Value::as_array), args.get("dir").and_then(Value::as_str)) {
        (Some(files), None) => files.iter().filter_map(Value::as_str).map(PathBuf::from).collect(),
        (None, Some(dir)) => {
            let mut p: Vec<PathBuf> = std::fs::read_dir(dir)
                .map_err(|e| format!("{dir}: {e}"))?
                .flatten()
                .map(|e| e.path())
                .filter(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("j")))
                .collect();
            p.sort();
            p
        }
        _ => return Err("give `files` or `dir`".into()),
    };
    paths.retain(|p| p.is_file());
    if paths.is_empty() {
        return Err("no JASS files".into());
    }
    let texts: Vec<(String, String)> = paths
        .iter()
        .map(|p| Ok((p.to_string_lossy().into_owned(), String::from_utf8_lossy(&std::fs::read(p).map_err(|e| format!("{}: {e}", p.display()))?).into_owned())))
        .collect::<Result<_, String>>()?;
    let sources: Vec<(wc3_build::from_jass::Source, jass_rs::Dialect)> = texts.iter().map(|(name, text)| (wc3_build::from_jass::Source { name, text }, jass_rs::Dialect::Jass)).collect();
    let flag = |k: &str| args.get(k).and_then(Value::as_bool).unwrap_or(false);
    let options = wc3_build::jass::Options { uglify: flag("uglify"), inline: flag("inline"), dead_code: flag("dead_code"), string_hash: flag("string_hash"), ..Default::default() };
    // JassHelper's `//! import "path"`: beside the file importing it.
    let import = |from: &str, path: &str| -> Option<(String, String)> {
        let p = std::path::Path::new(from).parent()?.join(path.replace('\\', "/"));
        Some((p.to_string_lossy().into_owned(), String::from_utf8_lossy(&std::fs::read(&p).ok()?).into_owned()))
    };
    let built = wc3_build::from_jass::module_with(&sources, &import).and_then(|m| wc3_build::jass::build_with(&m, &options));
    match built {
        Err(errors) => {
            let errors: Vec<Value> = errors.iter().map(|e| json!({"file": e.origin.file, "line": e.origin.line, "message": e.message})).collect();
            Ok(json!({"built": false, "errors": errors}))
        }
        Ok(wc3_build::jass::Built { script, notes, .. }) => match args.get("out").and_then(Value::as_str) {
            Some(out) => {
                std::fs::write(out, &script).map_err(|e| format!("{out}: {e}"))?;
                Ok(json!({"built": true, "files": paths.len(), "lines": script.lines().count(), "bytes": script.len(), "notes": notes, "out": out}))
            }
            None => Ok(json!({"built": true, "files": paths.len(), "lines": script.lines().count(), "bytes": script.len(), "notes": notes, "script": script})),
        },
    }
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
