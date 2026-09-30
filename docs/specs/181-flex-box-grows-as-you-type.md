# Flex box grows as you type

## User Story

Lina runs `dre-flex`. She sees one empty box on the canvas. She types "Hello", and the letters appear inside the box as she types, with the box growing to fit them. She mistypes a letter, presses Backspace, and it's gone. Happy with her box, she presses Ctrl-C to leave.

## Acceptance Criteria

- Running `dre-flex` opens a canvas with one empty box.
- `dre-flex` starts in WRITE mode.
- Each typed character appears inside the box right away.
- The box grows to fit the text as Lina types.
- Backspace removes the last character, and the box shrinks to fit.
- Ctrl-C quits without saving.

## Technical Design

Decisions:

- **A second binary in the `dre` crate.** `src/bin/dre-flex.rs` calls `dre::flex::run()`. Everything else lives in a new `src/flex/` module (`#[cfg(not(target_arch = "wasm32"))]`). It reuses `tty`, `kitty`, `TerminalRenderer` and the `view` primitives unchanged. `State`, `Mode`, `Editor` and `DreController` are untouched.
- **One box, one string.** The state holds a single `text`, not a box collection. Structure gets added when a story asks for a second box.
- **No cursor.** Edits only ever happen at the end of the text, so `state::text_edit` is not reused.
- **No effects, no store.** Nothing is loaded or saved, so `reduce` returns only the next state.
- **No footer.** Nothing is drawn except the box. The mode lives only in `FlexState`.

### `flex::state`: what it knows and does

```rust
pub(crate) enum FlexMode { Write }            // Move arrives in spec 182
pub(crate) struct FlexState { text: String, mode: FlexMode, running: bool }

impl Default for FlexState   // text "", Write, running
pub(crate) fn reduce(state: FlexState, key: &str) -> FlexState
```

- `"\x03"` (Ctrl-C) sets `running = false`.
- `"\x7f"` (Backspace) pops the last `char`. On an empty text it does nothing.
- A single printable char is pushed onto `text`.
- Anything else (escape sequences, other control bytes, `tty::RESIZE`) leaves the state unchanged.

### `flex::view`: from state to scene

```rust
pub(crate) fn scene(state: &FlexState, window: Area) -> Scene<'_>
```

- The whole window is the canvas. The scene is a single `(window, placements)` entry.
- One `PlacementNode::Box` (`ALL_SIDES`, `border: BORDER`, not rounded) with width `interior(text) + 2` and height `BOX_HEIGHT`. A `Label` sits at `label_centre`, on row `BOX_HEIGHT / 2`. `view::centre` centres both in the window. The growing and shrinking come from `interior()`. An empty box is 3 cells wide.

### `flex::run`: the loop and its collaborators

```rust
pub(crate) trait FlexScreen { fn render(&mut self, state: &FlexState) -> io::Result<()>; fn resize(&mut self) -> io::Result<()>; }
pub(crate) fn run_loop(keys: &mut dyn KeySource, screen: &mut dyn FlexScreen) -> io::Result<()>
pub fn run() -> ExitCode
```

- `run_loop` starts from `FlexState::default()`. While running, it renders, reads a key, calls `screen.resize()` on `tty::RESIZE` and otherwise calls `reduce`. It is tested with `MockKeySource` and a mocked `FlexScreen`.
- `TerminalFlexScreen { renderer: TerminalRenderer, out: Stdout }` renders `flex::scene(state, renderer.area())` and flushes. On resize it calls `renderer.on_resize(tty::probe()?)`.
- `run()` repeats the setup in `editor::bootstrap::run`: `kitty::require`, `tty::probe`, `GlyphCache` and `TerminalRenderer`, `RawMode::enter`, `install_resize_pipe`. It wires in `TtyKeySource` and `TerminalFlexScreen`. `KeySource` and `TtyKeySource` move from `editor::controller::key_source` up to where both binaries can see them (visibility change only, no behaviour change).
- There's no LED flash and no command status line.

### Tests

- `reduce`: typing appends, Backspace pops (including on empty text), Ctrl-C stops running, escape sequences are ignored.
- `scene`: the box width follows the text (`""` → 3, `"Hello"` → 7), the label is inside the box, and nothing but the box and its label is placed.
- `run_loop`: it renders before each key, stops after `"\x03"`, and resizes on `tty::RESIZE` without reducing.
