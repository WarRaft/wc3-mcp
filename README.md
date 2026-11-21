# wc3-mcp

An [MCP](https://modelcontextprotocol.io) server for Warcraft III's files,
for an AI assistant (Claude Code and others) to read, query and edit them.
It speaks JSON-RPC 2.0 over stdio, one message per line, and is built on
the same crates as the [`wc3-rs`](../wc3-rs) editor:
[`slk-rs`](../slk-rs) for SLK tables and [`storm-rs`](../storm-rs) for MPQ
archives and maps.

## Tools

Rows and columns count from 1. Row 1 holds the field names in the game's
tables, so a column is named by its field (`goldcost`), its letter (`H`) or
its number (`8`). A table is read from a file on disk (`path`), or from a
file inside an MPQ archive or a map: `path` is then its name inside
(`Units\UnitBalance.slk`, `/` works too) and `archive` the archive.
Writing is to files on disk only.

| tool       | does |
|------------|------|
| `slk_info` | the size, each column's number, letter and name, how many problems the file has |
| `slk_read` | rows as JSON objects keyed by column name (and `row`): pick `columns`, keep rows whose cells equal given values (`where`) or contain a text (`contains`), page with `offset` / `limit` (100 by default, 2000 at most); `total` is how many matched |
| `slk_get`  | one cell: its value, kind (number, string, bool, error), formula and comment |
| `slk_set`  | writes one cell of a file, changing only what it must in the text (`slk_rs::set_cell`); `null` clears it |
| `slk_lint` | parse errors (line, column) and lint warnings |

Values come as JSON: whole numbers as integers, other numbers as numbers,
`TRUE` / `FALSE` as booleans, empty cells as `null`, the rest as strings.
`slk_set` reads its value like typed input in the editor's table: a number,
`TRUE` / `FALSE`, an error like `#VALUE!`, or text; a cell that holds text
keeps holding text, and `"quoted"` input is always text.

Answers are compact JSON, a tool's failure a result with `isError` and the
reason, so the model reads what went wrong.

```json
{"name": "slk_read", "arguments": {
  "path": "Units/UnitBalance.slk",
  "archive": "C:/Games/Warcraft III/War3Patch.mpq",
  "where": {"unitBalanceID": "hfoo"},
  "columns": ["goldcost", "lumbercost", "HP"]}}
→ {"total":1,"offset":0,"rows":[{"row":10,"goldcost":135,"lumbercost":0,"HP":420}]}
```

### MPQ archives and maps

Archives (`.mpq`) and maps (`.w3x`, `.w3m`), with `storm-rs`. Names inside
use `\` (`/` works too); masks are StormLib's, `*` and `?` in any case over
the whole name (`Units\*.slk`, `war3map.*`). An archive without a
`(listfile)`, as most protected maps are, only lets you find a file you can
name: `listfiles` takes files on disk with more names, one per line (the
game's own list, say).

| tool          | does |
|---------------|------|
| `mpq_info`    | the header and tables: format version, sizes, how many files, how many with a known name, whether it has `(listfile)`, `(attributes)`, `(signature)` |
| `mpq_list`    | the files with a known name, sorted: name, size, compressed size, block flags; `masks`, `offset` / `limit` (500 by default, 5000 at most); `unnamed` counts the rest |
| `mpq_read`    | one file: UTF-8 text as text, from `offset`, up to 200,000 bytes at a time (`more` says there's more); anything else as its size and the hex of up to 4096 bytes |
| `mpq_extract` | files into a folder on disk, the name's `\` parts as folders; a name that would leave it (`..\x`) is skipped and reported |
| `mpq_verify`  | the files that fail the CRC32 / MD5 of `(attributes)` or their sector CRCs |
| `mpq_create`  | a new archive of every file under a folder, with `(listfile)` and `(attributes)`; never overwrites |
| `mpq_add`     | adds or replaces a file, from a file on disk (`file`) or text (`text`) |
| `mpq_remove`  | removes files by name |
| `mpq_rename`  | renames a file |

A SLK table inside an archive is read with the `slk_*` tools and `archive`.

## Using it

```sh
cargo build --release   # target/release/wc3-mcp
```

Claude Code: the `wc3-rs` repository registers it in its `.mcp.json`, from
`../wc3-mcp/target/release/wc3-mcp`. Elsewhere:

```sh
claude mcp add wc3 -- /path/to/wc3-mcp/target/release/wc3-mcp
```

## Layout

- `rpc.rs`: the protocol: `initialize`, `ping`, `tools/list`, `tools/call`;
- `slk.rs`: the SLK tools;
- `mpq.rs`: the MPQ tools;
- `source.rs`: reading a file from disk or from inside an archive.

To come: object data (`w3abdhqtu-rs`).

## Tests

```sh
cargo test
```
