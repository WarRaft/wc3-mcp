# wc3-mcp

An [MCP](https://modelcontextprotocol.io) server for Warcraft III's files,
for an AI assistant (Claude Code and others) to read, query and edit them.
It speaks JSON-RPC 2.0 over stdio, one message per line, and is built on
the same crates as the [`wc3-rs`](../wc3-rs) editor:
[`slk-rs`](../slk-rs) for SLK tables, [`storm-rs`](../storm-rs) for MPQ
archives and maps, [`w3abdhqtu-rs`](../w3abdhqtu-rs) for object data,
[`wts-rs`](../wts-rs) for string tables, [`w3c-rs`](../w3c-rs) for
cameras, [`w3r-rs`](../w3r-rs) for regions, [`w3s-rs`](../w3s-rs) for sounds, [`w3e-rs`](../w3e-rs) for
terrain, [`w3i-rs`](../w3i-rs) for map info.

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

### Object data

A map's changes to the game's objects, with `w3abdhqtu-rs`: abilities
(`war3map.w3a`), destructables (`.w3b`), doodads (`.w3d`), buffs (`.w3h`),
upgrades (`.w3q`), items (`.w3t`), units (`.w3u`). A file is read from
disk or from inside a map (`archive`), and changed on disk. `game`, the
game's folders as a project's `game=` lists them, gives the fields' names
and types and the checks: their `War3*.mpq` and loose files are laid over
each other as the game does (`storm_rs::archive::Layers`), and the
metadata read from them (`Metadata::load`). `roots`, the map's roots as a
project's `[map] root=` lists them (folders or maps), go over the game's
folders: the last layer that has a file wins, the same lookup the editor
uses (`wc3_server::game::Lookup`).

An object is named by its id: a custom one's (`A000`), a changed standard
one's (`AHbz`). A field is its id, and for abilities, upgrades and doodads
its `level` and data `column` (`Data A` is 1).

| tool         | does |
|--------------|------|
| `obj_list`   | the objects: id, base, table (`original` / `custom`), name (when the file sets it), how many fields; filter by text or table, page |
| `obj_get`    | one object's fields: key as the text writes it (`DataA[3]` with `game`, else `Hbz1[3].A`), id, level, column, the type stored and the value; with `game` the field's name and type |
| `obj_text`   | the file as the editor's text (BNI), with names and types as comments, paged by lines |
| `obj_check`  | the problems the game's metadata shows (wrong types, values past World Editor's limits, unknown fields, levels and columns, fields of other objects), each with its object and key; needs `game` |
| `obj_set`    | writes one field (by its key, `Cool`, `DataA`, or its id), or removes it (`null`); a number takes the field's type (integer, real, unreal) from the metadata |
| `obj_new`    | adds a custom object: its id and the standard object it starts from |
| `obj_delete` | removes an object |

A string value that is `TRIGSTR_<n>` (most names and tooltips in maps)
comes with its text from the map's string table: read from `archive` by
itself, or from `strings`, a `war3map.wts` on disk. `obj_list`'s names are
the text; `obj_get`'s fields keep the `value` and add the `text`.

### String tables

A map's `war3map.wts` (and `war3campaign.wts`), with `wts-rs`: the text
object data, triggers and the map's info name as `TRIGSTR_<n>`. A table is
read from disk (`path`) or from inside a map (`archive`; `war3map.wts` by
default), and changed on disk, keeping its byte-order mark and line
breaks.

| tool          | does |
|---------------|------|
| `wts_list`    | the strings: number, the comments World Editor writes (what uses it), the text (the first 200 characters, `length` the whole); filter by text, page |
| `wts_get`     | strings by number, whole |
| `wts_resolve` | text with every `TRIGSTR_<n>` the table has replaced by its text |
| `wts_set`     | writes, adds or removes (`null`) one string, changing only its lines |
| `wts_lint`    | parse errors and lint warnings (a number defined twice) |

### Cameras

A map's `war3map.w3c`, with `w3c-rs`: the cameras World Editor places,
`gg_cam_<name>` in the script. Read from disk (`path`) or from inside a
map (`archive`; `war3map.w3c` by default), changed on disk.

The file doesn't say its layout: since 1.31 each camera also has a local
pitch, yaw and roll. Which game the map is for is the first four bytes of
its `war3map.w3i` (18 Reign of Chaos, 25 The Frozen Throne, 28 1.31, 31
1.32), read next to the file, in the same folder or the same map; without
one, the layout is 1.27a's. `w3i_version` sets it by hand. Every answer
says the layout and where it came from (`layout_from`).

| tool         | does |
|--------------|------|
| `w3c_list`   | the cameras: index, name, the script's variable, every field |
| `w3c_set`    | changes fields or the name of one camera (by index or name) |
| `w3c_new`    | adds a camera, World Editor's defaults for what's left out; makes the file if there's none |
| `w3c_delete` | removes a camera |

### Regions

A map's `war3map.w3r`, with `w3r-rs`: the regions World Editor places
(`gg_rct_*` in the script). Read from disk (`path`) or from inside a map
(`archive`; `war3map.w3r` by default), changed on disk. A region is named
by its name or by its `index` (World Editor's number for it, which stays
when others are removed).

| tool         | does |
|--------------|------|
| `w3r_list`   | every region: name, variable, index, left, bottom, right, top, weather, sound, color |
| `w3r_set`    | changes some fields of a region |
| `w3r_new`    | adds a region (a new file when there's none): the rectangle is needed; the next index, `Region NNN`, no weather, no sound, white otherwise |
| `w3r_delete` | removes a region |

Names and indexes stay unique, and the rectangle's left isn't past its
right (nor bottom past top): a change that breaks one is refused,
nothing written.

### Sounds

A map's `war3map.w3s`, with `w3s-rs`: the sounds the Sound Editor
defines (`gg_snd_*` in the script). Read from disk (`path`) or from
inside a map (`archive`; `war3map.w3s` by default), changed on disk. A
sound is named by its variable or its index in the file. A number may be
`"default"`: World Editor's "use default", the game's own value.

| tool         | does |
|--------------|------|
| `w3s_list`   | every sound: index, variable, file, EAX setting, flags (and their names), fades, volume, pitch, channel (and its name), distances, cone |
| `w3s_set`    | changes some fields of a sound |
| `w3s_new`    | adds a sound (a new file when there's none): `name` and `file` are needed; World Editor's values otherwise |
| `w3s_delete` | removes a sound |

Variables stay unique: a change that repeats one is refused, nothing
written.

### Terrain

A map's `war3map.w3e`, with `w3e-rs`: its tilesets, and the grid of
points with their height, water, ground texture, cliffs and flags. Read
from disk (`path`) or from inside a map (`archive`; `war3map.w3e` by
default), changed on disk. A point is addressed by column `x` and row `y`
from the bottom left; its world position is the grid's offset plus 128
per step.

| tool         | does |
|--------------|------|
| `w3e_info`   | version, base tileset (its name from the game's files with `game` and `roots`), the ground and cliff textures with how many points use each, the size, the world bounds |
| `w3e_points` | the points of a region (4096 at most): world position, height (and from zero), water, texture (index and rawcode), ramp, blight, water, boundary, edge, variations, cliff level and texture |
| `w3e_set`    | sets fields on every point of a region; a texture by index or rawcode from the ground list; values are checked against the file first |

### Map info

A map's `war3map.w3i` (the Scenario menu: name, author, description,
camera bounds, flags, loading screen, fog, players, forces, upgrades,
techs, random tables), with `w3i-rs`, as its text form: the same BNI the
wc3-rs editor shows, a `[w3i]` section, then a section per player, force,
upgrade, tech and random table. Read from disk (`path`) or from inside a
map (`archive`; `war3map.w3i` by default), written on disk.

| tool        | does |
|-------------|------|
| `w3i_text`  | the text form; the comment after a value says what it means |
| `w3i_write` | writes the text form back; it's checked first, and every error comes back with its line, nothing written |

## Planned: JASS

Not built yet. These are the next tools, and what the modules underneath
are shaped for (jass-rs, the wc3-rs server):

1. **Check a script**: syntax and semantics, as diagnostics. Semantics
   first means names and order: an unknown function, variable or type, a
   call with the wrong number of arguments; checking types (arguments,
   `set`, `return`, conditions, the `handle` hierarchy from `common.j`)
   comes after.
2. **Lint and bad patterns**, by the same rules and project settings
   (`[lint]` in `wc3.bni`) the editor uses.
3. **List natives** (and `Blizzard.j`'s functions) from the project's
   `game=` folders, with the comments written above them, where a
   `common.j` (UjAPI's, say) has them.
4. **Refactor**: rename a symbol everywhere it's used, scopes and
   shadowing taken care of, so the model asks for a rename instead of
   working out the language's rules itself.

Two kinds of checking, because the build sorts functions to get around
JASS calling only functions declared above:

- **pure**: `war3map.j` as the game reads it; a call before the
  function's declaration, a `local` after a statement, and the rest of
  the `pure` group are errors. A map's script (`war3map.j`, inside a map
  or by that name) is checked this way by itself.
- **development**: the project's sources, where those are fine (the
  build orders functions; locals go anywhere). A project can still turn
  `pure` on in `[lint]`; a tool can ask for either.

The wc3-rs server is a library as well as a binary (`wc3_server`), and
these tools will use its project model in-process (workspace, symbol
index, rename, analysis), so the editor and the model see the same thing.
The object data tools already read the game's metadata through it.

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
- `obj.rs`: the object data tools;
- `wts.rs`: the string table tools;
- `w3c.rs`: the camera tools;
- `w3r.rs`: the region tools;
- `w3s.rs`: the sound tools;
- `w3e.rs`: the terrain tools;
- `w3i.rs`: the map info tools;
- `source.rs`: reading a file from disk or from inside an archive.

The game's metadata comes from the editor's server, used as a library
(`wc3_server::objects::metadata`), so both read the game the same way.


## Tests

```sh
cargo test
```
