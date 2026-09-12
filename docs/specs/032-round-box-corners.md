# 032 - Round box corners

## Story

Bob selects a box and presses `r` repeatedly. Its corners cycle through three
radii — 0px (square), then 4px round, then 8px round — and pressing `r`
again after 8px wraps back to 0px.

## Acceptance Criteria

- `r` cycles the selected box's corner radius through three levels in order:
  0px → 4px → 8px → back to 0px.
- A new box starts with 0px (square) corners.
- `r` only changes the selected box's corners; other boxes are untouched.
- The box's size and position stay exactly as they are today — only the
  drawn corners change.
- On a canvas with no boxes, `r` does nothing.
- In insert mode, `r` types the letter "r" into the label.

## Technical Design

- `Box` (`state.py`) gains `radius: Literal[0, 4, 8] = 0`, the corner radius in
  pixels. It is an absolute pixel value, independent of `border`, so `t` and
  `r` each change one thing only.
- `handle_command` (`state.py`) gains an `"r"` branch mirroring `"t"`: the same
  `if not state.selected: return state` guard, then `rewrite` with
  `radius=(box.radius + 4) % 12`, giving 0 → 4 → 8 → 0. Insert mode needs no
  change — `r` falls through `handle_insert`'s printable-character branch.
- `radius` names the **inner** arc. The outer arc is concentric at
  `outer = radius + border`, so the band keeps constant thickness `border`
  around the bend. `radius = 0` is special-cased to a fully square corner with
  no arc drawn at all, so a new box is square regardless of its border
  thickness.
- Corner geometry is the rounded-rectangle distance field. For a point
  `(px, py)` in box pixel coordinates, with `R = radius + border` and box size
  `W × H`, let `(cx, cy) = clamp((px, py), (R, R), (W - R, H - R))` and
  `d = dist((px, py), (cx, cy))`. Then `d > R` is outside (transparent),
  `radius < d <= R` is edge colour, and `d <= radius` is fill. On the straight
  runs one or both clamps collapse and this reduces exactly to today's
  `border`-inset rules, so the flat edges stay pixel-crisp.
- The arcs are anti-aliased by 4×4 supersampling: each corner pixel takes 16
  subsample points, each classified by the rule above, and the 16 RGBA results
  are averaged. Straight edges fall on exact pixel boundaries and so resolve to
  uniform coverage; only the curve gets fractional alpha.
- The average is taken in **premultiplied** alpha and un-premultiplied before
  the pixel is written: `a = mean(alphas)`, and `rgb = sum(c * a) / sum(a)` per
  channel, or `(0, 0, 0, 0)` when `sum(a)` is zero. Straight averaging would
  drag the fringe toward black against `TRANSPARENT` and would put a
  wrong-density ring along the inner arc, where opaque edge colour meets fill
  at `FILL_ALPHA`.
- `GraphicsRenderer._outline_box` (`render.py`) becomes row-based: it builds
  each row's profile from the radius rather than repeating two prebuilt rows.
  Supersampling is confined to pixels inside a corner band (`x < R` or
  `x >= W - R` on a row with `y < R` or `y >= H - R`); every other span is
  still a flat repeat of `edge` or `fill` bytes, so the cost stays proportional
  to the corners, not the box. `radius = 0` short-circuits to today's two-row
  path unchanged.
- Rows are computed in full-box coordinates, so the existing
  `first_x`/`last_x`/`first_y`/`last_y` screen clipping keeps working: a box
  half off-screen still gets the corners the whole box would have had.
- `_key`'s `shape` tuple (`render.py`) gains `node.radius`, so the sprite cache
  distinguishes the three radii. Each distinct shape is supersampled once and
  reused for every later frame.
- No clamp for small boxes is needed: `BOX_HEIGHT` is 3 cells and the narrowest
  box is 3 cells, which at the real 8×32 cell is 24 × 96 px, comfortably larger
  than `2 * (8 + 4)`.
- `TerminalRenderer` is untouched. Rounding is purely a graphics-layer concern;
  the text layer only stamps blank fill-coloured cells beneath the sprites.
