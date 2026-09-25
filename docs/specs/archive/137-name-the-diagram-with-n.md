## Story

Doug opens dre with a brand-new diagram and draws a few boxes. The corner reads `[no name] • dre`. He presses `n` in normal mode. A prompt appears in the bottom-right corner, in place of `[no name]`, with a gray placeholder, "type a name", and the cursor at its start, so he knows to type the name there. He types `plans` and presses Enter. The corner now reads `plans • dre`, and `plans.dre` is written with his boxes. Happy, he keeps drawing, knowing every change is saved.

## Acceptance Criteria

- Pressing `n` in normal mode opens a name prompt in the bottom-right corner, in place of the name (`[no name]` or the current name). The ` • dre` suffix stays. It shows a gray placeholder reading "type a name", with the cursor at the start of it.
- Typing hides the placeholder and shows what Doug types.
- Enter with a name closes the prompt. The corner reads `<name> • dre`, and the diagram saves after every change from then on, like any diagram opened with a file name. The file is written as soon as the diagram has changes that aren't saved yet, so a diagram Doug already drew on is written on Enter, and one with no changes is written on its next change.
- dre always adds `.dre` to the typed name, so `plans.dre` becomes `plans.dre.dre`.
- If `<name>.dre` already exists, it is overwritten.
- Esc closes the prompt. The diagram stays as it was, and nothing is written.
- Enter with nothing typed does nothing. The prompt stays open.
- If the diagram already has a name, `n` works the same way and changes it. From then on the diagram saves to `<new name>.dre`. The file under the old name is left as it is.

## Technical Design

### Approach

`n` opens a new `Mode::NamePrompt { name: String }`. Naming only sets `save_to`; the file is written by the existing `AutosavingReducer` (spec 112). Nothing changes in the store, `Files` or `Editor`.

### Components

- `Mode::NamePrompt { name }` (`state/mode.rs`): starts with an empty `name`.
- `Action` (`state/action.rs`): `OpenNamePrompt`, `NameAppend(char)`, `NameBackspace`, `NameConfirm`, `NameCancel`, and `ActionMode::NamePrompt`. `Action::mode()` maps an action to a reducer statically, so `Confirm` and `Cancel` can't be shared with `SavePrompt`, whose Enter quits. Typing and backspace get their own actions for the same reason.
- `input.rs`: in Command mode `n` (unbound today) parses to `OpenNamePrompt`. A `name_prompt_parse` handles Enter, Esc, Backspace and printable ASCII. The printable-character filter is shared with `save_prompt_parse`.
- `state/name_prompt.rs`, wired in `apply` next to `save_prompt`:
  - `OpenNamePrompt` is handled in `command::reduce` and sets `Mode::NamePrompt { name: "" }`.
  - `NameAppend` and `NameBackspace` edit `name`.
  - `NameConfirm` with an empty name does nothing, so the prompt stays open. Otherwise it calls `set_save_to(Some(format!("{name}.dre")))` and sets `Mode::Command`. The extension is always added, so `plans.dre` gives `plans.dre.dre`.
  - `NameCancel` sets `Mode::Command`. `save_to` and the document are untouched.
- `history.rs`: none of the name actions are undoable, so they leave history alone. The mode's `Idle` handling mirrors `SavePrompt`, which leaves the selection alone.
- `render/mod.rs` and `layout.rs`: the footer is built from `state.footer()`. In `NamePrompt` mode the name slot of the corner is replaced by the prompt: `type a name` in gray with the `Cursor` placement on its first character when `name` is empty, otherwise `name` with the cursor after the last character. ` • dre` stays. The footer text comes from `State::footer()`, so the prompt variant is computed in the same place as `footer_text` and the layout draws the gray placeholder and the cursor from it.

### How the file gets written

`AutosavingReducer` saves when `save_to` is `Some`, the mode isn't `Insert`, and `history_len != saved_len`. It doesn't need any change.

- **Unnamed diagram with edits:** its edits were never saved, so `history_len != saved_len` when `NameConfirm` sets `save_to`. The write happens on that same Enter.
- **Unnamed diagram with no edits:** the first real edit writes the file.
- **Rename:** history is already saved, so nothing is written on Enter. The next edit writes `<new name>.dre`, and the old file stays on disk untouched. This is accepted.
- **Overwrite:** an existing `<name>.dre` is overwritten by the normal write, with no check.
- **Ctrl-C:** still clears `save_to`, so nothing more is saved.

### Decisions

- **Separate mode:** the two prompts differ in extension handling, empty Enter and quitting. A flag on `SavePrompt` would be two modes in one.
- **Prompt in the corner:** the prompt replaces the name slot, not the hint text. Spec 138's rule that the hint stays visible in the name prompt no longer holds; when we do 138 the hint is shown only when the prompt is closed.
- **`q` prompt:** untouched.

### Tests

- `input`: `n` in Command mode parses to `OpenNamePrompt`; in `NamePrompt` printable keys, Backspace, Enter and Esc parse to the name actions.
- `name_prompt`: typing and backspace edit `name`; Enter with a name sets `save_to` to `<name>.dre` and returns to Command; `plans.dre` gives `plans.dre.dre`; Enter with an empty name leaves the prompt open and `save_to` unchanged; Esc closes the prompt and leaves `save_to` and the document as they were; `n` on a named diagram then Enter changes `save_to` to the new path.
- `history`: the name actions are not undoable.
- `render`: the corner shows the gray placeholder with the cursor at its start when `name` is empty, the typed name with the cursor at its end otherwise, and ` • dre` in both.
- `AutosavingReducer`: naming a diagram that has unsaved edits calls `save` once, and naming one with no unsaved edits doesn't call `save`.
