//! The placement tools, on `doo-rs`: `war3map.doo` (the doodads and
//! destructables a map places, and the doodads its cliffs place) and
//! `war3mapUnits.doo` (its units, buildings, items and start locations);
//! which one a file is, its name says. A file is read from disk (`path`)
//! or from inside a map (`archive`, `war3map.doo` by default); changes
//! are written to files on disk.
//!
//! A thing placed is named by its `index`, its place in the file (from
//! 0). Its text is the editor's text form: a `[doodad]` or `[unit]`
//! section; `doo_get` gives it, `doo_set` and `doo_new` take it. The
//! layout (1.32 added skins) comes from the `war3map.w3i` beside the
//! file, else from what the file reads as.

use serde_json::{Value, json};

use doo_rs::{Doodads, FileKind, Layout, Random, Units};

use crate::source::{arg_str, archive_schema, file, path_schema};

/// The most things `doo_list` gives at once, by default.
const LIMIT: u64 = 200;

/// Every tool, for `tools/list`.
pub fn tools() -> Value {
    let path = path_schema("war3map.doo", "The placement file (war3map.doo doodads, war3mapUnits.doo units)");
    let disk = json!({"type": "string", "description": "The war3map.doo or war3mapUnits.doo on disk"});
    let indexes = json!({"type": "array", "items": {"type": "integer", "minimum": 0}, "description": "Places in the file, from 0"});
    json!([
        {
            "name": "doo_list",
            "description": "What a map places: war3map.doo (doodads and destructables) or war3mapUnits.doo (units, buildings, items, start locations `sloc`). Each thing with its index (its place in the file), type id, position, angle (radians), scale; doodads with flags (0 invisible, 1 visible non-solid, 2 normal) and life; units with owner (0 = Player 1), hp, mana (-1 default), level, items, abilities, how it's random, number. Filters: `id` (type ids), `player`, a rectangle (left, bottom, right, top): what stands at a point. `offset` and `limit` (200) page through. With the game's files (`game`, `roots`, the project's by default), each thing's name and its model's path too, as the World Editor finds them (the game's tables, the map's object data over them).",
            "inputSchema": {"type": "object", "properties": {
                "path": path, "archive": archive_schema(),
                "game": crate::source::game_schema(), "roots": crate::source::roots_schema(), "project": crate::source::project_schema(),
                "id": {"type": "array", "items": {"type": "string"}, "description": "Type ids to keep (hfoo, LTlt ...)"},
                "player": {"type": "integer", "description": "Units of this owner only (0 = Player 1)"},
                "left": {"type": "number"}, "bottom": {"type": "number"}, "right": {"type": "number"}, "top": {"type": "number"},
                "offset": {"type": "integer", "minimum": 0}, "limit": {"type": "integer", "minimum": 1},
            }},
        },
        {
            "name": "doo_get",
            "description": "The text of things placed, by index: the file's header section and a [doodad] or [unit] section each (id, variation, x, y, z, angle, scale, skin, flags, player, hp, mana, item drops `set=chance,id,...`, gold, level, stats, `item=slot,id`, `ability=id,autocast,level`, `random=...`, color, waygate, number). Edit a section and give it to doo_set.",
            "inputSchema": {"type": "object", "properties": {"path": path, "archive": archive_schema(), "index": indexes}, "required": ["index"]},
        },
        {
            "name": "doo_set",
            "description": "Replaces the thing at `index` of a placement file on disk with `text`: one [doodad] or [unit] section, as doo_get gives it. A field left out takes World Editor's default.",
            "inputSchema": {"type": "object", "properties": {
                "path": disk, "index": {"type": "integer", "minimum": 0},
                "text": {"type": "string", "description": "One [doodad] or [unit] section"},
            }, "required": ["path", "index", "text"]},
        },
        {
            "name": "doo_new",
            "description": "Adds things to the end of a placement file on disk: `text` holds a [doodad] or [unit] section each (at least id, x, y; the rest take World Editor's defaults). Give each a `number` no other has: World Editor's number for it (gg_unit_hfoo_0012 in the script).",
            "inputSchema": {"type": "object", "properties": {"path": disk, "text": {"type": "string"}}, "required": ["path", "text"]},
        },
        {
            "name": "doo_delete",
            "description": "Removes things, by index, from a placement file on disk. The things after them move up: indexes change.",
            "inputSchema": {"type": "object", "properties": {"path": disk, "index": indexes}, "required": ["path", "index"]},
        },
    ])
}

/// Runs the tool `name`, if it's one of these.
pub fn call(name: &str, args: &Value) -> Option<Result<Value, String>> {
    Some(match name {
        "doo_list" => open(args).and_then(|f| list(&f, args)),
        "doo_get" => open(args).and_then(|f| get(&f, args)),
        "doo_set" | "doo_new" | "doo_delete" => change(name, args),
        _ => return None,
    })
}

/// Either file.
enum Placed {
    Doodads(Doodads),
    Units(Units),
}

/// The file at `args.path`, or inside `args.archive`, in the layout the
/// `war3map.w3i` beside it says (else the one it reads in).
fn open(args: &Value) -> Result<Placed, String> {
    let (path, archive) = file(args, "war3map.doo")?;
    let bytes = crate::source::read_bytes(path, archive)?;
    let layout = crate::source::sibling(path, archive, "war3map.w3i").as_deref().and_then(Layout::from_w3i);
    read(path, &bytes, layout)
}

fn read(path: &str, bytes: &[u8], layout: Option<Layout>) -> Result<Placed, String> {
    let err = |e: doo_rs::BinError| format!("{path}: {e}");
    Ok(match (FileKind::of(path), layout) {
        (FileKind::Doodads, Some(l)) => Placed::Doodads(doo_rs::parse_doodads(bytes, l).map_err(err)?),
        (FileKind::Doodads, None) => Placed::Doodads(doo_rs::parse_doodads_any(bytes).map_err(err)?),
        (FileKind::Units, Some(l)) => Placed::Units(doo_rs::parse_units(bytes, l).map_err(err)?),
        (FileKind::Units, None) => Placed::Units(doo_rs::parse_units_any(bytes).map_err(err)?),
    })
}

fn number(v: f32) -> Value {
    v.to_string().parse::<f64>().ok().and_then(serde_json::Number::from_f64).map_or(Value::Null, Value::Number)
}

fn place(p: &doo_rs::Placement) -> serde_json::Map<String, Value> {
    let mut m = serde_json::Map::new();
    m.insert("id".into(), json!(p.id.to_string()));
    m.insert("variation".into(), json!(p.variation));
    m.insert("x".into(), number(p.x));
    m.insert("y".into(), number(p.y));
    m.insert("z".into(), number(p.z));
    m.insert("angle".into(), number(p.angle));
    m.insert("scale".into(), json!([number(p.scale[0]), number(p.scale[1]), number(p.scale[2])]));
    m
}

fn list(f: &Placed, args: &Value) -> Result<Value, String> {
    let ids: Option<Vec<String>> =
        args.get("id").and_then(Value::as_array).map(|l| l.iter().filter_map(Value::as_str).map(str::to_string).collect());
    let player = args.get("player").and_then(Value::as_u64);
    let bound = |k: &str, d: f64| args.get(k).and_then(Value::as_f64).unwrap_or(d);
    let (l, b, r, t) = (bound("left", f64::MIN), bound("bottom", f64::MIN), bound("right", f64::MAX), bound("top", f64::MAX));
    let keep = |p: &doo_rs::Placement| {
        ids.as_ref().is_none_or(|ids| ids.iter().any(|i| *i == p.id.to_string()))
            && (l..=r).contains(&(p.x as f64))
            && (b..=t).contains(&(p.y as f64))
    };
    let offset = args.get("offset").and_then(Value::as_u64).unwrap_or(0) as usize;
    let limit = args.get("limit").and_then(Value::as_u64).unwrap_or(LIMIT) as usize;
    let (items, total, head): (Vec<(usize, Value)>, usize, Value) = match f {
        Placed::Doodads(d) => {
            let items = d.doodads.iter().enumerate().filter(|(_, x)| keep(&x.placement) && player.is_none()).map(|(i, x)| {
                let mut m = place(&x.placement);
                m.insert("flags".into(), json!(x.flags));
                m.insert("life".into(), json!(x.life));
                m.insert("number".into(), json!(x.number));
                (i, Value::Object(m))
            });
            let items: Vec<_> = items.collect();
            (items, d.doodads.len(), json!({"file": "doodads", "version": d.version, "subversion": d.subversion, "layout": d.layout.describe(), "cliff_doodads": d.specials.len()}))
        }
        Placed::Units(u) => {
            let items = u
                .units
                .iter()
                .enumerate()
                .filter(|(_, x)| keep(&x.placement) && player.is_none_or(|p| u64::from(x.player) == p))
                .map(|(i, x)| {
                    let mut m = place(&x.placement);
                    m.insert("player".into(), json!(x.player));
                    m.insert("hp".into(), json!(x.hp));
                    m.insert("mana".into(), json!(x.mana));
                    m.insert("level".into(), json!(x.level));
                    m.insert("items".into(), json!(x.items.iter().map(|(s, id)| json!([s, id.to_string()])).collect::<Vec<_>>()));
                    m.insert("abilities".into(), json!(x.abilities.len()));
                    let random = match &x.random {
                        Random::Any { level, class } => json!({"any": {"level": level, "class": class}}),
                        Random::Group { group, position } => json!({"group": group, "position": position}),
                        Random::Table(t) => json!({"table": t.iter().map(|(id, c)| json!([id.to_string(), c])).collect::<Vec<_>>()}),
                        Random::None(_) => Value::Null,
                    };
                    if !random.is_null() {
                        m.insert("random".into(), random);
                    }
                    m.insert("number".into(), json!(x.number));
                    (i, Value::Object(m))
                })
                .collect();
            (items, u.units.len(), json!({"file": "units", "version": u.version, "subversion": u.subversion, "layout": u.layout.describe()}))
        }
    };
    let matched = items.len();
    // Names and models, with the game's files.
    let lookup = crate::source::lookup(args);
    let known = (!lookup.is_empty()).then(|| {
        let (game, _) = lookup.open();
        (wc3_editor::types::Types::load(&game), wc3_server::names::with_objects(&lookup))
    });
    let units = matches!(f, Placed::Units(_));
    let page: Vec<Value> = items
        .into_iter()
        .skip(offset)
        .take(limit)
        .map(|(i, mut v)| {
            v["index"] = json!(i);
            if let Some((types, names)) = &known {
                let id = v["id"].as_str().unwrap_or_default().to_string();
                let variation = v["variation"].as_u64().unwrap_or(0) as u32;
                if let Some(m) = parser_rs::Rawcode::parse(&id).and_then(|code| wc3_editor::placed::model_of(types, code, variation, units)) {
                    v["name"] = json!(names.objects.get(m.kind, &id).map(|n| n.shown()));
                    v["model"] = json!(m.paths.as_ref().and_then(|p| p.first()));
                }
            }
            v
        })
        .collect();
    let mut out = head;
    out["total"] = json!(total);
    out["matched"] = json!(matched);
    out["offset"] = json!(offset);
    out["things"] = json!(page);
    Ok(out)
}

fn indexes(args: &Value, len: usize) -> Result<Vec<usize>, String> {
    let list: Vec<usize> = match args.get("index") {
        Some(Value::Number(n)) => vec![n.as_u64().ok_or("`index` is 0 or more")? as usize],
        Some(Value::Array(l)) => l.iter().map(|v| v.as_u64().map(|n| n as usize).ok_or("an index is 0 or more")).collect::<Result<_, _>>()?,
        _ => return Err("`index` is missing".into()),
    };
    match list.iter().find(|&&i| i >= len) {
        Some(i) => Err(format!("no index {i}: the file has {len}")),
        None => Ok(list),
    }
}

fn get(f: &Placed, args: &Value) -> Result<Value, String> {
    Ok(Value::String(match f {
        Placed::Doodads(d) => {
            let pick = indexes(args, d.doodads.len())?;
            let doodads = pick.iter().map(|&i| d.doodads[i].clone()).collect();
            doo_rs::text::doodads_to_text(&Doodads { doodads, specials: Vec::new(), ..d.clone() })
        }
        Placed::Units(u) => {
            let pick = indexes(args, u.units.len())?;
            let units = pick.iter().map(|&i| u.units[i].clone()).collect();
            doo_rs::text::units_to_text(&Units { units, ..u.clone() })
        }
    }))
}

/// Reads sections in `text` as `f`'s kind and versions; their errors
/// with lines of `text`.
fn sections(f: &Placed, text: &str) -> Result<Placed, String> {
    let (head, skip) = match f {
        Placed::Doodads(d) => (format!("[doodads]\nversion={}\nsubversion={}\nspecial_version=0\n", d.version, d.subversion), 4),
        Placed::Units(u) => (format!("[units]\nversion={}\nsubversion={}\n", u.version, u.subversion), 3),
    };
    let text = head + text;
    let errors = |e: Vec<parser_rs::ParseError>| {
        crate::source::text_errors(e.iter().map(|e| (e.line.saturating_sub(skip), e.message.as_str())))
    };
    Ok(match f {
        Placed::Doodads(d) => {
            let p = doo_rs::text::doodads_from_text(&text, d.layout);
            Placed::Doodads(p.file.ok_or_else(|| errors(p.errors))?)
        }
        Placed::Units(u) => {
            let p = doo_rs::text::units_from_text(&text, u.layout);
            Placed::Units(p.file.ok_or_else(|| errors(p.errors))?)
        }
    })
}

fn change(name: &str, args: &Value) -> Result<Value, String> {
    let path = arg_str(args, "path")?;
    let bytes = std::fs::read(path).map_err(|e| format!("{path}: {e}"))?;
    let layout = crate::source::sibling(path, None, "war3map.w3i").as_deref().and_then(Layout::from_w3i);
    let mut f = read(path, &bytes, layout)?;
    let added = match name {
        "doo_set" | "doo_new" => Some(sections(&f, arg_str(args, "text")?)?),
        _ => None,
    };
    let (len, what) = match (&mut f, added) {
        (Placed::Doodads(d), Some(Placed::Doodads(new))) => {
            if !new.specials.is_empty() {
                return Err("nothing written: [special] (a cliff's doodad) isn't placed this way".into());
            }
            if name == "doo_set" {
                let [one] = <[_; 1]>::try_from(new.doodads).map_err(|_| "nothing written: `text` is one [doodad] section")?;
                let i = indexes(args, d.doodads.len())?[0];
                d.doodads[i] = one;
            } else {
                d.doodads.extend(new.doodads);
            }
            (d.doodads.len(), "doodads")
        }
        (Placed::Units(u), Some(Placed::Units(new))) => {
            if name == "doo_set" {
                let [one] = <[_; 1]>::try_from(new.units).map_err(|_| "nothing written: `text` is one [unit] section")?;
                let i = indexes(args, u.units.len())?[0];
                u.units[i] = one;
            } else {
                u.units.extend(new.units);
            }
            (u.units.len(), "units")
        }
        (Placed::Doodads(d), None) => {
            let mut gone = indexes(args, d.doodads.len())?;
            gone.sort_unstable();
            gone.dedup();
            for i in gone.into_iter().rev() {
                d.doodads.remove(i);
            }
            (d.doodads.len(), "doodads")
        }
        (Placed::Units(u), None) => {
            let mut gone = indexes(args, u.units.len())?;
            gone.sort_unstable();
            gone.dedup();
            for i in gone.into_iter().rev() {
                u.units.remove(i);
            }
            (u.units.len(), "units")
        }
        _ => unreachable!("sections reads the file's kind"),
    };
    let out = match &f {
        Placed::Doodads(d) => d.to_bytes(),
        Placed::Units(u) => u.to_bytes(),
    }
    .map_err(|e| format!("nothing written: {e}"))?;
    crate::source::write(path, out)?;
    let mut out = json!({"written": path});
    out[what] = json!(len);
    Ok(out)
}

#[cfg(test)]
#[path = "doo.test.rs"]
mod tests;
