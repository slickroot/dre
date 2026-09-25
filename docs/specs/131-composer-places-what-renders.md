# The screen is a stack of areas

This is a technical spec, not a user story. Its one visible change is that the
status line is replaced by a plain footer bar.

## Problem

The diagram has a clean pipeline: `layout` turns the `Document` into
`Placement`s and a renderer paints them. Everything else on screen is extra
code added inside the terminal renderer:

- `TerminalRenderer::render` draws the diagram centred on the whole window,
  then `render_status_line` moves the cursor to the last row and writes the
  status line over it with ANSI escapes. Nothing reserves that row, so the
  diagram is centred as if the row were free.
- `State::status_input()` builds presentation (`ModeLabel`,
  `"Save as: …█"`, `[+]`, the default filename), so `state/` imports
  `status_line`.
- Each renderer centres the diagram with its own rule, and the two disagree:
  the terminal cuts an overflowing diagram evenly (spec 126), while
  `SvgRenderer::centered_on` pins it to the left.
- The web has to ask `Session::extent()` for the drawing's size and pass it
  back into `svg(...)`, only because the renderer does the centring.

Adding one more thing to the screen means adding more renderer-specific code.

## Acceptance Criteria

- The screen is a stack of rows: the diagram on top, taking what is left,
  and a 3-row footer at the bottom. The footer is one box filled with a
  palette colour, as wide as the window.
- The diagram is centred in the rows above the footer and never draws into
  the footer.
- The status line is gone: no mode label, no filename, no `[+]`, no box count.
  The save prompt still works but is not shown (accepted for now).
- The terminal, the web and SVG export all draw the same stack.

## Technical Design

### The pipeline

```
render::editor(state, window) ──→ Vec<(Area, Vec<Placement>)>
  composer::stack(heights, window) ──→ [Area; N]
  layout::diagram(doc)             ──→ Vec<Placement>   from (0,0)
  layout::footer(width, height)    ──→ Vec<Placement>   from (0,0)
  centre / shift each list into its area

Renderer: calls editor, clips each list to its area, paints.
```

`editor` is the one place that says which layout goes in which area. It is
written by hand: each area gets a name from `stack`, and the same line that
uses the name builds that area's placements. The renderers know nothing about
a body or a footer.

### Modules

| Module | Knows the window? | Holds |
|---|---|---|
| `layout` | no | `diagram(&Document)`, `footer(width, height)`: geometry from `(0,0)` |
| `composer` | no, it is given one | `Area`, `stack(heights, window) -> [Area; N]` |
| `render/` | yes | `editor(state, window)`: pairs areas with layouts; renderers clip and paint |

`layout` stays window-free, as spec 069 decided. `composer` only splits a
window it is given. Dependencies run one way: `render` → `composer`,
`render` → `layout`.

### `layout`

- `layout::layout` is renamed `layout::diagram`. Nothing else changes: same
  input, same output, origin-based.
- New `layout::footer(width: i64, height: i64) -> Vec<Placement>`: one `Box`
  placement at `(0,0)`, `width` wide and `height` tall, with `colour` and
  `fill` set to a palette colour (index 0 for now) and `rounded: false`.
- `FOOTER_ROWS = 3` lives in `layout`, next to `footer`.

### `composer.rs`

```rust
pub(crate) struct Area { pub col: i64, pub row: i64, pub cols: i64, pub rows: i64 }

pub(crate) fn stack<const N: usize>(heights: [Option<i64>; N], window: Area) -> [Area; N];
```

Rows are reserved top to bottom, in the order given. Arrays, not slices:
`let [body, foot] = stack([None, Some(3)], window)` only compiles when the
number of heights matches the number of names.

- Every area is the full width of the window. Width is not modelled.
- `Some(n)` gets exactly `n` rows.
- `None` rows share what is left equally. When it does not divide evenly,
  the extra rows go to the first `None` rows (21 across 2 → 11, 10), the same
  side spec 126 gives the extra column.
- Example: `stack([None, Some(3)], 80×24)` →
  `[Area { 0, 0, 80, 21 }, Area { 0, 21, 80, 3 }]`.

`composer` knows nothing about the diagram, the footer, `State` or
`Placement`.

### Renderers

`Renderer::render(&State)` keeps its signature. The screen is described once,
in `render/mod.rs`:

```rust
pub(crate) fn editor(state: &State, window: Area) -> Vec<(Area, Vec<Placement>)> {
    let [body, foot] = composer::stack([None, Some(FOOTER_ROWS)], window);
    vec![
        (body, centre(with_cursor(layout::diagram(state.doc()), state.selected.clone()), body)),
        (foot, shift(layout::footer(foot.cols, foot.rows), foot)),
    ]
}
```

The placements it returns are in window coordinates. Each renderer does:

```rust
for (area, placements) in editor(state, window) {
    paint(&placements, area); // clipped to area
}
```

- **`editor`** is hand-written. A new screen part is a new height in the
  `stack` call, a new name in the `let`, and a new pair in the `vec!`.
- **`shift` and `centre`** live in `render/mod.rs`, next to `editor`.
  `shift` moves placements by the area's `(col, row)`. `centre` is
  `Frame::centre_on` moved there and applied to an area instead of the
  window: `(area - span).div_euclid(2)` on both axes, then `shift`. A diagram
  that overflows its area is cut evenly on both sides (spec 126), and the
  extra column of an odd difference goes on the left. The terminal and the
  web now centre the same way.
- **Clipping:** each placement is clipped to its own area, not the window.
- **`TerminalRenderer`**: its window comes from its `Window` (`cols`, `rows`).
  `Frame`'s origin is always `(0,0)`. `Frame::crop` and `Frame::shows` clip to
  the placement's area, and so do the label characters written into the text
  grid. `render_status_line` is deleted.
- **`SvgRenderer`**: with a canvas (the web), the window is the canvas. With
  no canvas (export), the window is the drawing's extent plus `FOOTER_ROWS`
  rows, so the diagram fits the body exactly. Each area is a nested
  `<svg x y width height viewBox>` using the area's absolute coordinates, so
  it clips without shifting anything. `centered_on` and `centering_offset`
  are deleted.

### Callers

- **Terminal:** unchanged.
- **Web:** `WebSession::svg(cols, rows)`. The `extent_width` and
  `extent_height` arguments and `Session::extent()` are deleted, because the
  renderer centres by itself. This changes the wasm API, so the website must
  call `svg(cols, rows)`.
- **Export:** unchanged in code. Exported SVGs, including
  `docs/example.svg`, now end with the footer bar.

### Deleted

- `status_line.rs`: `StatusLine`, `Segment`, `Style`, `ModeLabel`,
  `StatusInput`, `status_line`, `DIM_ALPHA` and their tests.
- `State::status_input()`, and `state/`'s import of `status_line`.
- `TerminalRenderer::render_status_line`, and `composite` if nothing else
  uses it.
- `Frame::centre_on` (moved to `render/mod.rs` as `centre`),
  `SvgRenderer::centered_on`, `centering_offset`, and `Session::extent`.
- Status-line and save-prompt rendering tests in `render/terminal.rs`.

### Tests

- `composer::stack`: a fixed row gets exactly its height; `None` takes the
  rest; two `None` rows split equally with the extra row going to the first;
  areas run top to bottom and are full width.
- `layout::footer`: one filled box at `(0,0)` with the given width and
  height, palette colour 0, square corners.
- `render::editor`: returns the body then the footer; the body is the window
  minus the last 3 rows and holds the diagram centred in it, with the cursor
  when something is selected; the footer is the last 3 rows and holds one box
  at the footer's origin, as wide as the window.
- `render::centre`: the spec 126 cases move here from `Frame::centre_on`,
  applied to an area with an offset.
- `TerminalRenderer`: the diagram is centred in the rows above the footer;
  the footer box fills the last 3 rows; a diagram box overhanging the body
  does not draw into the footer.
- `SvgRenderer`: with a canvas, the same stack as the terminal; with no
  canvas, the `viewBox` is the drawing's extent plus the footer, and the
  diagram is not cropped.

### Considered and rejected

- **A TUI framework (ratatui).** dre's diagram is kitty pixel sprites and
  SVG, not a grid of characters, and a framework only renders to terminals.
  It would take over the loop and buffer that already work. The only useful
  part, splitting rectangles, is `stack`: a few lines.
- **Keeping the status line, split into left, filler and right placements.**
  Dropped by decision: it is too much like Vim. A plain footer is the first
  step, and what fills it is a later story.
- **A tree of views (`Column`, `Row`, `Stack`, `Align`).** More than needed.
  A stack of rows with an optional height covers a header, a body and a
  footer.
- **The stack in `layout`.** `layout` is window-free on purpose (spec 069).
- **Screen-building functions in `composer`.** `composer` only splits a
  window into areas. Deciding what goes in each area is `render::editor`'s
  job.
- **The pairing written in each renderer** (`let [body, foot] = stack(…)` in
  both `TerminalRenderer` and `SvgRenderer`). The screen would be described
  twice and could drift.
- **A table of slots** (`height`, `fit`, `content` function) that `editor`
  zips with the areas. It makes the link structural, but it is more machinery
  than two named areas need. The hand-written `editor` is read top to bottom.
- **`stack` over slices returning `Vec<Area>`.** A mismatch between the
  heights and the names would only show at run time.
- **A flag on `State` for editor-only content.** Which output is being drawn
  is not a fact about the editing session.
- **Width as `Full` on `Placement`.** Every area is full width, and the
  renderer passes the area's width to `layout::footer`.
