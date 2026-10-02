Doug is typing a label in dre-flex. As soon as he enters typing mode a `|` blinks at the end of his text, right where the next character would land. It blinks once per second and sits in the empty cell after the text, so nothing moves. When Doug backspaces to empty, the `|` is still there, blinking. The moment he leaves typing mode, it's gone.

## Acceptance Criteria

1. Pressing the key that starts typing on a box with no text shows the `|` immediately, before any character is typed.
2. The `|` sits in the cell immediately after the last typed character, and typing another character leaves the `|` one cell further along, with no other text shifting.
3. The `|` is visible for roughly half a second and hidden for roughly half a second, on that repeating cycle.
4. Backspacing to an empty label leaves the `|` visible in that position, still blinking.
5. The `|` blinks only in typing mode; once Doug leaves typing mode it is not drawn.
6. The `|` does not occupy a cell — nothing after it moves.
7. A blink changes the caret cell and nothing else: no other text is rewritten and no other image is re-placed.

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
whose animation support does not play these frames, the old spec's own trade was *"a terminal that
never plays frames shows only the root, hence an invisible caret. That trade is accepted"* — the
wrong trade when the root frame is invisible.

The old spec rejected `X=1` replacement frames as *"unverified on WezTerm, which is the terminal
this change is for."* That reasoning is void: WezTerm does not render this animation at all, so
WezTerm's opinion about `X=1` was never the deciding factor.

So the terminal cannot be the clock, and dre draws the caret itself.

### A tick is one placement, not a frame

The tick does **not** rebuild the scene. `Frame::into_bytes` (`src/render/terminal.rs:491-509`)
emits, in order: home cursor, every text row of the canvas, `soft_clear`, then one `a=p` per
image. Driving the caret through that path means rewriting every character cell of the diagram
plus every image placement twice a second, to change one cell — and the text rows dominate,
because box-drawing characters are three bytes each in UTF-8.

So a tick writes **exactly one escape sequence** and nothing else:

```
lit  -> dark   a=d,d=i,i=<caret>,p=<placement>
dark -> lit    a=p,i=<caret>,p=<placement>,z=<depth>
```

The lowercase `d=i` deletes one placement *"without necessarily freeing up the stored image
data"*, so re-placing costs no retransmission and the glyph stays cached. The placement is
identified by the `(i, p)` pair, so the caret gets its own placement id and never collides with
another image's. It is wrapped in synchronized output (DEC 2026) like every other update
([`171`](archive/171-each-frame-is-shown-whole.md)), which costs twenty bytes and keeps the
no-tearing guarantee uniform.

kitty documents why a stable placement id is the right identity: *"If you send two placements with
the same image id and placement id the second one will replace the first. This can be used to
resize or move placements around the screen, without flicker."*

`kitty` gains `delete_placement(id, placement)` beside the existing `delete`. `place` already emits
the lit half and already moves the cursor, so it needs no change.

### The remembered cell is valid only between ticks

This is the whole correctness argument, and it rests on one fact: **`soft_clear` is `a=d,d=a`, so
it deletes every placement on screen, and it runs at the top of every full render**
(`src/render/terminal.rs:500`). No full frame can leave a stale placement behind, and no full
frame can leave the remembered cell stale either, because a full render is exactly what resets it.

The renderer therefore holds:

- `typing_caret: Option<TypingCaretCell>` — the image id and the cropped `col`/`row` where the
  caret belongs, or `None` when the scene has no caret in it. **Phase-independent:** it records
  where the caret is, not whether it is currently lit.
- `typing_caret_lit: bool` — the phase the renderer last emitted.

Rules, and they are all of them:

- A **full render** always places the caret as an ordinary placement and always records
  `typing_caret` from the scene, recording the cell only when the placement survived `crop`, so an
  off-screen caret is `None`. It then sets `typing_caret_lit` from the phase it was handed, and
  **when that phase is dark it appends the `delete_placement` into the same frame**.

The caret's placement id is **reserved, not allocated**. `Frame::placement_ids()` numbers each
frame's placements from a counter, so a full render would otherwise hand the caret a different id
every time and the tick's `d=i,i=<caret>,p=<placement>` would never name the placement a full
render created — the caret would never hide, and re-placing would add a duplicate. So the full
render emits the caret **at** the reserved constant, which is why `Placed` carries an optional
placement id and every other image still takes a counter id. The constant is chosen out of reach of
the counter.
- `kitty::show` transmits and displays in one `a=T`, which is why the caret cannot use it: it carries
no `p=`, so a first-sight caret placement lands on kitty's default placement id while every
`delete_placement` and every tick names the reserved constant. The delete would then target a
placement the terminal never created, the default-id placement would survive, and the caret would
be stuck visibly lit. So the caret's first sight is **two** commands, `a=t` to transmit the glyph
without displaying it, then `a=p,i=<caret>,p=<reserved>` to place it at the reserved id. From then
on it is an ordinary cached placement, and **first sight looks exactly like every later frame**.

`Image::Fresh` and `kitty::show` keep their current behaviour, because every other caller — the
legacy caret, box fills, brackets — is unaffected by placement identity and changing them all
would be a much wider change than this one needs. The caret gets its own path alongside them.

The alternative, threading `p=` through `a=T`, was not used: whether a transmit-and-display accepts
a placement id is not verified here, and this repo's rule is to avoid an unverified key rather than
branch on it. `a=t` and `a=p` are both unambiguous in the protocol.

A **tick** does nothing at all when `typing_caret` is `None`, or when the requested phase equals
  `typing_caret_lit`. Otherwise it emits the single delete or the single place, and updates
  `typing_caret_lit`.
- Nothing else may touch either field. In particular there is no delete on leaving Write mode,
  because the scene there has no caret in it, so `typing_caret` is `None` and `soft_clear` has
  already wiped the placement.

A dark full render therefore emits **two** commands, the place and the delete, against the tick's
one. That is the price of the phase-independent cell, and it buys three things: `blink` always has
a cell to restore, so the caret can never be lost; the image is always transmitted on first sight,
so there is no fresh-versus-cached subtlety to track; and `draw_typing_caret` needs no phase at
all. Both commands land inside the frame's single synchronized-output block, so the terminal applies
them atomically and the caret never visibly appears. A full render already emits hundreds of
commands, so two more is not a cost, and the one-command-per-tick property of criterion 7 is
untouched.

The alternative — record the cell without placing, and track whether the image has been transmitted
so `blink` knows to use `place_fresh` — needs an extra field and a branch in `blink` for a case
that only arises on a very first dark render. It was rejected as more state for the same result.

### The phase lives in the run loop, and nowhere else

`flex::run_loop` (`src/flex/mod.rs:59-74`) renders, then blocks on `keys.next_key()`, which is
`tty::read_key`, which polls with `PollTimeout::NONE` (`src/tty.rs:124-150`). That poll is the only
thing between the loop and a half-second timer, so the timeout goes there.

The phase is a `bool` local to `run_loop`, threaded into rendering as an argument and never stored
on `FlexState`. `PartialEq for FlexState` (`src/flex/state.rs:106-110`) stays untouched and the
~100 existing `assert_eq!`s keep passing. The old spec's worry that an app-side phase *"would have
to be excluded from state equality"* does not arise.

The renderer is the single source of truth for what is *currently on screen*, via
`typing_caret_lit`. `run_loop` holds what it *wants* next. Passing the phase in and letting the
renderer compare avoids the two-sources-of-truth bug that a duplicated `lit` flag would create.

The phase is **reset to lit whenever the mode is not Write**. Outside typing mode there is no caret,
so the phase carries no meaning and must not be allowed to survive into the next Write session:
without the reset, a user who leaves write mode on a dark phase re-enters to no caret at all,
because the render that enters write mode would use the stale `false`. That is acceptance criterion
1 failing on the second visit rather than the first.

The loop, with the `TICK` branch taking its own path so the top-of-loop render is skipped:

```
loop {
    if state.mode != Write { lit = true }
    render(state, lit)
    loop {
        key = next_key(timeout)
        if key == TICK {
            lit = !lit
            blink(lit)
            continue
        }
        break
    }
    if key == RESIZE { resize(); continue }
    (state, effect) = reduce(state, key)
    if effect == Quit { return }
}
```

The inner `loop` is load-bearing. A plain `continue` on `TICK` would fall back through to the
top-of-loop `render`, which is precisely the full repaint this design exists to avoid, so a tick
waits for the next key in place instead. `RESIZE` and every keystroke still re-render.

### A timeout only while typing

`run_loop` passes a finite timeout only when `state.mode == FlexMode::Write`. In every other mode
`read_key` still blocks on `PollTimeout::NONE` and dre is exactly as idle as it was before. Leaving
Write mode stops the ticking on the next keypress.

The old spec rejected an app-side tick because it *"repaints the whole frame 4×/sec forever"*. That
objection dies twice over here: nothing is repainted, only one placement moves; and the clock stops
outside Write mode anyway.

### A tick is a sentinel key, like a resize

`tty::read_key` already signals a non-key by returning a sentinel string — `tty::RESIZE` arrives
down the same `io::Result<String>` as any keystroke. The tick follows that precedent:
`read_key` returns `tty::TICK` when `poll` times out. `run_loop` handles `TICK` before
`state::reduce`, so a tick is never reduced as a keystroke.

`read_key` gains a timeout parameter, and so does the trait:

```rust
fn next_key(&self, timeout_ms: Option<u32>) -> io::Result<String>;
```

`run_loop` passes `Some(BLINK_HALF_MS)` in Write mode and `None` otherwise. Adding the argument is
the only change to `KeySource`; `#[automock]` absorbs it.

### Collaborators and responsibilities

- **`tty::read_key`** — polls with the given timeout and returns `tty::TICK` on expiry. On expiry
  the resize pipe is still checked first, so a resize that races the timeout still wins.
- **`flex::run_loop`** — owns the phase, chooses the timeout, routes `TICK`.
- **`FlexScreen`** — gains `blink(lit)`, alongside `render(state, lit)` and `resize`.
- **`flex::view::scene` / `paint`** — takes the phase and emits the node only when lit, so a
  keystroke landing on a dark phase does not re-place the caret.
- **`TerminalRenderer`** — owns the remembered cell, the two commands, and the cached glyph image.
- **`Renderer::render`** — gains the phase as a parameter, because `TerminalRenderer::frame` sits
  behind the trait and has to record what it emitted. Only `TerminalFlexScreen` has a phase; every
  other implementation of the trait — `cli::export`, the legacy editor, the web build — passes a
  literal `true`, meaning "no blinking caret here". That literal carries meaning, so it is the one
  place in the change where `true` is not obviously meaningless.

### The node itself

Unchanged from the first cut, and still correct:

- `PlacementNode::TypingCaret { colour: Rgb, bold: bool }` (`src/view.rs`). Colour and bold only,
  no position logic. `is_decoration()` includes it, so criterion 6 holds — it never affects measure
  and never reserves a cell.
- `paint` emits it only when `state.mode == FlexMode::Write` and `path == state.selected`, which is
  criterion 5.
- Position is `text.x + text.chars().count()`, not `text.x + text.width`. Empty text gives `x + 0`,
  the cell where the first character lands, which is what criterion 1 asks for and what survives
  criterion 4. `interior`'s `.max(1)` (`src/view.rs:114-116`) stays a layout concern. No clamp:
  `Frame::crop` decides visibility, and an off-screen caret is simply not remembered.
- `scene` already takes `new: &HashSet<Vec<usize>>` for the grow animation, so a per-frame
  non-state parameter is the established shape.

### The caret is a plain glyph image

With no animation there are no frames, so `kitty::blink`, `Image::Blinking`,
`Frame::place_blinking` and `BLINK_GAP_MS` are all **deleted**, along with their tests.

`draw_typing_caret` keeps the `CaretKey` = `{colour, bold}` cache
(`HashMap<CaretKey, ImageId>`, mirroring `GlyphKey` and `glyph_images`) and gets the glyph from
`glyph_source.glyph('|', colour, bold)`, exactly as `draw_label` obtains each character. First
sight transmits and places as two commands at the reserved placement id, described above.

### SVG export is still unreachable

`SvgRenderer` is called from exactly one place, `cli::export`, and it renders `view::body` for a
**legacy** `State` (`src/cli.rs:86-96`). `flex::view::scene` is never handed to it. The arm for
`TypingCaret` in `src/render/svg.rs` exists only for exhaustiveness and emits nothing. A frozen
cursor has no meaning, and a static `|` would be indistinguishable from a pipe in the label.

### Constants

`BLINK_HALF_MS` lives in `src/flex/mod.rs` and is the value passed to `read_key`. The caret's
placement id is a named constant in `src/render/terminal.rs`. Tests assert against both constants,
never against a literal.

### Tests

Framework is inline `#[test]` with plain `assert!`/`assert_eq!`, no snapshots.

`src/tty.rs` — a pipe stands in for the terminal, so a timeout is testable without a real tty:

- `read_key_returns_a_tick_when_the_timeout_expires_with_no_byte`
- `read_key_returns_the_byte_when_it_arrives_before_the_timeout`
- `a_resize_arriving_during_the_timeout_still_wins_over_the_tick`
- `read_key_without_a_timeout_never_returns_a_tick`

`src/flex/mod.rs`:

- `a_tick_flips_the_phase_without_re_rendering`
- `a_tick_is_not_reduced_as_a_keystroke`
- `a_timeout_is_asked_for_in_write_mode_only`
  - `entering_write_mode_renders_with_the_caret_lit`
  - `re_entering_write_mode_on_a_dark_phase_renders_with_the_caret_lit` — leave write mode while
    dark, re-enter, and the caret must be there on the first render
- the five existing `run_loop` tests, updated for the `next_key` argument

`src/render/terminal.rs`:

- `a_lit_render_places_the_caret_and_a_dark_one_leaves_nothing_placed` — the dark half asserts no
  *surviving* placement, not the absence of a placement command: a dark render places and then
  deletes, in one synchronized block.
- `the_caret_survives_a_keystroke_that_lands_on_the_dark_phase` — the regression test for the bug
  this design shipped with: render lit, tick to dark, render dark, tick to lit, and the placement
  must come back.
- `a_tick_that_cannot_find_a_cell_emits_nothing`
  - `the_first_sight_placement_uses_the_reserved_placement_id` — first sight is `a=t` then
    `a=p,…,p=<reserved>`, never a bare `a=T`, so a later `delete_placement` can name it
  - `a_dark_first_sight_leaves_nothing_placed` — the delete must be able to name the placement that
    first sight just created
- `a_tick_emits_exactly_one_command_and_no_soft_clear_or_text`
- `going_dark_deletes_the_placement_and_going_lit_re_places_it`
- `a_tick_that_does_not_change_the_phase_emits_nothing`
- `a_tick_emits_nothing_when_the_scene_had_no_caret`
- `a_full_render_replaces_the_remembered_cell`
- `a_caret_cropped_off_screen_is_not_remembered`
- `the_caret_is_re_placed_and_not_re_transmitted_when_typing_moves_it`
- `the_legacy_caret_is_still_a_solid_block` — regression guard for `draw_caret`

`src/flex/view.rs`, unchanged from the first cut and still valid:

- `write_mode_puts_a_typing_caret_one_cell_past_the_label`
- `an_empty_label_puts_the_caret_in_the_first_cell`
- `typing_the_next_character_moves_the_caret_right_by_one_cell`
- `backspacing_to_empty_leaves_the_caret_in_the_first_cell`
- `move_mode_draws_no_typing_caret`
- `the_caret_placement_is_one_cell_wide_and_does_not_widen_the_label`
- `the_dark_phase_draws_no_typing_caret` — `view::scene` omits the node when dark, so a scene
  rendered on a dark phase contains no `TypingCaret` at all. The renderer's own place-then-delete is
  belt-and-braces against a scene that does carry one.

`src/kitty.rs`:

- `delete_placement_removes_one_placement_and_keeps_the_image`
- the three `blink_*` tests are deleted with the function

### Rejected

- **Terminal-driven animation** (transparent root, one lit frame, `a=a,r=1,z=` loop) —
  implemented and measured: steady on WezTerm, invisible on Ghostty. Recorded above.
- **`X=1` replacement frames** — the correct per-protocol way to let a transparent off-frame erase a
  lit root. Still unverified on WezTerm, and unnecessary once the app moves one placement.
- **A thread driving `kitty::blink`** — races the synchronized-update block, and `soft_clear`
  deletes its placement on the next keystroke anyway.
- **Re-rendering the whole frame on each tick** — rewrites every text cell and every image
  placement twice a second to move one, and scales with diagram size. See "A tick is one
  placement, not a frame".
- **Storing the phase on `FlexState`** — would need excluding from `PartialEq`, breaking the ~100
  existing state assertions.
- **A second `lit` flag in the renderer** alongside the one in `run_loop` — two sources of truth
  for what is on screen. The renderer compares against the phase it last emitted instead.
- **Deleting the caret placement on leaving Write mode** — the full render already omits it, and
  `soft_clear` already removed the placement.
- **Ticking in every mode** — the clock is finite only in Write mode; elsewhere dre blocks as
  before.
- **Reusing `PlacementNode::Caret`** with extra fields — drags the legacy solid block's meaning
  into flex and forces every legacy `Caret` construction site to change.
- **`text.x + text.width` as the caret origin** — puts the caret one cell right of where typing
  lands when the label is empty, so it jumps left on the first keystroke.