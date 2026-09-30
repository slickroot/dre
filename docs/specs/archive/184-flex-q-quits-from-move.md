# Flex: q quits from MOVE

## User Story

Lina has typed "Hello" and pressed Enter, so she's in MOVE. She's done for now, presses `q`, and `dre-flex` closes, the same way Ctrl-C would.

## Acceptance Criteria

- Pressing `q` in MOVE mode quits `dre-flex` without saving.

## Technical Design

Decisions:

- **Quitting is an effect, not state.** `reduce` stops setting `running = false`. It returns a `FlexEffect::Quit` instead, and `run_loop` acts on that effect by returning. The state says what the box is. Stopping is something the loop does.
- **`running` is dropped from `FlexState`.** Ctrl-C moves to the same effect, so `q` quits "the same way Ctrl-C would" by construction, and the loop has one way to stop. `FlexState` keeps only `text` and `mode`. This replaces 181's "no effects" decision and its `running` field.
- **At most one effect per key: `Option`, not `Vec`.** The main editor returns `Vec<Effect>`, but a flex key can only ever produce one `Quit`. `Option` rules out states that can't happen, like `[Quit, Quit]`. Switch to `Vec` when a key needs more than one effect.
- **`q` is exactly `"q"`**, like the main editor's `KeyMatch::Exact("q")` (`state/input.rs`). It gets one arm in `move_key`. In WRITE, `"q"` is still a printable char and is typed into the box.
- **No view changes.** `flex::view` never read `running`.

### `flex::state`: what it knows and does

```rust
pub(crate) enum FlexMode { Write, Move }
pub(crate) struct FlexState { text: String, mode: FlexMode }   // `running` removed
pub(crate) enum FlexEffect { Quit }

pub(crate) fn reduce(state: FlexState, key: &str) -> (FlexState, Option<FlexEffect>)
fn write_key(state: FlexState, key: &str) -> (FlexState, Option<FlexEffect>)
fn move_key(state: FlexState, key: &str) -> (FlexState, Option<FlexEffect>)
```

- `reduce`: `"\x03"` returns `(state, Some(Quit))` with the state unchanged, in any mode. Other keys go to `write_key` or `move_key`, as in 182.
- `write_key`: behaviour unchanged from 181/182. It always returns `None`.
- `move_key`: `"q"` → `(state, Some(Quit))`. `"i"` → `Write` (183), `None`. Any other key → the state unchanged, `None`.

### `flex::run`: what changes

```rust
pub(crate) fn run_loop(keys: &mut dyn KeySource, screen: &mut dyn FlexScreen) -> io::Result<()>
```

- The loop runs until an effect says stop, not `while state.running`. It renders, reads a key, calls `screen.resize()` on `tty::RESIZE` and otherwise calls `reduce`. On `Some(FlexEffect::Quit)` it returns `Ok(())` straight away, with no further render. Nothing is saved.
- `run()` and `TerminalFlexScreen` are unchanged.

### Tests

`reduce` unit tests:

- In `Move` with text `"Hello"`, `"q"` returns `Some(Quit)` and leaves the state unchanged (still `"Hello"`, still `Move`).
- In `Write`, `"q"` appends `'q'` to the text and returns `None`.
- `"\x03"` returns `Some(Quit)` in both `Write` and `Move`. This replaces the 181/182 "stops running" tests.
- Existing typing, Backspace, Enter and `i` tests also assert that the effect is `None`.

`run_loop` tests (`MockKeySource`, mocked `FlexScreen`):

- `"\r"` then `"q"`: it renders before each key, then returns after `"q"` without rendering again.
- The existing "stops after `\x03`" test now goes through the effect.
