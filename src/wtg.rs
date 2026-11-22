//! The trigger tools, on `wtg-rs`: `war3map.wtg`, a map's GUI triggers
//! with their categories and variables. How many parameters each of the
//! editor's functions takes isn't in the file but in World Editor's
//! `UI\TriggerData.txt`, looked up through `game` and `roots` as the
//! editor does (a modded editor's in a root wins). A file is read from
//! disk (`path`) or from inside a map (`archive`, `war3map.wtg` by
//! default); changes are written to files on disk.
//!
//! A trigger's text is the editor's: a `[trigger]` section whose events,
//! conditions and actions are lines of calls. Adding or removing
//! triggers keeps the `war3map.wct` beside the file in step: it has a
//! piece of code per trigger.

use serde_json::{Value, json};

use wtg_rs::{TriggerData, Triggers};

use crate::source::{arg_str, archive_schema, file, game_schema, path_schema, roots_schema};

/// The most triggers `wtg_list` gives at once, by default.
const LIMIT: u64 = 500;

const SYNTAX: &str = "A [trigger] section: name=\"...\", description=\"...\", category=<index>, flags when not the default (enabled=1; custom_text, initially_off, run_on_init, comment 0), then event=, condition=, action= lines, each a call: Name(params). Parameters: Name(...) a call, $name a variable, \"text\" a string (numbers too: \"30.00\"), a bare word a preset (PlayerALL), [index] after a variable for an array, `event:`/`condition:`/`action:` before a call of one of those. A disabled line starts with !, one inside another (if/then/else, loops) with > per level and :block after its kind (if/then/else: 0 condition, 1 then, 2 else).";

/// Every tool, for `tools/list`.
pub fn tools() -> Value {
    let path = path_schema("war3map.wtg", "The trigger file");
    let disk = json!({"type": "string", "description": "The war3map.wtg on disk"});
    let trigger = json!({"description": "A trigger: its name, or its index (its place, from 0)", "anyOf": [{"type": "integer"}, {"type": "string"}]});
    let triggers = json!({"type": "array", "items": trigger.clone(), "description": "Triggers: names or indexes"});
    let with = |extra: Value| {
        let mut props = json!({"game": game_schema(), "roots": roots_schema(), "project": crate::source::project_schema()});
        props.as_object_mut().expect("an object").extend(extra.as_object().expect("an object").clone());
        props
    };
    json!([
        {
            "name": "wtg_list",
            "description": "A map's GUI triggers (war3map.wtg): categories, variables (name, type, array, initial value) and triggers (index, name, category, flags, how many events, conditions, actions). Filters: `category` (index), `name` (part of it). `offset`, `limit` (500) page through triggers. Needs the game's UI\\TriggerData.txt: `game` and `roots`, or the project's (the nearest wc3.bni above the map).",
            "inputSchema": {"type": "object", "properties": with(json!({
                "path": path, "archive": archive_schema(),
                "category": {"type": "integer"}, "name": {"type": "string"},
                "offset": {"type": "integer", "minimum": 0}, "limit": {"type": "integer", "minimum": 1},
            }))},
        },
        {
            "name": "wtg_get",
            "description": format!("The text of triggers, by name or index: [wtg] then a [trigger] section each. {SYNTAX} Edit one and give it to wtg_set."),
            "inputSchema": {"type": "object", "properties": with(json!({"path": path, "archive": archive_schema(), "trigger": triggers})), "required": ["trigger"]},
        },
        {
            "name": "wtg_set",
            "description": "Replaces a trigger of a war3map.wtg on disk with `text`: one [trigger] section, as wtg_get gives it. Every function is checked against UI\\TriggerData.txt (`game`, `roots`).",
            "inputSchema": {"type": "object", "properties": with(json!({"path": disk, "trigger": trigger, "text": {"type": "string"}})), "required": ["path", "trigger", "text"]},
        },
        {
            "name": "wtg_new",
            "description": format!("Adds to a war3map.wtg on disk what `text` holds: [category], [variable] (name=, type=, array=, size=, initial=) and [trigger] sections, at the end. A trigger goes in a category by its index (wtg_list). The war3map.wct beside it gets an empty piece per new trigger. {SYNTAX}"),
            "inputSchema": {"type": "object", "properties": with(json!({"path": disk, "text": {"type": "string"}})), "required": ["path", "text"]},
        },
        {
            "name": "wtg_delete",
            "description": "Removes triggers from a war3map.wtg on disk, and their code from the war3map.wct beside it. Triggers after them move up: indexes change.",
            "inputSchema": {"type": "object", "properties": with(json!({"path": disk, "trigger": triggers})), "required": ["path", "trigger"]},
        },
        {
            "name": "wtg_text",
            "description": "A map's whole trigger file as text: [wtg], then [category], [variable] and [trigger] sections. Large for big maps: wtg_list and wtg_get show parts.",
            "inputSchema": {"type": "object", "properties": with(json!({"path": path, "archive": archive_schema()}))},
        },
        {
            "name": "wtg_write",
            "description": "Writes a war3map.wtg on disk from its whole text form (as wtg_text gives it), after checking it and every function against UI\\TriggerData.txt: every error, with its line, and nothing written. Keep war3map.wct's pieces in step with the triggers.",
            "inputSchema": {"type": "object", "properties": with(json!({"path": disk, "text": {"type": "string"}})), "required": ["path", "text"]},
        },
    ])
}

/// Runs the tool `name`, if it's one of these.
pub fn call(name: &str, args: &Value) -> Option<Result<Value, String>> {
    Some(match name {
        "wtg_list" => open(args).map(|(f, _)| list(&f, args)),
        "wtg_get" => open(args).and_then(|(f, _)| get(&f, args)),
        "wtg_text" => open(args).and_then(|(f, _)| wtg_rs::text::to_text(&f).map(Value::String)),
        "wtg_set" | "wtg_new" | "wtg_delete" | "wtg_write" => change(name, args),
        _ => return None,
    })
}

fn data(args: &Value) -> Result<TriggerData, String> {
    let data = crate::source::names(args).triggers;
    if data.is_empty() {
        return Err(format!(
            "no UI\\TriggerData.txt in {}: it says how many parameters each function takes, which the file doesn't",
            crate::source::layers_named(args)
        ));
    }
    Ok(data)
}

fn read(path: &str, bytes: &[u8], data: &TriggerData) -> Result<Triggers, String> {
    match wtg_rs::parse_any(bytes, data).map_err(|e| format!("{path}: {e}"))? {
        wtg_rs::File::Classic(f) => Ok(f),
        wtg_rs::File::Tree(_) => Err(format!("{path}: the layout of 1.31 and later isn't shown as text yet")),
    }
}

pub(crate) fn open(args: &Value) -> Result<(Triggers, TriggerData), String> {
    let (path, archive) = file(args, "war3map.wtg")?;
    // The file first: a file that isn't there is the first thing to say.
    let bytes = crate::source::read_bytes(path, archive)?;
    let data = data(args)?;
    let f = read(path, &bytes, &data)?;
    Ok((f, data))
}

fn ecas(e: &[wtg_rs::Eca], counts: &mut [usize; 3]) {
    for x in e {
        if let Some(c) = counts.get_mut(x.kind as usize) {
            *c += 1;
        }
        ecas(&x.children, counts);
    }
}

fn list(f: &Triggers, args: &Value) -> Value {
    let category = args.get("category").and_then(Value::as_i64);
    let part = args.get("name").and_then(Value::as_str).map(str::to_lowercase);
    let offset = args.get("offset").and_then(Value::as_u64).unwrap_or(0) as usize;
    let limit = args.get("limit").and_then(Value::as_u64).unwrap_or(LIMIT) as usize;
    let cat_name = |i: i32| f.categories.iter().find(|c| c.index == i).map(|c| c.name.clone());
    let found: Vec<Value> = f
        .triggers
        .iter()
        .enumerate()
        .filter(|(_, t)| category.is_none_or(|c| i64::from(t.category) == c))
        .filter(|(_, t)| part.as_ref().is_none_or(|p| t.name.to_lowercase().contains(p)))
        .map(|(i, t)| {
            let mut n = [0; 3];
            ecas(&t.ecas, &mut n);
            json!({
                "index": i, "name": t.name, "category": t.category, "category_name": cat_name(t.category),
                "enabled": t.enabled, "custom_text": t.is_custom, "initially_off": t.initially_off,
                "run_on_init": t.run_on_init, "comment": t.is_comment,
                "events": n[0], "conditions": n[1], "actions": n[2],
            })
        })
        .collect();
    let categories: Vec<Value> =
        f.categories.iter().map(|c| json!({"index": c.index, "name": c.name, "comment": c.is_comment})).collect();
    let variables: Vec<Value> = f
        .variables
        .iter()
        .map(|v| {
            json!({"name": v.name, "type": v.kind, "array": v.is_array != 0, "size": v.array_size,
                   "initial": (v.has_initial != 0).then_some(&v.initial)})
        })
        .collect();
    json!({
        "version": f.version, "categories": categories, "variables": variables,
        "triggers_total": f.triggers.len(), "matched": found.len(), "offset": offset,
        "triggers": found.into_iter().skip(offset).take(limit).collect::<Vec<_>>(),
    })
}

/// Where trigger `v` (a name or an index) is.
fn find(f: &Triggers, v: &Value) -> Result<usize, String> {
    match v {
        Value::Number(n) => n
            .as_u64()
            .map(|n| n as usize)
            .filter(|&n| n < f.triggers.len())
            .ok_or(format!("no trigger {n}: the file has {}", f.triggers.len())),
        Value::String(s) => f.triggers.iter().position(|t| &t.name == s).ok_or(format!("no trigger named {s:?}")),
        _ => Err("a trigger is a name or an index".into()),
    }
}

fn picks(f: &Triggers, args: &Value) -> Result<Vec<usize>, String> {
    match args.get("trigger") {
        Some(Value::Array(l)) => l.iter().map(|v| find(f, v)).collect(),
        Some(v) => Ok(vec![find(f, v)?]),
        None => Err("`trigger` is missing".into()),
    }
}

fn get(f: &Triggers, args: &Value) -> Result<Value, String> {
    let triggers = picks(f, args)?.into_iter().map(|i| f.triggers[i].clone()).collect();
    let part = Triggers { categories: Vec::new(), variables: Vec::new(), triggers, ..f.clone() };
    wtg_rs::text::to_text(&part).map(Value::String)
}

/// Reads sections in `text` as `f`'s version, checked with `data`; their
/// errors with lines of `text`.
fn sections(f: &Triggers, text: &str, data: &TriggerData) -> Result<Triggers, String> {
    let full = format!("[wtg]\nversion={}\ngame={}\n", f.version, f.game) + text;
    let p = wtg_rs::text::from_text(&full, Some(data));
    p.file.ok_or_else(|| crate::source::text_errors(p.errors.iter().map(|e| (e.line.saturating_sub(3), e.message.as_str()))))
}

fn change(name: &str, args: &Value) -> Result<Value, String> {
    let path = arg_str(args, "path")?;
    let data = data(args)?;
    let mut f = if name == "wtg_write" {
        let p = wtg_rs::text::from_text(arg_str(args, "text")?, Some(&data));
        p.file.ok_or_else(|| crate::source::text_errors(p.errors.iter().map(|e| (e.line, e.message.as_str()))))?
    } else {
        read(path, &std::fs::read(path).map_err(|e| format!("{path}: {e}"))?, &data)?
    };
    // The custom script beside it, kept in step: (removed, added).
    let mut wct: (Vec<usize>, usize) = (Vec::new(), 0);
    match name {
        "wtg_set" => {
            let i = find(&f, args.get("trigger").ok_or("`trigger` is missing")?)?;
            let new = sections(&f, arg_str(args, "text")?, &data)?;
            if !new.categories.is_empty() || !new.variables.is_empty() || new.triggers.len() != 1 {
                return Err("nothing written: `text` is one [trigger] section".into());
            }
            f.triggers[i] = new.triggers.into_iter().next().expect("one");
        }
        "wtg_new" => {
            let new = sections(&f, arg_str(args, "text")?, &data)?;
            wct.1 = new.triggers.len();
            f.categories.extend(new.categories);
            f.variables.extend(new.variables);
            f.triggers.extend(new.triggers);
        }
        "wtg_delete" => {
            let mut gone = picks(&f, args)?;
            gone.sort_unstable();
            gone.dedup();
            for &i in gone.iter().rev() {
                f.triggers.remove(i);
            }
            wct.0 = gone;
        }
        _ => {}
    }
    let bytes = f.to_bytes().map_err(|e| format!("nothing written: {e}"))?;
    // The custom script first: if it can't follow, nothing is written.
    let wct_path = std::path::Path::new(path).with_file_name("war3map.wct");
    let mut wct_note = Value::Null;
    if (!wct.0.is_empty() || wct.1 > 0) && wct_path.is_file() {
        let code = std::fs::read(&wct_path).map_err(|e| format!("{}: {e}", wct_path.display()))?;
        let mut custom = wct_rs::parse(&code).map_err(|e| format!("nothing written: {}: {e}", wct_path.display()))?;
        if custom.magic {
            return Err("nothing written: war3map.wct is 1.31's, with code only for some triggers: change it with wct_write".into());
        }
        for &i in wct.0.iter().rev() {
            if i < custom.triggers.len() {
                custom.triggers.remove(i);
            }
        }
        custom.triggers.extend((0..wct.1).map(|_| wct_rs::Code(None)));
        std::fs::write(&wct_path, custom.to_bytes().map_err(|e| format!("nothing written: {e}"))?)
            .map_err(|e| format!("{}: {e}", wct_path.display()))?;
        wct_note = json!({"written": wct_path, "pieces": custom.triggers.len()});
    }
    crate::source::write(path, bytes)?;
    Ok(json!({
        "written": path, "triggers": f.triggers.len(), "variables": f.variables.len(),
        "categories": f.categories.len(), "custom_script": wct_note,
    }))
}

#[cfg(test)]
#[path = "wtg.test.rs"]
mod tests;
