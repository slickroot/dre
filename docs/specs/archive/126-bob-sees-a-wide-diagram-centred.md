# Bob sees a wide diagram centred

## User Story

Bob opens a diagram that is wider than his terminal. He sees it centred, with its left and right edges cut off evenly. He moves his selection to a box that is off-screen past the right edge, and the view stays exactly where it is. Nothing slides under him, so he always knows where he is.

## Acceptance Criteria

- A diagram wider than the terminal is drawn centred, cut off evenly on both sides.
- Selecting or adding a box past the edge of the terminal doesn't move the view.

## Technical Design

Decided in the design session. The view depends only on the diagram and the terminal size. There is no view state, and the view does not follow the selection.

### Today

`Screen::centre_on` centres the diagram when it fits and sets `origin.x = -scroll_x` when it overflows. Nothing has dispatched `Action::ScrollBy` since the auto-scroll was dropped, so `scroll_x` is always 0 and a wide diagram is pinned to the left edge with only its right side cut off.

### `Screen::centre_on` (`render/terminal.rs`)

```rust
fn centre_on(&mut self, placements: &[Placement])
```

- Loses the `scroll_x` parameter and the overflow branch.
- Both axes use one rule: `(terminal - span).div_euclid(2)`. It gives a positive margin when the diagram fits and a negative origin when it overflows, which cuts the diagram evenly on both sides.
- When the difference is odd, the extra column goes on the left: a blank column when the diagram fits, a cut column when it overflows. Example: cols 10, span 13 → origin −2 (2 cut on the left, 1 on the right).
- Adding a box that widens the diagram re-centres it, so everything shifts by half the growth. This is accepted: "doesn't move the view" means the view never follows the selection or a new box. It does not mean boxes are pinned to screen columns.
- `render_diagram` keeps its order (`layout` → `with_cursor` → `centre_on`). The cursor sits on its label's last column, inside the box, so it cannot widen the span. With `scroll_x` gone, no selection-dependent input reaches `centre_on`.

### Deleted

- `State.scroll_x` and its `Default` entry.
- `Action::ScrollBy`, its `mode()` arm, its `min_depth` arm, its `is_undoable` handling, and the reducer arm in `state/command.rs`.
- `TerminalRenderer::columns()`: its only caller was the old auto-scroll.
- The tests of all of the above, including `rendered_with_scroll`.

A future "Bob pans the view" story gets its own design and does not reuse `scroll_x`.

### Tests

- `Screen::centre_on` unit tests, replacing the three `scroll_x` ones:
  - an overflowing diagram gets origin `(cols - span).div_euclid(2)`, which is negative;
  - an odd overflow cuts the extra column on the left (cols 10, span 13 → −2);
  - the existing "fits" test stays, without the `scroll_x` argument.
- One render-level test replaces `an_overflowing_diagram_uses_scroll_x_instead_of_centring`. It renders a diagram wider than the terminal and checks that the same number of columns is cropped off each side.
- No separate "selection doesn't move the view" test: `centre_on`'s signature cannot receive the selection.
