# The editor waits for input

This is a technical spec, not a user story: it has no user-facing behaviour.

## Problem

The terminal editor wakes up once a second with no input and renders the whole frame again.

`TtyKeySource::next_key` calls `tty::poll_read` with `IDLE_TIMEOUT_MS = 1000`. When the poll times out it returns `Ok(None)`. `DreController::run` renders at the top of every loop, so every timeout redraws the screen. The web session does the same thing with a `setTimeout` that calls `Session::go_idle`.

The timeout was added for spec 084 (the cursor hides when idle). Commit `3200800` ("Selection no longer hides after idling") removed that feature. `state::reduce(state, None)` is now a no-op, so each wake-up redraws an unchanged state. The idle `None` still runs through the key source, the controller, the reducer, `Session` and `WebSession`, and none of them uses it.

## Acceptance Criteria

- With no keypress and no resize, the terminal editor does not wake up and does not render.
- Each keypress and each resize still renders exactly as today.
- The web editor sets no timer. It changes only when a key is pressed.
- There is no idle `None` key anywhere: `IDLE_TIMEOUT_MS`, `Session::go_idle` and the web idle timer are gone.

## Technical Design

The idle "no key" idea is removed from the whole pipeline, not only from the terminal loop. Keys are always present, so every signature that took or returned `Option<String>` / `Option<&str>` for a key now takes or returns the key itself.

### `tty` (`src/tty.rs`)

`poll_read(fd, resize_fd, timeout_ms) -> io::Result<Option<String>>` becomes:

```rust
pub(crate) fn read_key(fd: RawFd, resize_fd: RawFd) -> io::Result<String>
```

- `poll` is called with `PollTimeout::NONE`, so it blocks until a byte or a resize arrives. The `ready == 0` branch goes away.
- The resize path returns `Ok(RESIZE.to_string())`, and the key path returns `Ok(key)`.
- EOF is still `Err(UnexpectedEof)`. EINTR is still retried.
- `ESCAPE_TIMEOUT_MS` and `read_byte_within_escape_timeout` do not change. Telling a lone Esc from an escape sequence is real behaviour, not polling.

Tests:
- Delete `poll_read_returns_none_when_no_byte_arrives_within_the_timeout`.
- Rewrite `poll_read_drains_the_resize_byte_so_it_is_not_reported_twice`: write the resize byte and then a key. The first `read_key` returns `RESIZE` and the second returns the key, not `RESIZE` again.
- Rename the other `poll_read_*` tests to `read_key_*`, drop the timeout argument, and unwrap `String` instead of `Option<String>`. The `poll_read_key` helper becomes a direct `read_key(...).unwrap()`.

### `KeySource` (`src/editor/controller/key_source.rs`)

```rust
pub(crate) trait KeySource {
    fn next_key(&mut self) -> io::Result<String>;
}
```

`TtyKeySource::next_key` calls `tty::read_key(self.fd, self.resize_fd)`. The `IDLE_TIMEOUT_MS` import goes away.

### `Reducer` (`src/editor/controller/reducer.rs`) and `state::reduce` (`src/state/mod.rs`)

```rust
fn reduce(&self, state: State, key: &str) -> (State, Vec<Effect>);
pub fn reduce(state: State, key: &str) -> (State, Vec<Effect>);
```

- Inside `state::reduce`, `key.and_then(|key| input::parse(&state, key))` becomes `input::parse(&state, key)`. The rest of the function is unchanged: an unparsed key still leaves the state as it is, and the save and `led_flash` logic stays the same.
- The explicit lifetime and the `#[allow(clippy::needless_lifetimes)]` on the trait were only needed for mocking `Option<&str>`. Drop them if `automock` accepts a plain `&str`.
- Delete the test `missing_input_is_a_no_op`. `unknown_key_returns_the_state_unchanged` already covers a key that does nothing.
- Every other caller that passed `Some(key)` now passes `key`.

### `DreController::run` (`src/editor/controller/mod.rs`)

The loop keeps its shape: **one render per event** (a key or a resize). It still renders at the top of each pass. There is no change detection: a key that changes nothing still redraws once. That cost only comes with a real keypress, and spec 171's synchronized output makes the redraw invisible. Adding `PartialEq` to `State` or a dirty flag is not worth it for this.

Simplifications that follow from the key always being present:
- `if key.is_some() && *state.mode() == Mode::Command` becomes `if *state.mode() == Mode::Command`.
- `key.as_deref().and_then(|k| state::command_label(&state, k))` becomes `state::command_label(&state, &key)`.
- `self.reducer.reduce(state, key.as_deref())` becomes `self.reducer.reduce(state, &key)`.

Tests:
- `keys_reading` takes `Vec<&str>`, and every `Some("x")` in the tests becomes `"x"`.
- Delete `an_idle_none_is_passed_to_reduce_as_none` and `an_idle_none_poll_in_command_mode_does_not_flash`.
- Mock `reduce` matchers that checked `key.is_none()` or `key == Some(..)` compare against the `&str` directly.

### `Session` (`src/lib.rs`)

- Delete `pub const IDLE_TIMEOUT_MS`.
- Delete `Session::go_idle`.
- `press_key` calls `state::reduce(..., key)`.

### `WebSession` (`web/src/lib.rs`)

All the idle timer code is deleted. `WebSession` holds the session directly:

```rust
#[wasm_bindgen]
pub struct WebSession {
    session: dre::Session,
}

#[wasm_bindgen]
impl WebSession {
    #[wasm_bindgen(constructor)]
    pub fn new() -> WebSession;
    pub fn press_key(&mut self, key: &str);   // self.session.press_key(key)
    pub fn svg(&self, cols: i32, rows: i32) -> String;   // reads self.session.state()
}
```

- Delete `IdleTimer`, both versions of `schedule_idle` and `clear_timeout`, the `idle_timer` and `on_change` fields, and the `Rc`/`RefCell` imports.
- The JS constructor becomes `new WebSession()`. Existing callers that still pass a callback keep working, because JS ignores extra constructor arguments.
- The landing page (`dre-website/src/main.ts`) already calls `draw()` right after every `press_key` (lines 57, 109 and 134). The `on_change` it passes at line 19 only redraws the same frame when the idle timer fires, and the one at line 127 is `() => {}`. Nothing is lost when the callback goes away. As a follow-up in that repo, both calls become `new WebSession()`.
- Drop `js-sys` and `web-sys` from `web/Cargo.toml` if nothing else uses them.
- The test helper `session()` becomes `WebSession::new()`.

### Collaborators after the change

```
TtyKeySource --read_key--> tty          (blocks; String)
DreController --next_key--> KeySource   (String)
DreController --reduce(&str)--> Reducer --> state::reduce
DreController --render--> Screen        (once per key or resize)
Session / WebSession --press_key(&str)--> state::reduce
```
