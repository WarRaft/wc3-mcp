//! The tools of trigger files of their own (`.trg`): a map's GUI triggers
//! taken out of its `war3map.wtg` into a folder, one trigger per file, a
//! folder per category (`000_Init/001_Spawn.trg`, the order in the
//! names), the map's global variables in `_variables.trg`. The files are
//! text, the source: they're read, checked, built into JASS and changed
//! here as the editor (wc3-rs) does, with its server's code
//! (`wc3_server::triggers`).
//!
//! What the tools show is for reading as well as editing: a trigger's
//! lines as World Editor's phrases (`UI\TriggerData.txt`,
//! `UI\TriggerStrings.txt` through `game` and `roots`, in the game's
//! language), each with its line in the file and its call; the editor's
//! functions, types and presets to look up what to write. A project is a
//! folder of trigger files (`dir`): its globals, what the map's script
//! names its triggers, and the references between them are the folder's.

use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use wc3_server::triggers::{Project, Triggers, op};
use wtg_rs::FunctionKind;
use wtg_rs::text::{Declared, Scope};

use crate::source::{arg_str, archive_schema, game_schema, path_schema, roots_schema};

/// The syntax of a trigger file, for the tools' descriptions.
const SYNTAX: &str = "A trigger file (.trg) is BNI text: at most one [trigger] section (name=\"...\" (none: the file's name), description=\"...\", flags when not the default: enabled=0, initially_off=1, run_on_init=1), then its lines, each `kind=Call(params)`: event=, condition=, action=, `!` before the kind when disabled (!action=), and inside another (if/then/else, loops: their blocks) `>` per level and `:block` after the kind (an if/then/else's 0 conditions, 1 then, 2 else; a loop's 0): `>action:1=DoNothing()`. Parameters: a bare word a preset (Player00), $name a variable ($hero[\"0\"] an array's, $gg_trg_<name> a trigger), \"text\" a value (numbers too: \"30.00\", object ids: \"hfoo\"), Name(...) a call. Custom script (CustomScriptCode) is a JASS string; \\n in it are its lines. Variables: [variable] sections, name=, type=, array=1, size=, initial=, scope=private (a global this file alone sees) or scope=local (a local of its trigger's actions); none: a global every trigger sees. Look functions up with trg_functions, types and presets with trg_types.";

/// Every tool, for `tools/list`.
pub fn tools() -> Value {
    let dir = json!({"type": "string", "description": "The folder of the project's trigger files (as trg_extract made it): its globals, and what refers to what, are the folder's. Left out: the nearest folder above the file holding _variables.trg, else the file's own"});
    let file = json!({"type": "string", "description": "A trigger file (.trg) on disk"});
    let with = |extra: Value| {
        let mut props = json!({"game": game_schema(), "roots": roots_schema(), "project": crate::source::project_schema()});
        props.as_object_mut().expect("an object").extend(extra.as_object().expect("an object").clone());
        props
    };
    json!([
        {
            "name": "trg_extract",
            "description": "Takes a map's GUI triggers out of its war3map.wtg into a folder as trigger files: a folder per category (000_Init), a .trg per trigger (001_Spawn.trg, the order the game builds them in the names), the global variables in _variables.trg. Files already there are left alone unless `replace`. Answers the folders and how many files (their names too, up to 100). A trigger's name in the map's script (gg_trg_...) stays World Editor's, so the map's references stay true. Needs UI\\TriggerData.txt: `game`, or the project's. Then trg_list, trg_read.",
            "inputSchema": {"type": "object", "properties": with(json!({
                "path": path_schema("war3map.wtg", "The trigger file"), "archive": archive_schema(),
                "dir": {"type": "string", "description": "The folder to put them in; made if missing"},
                "replace": {"type": "boolean", "description": "Replace files already there (default: refuse, writing nothing)"},
            })), "required": ["dir"]},
        },
        {
            "name": "trg_list",
            "description": "A project's trigger files (a folder): each trigger with its file, category (its folder), name, its name in the map's script (gg_trg_...), flags, how many events, conditions and actions, and how many problems; the variables each file declares (name, type, array, scope). `name` filters by part of a trigger's name.",
            "inputSchema": {"type": "object", "properties": with(json!({"dir": {"type": "string", "description": "The folder of trigger files"}, "name": {"type": "string"}})), "required": ["dir"]},
        },
        {
            "name": "trg_read",
            "description": "A trigger as World Editor shows it, to read: its fields, its variables, then Events, Conditions and Actions, each line its phrase (the editor's words, in the game's language; the line's parameters in [], a call in ()) with its line in the file and, below, its call as the file has it; blocks (If - Conditions, Then - Actions, Loop - Actions) indented; disabled lines marked; the problems last. Change the file's text by the lines (or trg_write). `game` for the phrases: without it, calls.",
            "inputSchema": {"type": "object", "properties": with(json!({"file": file, "dir": dir, "calls": {"type": "boolean", "description": "Each line's call below its phrase (default true)"}})), "required": ["file"]},
        },
        {
            "name": "trg_check",
            "description": "The problems of a project's trigger files (or of one): text that doesn't read, a function UI\\TriggerData.txt doesn't have or with the wrong number of parameters (with the game's files: `game`, or the project's), a variable no file declares or not seen where it's used (a local outside its trigger's actions, another file's private one), a $gg_trg_ reference no trigger has or that may mean two, two triggers the map's script would name alike.",
            "inputSchema": {"type": "object", "properties": with(json!({"dir": {"type": "string", "description": "The folder of trigger files: all of them"}, "file": file}))},
        },
        {
            "name": "trg_jass",
            "description": "The JASS a trigger file is built into: World Editor's layout (Trig_<name>_Conditions, Trig_<name>_Actions, InitTrig_<name>), its globals (udg_<name>; a private one udg_<trigger>__<name>), its locals at the top of the actions, loops (ForGroup, ForForce, Enum...) inside the actions.",
            "inputSchema": {"type": "object", "properties": with(json!({"file": file, "dir": dir})), "required": ["file"]},
        },
        {
            "name": "trg_functions",
            "description": "World Editor's functions (UI\\TriggerData.txt), to know what to write: with `query` (words, all in its name, title, category or hint) and `kind` (event, condition, action, call: a call is a parameter's value), the matches; with `name`, one in full: its phrase with {n} for its parameters, each parameter's type (trg_types for its presets), its default, what it returns (a call), its hint, and a line with its defaults, ready to put in a file.",
            "inputSchema": {"type": "object", "properties": with(json!({
                "query": {"type": "string"}, "name": {"type": "string"},
                "kind": {"type": "string", "enum": ["event", "condition", "action", "call"]},
                "limit": {"type": "integer", "minimum": 1, "description": "Matches at most (default 50)"},
            }))},
        },
        {
            "name": "trg_types",
            "description": "World Editor's types (UI\\TriggerData.txt): without `type`, each type's name, base, whether a value is typed in (a number, a string, an object id) and whether a variable can be of it; with `type`, its presets (what a bare word parameter may be: Player00 is Player 1 (Red)), filtered by `query`.",
            "inputSchema": {"type": "object", "properties": with(json!({
                "type": {"type": "string"}, "query": {"type": "string"},
                "limit": {"type": "integer", "minimum": 1, "description": "Presets at most (default 200)"},
            }))},
        },
        {
            "name": "trg_write",
            "description": format!("Writes a trigger file (made, with its folders, if missing) from its whole text, after reading it (and every function against UI\\TriggerData.txt with the game's files): every error, with its line, and nothing written. Then the problems it has in its project (an undeclared variable ...). {SYNTAX}"),
            "inputSchema": {"type": "object", "properties": with(json!({"file": file, "dir": dir, "text": {"type": "string"}})), "required": ["file", "text"]},
        },
        {
            "name": "trg_rename",
            "description": "Renames a trigger (its name=; the file keeps its name) and has the references to it follow: $gg_trg_... in every trigger file of the project, names in custom script (JASS tokens, not in comments or strings) and in the project's .j files. Refused, changing nothing, when the map's script would name another trigger as it then (gg_trg_ and the name, _ for each byte a JASS name can't have).",
            "inputSchema": {"type": "object", "properties": with(json!({"file": file, "dir": dir, "name": {"type": "string", "description": "The new name; empty for none (the file's)"}})), "required": ["file", "name"]},
        },
        {
            "name": "trg_variable",
            "description": "Changes or deletes a variable a trigger file declares, safe where it can be: renamed, its uses ($name, in every trigger file that sees it) are renamed with it. What would break uses (deleting a used one, a narrower scope, another type, array or not, custom script or .j code naming it in the script) is listed and nothing done unless `confirm`. A clashing name or scope is refused.",
            "inputSchema": {"type": "object", "properties": with(json!({
                "file": file, "dir": dir,
                "variable": {"type": "string", "description": "Its name now"},
                "delete": {"type": "boolean"},
                "name": {"type": "string"}, "type": {"type": "string", "description": "A type (trg_types): integer, unit, player ..."},
                "array": {"type": "boolean"}, "size": {"type": "integer", "minimum": 1},
                "scope": {"type": "string", "enum": ["global", "private", "local"]},
                "initial": {"type": "string", "description": "Its initial value; empty for none"},
                "confirm": {"type": "boolean", "description": "Do it though it breaks the places listed"},
            })), "required": ["file", "variable"]},
        },
    ])
}

/// Runs the tool `name`, if it's one of these.
pub fn call(name: &str, args: &Value) -> Option<Result<Value, String>> {
    Some(match name {
        "trg_extract" => extract(args),
        "trg_list" => list(args),
        "trg_read" => read(args).map(Value::String),
        "trg_check" => check(args),
        "trg_jass" => jass(args).map(Value::String),
        "trg_functions" => functions(args),
        "trg_types" => types(args),
        "trg_write" => write(args),
        "trg_rename" => rename(args),
        "trg_variable" => variable(args),
        _ => return None,
    })
}

// ---------------------------------------------------------------- the project

fn is_trg(p: &Path) -> bool {
    p.extension().is_some_and(|e| e.eq_ignore_ascii_case("trg"))
}

/// Code the map's script has besides the triggers: JASS, whose names may
/// be a trigger's or a variable's.
fn is_jass(p: &Path) -> bool {
    p.extension().is_some_and(|e| e.eq_ignore_ascii_case("j") || e.eq_ignore_ascii_case("ai"))
}

/// The project's files under `dir`: trigger files and JASS, with their
/// text, by path.
fn project_files(dir: &Path) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&d) else { continue };
        for e in entries.flatten() {
            let p = e.path();
            if p.file_name().is_some_and(|n| n.to_string_lossy().starts_with('.')) {
                continue;
            }
            if p.is_dir() {
                stack.push(p);
            } else if (is_trg(&p) || is_jass(&p))
                && let Ok(t) = std::fs::read(&p)
            {
                out.push((p.to_string_lossy().into_owned(), String::from_utf8_lossy(&t).into_owned()));
            }
        }
    }
    out.sort();
    out
}

/// The folder of the project of `file`: `args.dir`, else the nearest above
/// holding `_variables.trg`, else the file's own.
fn project_dir(args: &Value, file: &Path) -> PathBuf {
    if let Some(d) = args.get("dir").and_then(Value::as_str) {
        return PathBuf::from(d);
    }
    let own = file.parent().map_or_else(|| PathBuf::from("."), Path::to_path_buf);
    own.ancestors().take(4).find(|d| d.join(wtg_rs::text::VARIABLES_FILE).is_file()).map_or(own.clone(), Path::to_path_buf)
}

/// A project's files, read and parsed once: what each file's view, checks
/// and edits need of the others.
struct Loaded {
    /// Its files: path, text.
    files: Vec<(String, String)>,
    /// The names the JASS of each file has (an empty list for the rest).
    names: Vec<Vec<parser_rs::Occurrence>>,
    /// The global variables each trigger file declares: its path, the
    /// variable.
    globals: Vec<(String, Declared)>,
    /// The project's triggers, looked at from no file.
    triggers: Triggers,
}

impl Loaded {
    fn load(dir: &Path) -> Loaded {
        let files = project_files(dir);
        let names = files
            .iter()
            .map(|(p, t)| if is_jass(Path::new(p)) { jass_rs::semantic::symbols(t, jass_rs::lexer::Dialect::detect(t)) } else { Vec::new() })
            .collect();
        let globals = files
            .iter()
            .filter(|(p, _)| is_trg(Path::new(p)))
            .flat_map(|(p, t)| wtg_rs::text::trigger_from_text(t, None).variables.into_iter().filter(|d| d.scope == Scope::Global).map(move |d| (p.clone(), d)))
            .collect();
        let pairs: Vec<(&str, &str)> = files.iter().map(|(p, t)| (p.as_str(), t.as_str())).collect();
        let triggers = Triggers::of("", "", &pairs);
        Loaded { files, names, globals, triggers }
    }

    /// The text of the file `me`.
    fn text(&self, me: &str) -> Option<&str> {
        self.files.iter().find(|(p, _)| p == me).map(|(_, t)| t.as_str())
    }

    /// The others of the file `me`: path, text.
    fn others(&self, me: &str) -> Vec<(String, String)> {
        self.files.iter().filter(|(p, _)| p != me).cloned().collect()
    }

    /// The project as `triggers::edit_in` takes it, from the file `me`.
    fn project<'a>(&'a self, me: &'a str) -> Project<'a> {
        Project {
            me,
            others: self.files.iter().zip(&self.names).filter(|((p, _), _)| p != me).map(|((p, t), n)| (p.as_str(), t.as_str(), n.as_slice())).collect(),
        }
    }

    /// The variables the file `me` (its text `text`) sees: the project's
    /// globals and its own.
    fn visible(&self, me: &str, text: &str) -> Vec<Declared> {
        let mut out: Vec<Declared> = self.globals.iter().filter(|(p, _)| p != me).map(|(_, d)| d.clone()).collect();
        out.extend(wtg_rs::text::trigger_from_text(text, None).variables);
        out
    }

    /// The view of the file `me` as the editor has it (`triggers::view`).
    fn view(&self, me: &str, text: &str, names: &wc3_server::names::Names) -> Value {
        let v = wc3_server::triggers::view(text, names, &self.visible(me, text), &self.triggers.looking_at(me));
        serde_json::from_str(&v).unwrap_or(Value::Null)
    }
}

/// The trigger file `args.file`, its project loaded: its path (as the
/// project has it) and the project.
fn open(args: &Value) -> Result<(String, Loaded), String> {
    let file = PathBuf::from(arg_str(args, "file")?);
    if !file.is_file() {
        return Err(format!("{}: no such file", file.display()));
    }
    let dir = project_dir(args, &file);
    let loaded = Loaded::load(&dir);
    let me = file.to_string_lossy().into_owned();
    if loaded.text(&me).is_none() {
        return Err(format!("{me} isn't in the project's folder {} (`dir`)", dir.display()));
    }
    Ok((me, loaded))
}

/// `path` from the project's folder, with `/`.
fn relative(path: &str, dir: &Path) -> String {
    Path::new(path).strip_prefix(dir).map_or(path.to_string(), |p| p.to_string_lossy().replace('\\', "/"))
}

/// Applies an answer of `triggers::edit_in` to `text`: the file's new text
/// and the other files' (path, new text); or its error.
fn applied(text: &str, answer: &str, others: &[(String, String)]) -> Result<(String, Vec<(String, String)>), String> {
    let a: Value = serde_json::from_str(answer).map_err(|e| format!("the server's answer: {e}"))?;
    if let Some(e) = a.get("error").and_then(Value::as_str) {
        return Err(e.to_string());
    }
    let edit = |text: &str, e: &[Value]| -> String {
        use wc3_server::document::{Document, Language, Position, Range};
        let n = |i: usize| e.get(i).and_then(Value::as_u64).unwrap_or(0) as u32;
        let mut doc = Document { language: Language::Bni, version: 0, text: text.to_string(), path: None };
        let new = e.last().and_then(Value::as_str).unwrap_or("");
        doc.apply(Range::new(Position::new(n(0), n(1)), Position::new(n(2), n(3))), new);
        doc.text
    };
    let mut mine = text.to_string();
    for e in a.get("edits").and_then(Value::as_array).into_iter().flatten() {
        mine = edit(&mine, e.as_array().map_or(&[][..], Vec::as_slice));
    }
    let mut files = Vec::new();
    for f in a.get("files").and_then(Value::as_array).into_iter().flatten() {
        let Some(f) = f.as_array() else { continue };
        let Some(id) = f.first().and_then(Value::as_str) else { continue };
        let Some((_, old)) = others.iter().find(|(p, _)| p == id) else { continue };
        files.push((id.to_string(), edit(old, &f[1..])));
    }
    Ok((mine, files))
}

fn write_all(files: &[(String, String)]) -> Result<(), String> {
    for (p, t) in files {
        std::fs::write(p, t).map_err(|e| format!("{p}: {e}"))?;
    }
    Ok(())
}

// ---------------------------------------------------------------- reading

fn extract(args: &Value) -> Result<Value, String> {
    let (f, _) = crate::wtg::open(args)?;
    // The map's custom script beside it: the code of custom text, and the map's own.
    let (path, archive) = crate::source::file(args, "war3map.wtg")?;
    let script = std::path::Path::new(path).with_file_name("war3map.wct");
    let wct = crate::source::read_bytes(&script.to_string_lossy(), archive).ok().and_then(|b| wc3_server::customtext::document(&script, &b)).map(|d| d.text);
    let made = wc3_server::triggers::extract_files(&f, wct.as_deref())?;
    let dir = PathBuf::from(arg_str(args, "dir")?);
    let replace = args.get("replace").and_then(Value::as_bool).unwrap_or(false);
    let there: Vec<&str> = made.files.iter().filter(|(p, _)| dir.join(p).exists()).map(|(p, _)| p.as_str()).collect();
    if !there.is_empty() && !replace {
        return Err(format!("{} file(s) are there already, nothing written (replace=true to replace them): {}", there.len(), there.join(", ")));
    }
    for folder in &made.folders {
        std::fs::create_dir_all(dir.join(folder)).map_err(|e| format!("{folder}: {e}"))?;
    }
    for (p, t) in &made.files {
        let path = dir.join(p);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("{p}: {e}"))?;
        }
        std::fs::write(&path, t).map_err(|e| format!("{p}: {e}"))?;
    }
    let custom = f.triggers.iter().filter(|t| t.is_custom != 0).count();
    Ok(json!({
        "dir": dir,
        "folders": made.folders,
        "file_count": made.files.len(),
        // A big map's hundreds of names aren't worth their tokens: trg_list
        // shows them.
        "files": (made.files.len() <= 100).then(|| made.files.iter().map(|(p, _)| p).collect::<Vec<_>>()),
        "custom_text_triggers": custom,
        "note": match (custom > 0, wct.is_some()) {
            (_, true) => "each trigger of custom text's code is beside its .trg (X.j), the project's code standing for its piece of the map's war3map.wct; the map's own custom script stays in war3map.wct",
            (true, false) => "no war3map.wct beside war3map.wtg: the triggers of custom text are without their code",
            _ => "",
        },
    }))
}

fn list(args: &Value) -> Result<Value, String> {
    let dir = PathBuf::from(arg_str(args, "dir")?);
    let filter = args.get("name").and_then(Value::as_str).map(str::to_lowercase);
    let names = crate::source::names(args);
    let loaded = Loaded::load(&dir);
    let mut triggers = Vec::new();
    let mut variables = Vec::new();
    for (path, text) in loaded.files.iter().filter(|(p, _)| is_trg(Path::new(p))) {
        let p = wtg_rs::text::trigger_from_text(text, None);
        let rel = relative(path, &dir);
        for d in &p.variables {
            let v = &d.variable;
            variables.push(json!({
                "file": rel, "name": v.name, "type": v.kind, "array": v.is_array != 0,
                "scope": d.scope.word().unwrap_or("global"),
            }));
        }
        let Some(t) = p.trigger else { continue };
        let stem = Path::new(path).file_stem().map_or(String::new(), |s| s.to_string_lossy().into_owned());
        let name = if t.name.is_empty() { stem } else { t.name.clone() };
        if filter.as_ref().is_some_and(|f| !name.to_lowercase().contains(f)) {
            continue;
        }
        let mut counts = [0usize; 3];
        count(&t.ecas, &mut counts);
        let problems = loaded.view(path, text, &names).get("errors").and_then(Value::as_array).map_or(0, Vec::len);
        triggers.push(json!({
            "file": rel,
            "category": Path::new(&rel).parent().map_or(String::new(), |c| c.to_string_lossy().into_owned()),
            "name": name,
            "script": format!("gg_trg_{}", wtg_rs::jass::ident(&name)),
            "enabled": t.enabled != 0, "initially_off": t.initially_off != 0, "run_on_init": t.run_on_init != 0,
            "custom_text": t.is_custom != 0, "comment": t.is_comment != 0,
            "events": counts[0], "conditions": counts[1], "actions": counts[2],
            "problems": problems,
        }));
    }
    Ok(json!({"dir": dir, "triggers": triggers, "variables": variables}))
}

fn count(e: &[wtg_rs::Eca], counts: &mut [usize; 3]) {
    for x in e {
        if let Some(c) = counts.get_mut(x.kind as usize) {
            *c += 1;
        }
        count(&x.children, counts);
    }
}

/// A view's spans as words: the line's own parameters in [] (a call in
/// its parentheses, as World Editor shows it), what isn't known marked
/// `?`.
fn words(spans: &Value, top: bool) -> String {
    let mut out = String::new();
    for s in spans.as_array().into_iter().flatten() {
        match s {
            Value::String(t) => out += t,
            Value::Object(o) => {
                let role = o.get("r").and_then(Value::as_str).unwrap_or("");
                let param = o.get("p").and_then(Value::as_str).is_some_and(|p| !p.is_empty());
                let inner = words(o.get("s").unwrap_or(&Value::Null), top && !param);
                // A call's phrase has its parentheses.
                let shown = if role == "u" && param { format!("{inner}?") } else { inner };
                out += &if top && param { format!("[{shown}]") } else { shown };
            }
            _ => {}
        }
    }
    out
}

fn read(args: &Value) -> Result<String, String> {
    let (me, loaded) = open(args)?;
    let text = loaded.text(&me).expect("opened");
    let names = crate::source::names(args);
    let v = loaded.view(&me, text, &names);
    let calls = args.get("calls").and_then(Value::as_bool).unwrap_or(true);
    let lines: Vec<&str> = parser_rs::lines::lines(&text).collect();
    let label = |i: usize, english: &str| v["labels"].get(i).and_then(Value::as_str).filter(|s| !s.is_empty()).unwrap_or(english).to_string();
    let mut out = String::new();
    let file = Path::new(&me).file_name().map_or(String::new(), |n| n.to_string_lossy().into_owned());
    if v["trigger"].as_bool() == Some(true) {
        let name = v["name"].as_str().filter(|n| !n.is_empty()).map_or_else(|| format!("(none: the file's) {}", file.trim_end_matches(".trg")), |n| format!("\"{n}\""));
        let mut flags = Vec::new();
        if v["enabled"] == json!(0) {
            flags.push("disabled");
        }
        if v["initially_off"] != json!(0) {
            flags.push("initially off");
        }
        if v["run_on_init"] != json!(0) {
            flags.push("runs on map initialization");
        }
        if v["custom_text"] != json!(0) {
            flags.push("custom text: its code is in war3map.wct");
        }
        if v["comment"] != json!(0) {
            flags.push("a comment, not a trigger");
        }
        out += &format!("Trigger {name} ({}), file {file}", v["script"].as_str().unwrap_or(""));
        if !flags.is_empty() {
            out += &format!("; {}", flags.join(", "));
        }
        out += "\n";
        if let Some(d) = v["description"].as_str().filter(|d| !d.is_empty()) {
            out += &format!("Comment: {}\n", d.replace('\n', "\n         "));
        }
    } else {
        out += &format!("File {file}: no [trigger], variables only\n");
    }
    if let Some(vars) = v["variables"].as_array().filter(|l| !l.is_empty()) {
        out += &format!("{}\n", label(3, "Variables"));
        for x in vars {
            let array = if x["array"] == json!(true) { format!("[{}]", x["size"]) } else { String::new() };
            let initial = x["initial"].as_str().map_or(String::new(), |i| format!(" = \"{i}\""));
            out += &format!("  L{} {}: {}{array}{initial} ({})\n", x["line"], x["name"].as_str().unwrap_or(""), x["type_name"].as_str().unwrap_or(""), x["scope"].as_str().unwrap_or(""));
        }
    }
    if v["trigger"].as_bool() == Some(true) {
        for (i, (key, english)) in [("events", "Events"), ("conditions", "Conditions"), ("actions", "Actions")].iter().enumerate() {
            out += &format!("{}\n", label(i, english));
            nodes(&mut out, &v[*key], 1, &lines, calls);
        }
    }
    let errors = v["errors"].as_array().cloned().unwrap_or_default();
    if !errors.is_empty() {
        out += "Problems\n";
        for e in errors {
            out += &format!("  line {}: {}\n", e[0], e[1].as_str().unwrap_or(""));
        }
    }
    if v["data"] == json!(false) {
        out += &format!("(No UI\\TriggerData.txt in {}: lines show as their calls.)\n", crate::source::layers_named(args));
    }
    Ok(out)
}

fn nodes(out: &mut String, list: &Value, depth: usize, lines: &[&str], calls: bool) {
    let pad = "  ".repeat(depth);
    for n in list.as_array().into_iter().flatten() {
        let line = n["line"].as_u64().unwrap_or(0) as usize;
        let off = if n["enabled"] == json!(false) { "(disabled) " } else { "" };
        *out += &format!("{pad}L{line} {off}{}\n", words(&n["text"], true));
        if calls && let Some(raw) = line.checked_sub(1).and_then(|i| lines.get(i)) {
            let call = raw.split_once('=').map_or(*raw, |(_, c)| c).trim();
            *out += &format!("{pad}     = {call}\n");
        }
        for b in n["blocks"].as_array().into_iter().flatten() {
            *out += &format!("{pad}  {} (block {})\n", b["label"].as_str().unwrap_or(""), b["group"]);
            nodes(out, &b["nodes"], depth + 2, lines, calls);
        }
    }
}

fn check(args: &Value) -> Result<Value, String> {
    let names = crate::source::names(args);
    let data = (!names.triggers.is_empty()).then_some(&names.triggers);
    let (dir, only) = match (args.get("dir").and_then(Value::as_str), args.get("file").and_then(Value::as_str)) {
        (_, Some(f)) => (project_dir(args, Path::new(f)), Some(f.to_string())),
        (Some(d), None) => (PathBuf::from(d), None),
        (None, None) => return Err("`dir` or `file` is missing".into()),
    };
    let mut problems = Vec::new();
    let loaded = Loaded::load(&dir);
    for (path, text) in loaded.files.iter().filter(|(p, _)| is_trg(Path::new(p))) {
        if only.as_ref().is_some_and(|f| Path::new(f) != Path::new(path)) {
            continue;
        }
        let mut found: Vec<(u64, String)> = Vec::new();
        if let Some(data) = data {
            found.extend(wtg_rs::text::trigger_from_text(text, Some(data)).errors.iter().map(|e| (e.line as u64, e.message.clone())));
        }
        for e in loaded.view(path, text, &names)["errors"].as_array().into_iter().flatten() {
            found.push((e[0].as_u64().unwrap_or(0), e[1].as_str().unwrap_or("").to_string()));
        }
        found.sort();
        found.dedup();
        let rel = relative(path, &dir);
        problems.extend(found.into_iter().map(|(line, message)| json!({"file": rel, "line": line, "message": message})));
    }
    Ok(json!({"dir": dir, "problems": problems, "checked_functions": data.is_some()}))
}

fn jass(args: &Value) -> Result<String, String> {
    let (me, loaded) = open(args)?;
    let text = loaded.text(&me).expect("opened");
    let names = crate::source::names(args);
    Ok(wc3_server::triggers::jass(text, &me, &names, &loaded.visible(&me, text)))
}

// ---------------------------------------------------------------- the editor's words

const KINDS: [&str; 4] = ["event", "condition", "action", "call"];

fn kinds(args: &Value) -> Result<Vec<u32>, String> {
    match args.get("kind").and_then(Value::as_str) {
        None => Ok(vec![0, 1, 2, 3]),
        Some(k) => KINDS.iter().position(|x| *x == k).map(|i| vec![i as u32]).ok_or(format!("no kind {k:?}: event, condition, action, call")),
    }
}

fn ui(args: &Value) -> Result<wtg_rs::ui::Ui, String> {
    let ui = crate::source::names(args).trigger_ui;
    if ui.is_empty() {
        return Err(format!("no UI\\TriggerData.txt in {}", crate::source::layers_named(args)));
    }
    Ok(ui)
}

fn type_name(ui: &wtg_rs::ui::Ui, ty: &str) -> String {
    ui.types.get(ty).cloned().unwrap_or_else(|| ty.to_string())
}

fn functions(args: &Value) -> Result<Value, String> {
    let ui = ui(args)?;
    let kinds = kinds(args)?;
    let category = |f: &wtg_rs::ui::Function| ui.categories.get(&f.category).map_or(String::new(), |c| c.name.clone());
    if let Some(name) = args.get("name").and_then(Value::as_str) {
        let found: Vec<Value> = kinds
            .iter()
            .filter_map(|&k| {
                let fk = FunctionKind::of(k)?;
                let f = ui.function(fk, name)?;
                let phrase: String = f
                    .phrase
                    .iter()
                    .map(|p| match p {
                        wtg_rs::ui::Piece::Text(t) => t.clone(),
                        wtg_rs::ui::Piece::Param(i) => format!("{{{i}}}"),
                    })
                    .collect();
                let params: Vec<Value> = f
                    .args
                    .iter()
                    .enumerate()
                    .map(|(i, ty)| {
                        let d = f.defaults.get(i).map_or("_", String::as_str);
                        json!({
                            "index": i,
                            "name": f.names.get(i).cloned().unwrap_or_default(),
                            "type": ty, "type_name": type_name(&ui, ty),
                            "default": ui.default_param(ty, d),
                            "presets": ui.presets_of(ty).len(),
                            "value": ui.takes_value(ty),
                        })
                    })
                    .collect();
                let call = ui.new_call(fk, name);
                let line = if k < 3 { format!("{}={call}", KINDS[k as usize]) } else { call };
                Some(json!({
                    "name": name, "kind": KINDS[k as usize], "title": f.title, "category": category(f),
                    "phrase": phrase, "params": params,
                    "returns": if f.returns.is_empty() { Value::Null } else { json!({"type": f.returns, "type_name": type_name(&ui, &f.returns)}) },
                    "hint": f.hint, "script": if f.script.is_empty() { name } else { f.script.as_str() },
                    "line": line,
                }))
            })
            .collect();
        if found.is_empty() {
            return Err(format!("no function {name:?} in UI\\TriggerData.txt (of these kinds)"));
        }
        return Ok(json!(found));
    }
    let words: Vec<String> = args.get("query").and_then(Value::as_str).unwrap_or("").split_whitespace().map(str::to_lowercase).collect();
    let limit = args.get("limit").and_then(Value::as_u64).unwrap_or(50) as usize;
    let mut out = Vec::new();
    let mut total = 0;
    for &k in &kinds {
        let Some(fk) = FunctionKind::of(k) else { continue };
        for (name, f) in ui.functions(fk) {
            let hay = format!("{name} {} {} {}", f.title, category(f), f.hint).to_lowercase();
            if !words.iter().all(|w| hay.contains(w)) {
                continue;
            }
            total += 1;
            if out.len() < limit {
                let mut item = json!({"name": name, "kind": KINDS[k as usize], "title": f.title, "category": category(f)});
                if !f.returns.is_empty() {
                    item["returns"] = json!(type_name(&ui, &f.returns));
                }
                out.push(item);
            }
        }
    }
    Ok(json!({"total": total, "functions": out}))
}

fn types(args: &Value) -> Result<Value, String> {
    let ui = ui(args)?;
    let variable: Vec<&str> = ui.variable_types().into_iter().map(|(t, _)| t).collect();
    let Some(ty) = args.get("type").and_then(Value::as_str) else {
        let mut all: Vec<(&String, &String)> = ui.types.iter().collect();
        all.sort();
        let list: Vec<Value> = all
            .into_iter()
            .map(|(t, n)| json!({"type": t, "name": n, "base": ui.base(t), "value": ui.takes_value(t), "variable": variable.contains(&t.as_str()), "presets": ui.presets_of(t).len()}))
            .collect();
        return Ok(json!(list));
    };
    let q = args.get("query").and_then(Value::as_str).unwrap_or("").to_lowercase();
    let limit = args.get("limit").and_then(Value::as_u64).unwrap_or(200) as usize;
    let presets: Vec<_> = ui.presets_of(ty).into_iter().filter(|(k, p)| q.is_empty() || k.to_lowercase().contains(&q) || p.display.to_lowercase().contains(&q)).collect();
    Ok(json!({
        "type": ty, "name": type_name(&ui, ty), "base": ui.base(ty), "value": ui.takes_value(ty),
        "total": presets.len(),
        "presets": presets.iter().take(limit).map(|(k, p)| json!({"preset": k, "display": p.display, "script": p.code})).collect::<Vec<_>>(),
    }))
}

// ---------------------------------------------------------------- changing

fn write(args: &Value) -> Result<Value, String> {
    let file = PathBuf::from(arg_str(args, "file")?);
    let text = arg_str(args, "text")?;
    let names = crate::source::names(args);
    let data = (!names.triggers.is_empty()).then_some(&names.triggers);
    let p = wtg_rs::text::trigger_from_text(text, data);
    if !p.errors.is_empty() {
        return Err(crate::source::text_errors(p.errors.iter().map(|e| (e.line, e.message.as_str()))));
    }
    if let Some(parent) = file.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
    }
    std::fs::write(&file, text).map_err(|e| format!("{}: {e}", file.display()))?;
    let loaded = Loaded::load(&project_dir(args, &file));
    let me = file.to_string_lossy().into_owned();
    let problems: Vec<Value> = loaded.view(&me, text, &names)["errors"].as_array().cloned().unwrap_or_default().into_iter().map(|e| json!({"line": e[0], "message": e[1]})).collect();
    Ok(json!({"written": file, "checked_functions": data.is_some(), "problems": problems}))
}

fn rename(args: &Value) -> Result<Value, String> {
    let (me, loaded) = open(args)?;
    let text = loaded.text(&me).expect("opened");
    let others = loaded.others(&me);
    let names = crate::source::names(args);
    let new = arg_str(args, "name")?;
    let old = wtg_rs::text::trigger_from_text(text, None).trigger.map(|t| t.name).unwrap_or_default();
    let project = loaded.project(&me);
    let (named, _) = applied(text, &wc3_server::triggers::edit_in(text, op::FIELD, 0, &format!("name\n{new}"), &names.trigger_ui, &project), &others)?;
    let answer = wc3_server::triggers::edit_in(&named, op::RENAMED, 0, &old, &names.trigger_ui, &project);
    let (mine, files) = applied(&named, &answer, &others)?;
    let mut all = vec![(me.clone(), mine)];
    all.extend(files);
    write_all(&all)?;
    let stem = Path::new(&me).file_stem().map_or(String::new(), |s| s.to_string_lossy().into_owned());
    let script = |n: &str| format!("gg_trg_{}", wtg_rs::jass::ident(if n.is_empty() { &stem } else { n }));
    Ok(json!({
        "from": script(&old), "to": script(new),
        "files": all.iter().map(|(p, _)| p).collect::<Vec<_>>(),
    }))
}

fn variable(args: &Value) -> Result<Value, String> {
    let (me, loaded) = open(args)?;
    let text = loaded.text(&me).expect("opened");
    let others = loaded.others(&me);
    let project = loaded.project(&me);
    let names = crate::source::names(args);
    let name = arg_str(args, "variable")?;
    let v = loaded.view(&me, text, &names);
    let var = v["variables"].as_array().into_iter().flatten().find(|x| x["name"] == json!(name)).cloned().ok_or(format!("{me} declares no variable {name}"))?;
    let line = var["line"].as_u64().unwrap_or(0) as usize;
    let delete = args.get("delete").and_then(Value::as_bool).unwrap_or(false);
    let arg = if delete {
        String::new()
    } else {
        let s = |k: &str, now: &Value| args.get(k).and_then(Value::as_str).map(str::to_string).unwrap_or_else(|| now.as_str().unwrap_or("").to_string());
        let array = args.get("array").and_then(Value::as_bool).unwrap_or(var["array"] == json!(true));
        let size = args.get("size").and_then(Value::as_u64).or(var["size"].as_u64()).unwrap_or(1);
        [s("name", &var["name"]), s("type", &var["type"]), if array { "1".into() } else { "0".into() }, size.to_string(), s("scope", &var["scope"]), s("initial", &var["initial"])].join("\n")
    };
    let ui = &names.trigger_ui;
    let check: Value = serde_json::from_str(&wc3_server::triggers::edit_in(text, op::CHECK_VARIABLE, line, &arg, ui, &project)).map_err(|e| e.to_string())?;
    if let Some(e) = check.get("error").and_then(Value::as_str) {
        return Err(e.to_string());
    }
    let places: Vec<Value> = check["places"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|p| json!({"file": p[0], "line": p[1], "why": p[2]}))
        .collect();
    let confirm = args.get("confirm").and_then(Value::as_bool).unwrap_or(false);
    if !places.is_empty() && !confirm {
        return Ok(json!({
            "done": false,
            "message": format!("{} place(s) would need fixing by hand; nothing done: confirm=true to do it anyway", places.len()),
            "places": places, "renamed": check["renamed"],
        }));
    }
    let answer = if delete {
        wc3_server::triggers::edit_in(text, op::DELETE, line, "", ui, &project)
    } else {
        wc3_server::triggers::edit_in(text, op::SET_VARIABLE, line, &arg, ui, &project)
    };
    let (mine, files) = applied(text, &answer, &others)?;
    let mut all = vec![(me.clone(), mine)];
    all.extend(files);
    write_all(&all)?;
    Ok(json!({
        "done": true, "renamed": check["renamed"], "places": places,
        "files": all.iter().map(|(p, _)| p).collect::<Vec<_>>(),
    }))
}

#[cfg(test)]
#[path = "trg.test.rs"]
mod tests;
