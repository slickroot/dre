# Status bar shows mode

## User Story

Bob opens dre and always sees a status bar at the bottom of the screen, visually separated from the canvas by a different background color, showing whether he's currently EDITING or COMMANDING.

## Acceptance Criteria

- The status bar is visible at all times, not just during the Save-as prompt.
- The status bar's background color is visibly different from the terminal's default background, separating it from the canvas.
- When in Insert mode, the status bar shows "EDITING".
- When in Command mode, the status bar shows "COMMANDING".
- When in the Save-as prompt, the status bar shows "Save as: ..." as it does today (unchanged).

## Technical Design

`TerminalRenderer::render` (the `Renderer` trait method) becomes the single
entry point for drawing a frame. Internally it calls two private methods:

- `render_diagram(&state, out)` — the existing box/arrow/label/cursor drawing
  (today's `render` body).
- `render_status_line(&state, out)` — draws the status bar, every frame,
  unconditionally.

`editor.rs`'s `edit` loop calls `renderer.render(&state, out)?` once per
frame and no longer calls a separate status line method. The `status()`
function and its local `CURSOR` const in `editor.rs` are removed.

`render_status_line` gets its text from a small private helper in
`terminal.rs`:

```rust
fn status_text(mode: &Mode) -> String {
    match mode {
        Mode::Insert => "EDITING".into(),
        Mode::Command => "COMMANDING".into(),
        Mode::SavePrompt { filename } => format!("Save as: {filename}{CURSOR}"),
    }
}
```

(`CURSOR` is the block-cursor char already defined in `terminal.rs`.)

The text is padded to terminal width as today, then wrapped in reverse-video
so the status bar's background is always visibly different from the canvas,
regardless of the terminal's color theme:

```
\x1b[{rows};1H\x1b[7m{line}\x1b[0m
```

Reverse video was chosen over an explicit background color because it's
theme-safe, and no other SGR codes exist elsewhere in the renderer that it
could conflict with.

