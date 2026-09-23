# Status line shows file and box info

Marouane is editing a diagram in dre. He glances at the status line at the bottom of the screen and sees everything at a glance: on the left, the current mode, the filename, and a `[+]` if he has unsaved changes; on the right, how many boxes are in the diagram and the app name "dre". He makes an edit, sees the `[+]` appear, saves, and watches it disappear — all without leaving the keyboard.

## Acceptance Criteria

- Status line shows, left-aligned: `mode | filename`
- If there are unsaved changes, `[+]` appears right after the filename; otherwise it's omitted
- Before the first save, filename shows the default `diagram.dre`
- Status line shows, right-aligned: `N boxes . dre` (always plural, e.g. "1 boxes", "2 boxes")
- Left and right groups are separated by blank space filling the remaining width

## Technical Design

### `State` gains a `dirty` flag

Add `dirty: bool` to `State` (default `false`). `state::snapshot()` sets `dirty = true` on every call. `snapshot()` already runs at exactly the moments a mutating command is about to change the document — once per `is_undoable(command)` command in `command_mode::reduce`, and once in `insert_mode::reduce` for `CommitAndAddChild` — so it already matches the set of actions that should mark the document as having unsaved changes. Individual keystrokes while typing a label (`Append`/`Backspace` in `insert_mode`) don't need their own snapshot/dirty call: entering edit mode already snapshotted once, so `dirty` is already `true` by the time typing starts. `dirty` is not cleared anywhere else — saving only happens when the process exits (`editor::open`), so there is no in-session point where it would flip back to `false`; that stays out of scope for this story.

### `editor_info()` becomes `state::status_line()`, a render-ready view model

Replace `editor_info(&State) -> EditorInfo` with `state::status_line(&State) -> StatusLine`, where:

```rust
pub struct StatusLine {
    pub left: String,
    pub right: String,
}
```

Both fields are fully formatted display text — no further business logic needed by a caller, only layout (padding/truncation to a terminal width, which is renderer-specific and stays out of `state.rs`).

- `Mode::Command` / `Mode::Insert`: `left = "{COMMANDING|EDITING} | {filename}{marker}"`, where `filename` is `state.save_to.clone().unwrap_or(DEFAULT_FILENAME.to_string())` (shown as-is, full path if one was given) and `marker` is `"[+]"` immediately after the filename when `state.dirty`, otherwise omitted. `right = "{n} boxes . dre"` where `n` comes from the existing recursive box-count walk (kept as a private helper in `state.rs`, reused by `status_line`).
- `Mode::SavePrompt { filename }`: `status_line` special-cases this too, so the renderer always calls one function regardless of mode. `left = "Save as: {filename}{CURSOR}"` (unchanged text from today's `status_text`), `right = String::new()`.

The `EDITING`/`COMMANDING`/`"Save as: ..."` mode-text mapping and the `CURSOR` glyph move from `render/terminal.rs::status_text` into `state.rs` as part of `status_line`, since they're now part of the rendered view model rather than terminal-only formatting. `BLANK` (fill character) and the truncate-to-`cols` logic stay in `render/terminal.rs`, since those depend on terminal width.

### `render_status_line` assembles the two halves

`TerminalRenderer::render_status_line` calls `state::status_line(state)`, then builds the line as `left + padding + right` where `padding` fills the remaining terminal width with `BLANK` (saturating to zero if `left.len() + right.len() >= cols`), and reuses the existing chain-and-truncate-to-`cols` step as a fallback/overflow guard, matching current truncation behaviour when the assembled line doesn't fit.

### Web binding

`web/src/lib.rs`'s `info(key)` (currently a generic `editor_info` key lookup) is replaced by exposing `status_line().left` / `.right` directly, since the view model itself is now the thing to render rather than a bag of individual facts.
