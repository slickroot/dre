# 066 - README explains how to start and quit dre

## Story

Bob has installed `dre`. He reads the README and learns how to start `dre`,
open or create a diagram, and quit, with or without saving.

## Acceptance Criteria

The README explains:

- `dre` opens an empty canvas.
- `dre plans.dre` opens `plans.dre`, or starts a new diagram under that name
  if the file doesn't exist.
- With a filename, `q` saves to that file and quits.
- Without a filename, `q` asks "Save as:" (pre-filled with `diagram.dre`).
  `Enter` saves and `Esc` quits without saving.
- `Ctrl-C` quits without saving.
- Every command-mode keybinding is documented in a `| Key | Description |`
  table that is generated from `COMMAND_KEYMAP` and pinned by a test, so the
  README cannot drift from the real keys.
- `parse` and `COMMAND_KEYMAP` are proven to agree by a unit test.
- Insert-mode, save-prompt, and Ctrl-C keys are documented too.
- The README follows a user-guide outline:
  What is dre / Requirements / Install / Getting started / Command mode /
  Other keys. No `.dre` file-format section in this story.

## Technical Design

### Single source of truth

To keep the documented keybindings from drifting from the real ones, one
constant owns every command-mode binding, and the README's command-mode table
is generated from it.

### `KeyBinding` + `COMMAND_KEYMAP` (in `command_mode.rs`)

```rust
pub(crate) struct KeyBinding {
    pub(crate) keys: &'static [&'static str], // e.g. &["u"]
    pub(crate) command: Command,              // e.g. Command::Undo
    pub(crate) description: &'static str,     // "Undo the last change"
}

pub(crate) const COMMAND_KEYMAP: &[KeyBinding] = &[ /* all 15 bindings */ ];
```

`Command` (command_mode.rs:7) stays as-is. `parse` (command_mode.rs:25-44)
keeps its hand-written `match`; behavior is unchanged. KeyBinding order in the
table = command-mode order.

Descriptions (hand-written once, used verbatim in the README table):

- `u` Undo — "Undo the last change"
- `b` NewBox — "Add a child box"
- `s` NewSibling — "Add a sibling box"
- `h` SelectParent — "Select the parent box"
- `l` SelectChild — "Select the first child box"
- `j` SelectNext — "Select the next sibling"
- `k` SelectPrevious — "Select the previous sibling"
- `i` EditLabel — "Edit the selected box's label"
- `I` RenameLabel — "Rename the selected box's label"
- `c` CycleColour — "Cycle the box's colour"
- `C` CycleSiblingsColour — "Cycle the colour of every sibling"
- `f` CycleFill — "Cycle the box's fill"
- `F` CycleSiblingsFill — "Cycle the fill of every sibling"
- `r` ToggleRounded — "Toggle rounded corners"
- `q` Quit — "Save and quit (or choose where to save)"

### Markdown generator (`command_mode.rs`)

`pub(crate) fn format_keymap_markdown() -> String` emits the table between the
README markers: `| Key | Description |` header, `| --- | --- |` separator, one
row per binding in keymap order.

### Tests (unit tests in `command_mode.rs`)

1. **Sync test** — proves `parse` agrees with `COMMAND_KEYMAP`:
   - for every binding and every key in `keys`: `parse(key) == Some(command)`;
   - for keys not bound in the map: `parse(key) == None`;
   - keys are unique across all bindings.
2. **README table test** (env-flag) — reads `README.md` via
   `CARGO_MANIFEST_DIR`, finds
   `<!-- keymap:start -->` / `<!-- keymap:end -->`:
   - with `UPDATE_README=1`: writes `format_keymap_markdown()` between the
     markers and saves the file;
   - otherwise: asserts the text between the markers equals
     `format_keymap_markdown()` (fails CI on drift; failure message tells the
     dev to run `UPDATE_README=1 cargo test`).

### README structure

README.md is edited top-to-bottom as follows:

1. **What is dre** — unchanged intro.
2. **Requirements** — unchanged.
3. **Install** — unchanged.
4. **Getting started** — satisfies story 066: `dre` opens an empty canvas;
   `dre plans.dre` opens the file or starts a new diagram under that name; `q`
   with a filename saves and quits; `q` without a filename shows `Save as:`
   (pre-filled `diagram.dre`) — `Enter` saves and quits, `Esc` quits without
   saving; `Ctrl-C` quits without saving. Also covers the three modes:
   command mode (default), insert mode (`i`/`I`), save prompt.
5. **Command mode** — the generated keybinding table between the
   `<!-- keymap:start -->` / `<!-- keymap:end -->` markers.
6. **Other keys** — hand-written: insert mode (`Esc` commits/cancels,
   `Backspace` removes, printable appends), save prompt (`Enter`, `Esc`,
   `Backspace`), `Ctrl-C` quits without saving.

No `.dre` file-format section in this story.

### Collaborators

- `command_mode.rs`: owns `Command`, `parse`, `COMMAND_KEYMAP`,
  `format_keymap_markdown`, and the unit tests that pin `parse` and the README
  to the constant.
- `README.md`: consumes the generated table; all prose hand-maintained.
- `writer.rs`: unchanged — Ctrl-C (writer.rs:144) and the `Save as:` prompt are
  already implemented and only documented, not modified.
