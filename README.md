# wc3-mcp

An [MCP](https://modelcontextprotocol.io) server for Warcraft III's files,
for an AI assistant (Claude Code and others) to read, query and edit them.
It speaks JSON-RPC 2.0 over stdio, one message per line, and is built on
the same crates as the [`wc3-rs`](../wc3-rs) editor:
[`slk-rs`](../slk-rs) for SLK tables, [`storm-rs`](../storm-rs) for MPQ
archives and maps, [`w3abdhqtu-rs`](../w3abdhqtu-rs) for object data,
[`wts-rs`](../wts-rs) for string tables, [`w3c-rs`](../w3c-rs) for
cameras, [`w3r-rs`](../w3r-rs) for regions, [`w3s-rs`](../w3s-rs) for sounds, [`wpm-rs`](../wpm-rs) for pathing, [`w3e-rs`](../w3e-rs) for
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

**The game's files: `game`, `roots`, or the project's.** Tools that show
the game's names or check against its data (`UI\TriggerData.txt`, the
metadata tables) look files up through layers: `game` (the game's
folders, lowest first) and `roots` (the map's roots over them). Each one
a call leaves out comes from the project of the file the tool works on
(the nearest `wc3.bni` above `archive`, `path`, `file` or `dir`, or the
one `project` names), the way the editor reads that file: a map the
project doesn't make its map is read alone over the game
(`wc3_server::game::Lookup::for_file`). Given ones win, so `"roots": []`
reads without the project's roots. A message that a file is missing
says where it was looked for.

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

### Pathing

A map's `war3map.wpm`, with `wpm-rs`: for every cell (four per terrain
cell, 32 world units) whether units can walk, fly, build or sail there,
blight, the boundary. Read from disk (`path`) or from inside a map
(`archive`; `war3map.wpm` by default), changed on disk. A cell is
addressed by column `x` and row `y` from the bottom left; with the map's
`war3map.w3e` beside the file, cells have world positions too. A flag is
named by its bit or its name (`no walking`, `0x02`).

| tool        | does |
|-------------|------|
| `wpm_info`  | the size, how many cells have each flag (and what share), the world bounds |
| `wpm_image` | a picture, PNG: the pathing as the game's pathing textures draw it (red no walking, green no flying, blue no building), or one flag in white; a region crops it, `scale` sets pixels per cell |
| `wpm_cells` | the cells of a region (4096 at most): flags as a number and by name, world position |
| `wpm_set`   | sets and clears flags on every cell of a region |

`wpm_image` answers with MCP image content (a text saying what's drawn,
then the PNG), so a model can look at the map. The PNG is written by
`png.rs` (zlib from `flate2`).

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
| `w3e_image`  | a picture from above (PNG, north up) as the World Editor's view draws it (`wc3_editor::picture`): each point in its ground texture's colour (blight, a cliff's ground beside it), lit by the slope, under its water by depth, darker on boundaries; textures and tables through `game` and `roots` (the project's by default) |
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

### Imports, minimap icons, custom script, shadows

Read from disk (`path`) or from inside a map (`archive`; the map's file
by default), written on disk.

| tool        | does |
|-------------|------|
| `imp_list`  | `war3map.imp`, the imported files: each one's path, where it is in the map (`war3mapImported\` and its name unless it has a custom path), its flags by name |
| `imp_text`, `imp_write` | its text form (`imp-rs`), and back after checking it |
| `mmp_list`  | `war3map.mmp`, the minimap icons: kind (and its name), place, color |
| `mmp_text`, `mmp_write` | its text form (`mmp-rs`), and back |
| `wct_text`, `wct_write` | `war3map.wct`, the custom script: the map's header code and each trigger's custom text after `// @wct` marker lines (`script: "lua"` for `--`), and back |
| `shd_image` | `war3map.shd`, a picture (PNG) of where trees and buildings cast shadow; its size from `war3map.w3e` beside it |

### Doodads and units

`war3map.doo` (doodads and destructables) and `war3mapUnits.doo` (units,
buildings, items, start locations), with `doo-rs`; which one, the file's
name says, and the layout (1.32 added skins) the `war3map.w3i` beside
it. A big map places tens of thousands of things, so they're listed
with filters and changed one by one, by `index`, their place in the
file. A thing's text is the editor's: a `[doodad]` or `[unit]` section.

| tool         | does |
|--------------|------|
| `doo_list`   | things placed, filtered by type ids, owner and a rectangle, a page at a time (`offset`, `limit` 200): position, angle, scale; flags and life; owner, hp, mana, level, items, how it's random |
| `doo_get`    | the text of some, by index |
| `doo_set`    | replaces one with a section of text |
| `doo_new`    | adds sections of text at the end |
| `doo_delete` | removes some, by index |

### Triggers

`war3map.wtg`, the GUI triggers, with `wtg-rs`. How many parameters each
function takes is in World Editor's `UI\TriggerData.txt`, not the file:
every tool reads it through `game` and `roots`, or the project's (see
above). A map made with a modified editor (natives added to the GUI)
needs its additions: a folder root with `UI\TriggerData.patch.txt` and
`UI\TriggerStrings.patch.txt`, which change the game's files by meaning. A trigger's text is the editor's: a `[trigger]` section, its
events, conditions and actions as lines of calls
(`action=TriggerSleepAction("2.00")`). Adding or removing triggers keeps
the `war3map.wct` beside the file in step (a piece of code per trigger).

| tool         | does |
|--------------|------|
| `wtg_list`   | categories, variables, and the triggers (filtered by category and name, a page at a time): name, category, flags, how many events, conditions and actions |
| `wtg_get`    | the text of triggers, by name or index |
| `wtg_set`    | replaces one with a `[trigger]` section; every function is checked against `TriggerData.txt` |
| `wtg_new`    | adds `[category]`, `[variable]` and `[trigger]` sections |
| `wtg_delete` | removes triggers, and their pieces of `war3map.wct` |
| `wtg_text`, `wtg_write` | the whole file as text, and back |

### Trigger files

Trigger files of their own (`.trg`), the source the
[wc3-rs](../wc3-rs) editor keeps a map's triggers in: one trigger per
file, a folder per category (`000_Init/001_Spawn.trg`, the order the
game builds them in in the names), the globals in `_variables.trg`. A
project is a folder of them (`dir`): its globals, the names the map's
script gives its triggers (`gg_trg_...`, World Editor's: `_` for each
byte a JASS name can't have, a `u` after a `_` at the end) and the
references between them are the folder's. The tools read, check, build
and change them with the editor's server (`wc3_server::triggers`), and
show them to read: a trigger's lines as World Editor's phrases (with
`game`, in the game's language), the editor's functions, types and
presets to write lines with. A folder of 750 trigger files is checked in
a second.

| tool            | does |
|-----------------|------|
| `trg_extract`   | a map's triggers out into a folder (files there kept unless `replace`); the folders and how many files, their names up to 100 |
| `trg_list`      | a project's triggers (file, category, name, its `gg_trg_`, flags, counts, problems) and variables |
| `trg_read`      | a trigger as the editor shows it: each line its phrase, its line in the file and its call; blocks indented; the problems |
| `trg_check`     | the problems: text, functions against `TriggerData.txt`, undeclared or unseen variables, `$gg_trg_` references no trigger has or that mean two, two triggers named alike in the script |
| `trg_jass`      | the JASS a file is built into |
| `trg_functions` | functions by words (`query`, `kind`), or one in full: phrase, parameters' types and defaults, what it returns, hint, a ready line |
| `trg_types`     | the types, or a type's presets |
| `trg_write`     | writes a file after checking it, then its problems in the project |
| `trg_rename`    | renames a trigger; the references to it follow (`$gg_trg_...`, custom script, `.j`, by tokens); refused when another would be named alike |
| `trg_variable`  | changes or deletes a variable: its uses renamed with it; what would break listed, done only with `confirm` |

### Projects

A project's settings, its `wc3.bni` (see the wc3-rs README, "Projects").

| tool            | does |
|-----------------|------|
| `project_info`  | the settings of the project a file or folder is in: game folders and roots (each a folder, a map or missing), sources, exclusions, builds, dialect, lint, the settings' problems, the text; and where the editor's files (`UI\TriggerData.txt` ...) and their patches come from |
| `project_write` | writes a `wc3.bni` (made if missing) after reading it as the editor does: errors with their lines and nothing written, warnings with what was written |
| `project_build` | builds a project's map, a `[build]` of its `wc3.bni` (see wc3-rs's README, "Building a map"): the roots' files and `war3map.j` made anew; where it went, or every error and nothing written; written with `errors` when its script has some that don't stop it (AngelScript's `ExecuteFunc` of a name that can't be worked out) |

### Code

The project's code checked as the editor checks it: the wc3-rs server's
project model, loaded in process for the project, so what comes back is
what VS Code shows.

| tool         | does |
|--------------|------|
| `code_check` | every problem of a project's code (JASS, Zinc, Lua, AngelScript): parse errors, lint with the project's `[lint]`, and each JASS name and type checked against the whole project and the game's scripts it runs with (`unknown-name`, `argument-count`, `type-mismatch`, `array-access`, `constant-assignment`), so a problem between files shows; one file's only, errors only, a limit |
| `jass_rename`  | renames a symbol of a project's code everywhere it means it, as the editor's rename does (the wc3-rs server's `rename`, in process): every file of the project, a map's custom script (`war3map.wct`, written back as its bytes) and scripts inside maps too, scopes and shadowing respected; refused with the reason and nothing written (a native or the game's, declared nowhere, a member of several structs, a bad or taken name, a map that can't be written, a file that isn't UTF-8; a name World Editor's script makes of the map's triggers, declared only in its `war3map.j` (`udg_`, `gg_trg_`, `gg_unit_` ...) or a trigger's `InitTrig_`: the trigger or variable is renamed instead, else it comes back when the map is saved); a copy in that `war3map.j` follows its source; `file`/`line` pick a declaration when several have the name; `dry_run` |
| `jass_build`   | builds JASS files (or a folder's) into one script the game reads, with `wc3-build`: this toolchain's JASS made the game's, `main` and `config` merged, globals and functions sorted (a cycle through `TriggerEvaluate`), locals to the top; with `uglify`, shorter names (everything the game doesn't need as it is renamed `a`, `b` ...); written to `out` or given back; errors with their file and line |
| `jass_natives` | what the game's scripts declare as a project sees them (`common.j`, `Blizzard.j`, `common.ai` through its `game=` and roots, the last found winning: UjAPI's `common.j` over the game's): natives, functions, globals, each with its declaration, script, line and the comment above it; by a part of the name (the exact name first) or of the comment, by kind or script |

### Object collections

`.w3o`, World Editor's Export All Object Data, with `w3o-rs`: every kind
of object data in one file. The `obj_` tools work on each kind's file.

| tool        | does |
|-------------|------|
| `w3o_info`  | which kinds it has, how many objects each |
| `w3o_split` | writes each kind's `war3map.w3?` to a folder |
| `w3o_join`  | writes a `.w3o` from the object data in a folder or a map |

### txt2

XGM's markup (`.txt2` articles and resource pages), with `txt2-rs`. A
text is a file on disk (`path`), one inside a map (`archive`), or the
text itself (`text`). A long article is read by its outline and a part
at a time; changes are made to the file as to any text.

| tool           | does |
|----------------|------|
| `txt2_syntax`  | the markup, every inline and block construct (from `txt2-rs`'s README) |
| `txt2_check`   | blocks without their closing line, closings without an opening, with line and column |
| `txt2_outline` | the parts with their first and last lines: headings, spoilers and their titles, quotes, layouts and columns, code, tables, lists, images, galleries, polls, tiles; nested as in the text |
| `txt2_get`     | one part's source, by title (exact, else the first containing it) or by the line it starts at; a heading's part runs to the next heading of its level |
| `txt2_html`    | the HTML the editor's preview shows: a fragment, or a whole page (`page`), answered or written to `out` |

## JASS

Built. These are the tools, and what the modules underneath are
shaped for (jass-rs, the wc3-rs server):

1. **Check a script**: syntax and semantics, as diagnostics: done
   (`code_check`: an unknown function, variable or type, a call with the
   wrong number of arguments, a value of the wrong type with the `handle`
   tree from `common.j`, across the project's files).
2. **Lint and bad patterns**, by the same rules and project settings
   (`[lint]` in `wc3.bni`) the editor uses: done, in `code_check`.
3. **List natives** (and `Blizzard.j`'s functions) from the project's
   `game=` folders, with the comments written above them, where a
   `common.j` (UjAPI's, say) has them: done, `jass_natives`.
4. **Refactor**: rename a symbol everywhere it's used, scopes and
   shadowing taken care of, so the model asks for a rename instead of
   working out the language's rules itself: done, `jass_rename` (the
   editor's rename, by the name and, when several declare it, its file
   and line).

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
these tools use its project model in-process (workspace, symbol
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

VS Code: the WC3 extension ships it (`wc3-rs/server/build.sh` builds it
for every platform into the extension's `bin/`) and registers it as the
**WC3** MCP server, started in the first workspace folder.

## Layout

- `rpc.rs`: the protocol: `initialize`, `ping`, `tools/list`, `tools/call`;
- `slk.rs`: the SLK tools;
- `mpq.rs`: the MPQ tools;
- `obj.rs`: the object data tools;
- `wts.rs`: the string table tools;
- `w3c.rs`: the camera tools;
- `w3r.rs`: the region tools;
- `w3s.rs`: the sound tools;
- `wpm.rs`: the pathing tools; `png.rs`: PNG and base64 for pictures;
- `w3e.rs`: the terrain tools;
- `w3i.rs`: the map info tools;
- `imp.rs`, `mmp.rs`, `wct.rs`, `shd.rs`: imports, minimap icons, custom script, shadows;
- `doo.rs`: doodads and units placed; `wtg.rs`: triggers (`war3map.wtg`); `trg.rs`: trigger files (`.trg`); `w3o.rs`: object collections;
- `txt2.rs`: txt2, XGM's markup;
- `project.rs`: a project's settings (`wc3.bni`);
- `source.rs`: reading a file from disk or from inside an archive; the layers a call reads the game's files through (given, or its project's).

The game's metadata comes from the editor's server, used as a library
(`wc3_server::objects::metadata`), so both read the game the same way.


## Tests

```sh
cargo test
```
