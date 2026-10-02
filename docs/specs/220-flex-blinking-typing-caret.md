Doug is typing a label in dre-flex. As soon as he enters typing mode a `|` blinks at the end of his text, right where the next character would land. It blinks once per second and sits in the empty cell after the text, so nothing moves. When Doug backspaces to empty, the `|` is still there, blinking. The moment he leaves typing mode, it's gone.

## Acceptance Criteria

1. Pressing the key that starts typing on a box with no text shows the `|` immediately, before any character is typed.
2. The `|` sits in the cell immediately after the last typed character, and typing another character leaves the `|` one cell further along, with no other text shifting.
3. The `|` is visible for roughly half a second and hidden for roughly half a second, on that repeating cycle.
4. Backspacing to an empty label leaves the `|` visible in that position, still blinking.
5. The `|` blinks only in typing mode; once Doug leaves typing mode it is not drawn.
6. The `|` does not occupy a cell — nothing after it moves.

## Technical Design

### Why this replaced terminal-driven animation

The first cut of this spec had the terminal drive the blink: transmit a transparent root frame and
one opaque `|` frame, then let `a=a,i=<id>,r=1,z=500,s=3,v=1` loop them. **It was implemented, and
it does not work.** In WezTerm the caret renders solid and never blinks; in Ghostty it never
renders at all. The reason is already recorded in
[`209-flex-a-box-grows-into-place.md`](archive/209-flex-a-box-grows-into-place.md), quirks 2 and 4,
both verified by hand in WezTerm:

- **WezTerm ignores `a=a`**, so it just loops whatever frames exist.
- **WezTerm skips a root frame with a gap of 0**, so the transparent root is skipped too.

That leaves one surviving frame. A one-frame loop is a constant: lit, forever. And for Ghostty,
whose animation support predates nothing usable, the spec's own trade was *"a terminal that never
plays frames shows only the root, hence an invisible caret. That trade is accepted"* — which is
precisely the wrong trade when the root frame is invisible.

The old spec rejected `X=1` replacement frames as *"unverified on WezTerm, which is the terminal
this change is for."* That reasoning is now void: WezTerm does not render this animation at all, so
WezTerm's opinion about `X=1` was never the deciding factor.

So the terminal cannot be the clock here. dre draws the caret itself.

### The tick lives in the run loop, not in a thread

`flex::run_loop` (`src/flex/mod.rs:59-74`) renders, then blocks on `keys.next_key()`, which is
`tty::read_key`, which polls with `PollTimeout::NONE` (`src/tty.rs:124-150`). That poll is the only
thing standing between the loop and a half-second timer, so that is where the timeout goes.

A **separate thread** was considered and rejected:

- `Frame::into_bytes` wraps every frame in synchronized output (DEC 2026) and writes to `Stdout`.
  A second thread writing escape sequences would interleave with a frame in flight and tear the
  block.
- `soft_clear` (`a=d,d=a`) runs at the top of every frame (`src/render/terminal.rs:500`), so the
  thread's placement would be deleted by the very next keystroke. The app would be repainting
  anyway, so the thread would have nothing to add.
- The thread would need the terminal writer behind a lock to avoid the first problem. At that
  point it is the same thing as waking the main loop, with more moving parts.

The tick is the main loop's own work: it owns `state` and the renderer, and "redraw the frame with
the caret on, then with it off" is already a single call it makes every keystroke.

### A timeout only while typing

The cost of an app-side tick is a whole-frame repaint twice a second. The old spec rejected this
flatly because it *"repaints the whole frame 4×/sec forever."* Two refinements make it bounded:

- The poll timeout is **finite only in Write mode**. In every other mode `read_key` still blocks on
  `PollTimeout::NONE` and dre is exactly as idle as it was before.
- While in Write mode the loop **renders only when the phase flips**, and the phase flips on each
  tick, so it is one repaint per half-second and no more.

Leaving Write mode stops the ticking immediately, and the next render omits the caret.

### The phase is a local, not a field

The old spec worried that an app-side phase *"would have to be excluded from state equality."* It
does not: the phase is a `bool` local to `run_loop`, threaded into rendering as an argument. It is
never stored on `FlexState`, so `PartialEq for FlexState` (`src/flex/state.rs:106-110`) is
untouched and the ~100 existing `assert_eq!`s keep passing.

`run_loop` reads it as: render, ask for a key with a timeout if in Write mode, toggle the phase and
render again if the returned sentinel is a tick.

### A tick is a sentinel key, like a resize

`tty::read_key` already signals a non-key by returning a sentinel string — `tty::RESIZE` comes back
down the same `io::Result<String>` as any keystroke. The tick follows that precedent: `read_key`
returns `tty::TICK` when `poll` times out.

This keeps the blast radius small. `KeySource::next_key(&self) -> io::Result<String>`
(`src/key_source.rs:6-9`) does not change shape, so `#[automock]` and every existing mock
expectation still compile. `run_loop` handles `TICK` the way it handles `RESIZE`: before
`state::reduce`, so a tick is never reduced as a keystroke.

`read_key` gains a timeout parameter. `TtyKeySource` carries it.

### The caret is a plain glyph image again

With no animation there are no frames, so the whole two-frame apparatus goes away: `kitty::blink`,
`Image::Blinking`, `Frame::place_blinking`, `BLINK_GAP_MS`, and the transparent-root construction.
The caret reverts to what a glyph already is — a cached image, placed on lit frames and simply not
emitted on dark ones. `soft_clear` guarantees that "not emitted" means "not on screen".

`TerminalRenderer::draw_typing_caret` keeps the `CaretKey` = `{colour, bold}` cache
(`HashMap<CaretKey, ImageId>`, mirroring `GlyphKey` and `glyph_images`) and gets the glyph from
`glyph_source.glyph('|', colour, bold)`, exactly as `draw_label` obtains each character.

On first sight it goes through the ordinary `place_fresh` → `kitty::show` path. That matters: the
old build went through `kitty::blink`, which transmits with no cursor positioning, so the image
only became visible from the *next* frame's `a=p`. `kitty::show` moves the cursor, so the caret
lands on the right cell on the very first render. Acceptance criterion 1 now holds literally.

### Where the node is decided

Unchanged from the first cut, and still correct:

- `PlacementNode::TypingCaret { colour: Rgb, bold: bool }` (`src/view.rs`). Colour and bold only,
  no position logic. `is_decoration()` includes it, so criterion 6 holds — it never affects measure
  and never reserves a cell.
- `flex::view::paint` (`src/flex/view.rs`) emits it only when `state.mode == FlexMode::Write` and
  `path == state.selected`, which is criterion 5.
- Position is `text.x + text.chars().count()`, not `text.x + text.width`. Empty text gives `x + 0`,
  the cell where the first character lands, which is what criterion 1 asks for and what survives
  criterion 4. `interior`'s `.max(1)` (`src/view.rs:114-116`) stays a layout concern. No clamp:
  `Frame::crop` decides visibility.

The one addition: `flex::view::scene` takes the lit phase and `paint` emits the node only when it
is lit. `scene` already takes `new: &HashSet<Vec<usize>>` for the grow animation, so a per-frame
non-state parameter is the established shape here.

`FlexScreen::render` gains the same parameter, which is a mechanical change to the mock in
`src/flex/mod.rs`'s tests.

### SVG export is still unreachable

`SvgRenderer` is called from exactly one place, `cli::export`, and it renders `view::body` for a
**legacy** `State` (`src/cli.rs:86-96`). `flex::view::scene` is never handed to it. The arm for
`TypingCaret` in `src/render/svg.rs` exists only for exhaustiveness and emits nothing. A frozen
cursor has no meaning, and a static `|` would be indistinguishable from a pipe in the label.

### Constants

- The half-period, the value passed to `PollTimeout`, and the value the tests assert against:
  one named constant in `src/flex/mod.rs`. Tests assert against the constant, never a literal.

### Tests

Framework is inline `#[test]` with plain `assert!`/`assert_eq!`, no snapshots.

`src/flex/mod.rs` — the run loop and its mock:

- `a_tick_re_renders_without_reducing_a_key`
- `the_phase_toggles_on_each_tick`
- `no_tick_is_asked_for_outside_write_mode`
- `entering_write_mode_renders_the_caret_lit_before_any_keystroke`

`src/tty.rs`:

- `read_key_times_out_with_a_tick_when_no_byte_arrives`
- `a_byte_arriving_before_the_timeout_is_read_as_a_key`

`src/flex/view.rs` — unchanged from the first cut, all still valid:

- `write_mode_puts_a_typing_caret_one_cell_past_the_label`
- `an_empty_label_puts_the_caret_in_the_first_cell`
- `typing_the_next_character_moves_the_caret_right_by_one_cell`
- `backspacing_to_empty_leaves_the_caret_in_the_first_cell`
- `move_mode_draws_no_typing_caret`
- `the_caret_placement_is_one_cell_wide_and_does_not_widen_the_label`

plus, for the new phase argument:

- `the_dark_phase_draws_no_typing_caret`

`src/render/terminal.rs`:

- `the_typing_caret_is_a_cached_glyph_image`
- `the_typing_caret_is_placed_on_a_lit_frame_and_omitted_on_a_dark_one`
- `the_typing_caret_is_re_placed_and_not_re_transmitted` — a second lit render at a new column
  emits `a=p` and no `a=T`
- `the_typing_caret_is_the_same_image_for_the_same_colour`
- `the_legacy_caret_is_still_a_solid_block` — regression guard for `draw_caret`

The tests that asserted on two-frame animation bytes and on canvas alpha are deleted with the
apparatus they tested.

### Rejected

- **Terminal-driven animation** (transparent root, one lit frame, `a=a,r=1,z=` loop) —
  implemented and measured: steady on WezTerm, invisible on Ghostty. The failure mode is recorded
  under "Why this replaced terminal-driven animation".
- **`X=1` replacement frames** — would make a transparent off-frame erase a lit root, which is the
  correct per-protocol way to express this. Still unverified on WezTerm, and no longer necessary
  once the app draws the frame itself.
- **A thread driving `kitty::blink`** — races the synchronized-update block, and `soft_clear`
  deletes its placement on the next keystroke anyway. Reasons above.
- **Storing the phase on `FlexState`** — would need excluding from `PartialEq`, breaking the ~100
  existing state assertions. A `run_loop` local threaded through `render` avoids all of it.
- **Ticking in every mode** — the old spec's objection to an app-side tick was that it repaints
  "4×/sec forever". A finite timeout only in Write mode makes it twice a second only while typing.
- **Reusing `PlacementNode::Caret`** with extra fields — drags the legacy solid block's meaning
  into flex and forces every legacy `Caret` construction site to change.
- **`text.x + text.width` as the caret origin** — puts the caret one cell right of where typing
  lands when the label is empty, so it jumps left on the first keystroke.