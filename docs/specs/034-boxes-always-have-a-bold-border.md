# 034 - Boxes always have a bold border

## Story

Bob adds a box to his canvas. It is drawn with a bold 4px border straight
away, and he never has to adjust the thickness.

## Acceptance Criteria

- Every box is drawn with a 4px border.
- There is no way to change a box's border thickness.
- Pressing `t` in command mode does nothing.
- Box sizes and positions stay exactly as they are today — only the drawn
  line changes.

## Technical Design
The thickness stops being state and becomes a constant. The geometry keeps a
`border` parameter so the pixel maths is still testable at more than one
value; nothing above the geometry can vary it.

### `sketch/state.py`

- Delete the `border` field from `Box`. A field that can never vary is not
  state.
- Delete the `if key == "t"` branch from `handle_command`. `t` falls through
  to the final `return state` and is thereafter indistinguishable from any
  other unbound key. No test pins it — a test of absence would only imply a
  binding that no longer exists.

### `sketch/render.py`

- Add a module-level `BORDER = 4`. This is the single home for the constant;
  spec 035 reads the same name for arrow strokes.
- `_outline_box` passes `BORDER` into `_square_pixels` and `RoundedBox`
  instead of reading `placement.node.border`.
- `_square_pixels`, `_body_row` and `RoundedBox` keep their `border`
  parameter unchanged. That parameter is the seam the pixel tests drive.
- Drop `node.border` from the shape tuple in `_key`. It can no longer vary,
  so it cannot distinguish two cached sprites.
- `RoundedBox.outer = min(radius + border, width // 2, height // 2)` is left
  exactly as it is. The outer corner arc widens from `radius + 1` to
  `radius + 4`; the stroke grows outward from the same inner curve.

### `sketch/layout.py`

Untouched. Layout never mentions `border`, so box sizes and positions are
unchanged for free. Boxes are always `BOX_HEIGHT = 3` rows, so the interior
survives a 4px border at any real cell size; the existing
`height <= 2 * border` guard stays as a safety net.

### Tests

- `GraphicsRendererBorderThicknessTest` (`tests/test_render.py`) moves down
  one level: it drives `_square_pixels` and `RoundedBox` directly with an
  explicit `border` argument rather than going through `Box(border=N)` and
  `_outline_box`. Coverage at several thicknesses survives the removal of the
  user-facing toggle.
- `test_a_rethickened_box_is_redrawn` and
  `test_each_border_thickness_is_cached_distinctly`
  (`tests/test_graphics.py`) are deleted. They assert that a field which no
  longer exists varies the cache.
- A new test asserts a box drawn through `_outline_box` has exactly four edge
  pixels at each side.
