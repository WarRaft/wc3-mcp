//! The txt2 tools, on `txt2-rs`: XGM's markup (articles, resource
//! pages). A text is a file on disk (`path`), one inside a map
//! (`archive`), or given as is (`text`). Big articles are read by their
//! outline and a section at a time; edits are the model's own, to the
//! file.

use serde_json::{Value, json};
use txt2_rs::{Block, BlockKind, Media, plain};

use crate::source::archive_schema;

/// The syntax, from `txt2-rs`'s README: its "Syntax" section.
fn syntax() -> &'static str {
    let readme = include_str!("../../txt2-rs/README.md");
    let start = readme.find("## The format").unwrap_or(0);
    let end = readme[start..].find("\n## The preview").map_or(readme.len(), |e| start + e);
    &readme[start..end]
}

/// Every tool, for `tools/list`.
pub fn tools() -> Value {
    let source = || {
        json!({
            "path": {"type": "string", "description": "A .txt2 file on disk, or with `archive` its name inside"},
            "archive": archive_schema(),
            "text": {"type": "string", "description": "Or the text itself"},
        })
    };
    let with = |extra: Value| {
        let mut props = source();
        props.as_object_mut().expect("an object").extend(extra.as_object().expect("an object").clone());
        props
    };
    json!([
        {
            "name": "txt2_syntax",
            "description": "txt2, XGM's markup (xgm.guru articles and resource pages): every inline and block construct, as this server reads it. Read it before writing txt2.",
            "inputSchema": {"type": "object", "properties": {}},
        },
        {
            "name": "txt2_check",
            "description": "Checks a txt2 text: blocks without their closing line (((кат without )), >> without <<, an open ``` or {{), closings without an opening. Each with its line and column.",
            "inputSchema": {"type": "object", "properties": source()},
        },
        {
            "name": "txt2_outline",
            "description": "The structure of a txt2 text, each part with its first and last line: headings, spoilers ((кат, >>) with their titles, quotes, aligned blocks, layouts and columns, code (language, lines), tables (rows, columns), lists, images, galleries, videos, polls, tiles, progress bars; `paragraphs: true` adds paragraphs. Read a part's text with txt2_get or the lines.",
            "inputSchema": {"type": "object", "properties": with(json!({"paragraphs": {"type": "boolean"}}))},
        },
        {
            "name": "txt2_get",
            "description": "The source of one part of a txt2 text, with its lines: by `title` (a spoiler's, heading's, poll's or tile's; exact, else the first that contains it, case ignored) or by `line` (the outermost part starting there). A heading's part runs to the next heading of its level or higher.",
            "inputSchema": {"type": "object", "properties": with(json!({
                "title": {"type": "string"},
                "line": {"type": "integer", "minimum": 1},
            }))},
        },
        {
            "name": "txt2_html",
            "description": "A txt2 text as HTML, the way the editor's preview shows it: a fragment (each block with data-line), or with `page: true` a whole page with its stylesheet (FontAwesome from cdnjs). `out`: write it to that file instead of answering with it.",
            "inputSchema": {"type": "object", "properties": with(json!({
                "page": {"type": "boolean"},
                "out": {"type": "string", "description": "A file to write the HTML to (created or replaced)"},
            }))},
        },
    ])
}

/// Runs the tool `name`, if it's one of these.
pub fn call(name: &str, args: &Value) -> Option<Result<Value, String>> {
    Some(match name {
        "txt2_syntax" => Ok(Value::String(syntax().to_string())),
        "txt2_check" => read(args).map(|t| check(&t)),
        "txt2_outline" => read(args).map(|t| {
            let doc = txt2_rs::parse(&t);
            let paragraphs = args.get("paragraphs").and_then(Value::as_bool).unwrap_or(false);
            json!({"lines": parser_rs::lines::line_count(&t), "errors": doc.errors.len(), "outline": outline(&doc.blocks, paragraphs)})
        }),
        "txt2_get" => read(args).and_then(|t| get(&t, args)),
        "txt2_html" => read(args).and_then(|t| html(&t, args)),
        _ => return None,
    })
}

fn read(args: &Value) -> Result<String, String> {
    if let Some(text) = args.get("text").and_then(Value::as_str) {
        return Ok(text.to_string());
    }
    let path = args.get("path").and_then(Value::as_str).ok_or("`path` or `text` is missing")?;
    crate::source::read(path, args.get("archive").and_then(Value::as_str))
}

fn check(text: &str) -> Value {
    let doc = txt2_rs::parse(text);
    let errors: Vec<Value> = doc.errors.iter().map(|e| json!({"line": e.line, "col": e.col, "message": e.message})).collect();
    json!({"ok": errors.is_empty(), "errors": errors})
}

/// A part of the outline, and the blocks in it.
fn node(b: &Block, paragraphs: bool) -> Option<Value> {
    let mut v = json!({"line": b.line, "end": b.end});
    let mut children: &[Block] = &[];
    let kind = match &b.kind {
        BlockKind::Paragraph(lines) if paragraphs => {
            let first = lines.first().map(|l| plain(l)).unwrap_or_default();
            v["text"] = json!(first.chars().take(80).collect::<String>());
            "paragraph"
        }
        BlockKind::Paragraph(_) | BlockKind::Rule => return None,
        BlockKind::Heading { level, text } => {
            v["level"] = json!(level);
            v["title"] = json!(plain(text).trim());
            "heading"
        }
        BlockKind::Spoiler { title, blocks } => {
            v["title"] = json!(title.as_ref().map(|t| plain(t).trim().to_string()));
            children = blocks;
            "spoiler"
        }
        BlockKind::Quote(blocks) => {
            children = blocks;
            "quote"
        }
        BlockKind::Aligned { align, blocks } => {
            v["align"] = json!(format!("{align:?}").to_lowercase());
            children = blocks;
            "aligned"
        }
        BlockKind::Layout(blocks) => {
            children = blocks;
            "layout"
        }
        BlockKind::Column { width, blocks } => {
            v["width"] = json!(width);
            children = blocks;
            "column"
        }
        BlockKind::Code { lang, text } => {
            v["lang"] = json!(lang);
            v["lines"] = json!(parser_rs::lines::line_count(&text));
            "code"
        }
        BlockKind::Raw(_) => "raw",
        BlockKind::List(l) => {
            v["items"] = json!(l.items.len());
            v["ordered"] = json!(l.ordered);
            "list"
        }
        BlockKind::Table(t) => {
            v["rows"] = json!(t.rows.len());
            v["columns"] = json!(t.rows.iter().map(Vec::len).max().unwrap_or(0));
            v["header"] = json!(t.header);
            "table"
        }
        BlockKind::Progress(..) => "progress",
        BlockKind::Media(e) => {
            v["url"] = json!(e.url);
            match e.kind {
                Media::Image => "image",
                Media::Video => "video",
                Media::Audio => "audio",
                Media::Gist => "gist",
            }
        }
        BlockKind::Gallery(list) => {
            v["images"] = json!(list.iter().map(|e| e.url.clone()).collect::<Vec<_>>());
            "gallery"
        }
        BlockKind::Poll(p) => {
            v["title"] = json!(p.title);
            v["options"] = json!(p.options.len());
            "poll"
        }
        BlockKind::Tile(t) => {
            v["title"] = json!(t.title);
            v["url"] = json!(t.url);
            "tile"
        }
    };
    v["kind"] = json!(kind);
    let inner = outline(children, paragraphs);
    if !inner.is_empty() {
        v["parts"] = json!(inner);
    }
    Some(v)
}

fn outline(blocks: &[Block], paragraphs: bool) -> Vec<Value> {
    blocks.iter().filter_map(|b| node(b, paragraphs)).collect()
}

/// A block's title, if it has one.
fn title(b: &Block) -> Option<String> {
    Some(match &b.kind {
        BlockKind::Heading { text, .. } => plain(text),
        BlockKind::Spoiler { title: Some(t), .. } => plain(t),
        BlockKind::Poll(p) => p.title.clone(),
        BlockKind::Tile(t) => t.title.clone(),
        _ => return None,
    }
    .trim()
    .to_string())
}

/// Every block, with its siblings' list and its place in it, depth first.
fn walk<'a>(blocks: &'a [Block], out: &mut Vec<(&'a [Block], usize)>) {
    for (i, b) in blocks.iter().enumerate() {
        out.push((blocks, i));
        let inner: &[Block] = match &b.kind {
            BlockKind::Spoiler { blocks, .. }
            | BlockKind::Quote(blocks)
            | BlockKind::Aligned { blocks, .. }
            | BlockKind::Layout(blocks)
            | BlockKind::Column { blocks, .. } => blocks,
            _ => &[],
        };
        walk(inner, out);
    }
}

fn get(text: &str, args: &Value) -> Result<Value, String> {
    let doc = txt2_rs::parse(text);
    let mut all = Vec::new();
    walk(&doc.blocks, &mut all);
    let found = if let Some(want) = args.get("title").and_then(Value::as_str) {
        let want = want.trim().to_lowercase();
        let titled = |exact: bool| {
            all.iter().copied().find(|(list, i)| {
                title(&list[*i]).is_some_and(|t| if exact { t.to_lowercase() == want } else { t.to_lowercase().contains(&want) })
            })
        };
        titled(true).or_else(|| titled(false)).ok_or(format!("no part titled {want:?}: see txt2_outline"))?
    } else if let Some(line) = args.get("line").and_then(Value::as_u64) {
        all.iter().copied().find(|(list, i)| list[*i].line == line as usize).ok_or(format!("no part starts at line {line}: see txt2_outline"))?
    } else {
        return Err("give `title` or `line`".into());
    };
    let (list, i) = found;
    let b = &list[i];
    let mut end = b.end;
    // A heading's part: to the next heading of its level or higher.
    if let BlockKind::Heading { level, .. } = b.kind {
        let next = list[i + 1..].iter().find(|n| matches!(n.kind, BlockKind::Heading { level: l, .. } if l <= level));
        end = match next {
            Some(n) => n.line - 1,
            None => list.last().map_or(b.end, |l| l.end),
        };
    }
    let lines: Vec<&str> = parser_rs::lines::split(&text).collect();
    let part: Vec<&str> = lines[b.line - 1..end.min(lines.len())].to_vec();
    let part = part.join("\n");
    Ok(json!({"line": b.line, "end": end, "title": title(b), "text": part}))
}

fn html(text: &str, args: &Value) -> Result<Value, String> {
    let doc = txt2_rs::parse(text);
    let page = args.get("page").and_then(Value::as_bool).unwrap_or(false);
    let title = args.get("path").and_then(Value::as_str).map_or("txt2", |p| p.rsplit(['/', '\\']).next().unwrap_or(p));
    let out = if page { txt2_rs::html::page(&doc, title) } else { txt2_rs::html::render(&doc) };
    match args.get("out").and_then(Value::as_str) {
        Some(file) => {
            std::fs::write(file, &out).map_err(|e| format!("{file}: {e}"))?;
            Ok(json!({"written": file, "bytes": out.len(), "errors": doc.errors.len()}))
        }
        None => Ok(Value::String(out)),
    }
}

#[cfg(test)]
#[path = "txt2.test.rs"]
mod tests;
