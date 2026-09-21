# 089: Web cursor hides when idle

## User Story

When Bob stops pressing keys for a moment on dre.elaich.com with a box selected, the
cursor over that box disappears so he can read the full label. It comes back as soon as
he presses a key.

## Acceptance Criteria

1. In command mode with a box selected, the cursor disappears after roughly one second
   with no keypresses.
2. While the cursor is hidden, the selected box's label is fully readable — the last
   character is never covered.
3. Pressing any key makes the cursor reappear immediately, on the box that was selected.
4. If Bob goes quiet again, it hides again — the cycle repeats.
5. While Bob is typing inside a label (insert mode), the caret stays visible and never
   auto-hides.

## Technical Design

The web reuses the terminal's hide/restore rules from spec 084 unchanged. Restore already works
on the web: `Session::press_key` calls `state::handle_key`, which puts `last_selected` back before
dispatching. What the web lacks is the **trigger**. In the terminal, the editor loop owns the
clock (`poll_read(IDLE_TIMEOUT_MS)` times out, then `hide_idle_cursor`). The browser has no such
loop, and `std::time::Instant` still traps on `wasm32-unknown-unknown` (checked on rustc 1.98.1),
so the core stays clock-free and the web crate owns its own timer.

The page never learns about modes, selections, or hiding. It supplies one `onChange` callback.

### Core (`src/`)

- Move the threshold out of `src/editor.rs` into one shared constant, `pub const IDLE_TIMEOUT_MS: u16 = 1000;`
  exported from `dre`. The terminal loop and `WebSession` both read it.
- `Session` (`src/lib.rs`) gets one hook, `pub` and `#[doc(hidden)]` like `run`:

```rust
pub fn go_idle(&mut self) {
    self.state = state::hide_idle_cursor(std::mem::take(&mut self.state));
}
```

  Named for the event ("no key for a while"), not the effect. The mode rules stay in
  `state.rs`: command mode with a selection hides; insert mode, save prompt, and nothing
  selected are no-ops (criterion 5). Returns nothing, same shape as `press_key`.
- `state.rs`, `handle_key`, the renderers, and layout do not change.

### Web (`web/src/lib.rs`)

`WebSession` owns the timer. The timeout closure must mutate the same session `press_key`
mutates, so the session is shared:

```rust
pub struct WebSession {
    session: Rc<RefCell<dre::Session>>,
    idle_timer: Option<IdleTimer>,     // pending setTimeout handle + its Closure
    on_change: js_sys::Function,
}
```

- Constructor takes the callback: `new(on_change: js_sys::Function)`.
- `press_key(key)`:
  1. `clearTimeout` on the pending timer, if any.
  2. Run the key on the session (restores the selection if hidden, criterion 3).
  3. Schedule a new `setTimeout(IDLE_TIMEOUT_MS)`. When it fires it calls `go_idle()` on the
     shared session, then `on_change()`.
- Cancel-and-reschedule on every key gives "one second since the last key" with no polling and
  no elapsed-time arithmetic, and the cycle repeats naturally (criterion 4).
- `on_change` fires unconditionally when the timer expires. A redundant redraw in insert mode
  costs one SVG render per pause, so `go_idle` needs no `bool` return.
- New dependencies for `dre-web`: `js-sys`, `web-sys` (the `Window` feature for
  `set_timeout_with_callback_and_timeout_and_arguments_0` / `clear_timeout_with_handle`).
- `svg()` and `extent()` borrow the shared session; the hidden state renders through the existing
  "no selection, no cursor" path, so the label is fully readable (criterion 2).

### Page (outside this repo)

One line: pass a callback that re-renders (`new WebSession(() => render())`). No timer, no mode
knowledge.

### Tests

- `state.rs`: existing 084 tests cover the hide/restore rules.
- `src/lib.rs`: `go_idle` after selecting a box hides the cursor, and the next `press_key`
  restores it on the same box; `go_idle` in insert mode leaves the caret.
- Timer wiring (`setTimeout`, `clearTimeout`, callback) is browser-only. Keep the closure body
  thin (`go_idle`, then `on_change`) so the logic under test lives in `Session`.
