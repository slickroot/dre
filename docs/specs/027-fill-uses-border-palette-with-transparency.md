# Fill uses border palette with transparency

## User Story

As a user, when I fill a box in the kitty graphics view, I want the fill to use the same colour palette as the border (at 30% opacity), so my diagram has a consistent, faded fill that matches its border colour scheme.

## Acceptance Criteria

- In the kitty graphics renderer, pressing `f` on a selected box cycles its fill through the same 5 palette colours used for borders.
- Cycling starts from "no fill" (transparent interior), then colour 1 → colour 2 → ... → colour 5 → back to "no fill."
- When a fill colour is set, the box interior renders as that palette colour blended at 30% opacity against the background.
- This applies only to the kitty graphics renderer (terminal ANSI rendering is unaffected).

## Technical Design

- **Fill cycling reuses border cycling.** `Box.fill` already exists (`state.py:14`) but currently cycles via a separate `next_fill`/`CYCLE=9` scheme built for the old ANSI-only fill behaviour. Since fill now only matters for the kitty renderer and should cycle through the same 5 palette colours as border (`PLAIN` → 0 → 1 → 2 → 3 → 4 → `PLAIN`), the `f` key handler in `handle_command` (`state.py:118-127`) will call `next_colour(box.fill)` instead of `next_fill(box.fill)`. The now-unused `next_fill` function and `CYCLE` constant are removed from `state.py`.
- **No change to ANSI/terminal rendering.** `TerminalRenderer` continues to read `box.fill` as before; since fill values are now always in the `PLAIN`/`0-4` range, its existing `40+fill` background-code logic still works mechanically — we are not preserving its old visual behaviour/range, per the decision that ANSI fill is out of scope.
- **Alpha compositing is done by the terminal, not by us.** The kitty protocol transmits RGBA32 sprites; rather than precompute a blended RGB against an assumed background, we set the fill pixels' alpha channel and let the terminal composite it. A new constant `FILL_ALPHA = 77` (≈30% of 255, rounded) is added next to `OPAQUE`/`TRANSPARENT` in `render.py:36-37`.
- **New `_fill_colour` helper in `render.py`**, alongside `_colour`:
  ```python
  def _fill_colour(fill: int) -> Tuple[int, int, int, int]:
      return TRANSPARENT if fill == PLAIN else PALETTE[fill] + (FILL_ALPHA,)
  ```
- **`GraphicsRenderer._outline_box` (`render.py:110-131`) is updated** so interior pixels use `_fill_colour(placement.node.fill)` instead of the hardcoded `TRANSPARENT`, while border/edge pixels keep using `_colour(placement.node.colour) + (OPAQUE,)` unchanged. This only affects `Box` nodes since `fill` is a `Box`-only field.
