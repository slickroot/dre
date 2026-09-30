# Svg arrows match terminal

Amara exports her diagram to SVG and opens it in a browser. She expects the arrows to look the same as they did in her terminal — instead the arrow shafts look thinner and the arrowhead edges look longer, so the diagram feels visually inconsistent between the two. After the fix, she opens the same diagram in both the terminal and as an exported SVG and the arrows look visually the same.

## Acceptance Criteria

- Arrow shaft stroke width in the SVG export visually matches the arrow stroke width in the terminal render.
- Arrowhead edge length in the SVG export visually matches the arrowhead edge length in the terminal render.
- Verified by eyeballing a diagram with arrows side-by-side in the terminal and in a browser-opened SVG export.

## Technical Design

### Root cause

The terminal renderer and the SVG renderer already share `ARROWHEAD_EDGE_LENGTH` and `ARROWHEAD_ANGLE_DEG` (`src/render/mod.rs`), and yet the arrowhead still looks longer in the SVG export. That's because a terminal "pixel" and an SVG "pixel" are different units: the terminal's cell pixels come from the real device pixel dimensions reported by the terminal emulator (`TIOCGWINSZ`, `src/tty.rs`), while the SVG's `CELL_WIDTH`/`CELL_HEIGHT` (`src/render/mod.rs`) are nominal SVG user units that a browser maps to CSS reference pixels. Sharing a raw absolute-magnitude constant (edge length in "pixels", stroke width in "pixels") across the two renderers doesn't produce a matching visual size, because the same number means a different physical size in each coordinate space. There's no DPI/scale-factor concept anywhere in the codebase to convert between the two, and deriving one (a proportional `k` based on cell size) would introduce rounding issues without clear benefit.

`ARROWHEAD_ANGLE_DEG` is dimensionless (a ratio/angle), so it's unaffected by this mismatch and can safely stay shared.

### Fix

- Split `ARROW_STROKE` and `ARROWHEAD_EDGE_LENGTH` into independent, per-renderer constants — one in `src/render/terminal.rs`, one in `src/render/svg.rs` — each hand-tuned by eyeballing a side-by-side terminal/browser comparison. No formula or proportion derives one from the other or from cell size/DPI.
- Keep `ARROWHEAD_ANGLE_DEG` as the single shared constant in `src/render/mod.rs`.
- `arrowhead_depth()` and `arrowhead_slope()` (`src/render/mod.rs`) take `edge_length: f64` as a parameter instead of closing over `ARROWHEAD_EDGE_LENGTH`; the angle stays baked in from the shared constant. Each renderer passes its own local edge-length constant.
- Remove SVG's incidental `ARROW_STROKE = BORDER / 4` derivation (`src/render/svg.rs:11-12`) — it has no real relationship to the box-drawing `BORDER` constant. Replace it with a standalone constant tuned to visually match the terminal.
- Terminal's existing `ARROW_STROKE = 3` (`src/render/terminal.rs:17`) stays as is — it already looks correct in the terminal.
- The exact tuned values for the new SVG-side constants are determined empirically during implementation, verified against this spec's own acceptance criteria (visual eyeball comparison, terminal vs. browser-opened SVG).
