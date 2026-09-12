# 030 - Toggle a box's border thickness

## Story

Bob selects a box and presses `t`. Its border becomes visibly thicker. He
presses `t` again and it goes back to thin.

## Acceptance Criteria

- `t` toggles the selected box's border between thin and thick.
- A new box starts with a thin border.
- `t` only changes the selected box's border; other boxes are untouched.
- The box's size and position stay exactly as they are today — only the drawn
  line gets thicker, it doesn't eat into the label area.
- On a canvas with no boxes, `t` does nothing.
- In insert mode, `t` types the letter "t" into the label.

## Technical Design

- `Box` (`state.py`) gains a new field `border: Literal[1, 2] = 1` — `1` is
  thin (today's default), `2` is thick. Left as a number rather than a bool
  so more thickness levels can be added later without a shape change.
- `handle_command` (`state.py`) gets a `"t"` branch mirroring the existing
  `"c"`/`"f"` colour/fill toggles: guarded by `if not state.selected: return
  state`, then `rewrite`s the selected box with
  `lambda box: replace(box, border=3 - box.border)` to flip between 1 and 2.
- Insert mode needs no change — `t` already falls through
  `handle_insert`'s printable-character branch and gets typed into the label.
- This is a pixel-rendering concern only; `layout.py` and `Placement`
  geometry (box width/height in cells, tracks, etc.) are untouched. A
  terminal cell is several pixels wide, and today's border is already drawn
  as a single edge-colour pixel at the boundary of the box's border cell
  (the rest of that cell is fill colour). Thickening the border just paints
  `border` edge pixels instead of `1` at each boundary (top/bottom edge rows,
  left/right edge columns) in `GraphicsRenderer._outline_box` — still
  comfortably inside the same one-cell border allocation, so it doesn't
  encroach on the label area.
- `_key`'s cache `shape` tuple for `Box` nodes must include `node.border`
  alongside `(colour, fill)` so a border toggle isn't served a stale cached
  sprite.
- Terminal (text) renderer is unaffected — out of scope for this story.
