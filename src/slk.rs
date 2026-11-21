//! The SLK tools, on `slk-rs`: a table's columns, its rows by query, one
//! cell to read or write, and its problems.
//!
//! Rows and columns count from 1. Row 1 holds the field names in the
//! game's tables, so a column is named by its field (`goldcost`), by its
//! letter (`H`) or by its number (`8`). A table is read from a file on
//! disk (`path`), or from a file inside an MPQ archive or a map (`path`
//! inside `archive`); writing is to files on disk only.

use serde_json::{Map, Value, json};
use slk_rs::{Document, Table};

/// Every tool, for `tools/list`.
pub fn tools() -> Value {
    let source = json!({
        "path": {"type": "string", "description": "The .slk file; with `archive`, its name inside it (Units\\UnitBalance.slk)"},
        "archive": {"type": "string", "description": "An MPQ archive or a map (.mpq, .w3x, .w3m) to read `path` from"},
    });
    let with = |extra: Value| {
        let mut props = source.as_object().expect("an object").clone();
        props.extend(extra.as_object().expect("an object").clone());
        Value::Object(props)
    };
    let column = json!({"description": "The column: its name in row 1 (goldcost), its letter (H) or its number (8)", "type": ["string", "integer"]});
    json!([
        {
            "name": "slk_info",
            "description": "A SLK table's size and columns: each column's number, letter and name (row 1), and how many problems the file has.",
            "inputSchema": {"type": "object", "properties": source, "required": ["path"]},
        },
        {
            "name": "slk_read",
            "description": "Rows of a SLK table, as JSON objects keyed by the column names in row 1 (with the row number as `row`). Pick columns, keep rows whose cells equal given values (`where`) or that contain a text anywhere (`contains`), and page through them (`offset`, `limit`). Numbers and TRUE/FALSE come as JSON numbers and booleans; empty cells as null. `total` is how many rows matched.",
            "inputSchema": {
                "type": "object",
                "properties": with(json!({
                    "columns": {"type": "array", "items": {"type": ["string", "integer"]}, "description": "The columns to give (names, letters or numbers); all when left out"},
                    "where": {"type": "object", "description": "Column → text: keep rows whose cell shows exactly that text ({\"unitBalanceID\": \"hfoo\"})", "additionalProperties": {"type": "string"}},
                    "contains": {"type": "string", "description": "Keep rows with a cell that contains this text, in any case"},
                    "offset": {"type": "integer", "minimum": 0, "description": "Rows to skip, of those that match (default 0)"},
                    "limit": {"type": "integer", "minimum": 1, "maximum": 2000, "description": "Rows to give at most (default 100)"},
                })),
                "required": ["path"],
            },
        },
        {
            "name": "slk_get",
            "description": "One cell of a SLK table: its text, its kind (number, string, bool, error), and its formula and comment if it has them.",
            "inputSchema": {
                "type": "object",
                "properties": with(json!({"row": {"type": "integer", "minimum": 1}, "column": column})),
                "required": ["path", "row", "column"],
            },
        },
        {
            "name": "slk_set",
            "description": "Writes one cell of a SLK file on disk, changing only what it must in the text. The value is read like typed input: TRUE/FALSE, an error like #VALUE!, a number, or else text; a cell that holds text keeps holding text, and \"quoted\" input is always text. null or an empty string clears the cell. A row or column past the table grows it.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "The .slk file on disk"},
                    "row": {"type": "integer", "minimum": 1},
                    "column": column,
                    "value": {"type": ["string", "number", "boolean", "null"], "description": "The new value; null clears the cell"},
                },
                "required": ["path", "row", "column", "value"],
            },
        },
        {
            "name": "slk_lint",
            "description": "A SLK file's problems: parse errors (with line and column) and lint warnings (a cell set twice, a cell outside the size the B record gives).",
            "inputSchema": {"type": "object", "properties": source, "required": ["path"]},
        },
    ])
}

/// Runs the tool `name`; an error is a message for the model.
pub fn call(name: &str, args: &Value) -> Result<Value, String> {
    match name {
        "slk_info" => info(&open(args)?),
        "slk_read" => read(&open(args)?, args),
        "slk_get" => get(&open(args)?, args),
        "slk_set" => set(args),
        "slk_lint" => lint(&open(args)?),
        _ => Err(format!("unknown tool {name}")),
    }
}

/// A parsed table.
struct Sheet {
    doc: Document,
    table: Table,
}

fn open(args: &Value) -> Result<Sheet, String> {
    let path = string(args, "path")?;
    let archive = args.get("archive").and_then(Value::as_str);
    let text = crate::source::read(path, archive)?;
    let doc = slk_rs::parse(&text);
    let table = Table::new(&doc);
    Ok(Sheet { doc, table })
}

fn string<'a>(args: &'a Value, key: &str) -> Result<&'a str, String> {
    args.get(key)
        .and_then(Value::as_str)
        .ok_or(format!("`{key}` is missing"))
}

/// `A`, `B`, ..., `Z`, `AA`, ...
fn letter(mut c: u32) -> String {
    let mut s = String::new();
    while c > 0 {
        s.insert(0, (b'A' + ((c - 1) % 26) as u8) as char);
        c = (c - 1) / 26;
    }
    s
}

/// A column letter's number: `H` is 8.
fn from_letter(s: &str) -> Option<u32> {
    if s.is_empty() || s.len() > 3 || !s.bytes().all(|b| b.is_ascii_uppercase()) {
        return None;
    }
    Some(s.bytes().fold(0, |n, b| n * 26 + u32::from(b - b'A' + 1)))
}

/// The column `spec` names: a number, a name in row 1 (exactly, then in
/// any case), or a letter.
fn column(table: &Table, spec: &Value) -> Result<u32, String> {
    if let Some(n) = spec.as_u64() {
        return u32::try_from(n)
            .ok()
            .filter(|&n| n > 0)
            .ok_or(format!("no column {n}"));
    }
    let s = spec
        .as_str()
        .ok_or("a column is a name, a letter or a number")?;
    if let Ok(n) = s.parse::<u32>()
        && n > 0
    {
        return Ok(n);
    }
    let names = |eq: &dyn Fn(&str) -> bool| (1..=table.cols()).find(|&c| eq(&table.text(1, c)));
    names(&|n| n == s)
        .or_else(|| names(&|n| n.eq_ignore_ascii_case(s)))
        .or_else(|| from_letter(s))
        .ok_or(format!(
            "no column '{s}': name it by its name in row 1, its letter or its number"
        ))
}

/// A cell's value as JSON: numbers and booleans as such, text as strings,
/// nothing as null.
fn json_value(table: &Table, r: u32, c: u32) -> Value {
    match table.get(r, c).map(|cell| &cell.value) {
        None | Some(slk_rs::Value::Empty) => Value::Null,
        Some(slk_rs::Value::Number(n)) => match n.parse::<i64>() {
            Ok(i) => json!(i),
            Err(_) => n
                .parse::<f64>()
                .ok()
                .and_then(serde_json::Number::from_f64)
                .map_or(json!(n), Value::Number),
        },
        Some(slk_rs::Value::Bool(b)) => json!(b),
        Some(v) => json!(v.display()),
    }
}

/// Each column's key in a row object: its name in row 1, or its letter
/// when it has none or the name is taken.
fn keys(table: &Table) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    (1..=table.cols())
        .map(|c| {
            let name = table.text(1, c);
            if !name.is_empty() && name != "row" && seen.insert(name.clone()) {
                name
            } else {
                letter(c)
            }
        })
        .collect()
}

fn info(sheet: &Sheet) -> Result<Value, String> {
    let t = &sheet.table;
    let columns: Vec<Value> = (1..=t.cols())
        .map(|c| json!({"column": c, "letter": letter(c), "name": t.text(1, c)}))
        .collect();
    let lint = slk_rs::lint(&sheet.doc).len();
    Ok(json!({
        "rows": t.rows(),
        "columns": t.cols(),
        "bounds": t.bounds.map(|(r, c)| json!({"rows": r, "columns": c})),
        "problems": sheet.doc.errors.len() + lint,
        "column_names": columns,
    }))
}

fn read(sheet: &Sheet, args: &Value) -> Result<Value, String> {
    let t = &sheet.table;
    let keys = keys(t);
    let picked: Vec<u32> = match args.get("columns").and_then(Value::as_array) {
        Some(list) => list
            .iter()
            .map(|c| column(t, c))
            .collect::<Result<_, _>>()?,
        None => (1..=t.cols()).collect(),
    };
    let mut wanted = Vec::new();
    if let Some(map) = args.get("where").and_then(Value::as_object) {
        for (col, value) in map {
            let text = match value {
                Value::String(s) => s.clone(),
                other => other.to_string(),
            };
            wanted.push((column(t, &json!(col))?, text));
        }
    }
    let needle = args
        .get("contains")
        .and_then(Value::as_str)
        .map(str::to_lowercase);
    let offset = args.get("offset").and_then(Value::as_u64).unwrap_or(0) as usize;
    let limit = args
        .get("limit")
        .and_then(Value::as_u64)
        .unwrap_or(100)
        .clamp(1, 2000) as usize;

    let matches = (2..=t.rows()).filter(|&r| {
        wanted.iter().all(|(c, text)| t.text(r, *c) == *text)
            && needle
                .as_ref()
                .is_none_or(|n| (1..=t.cols()).any(|c| t.text(r, c).to_lowercase().contains(n)))
    });
    let mut total = 0;
    let mut rows = Vec::new();
    for r in matches {
        if total >= offset && rows.len() < limit {
            let mut obj = Map::new();
            obj.insert("row".into(), json!(r));
            for &c in &picked {
                let key = keys
                    .get(c as usize - 1)
                    .cloned()
                    .unwrap_or_else(|| letter(c));
                obj.insert(key, json_value(t, r, c));
            }
            rows.push(Value::Object(obj));
        }
        total += 1;
    }
    Ok(json!({"total": total, "offset": offset, "rows": rows}))
}

fn get(sheet: &Sheet, args: &Value) -> Result<Value, String> {
    let t = &sheet.table;
    let r = args
        .get("row")
        .and_then(Value::as_u64)
        .filter(|&r| r > 0)
        .ok_or("`row` is a number from 1")? as u32;
    let c = column(t, args.get("column").ok_or("`column` is missing")?)?;
    let cell = t.get(r, c);
    let kind = match cell.map(|c| &c.value) {
        None | Some(slk_rs::Value::Empty) => "empty",
        Some(slk_rs::Value::Number(_)) => "number",
        Some(slk_rs::Value::Str(_)) => "string",
        Some(slk_rs::Value::Bool(_)) => "bool",
        Some(slk_rs::Value::Error(_)) => "error",
        Some(_) => "text",
    };
    Ok(json!({
        "row": r,
        "column": c,
        "letter": letter(c),
        "name": t.text(1, c),
        "kind": kind,
        "value": json_value(t, r, c),
        "formula": cell.and_then(|c| c.formula.clone()),
        "comment": cell.and_then(|c| c.comment.clone()),
    }))
}

fn set(args: &Value) -> Result<Value, String> {
    let path = string(args, "path")?;
    let text = crate::source::read(path, None)?;
    let doc = slk_rs::parse(&text);
    let table = Table::new(&doc);
    let r = args
        .get("row")
        .and_then(Value::as_u64)
        .filter(|&r| r > 0)
        .ok_or("`row` is a number from 1")? as u32;
    let c = column(&table, args.get("column").ok_or("`column` is missing")?)?;
    let input = match args.get("value") {
        None | Some(Value::Null) => String::new(),
        Some(Value::String(s)) => s.clone(),
        Some(Value::Bool(true)) => "TRUE".into(),
        Some(Value::Bool(false)) => "FALSE".into(),
        Some(other) => other.to_string(),
    };
    let before = table.text(r, c);
    let value = slk_rs::parse_input(&input, table.get(r, c).map(|cell| &cell.value));
    let edits = slk_rs::set_cell(&doc, r, c, value.as_ref());
    if !edits.is_empty() {
        crate::source::write(path, slk_rs::apply(&text, &edits))?;
    }
    let after = Table::new(&slk_rs::parse(&crate::source::read(path, None)?)).text(r, c);
    Ok(
        json!({"row": r, "column": c, "letter": letter(c), "before": before, "after": after, "changed": !edits.is_empty()}),
    )
}

fn lint(sheet: &Sheet) -> Result<Value, String> {
    let errors: Vec<Value> = sheet
        .doc
        .errors
        .iter()
        .map(|e| json!({"line": e.line, "column": e.col, "message": e.message}))
        .collect();
    let warnings: Vec<Value> = slk_rs::lint(&sheet.doc)
        .iter()
        .map(|d| json!({"line": d.line, "rule": d.rule, "message": d.message}))
        .collect();
    Ok(json!({"errors": errors, "warnings": warnings}))
}

#[cfg(test)]
#[path = "slk.test.rs"]
mod tests;
