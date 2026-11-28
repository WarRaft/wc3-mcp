//! The project tools: a project's settings, its `wc3.bni` (what the
//! editor reads: `[project] game=`, `[map] root=`, `[code] sources=`,
//! `[build]`, `[lint]`; see the wc3-rs README, "Projects"). The other
//! tools take their `game` and `roots` from the project of the file they
//! work on when not given (`source::lookup`), so these are for seeing
//! and changing those settings.

use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use crate::source::arg_str;
use wc3_server::project::{LintSetting, MANIFEST};

/// Every tool, for `tools/list`.
pub fn tools() -> Value {
    json!([
        {
            "name": "project_info",
            "description": "A project's settings (wc3.bni, the nearest one in the folder of `path` or above): its game folders and map roots (each: does it exist, a folder or a map), code sources and exclusions, builds, the language sections ([jass] [vjass] [zinc] [cjass] [lua] [as]: which are on, strict JASS and its files, cJass's features, the dialect a .j file is read in), lint settings, the external tools, the problems of the settings, the manifest's text; and the editor's files the layers give (UI\\TriggerData.txt ...: from where, and the patches beside them, X.patch.txt changing X by meaning). Change the settings with project_write.",
            "inputSchema": {"type": "object", "properties": {
                "path": {"type": "string", "description": "A project's wc3.bni, or any file or folder in it (a map, a trigger folder)"},
            }, "required": ["path"]},
        },
        {
            "name": "project_write",
            "description": "Writes a project's wc3.bni from its whole text (made if missing), after reading it as the editor does: every error, with its line, and nothing written; warnings come back with what was written. Sections: [project] game= (game folders, lowest first), [map] root= (folders or maps whose top is the map's, lowest first; patches to the editor's files go in a folder root: UI\\TriggerData.patch.txt), [code] sources=, exclude= (masks), [build] name=, the language sections [jass] enable=, strict=, file= (the strict program's files, in order), string_limit=, concat_limit=, [vjass] [zinc] [cjass] [lua] [as] enable= (1/0; vJASS and Zinc on by default, cJass off; cJass's features by name: [cjass] macros=0 ...; a file's header sets the same keys: // @vjass.enable=0), [lint] <rule>=off|warning|error, [external] NAME=\"program\",\"arg\" (the tools JassHelper's //! external NAME runs on the map built: \"wine\",\"tools/ObjectMerger.exe\" for a Windows one). Paths are against the project's folder.",
            "inputSchema": {"type": "object", "properties": {
                "path": {"type": "string", "description": "The wc3.bni, or the project's folder"},
                "text": {"type": "string", "description": "The whole text"},
            }, "required": ["path", "text"]},
        },
        {
            "name": "project_build",
            "description": "Builds a project's map (a [build] of its wc3.bni: `out=` where, `build/<name>.w3x` by default; `uglify=1` short names; `inline=1` one-line functions inlined; `dead_code=1` what nothing uses left out; `string_hash=1` StringHash of a literal worked out): the map's files from its roots (the last that has a file wins; patches, .trg files, the project's code and wc3.bni left out), and war3map.j made anew of the map's files, its triggers (the project's .trg files, else the roots' war3map.wtg), the map's custom script (war3map.wct) and the project's code (vJASS, Zinc). When the first root is a map, the map built is a copy of it with the rest written in (a protected map's unnamed files stay). Gives where it was written, how many files, the script's lines; or every error, with its file and line, and nothing written.",
            "inputSchema": {"type": "object", "properties": {
                "path": {"type": "string", "description": "The project's wc3.bni, or any file or folder in it"},
                "name": {"type": "string", "description": "The [build]'s name; the first one when left out"},
            }, "required": ["path"]},
        },
    ])
}

/// Runs the tool `name`, if it's one of these.
pub fn call(name: &str, args: &Value) -> Option<Result<Value, String>> {
    Some(match name {
        "project_info" => info(args),
        "project_write" => write(args),
        "project_build" => build(args),
        _ => return None,
    })
}

fn issues(issues: &[wc3_server::project::Issue]) -> Value {
    issues.iter().map(|i| json!({"line": i.line, "level": i.level.as_str(), "message": i.message})).collect()
}

fn info(args: &Value) -> Result<Value, String> {
    let at = PathBuf::from(arg_str(args, "path")?);
    let at = std::path::absolute(&at).unwrap_or(at);
    let (project, manifest, problems) =
        crate::source::load_project(&at).ok_or(format!("{}: no {MANIFEST} in it or above it (project_write makes one)", at.display()))?;
    let text = std::fs::read_to_string(&manifest).unwrap_or_default();
    let place = |p: &Path| {
        let kind = if p.is_dir() { "folder" } else if p.is_file() { "map" } else { "missing" };
        json!({"path": p, "kind": kind})
    };
    let lint: serde_json::Map<String, Value> = project
        .lint
        .iter()
        .chain(&project.lint_groups)
        .map(|(k, v)| {
            let v = match v {
                LintSetting::Off => "off",
                LintSetting::On(level) => level.as_str(),
            };
            (k.clone(), json!(v))
        })
        .collect();
    // The editor's files, as the project's map reads them.
    let lookup = wc3_server::game::Lookup::of(&project);
    let (game, open_errors) = lookup.open();
    let source = |s: wc3_server::game::Source| match s {
        wc3_server::game::Source::Archive(p) | wc3_server::game::Source::File(p) => p,
        wc3_server::game::Source::Storage(folder, name) => folder.join(name),
    };
    let editor: Vec<Value> = wc3_server::names::PATCHED
        .iter()
        .map(|name| {
            let patch = wc3_server::game::patch_name(name);
            json!({
                "file": name,
                "from": game.read(name).map(|(_, s)| source(s)),
                "patch": patch,
                "patch_from": game.read(&patch).map(|(_, s)| source(s)),
            })
        })
        .collect();
    let s = &project.settings;
    let languages = json!({
        "jass": s.jass, "vjass": s.vjass_on(), "zinc": s.zinc_on(), "cjass": s.cjass_on(), "lua": s.lua, "as": s.angelscript,
        "strict": s.strict, "strict_files": project.strict_files.iter().map(|p| place(p)).collect::<Vec<_>>(),
        "cjass_features": s.cjass_features.iter().map(|(f, on)| json!({"feature": f, "on": on})).collect::<Vec<_>>(),
        "jass_dialect": jass_rs::Dialect::of(s).map(|d| d.name()),
    });
    Ok(json!({
        "manifest": manifest,
        "root": project.root,
        "game": project.game.iter().map(|p| place(p)).collect::<Vec<_>>(),
        "hd": project.hd,
        "de": project.de,
        "locale": project.locale,
        "roots": project.roots.iter().map(|p| place(p)).collect::<Vec<_>>(),
        "sources": project.sources.iter().map(|g| g.text.clone()).collect::<Vec<_>>(),
        "exclude": project.exclude.iter().map(|g| g.text.clone()).collect::<Vec<_>>(),
        "builds": project.builds.iter().map(|b| serde_json::json!({"name": b.name, "out": b.out(&project), "script": b.script.extension(), "uglify": b.uglify, "inline": b.inline, "dead_code": b.dead_code, "string_hash": b.string_hash})).collect::<Vec<_>>(),
        "languages": languages,
        "lint": lint,
        "externals": project.externals.iter().map(|(t, argv)| json!({"name": t, "command": argv})).collect::<Vec<_>>(),
        "problems": issues(&problems),
        "layers_not_opened": open_errors,
        "editor_files": editor,
        "text": text,
    }))
}

fn write(args: &Value) -> Result<Value, String> {
    let path = PathBuf::from(arg_str(args, "path")?);
    let text = arg_str(args, "text")?;
    let manifest = if path.is_dir() { path.join(MANIFEST) } else { path };
    if !wc3_server::project::is_manifest(&manifest) {
        return Err(format!("{}: a project's settings are its {MANIFEST}", manifest.display()));
    }
    let root = manifest.parent().ok_or(format!("{}: no folder", manifest.display()))?;
    if !root.is_dir() {
        return Err(format!("{}: no such folder", root.display()));
    }
    let doc = bni_rs::parse(text);
    let (_, problems) = wc3_server::project::load(root, text);
    let mut errors: Vec<(usize, String)> = doc.errors.iter().map(|e| (e.line, e.message.clone())).collect();
    errors.extend(problems.iter().filter(|i| i.level == parser_rs::Level::Error).map(|i| (i.line, i.message.clone())));
    if !errors.is_empty() {
        errors.sort();
        return Err(crate::source::text_errors(errors.iter().map(|(l, m)| (*l, m.as_str()))));
    }
    std::fs::write(&manifest, text).map_err(|e| format!("{}: {e}", manifest.display()))?;
    Ok(json!({"written": manifest, "problems": issues(&problems)}))
}

#[cfg(test)]
#[path = "project.test.rs"]
mod tests;

fn build(args: &Value) -> Result<Value, String> {
    let at = PathBuf::from(arg_str(args, "path")?);
    match wc3_server::build::build_at(&at, args.get("name").and_then(Value::as_str)) {
        Ok(b) => Ok(json!({"build": b.name, "out": b.out, "base": b.base, "files": b.files, "triggers": b.triggers, "script": b.script, "script_lines": b.script_lines, "notes": b.notes, "errors": b.errors})),
        Err(errors) => Ok(json!({"errors": errors})),
    }
}
