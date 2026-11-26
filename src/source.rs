//! Where a file's text comes from: a file on disk, or a file inside an
//! MPQ archive or a map (`.mpq`, `.w3x`, `.w3m`), read with `storm-rs`.
//! Text is read as UTF-8, invalid bytes replaced.

use std::path::{Path, PathBuf};

/// The text of `path`, or of the file named `path` inside `archive`.
pub fn read(path: &str, archive: Option<&str>) -> Result<String, String> {
    let bytes = read_bytes(path, archive)?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

/// The bytes of `path`, or of the file named `path` inside `archive`.
pub fn read_bytes(path: &str, archive: Option<&str>) -> Result<Vec<u8>, String> {
    Ok(match archive {
        None => std::fs::read(path).map_err(|e| format!("{path}: {e}"))?,
        Some(archive) => {
            let mpq = storm_rs::MpqArchive::open(archive).map_err(|e| format!("{archive}: {e}"))?;
            let name = storm_rs::archive::mpq_name(path);
            if !mpq.has_file(&name) {
                return Err(format!("{archive}: no {name} in it"));
            }
            mpq.read_file(&name)
                .map_err(|e| format!("{archive}: {name}: {e}"))?
        }
    })
}

/// Writes `data` to the file `path` on disk, which must exist.
pub fn write(path: &str, data: impl AsRef<[u8]>) -> Result<(), String> {
    if !Path::new(path).is_file() {
        return Err(format!("{path}: not a file on disk"));
    }
    std::fs::write(path, data).map_err(|e| format!("{path}: {e}"))
}

/// Where the game's files are looked up: `args.game` (the game's
/// folders, lowest first, as a project's `game=`) and `args.roots` (the
/// map's roots, folders or maps, as `[map] root=`), each when given;
/// what isn't given comes from the call's project ([`project`]), the way
/// the editor reads that file (`Lookup::for_file`: a map the project
/// doesn't make its map is read alone over the game). The last layer
/// that has a file wins.
pub fn lookup(args: &serde_json::Value) -> wc3_server::game::Lookup {
    let given = |key: &str| -> Option<Vec<PathBuf>> {
        args.get(key)
            .and_then(serde_json::Value::as_array)
            .map(|l| l.iter().filter_map(serde_json::Value::as_str).map(PathBuf::from).collect())
    };
    let (game, roots) = (given("game"), given("roots"));
    let from_project = match (&game, &roots) {
        (Some(_), Some(_)) => wc3_server::game::Lookup::default(),
        _ => project(args)
            .map(|(p, at)| wc3_server::game::Lookup::for_file(&p, &at))
            .unwrap_or_default(),
    };
    wc3_server::game::Lookup {
        game: game.unwrap_or(from_project.game),
        roots: roots.unwrap_or(from_project.roots),
        // A Reforged game folder's HD and locale: the project's.
        hd: args.get("hd").and_then(serde_json::Value::as_bool).unwrap_or(from_project.hd),
        de: from_project.de,
        locale: from_project.locale,
    }
}

/// The project a call is about, and the file it's about in it: the
/// project of `args.project` (its `wc3.bni`, or a file or folder in it),
/// else of the file the tool works on (`archive`, a file inside it;
/// `path`, `file`, `dir`), found as the editor finds it: the nearest
/// `wc3.bni` in its folder or above. `None` outside every project.
pub fn project(args: &serde_json::Value) -> Option<(wc3_server::project::Project, PathBuf)> {
    let arg = |key: &str| args.get(key).and_then(serde_json::Value::as_str).filter(|s| !s.is_empty());
    let at = match (arg("project"), arg("archive")) {
        (Some(p), _) => PathBuf::from(p),
        // A file inside the map, so the map is what it's in.
        (None, Some(a)) => PathBuf::from(a).join(arg("path").unwrap_or("war3map.w3i")),
        (None, None) => PathBuf::from(arg("path").or(arg("file")).or(arg("dir"))?),
    };
    let at = std::path::absolute(&at).unwrap_or(at);
    let (project, _, _) = load_project(&at)?;
    Some((project, at))
}

/// The project `at` (its `wc3.bni`, or a file or folder in it) belongs
/// to: its settings, the manifest's path and the problems of its
/// settings. `None` with no `wc3.bni` there or above, or one that can't
/// be read.
pub fn load_project(at: &Path) -> Option<(wc3_server::project::Project, PathBuf, Vec<wc3_server::project::Issue>)> {
    let manifest = if wc3_server::project::is_manifest(at) && at.is_file() {
        at.to_path_buf()
    } else {
        at.ancestors().map(|d| d.join(wc3_server::project::MANIFEST)).find(|m| m.is_file())?
    };
    let src = std::fs::read_to_string(&manifest).ok()?;
    let (project, issues) = wc3_server::project::load(manifest.parent()?, &src);
    Some((project, manifest, issues))
}

/// Where the game's files were looked for, for a message saying one
/// isn't there: the folders given, or the project's.
pub fn layers_named(args: &serde_json::Value) -> String {
    let given = |key: &str| args.get(key).is_some_and(serde_json::Value::is_array);
    match (given("game"), given("roots"), project(args)) {
        (true, true, _) => "`game` and `roots`".into(),
        (_, _, Some((p, _))) => format!("the game folders and roots of {} (and `game`, `roots` where given)", p.manifest().display()),
        _ => "`game` and `roots` (none given, and no wc3.bni of a project above the file to take them from: give them, or `project`)".into(),
    }
}

/// The schema of `roots`, for the tools that take `game`.
pub fn roots_schema() -> serde_json::Value {
    serde_json::json!({"type": "array", "items": {"type": "string"}, "description": "The map's roots over the game's folders, lowest first (as a project's [map] root=): folders or maps whose files replace the game's. Default: the project's"})
}

/// The schema of `game`, for the tools that show the game's names.
pub fn game_schema() -> serde_json::Value {
    serde_json::json!({"type": "array", "items": {"type": "string"}, "description": "The game's folders, lowest first (as a project's game=): names come from their files (UI\\WorldEditStrings.txt ...), in the game's language. Default: the project's"})
}

/// The schema of `project`, beside `game` and `roots`.
pub fn project_schema() -> serde_json::Value {
    serde_json::json!({"type": "string", "description": "A project's wc3.bni, or a file or folder in it: `game` and `roots` not given come from its settings. Default: the project of the file worked on (the nearest wc3.bni above it)"})
}

/// The names the game's files give, through `args.game` and
/// `args.roots` (none without them).
pub fn names(args: &serde_json::Value) -> wc3_server::names::Names {
    wc3_server::names::names(&lookup(args))
}

/// The file a tool reads: `args.path`, or `default` inside `args.archive`
/// when there's no path; and the archive.
pub fn file<'a>(args: &'a serde_json::Value, default: &'a str) -> Result<(&'a str, Option<&'a str>), String> {
    let archive = args.get("archive").and_then(serde_json::Value::as_str);
    match (args.get("path").and_then(serde_json::Value::as_str), archive) {
        (Some(p), _) => Ok((p, archive)),
        (None, Some(_)) => Ok((default, archive)),
        (None, None) => Err("`path` is missing".into()),
    }
}

/// The bytes of the file named `name` beside `path` (the same folder, or
/// the same folder inside `archive`).
pub fn sibling(path: &str, archive: Option<&str>, name: &str) -> Option<Vec<u8>> {
    let beside = match archive {
        Some(_) => match path.rfind(['\\', '/']) {
            Some(i) => format!("{}{name}", &path[..=i]),
            None => name.to_string(),
        },
        None => Path::new(path).with_file_name(name).to_string_lossy().into_owned(),
    };
    read_bytes(&beside, archive).ok()
}

/// The errors of a text form, as one message.
pub fn text_errors<'a>(errors: impl IntoIterator<Item = (usize, &'a str)>) -> String {
    let lines: Vec<String> = errors.into_iter().map(|(l, m)| format!("line {l}: {m}")).collect();
    format!("nothing written:\n{}", lines.join("\n"))
}

/// The text argument `key`.
pub fn arg_str<'a>(args: &'a serde_json::Value, key: &str) -> Result<&'a str, String> {
    args.get(key).and_then(serde_json::Value::as_str).ok_or(format!("`{key}` is missing"))
}

/// The schemas of `path` (for `default`) and `archive`.
pub fn path_schema(default: &str, what: &str) -> serde_json::Value {
    serde_json::json!({"type": "string", "description": format!("{what}: {default} on disk, or with `archive` its name inside ({default} when left out)")})
}

pub fn archive_schema() -> serde_json::Value {
    serde_json::json!({"type": "string", "description": "A map (.w3x, .w3m) to read the file from"})
}
