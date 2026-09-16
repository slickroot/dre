# 057 - Prompt to save new diagram on quit

## User Story

As a `dre` user, I want to be prompted to save my new diagram when I quit, so that I don't lose my work by accident.

## Acceptance Criteria

- Pressing `q` on a diagram that has never been saved to a file shows a filename prompt.
- The prompt is pre-filled with a default filename of `diagram.dre`.
- Typing a filename and confirming saves the diagram to that file (in the current diagram format) and then quits `dre`.
- If the filename I type doesn't end in `.dre`, `.dre` is appended automatically.
- Pressing Escape at the prompt quits `dre` without saving anything.
- After a successful save, a real `.dre` file exists on disk containing the diagram, and `dre` has exited.

## Technical Design

### Decisions

- **File format:** a plain-text indented outline, with named groups for deep trees.
- **Auto grouping:** when a node would be written at depth 2 (two levels of indentation) and has children, a `@slug` reference goes in its place. The node's subtree becomes its own top-level block, and the same rule applies inside that block. A leaf at depth 2 stays inline.
- **Group names:** a slug of the label. The first occurrence gets the plain slug, later ones get `-2`, `-3`, …, and an empty slug becomes `box`.
- **Labels:** always double-quoted, with `\` and `"` escaped as `\\` and `\"`.
- **Pure core, impure shell:** reducers stay pure. Confirming a save sets a pending-effect field on `State`, and `writer::run` does the actual write.
- **No save tracking:** nothing can load or save a file yet, so every diagram is unsaved and `q` always opens the prompt. Add a `path` field only when loading or saving without quitting arrives.
- **Prompt state:** the filename buffer lives inside the mode, as `Mode::SavePrompt { filename }`.
- **Prompt display:** a vim-style bottom line drawn by `frame()`. Layout and render stay unchanged.
- **Out of scope:** write failures propagate with `?` like any other I/O error. Empty filenames, empty canvases and overwriting existing files get no special handling.

### Format

```
"API gateway" colour=2 rounded
  "Auth"
  "Orders" fill=1
    @postgres

"Billing"

@postgres "Postgres"
  "Replica"
    "Backup"
```

- Top-level boxes are written first, in order, at indent 0. Each level of nesting adds two spaces.
- A line is `"label"`, followed by any of `colour=N`, `fill=N` and `rounded`, in that order. Settings still at their defaults (`PLAIN`, `false`) are left out.
- Group blocks come after all top-level trees, in the order they are first referenced. A blank line separates each tree and block. A block header is `@slug "label" settings…`.
- The file ends with a trailing newline. Selection, mode and undo history are not saved.

### Components

**`src/dre_format.rs`** (new, pure)
- `serialize(boxes: &[Node]) -> String`: builds the whole file.
- `quote(label: &str) -> String`: wraps the label in quotes and escapes `\` and `"`.
- `slug(label: &str) -> String`: lowercases ASCII letters and digits, turns every run of other characters into a single `-`, trims `-` from both ends, and falls back to `box`.
- Keeps a set of used slugs during a single `serialize` call to add the dedupe suffixes.
- Collaborators: `state::Node`, `state::PLAIN`.

**`src/save_prompt_mode.rs`** (new, pure; same shape as `insert_mode.rs`)
- `enum Command { Confirm, Cancel, Backspace, Append(char) }`
- `parse(key)`: `"\r"` → `Confirm`, `"\x1b"` → `Cancel`, `"\x7f"` → `Backspace`, printable ASCII `0x20..=0x7e` → `Append`.
- `reduce(state, command)`:
  - `Append` / `Backspace` edit the `filename` in `Mode::SavePrompt`.
  - `Confirm` sets `state.save_to = Some(with_extension(filename))` and `state.running = false`.
  - `Cancel` sets `state.running = false` and leaves `save_to` as `None`.
- `with_extension(filename) -> String`: adds `.dre` unless the name already ends with it.
- Collaborators: `state::{State, Mode}`.

**`src/state.rs`** (changed)
- `Mode` gains `SavePrompt { filename: String }` and is no longer `Copy`. `handle_key` matches on `&state.mode` and sends `SavePrompt` keys to `save_prompt_mode`.
- `State` gains `save_to: Option<String>`, which defaults to `None` and is the pending-effect field.
- New `pub(crate) const DEFAULT_FILENAME: &str = "diagram.dre";`

**`src/command_mode.rs`** (changed)
- `quit` switches to `Mode::SavePrompt { filename: DEFAULT_FILENAME.to_string() }` and no longer clears `running`.
- The tests `q_stops_the_state_and_preserves_boxes_and_selection` and `q_does_not_clobber_an_existing_undo_snapshot` are updated: `q` now opens the prompt, and the Escape and Enter paths are tested in `save_prompt_mode`.

**`src/writer.rs`** (changed, impure shell)
- `run`: after `handle_key`, if `state.save_to` is `Some(path)`, call `fs::write(path, dre_format::serialize(&state.doc.boxes))?`. The loop then ends because `running` is `false`. Ctrl-C still exits straight away without the prompt.
- `frame`: when `state.mode` is `SavePrompt { filename }`, replace the last rendered line with `prompt_line(filename, cols)`.
- `prompt_line(filename: &str, cols: i64) -> String` (pure): `"Save as: {filename}{CURSOR}"`, padded with spaces or cut to `cols`.
- Collaborators: `dre_format`, `state`, `render::CURSOR`.

### Test Plan (outside-in)

1. `save_prompt_mode`: `q` in command mode opens the prompt with `diagram.dre` filled in.
2. `save_prompt_mode`: typing and backspace edit the filename.
3. `save_prompt_mode`: Enter sets `save_to` (adding `.dre` when it's missing) and stops the app. Escape stops the app with no `save_to`.
4. `dre_format`: quoting, settings order and omitted defaults, indentation, multiple top-level boxes.
5. `dre_format`: a depth-2 node with children becomes a group reference, groups nest inside groups, slugs and dedupe suffixes.
6. `writer`: `prompt_line` padding, truncation and cursor. `frame` shows the prompt on the last row only while in `SavePrompt`.
7. `writer::run`: writes `serialize(boxes)` to `save_to`. Tested with a temp directory, or kept as a thin untested wrapper if `run` can't be tested as it stands.
