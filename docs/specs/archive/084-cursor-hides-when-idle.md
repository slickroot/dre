# 084: Cursor hides when idle

## User Story

When I stop pressing keys for a moment while working in the diagram, the block cursor
over the selected box disappears so I can read the box's full label — and it reappears
the instant I press any key.

## Acceptance Criteria

1. In command mode with a box selected, the cursor covering that box's label disappears
   after roughly one second with no keypresses.
2. While the cursor is hidden, the selected box's label is fully readable — the last
   character is never covered.
3. Pressing any key makes the cursor reappear immediately, on the currently selected box.
4. If I go quiet again, it hides again — the cycle repeats.
5. While I'm typing inside a label (insert mode), the caret stays visible and never
   auto-hides.

## Technical Design

Hiding the cursor is a pure state change: there is no `show_cursor` flag, no renderer change,
and no new layout machinery. The existing "no selection, no cursor" path is reused
(`layout::with_cursor` returns placements unchanged when nothing matches; the renderer already
tests `no_cursor_is_drawn_without_a_selection`).

### State (`src/state.rs`)

Add a backup of the selection:

```rust
pub(crate) struct State {
    // ...
    pub(crate) last_selected: Option<Path>,
}
```

- `last_selected` defaults to `None` everywhere and is **only** populated by `hide_idle_cursor`.
- No undo/history interaction: `hide_idle_cursor` mutates `selected`/`last_selected` directly,
  never calls `snapshot`, so an idle hide is not undoable and never pollutes history.

```rust
pub(crate) fn hide_idle_cursor(mut state: State) -> State {
    if state.mode == Mode::Command {
        state.last_selected = state.doc.selected.clone();
        state.doc.selected = None;
    }
    state
}
```

`handle_key` restores the selection before dispatching, so **every** key lands on the box the
cursor was hiding on (acceptance criterion 3):

```rust
pub(crate) fn handle_key(mut state: State, key: &str) -> State {
    if let Some(selected) = state.last_selected.take() {
        state.doc.selected = Some(selected);
    }
    match &state.mode { /* unchanged dispatch */ }
}
```

### Editor loop (`src/main.rs`)

One loop, one timed wait. `terminal.rs` exposes a `poll_read(timeout)` that sleeps up to
`timeout` milliseconds and returns `Some(key)` when a byte is ready or `None` on expiry,
replacing the blocking `read_exact` call.

```rust
while state.running {
    renderer.render(&state.doc, &mut stdout)?;
    renderer.status_line(status(&state).as_deref(), &mut stdout)?;
    stdout.flush()?;

    match terminal::poll_read(1000)? {
        Some(key) if key == INTERRUPT => break,
        Some(key) => state = handle_key(state, &key),
        None       => state = hide_idle_cursor(state),
    }
}
```

- `poll_read(1000)` blocks exactly like `read` today but wakes early on a keypress or after
  roughly one second of silence — measured from the last key, which resets implicitly each
  iteration.
- On idle expiry in `Mode::Command` with a selection, `hide_idle_cursor` clears the selection
  and the next frame renders without the block cursor. Outside command mode (insert, save
  prompt, or nothing selected) it is a no-op, so the caret and the save-prompt `█` never
  auto-hide (criterion 5).
- The first key after a hide restores the selection and immediately runs its command
  (restore-then-run). The cycle repeats: silence hides again (criterion 4).

No renderer, layout, or command-mode code changes.