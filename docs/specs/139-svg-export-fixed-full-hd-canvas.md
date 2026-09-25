## Story

Doug has `plans.dre` open. He runs `dre --svg plans.dre` and opens `plans.svg`. Whatever the size of his diagram, the picture is a Full HD canvas, 1920 × 1080, with the diagram centred on it and the text at the same size every time. He exports a tiny diagram and a huge one, and the text looks identical in both. Happy, he drops them side by side in his slides.

## Acceptance Criteria

- Every exported SVG is exactly 1920 × 1080, whatever the size of the diagram.
- The whole canvas is filled with the dark background (`#0A0B0D`), including the empty space around the diagram.
- The diagram is centred on the canvas.
- The text and the boxes are the same size as today's export. Nothing inside the diagram is scaled.
- If the diagram is larger than the canvas, it is cut off evenly at the canvas edges, like in the terminal.
- The live editor and its bottom-right `<name> • dre` are unchanged.

## Technical Design

Decisions from the design session. Everything lives in `src/render/svg.rs`, plus the `src/cli.rs` call site.

### Model

- `SvgRenderer` holds `canvas: (i64, i64)` in **pixels** and `mode: Mode`.
- `enum Mode { Export, Editor }` replaces the `Option<(i64, i64)>` switch.
  - `Export`: draws with `body` and no cursor, as the CLI does today.
  - `Editor`: draws with `editor`, which adds the cursor and footer, as the web does today.
- `SvgRenderer::default()` is `Export` on a Full HD canvas. The constants `FULL_HD_WIDTH = 1920` and `FULL_HD_HEIGHT = 1080` live in `svg.rs`.
- `SvgRenderer::with_canvas(cols, rows)` is `Editor` with a canvas of `cols * CELL_WIDTH` by `rows * CELL_HEIGHT` pixels. Its signature is unchanged, so `web/src/lib.rs` needs no change.
- The fit-to-diagram mode (`extent`, and the `None` branch of `window`) is deleted.

### Layout

- The layout window is the pixel canvas in whole cells: `canvas / (CELL_WIDTH, CELL_HEIGHT)`, rounded down.
  - Full HD gives 240 × 67 cells. 1080 / 16 = 67.5, and the missing half row is accepted.
  - The diagram is centred by the existing `centre` inside that window. It sits 4 px above true centre vertically and is exactly centred horizontally.
  - The last 8 px of the canvas is background only.
- A diagram bigger than the window is cut evenly at the window edges by the existing nested `<svg>` clipping. No layout code changes. The cut is at the cell edge, so at the bottom it is 1072 px, not 1080.
- A `with_canvas` size larger than Full HD just means a bigger window and a bigger canvas. The diagram is centred, the extra area is background, and nothing is scaled.

### Document

- `document()` takes the pixel canvas and emits `<svg width="{w}" height="{h}" viewBox="0 0 {w} {h}">` on the root. Every SVG now has explicit pixel `width` and `height`, so 1 SVG unit is 1 px and the text is always the same size. This includes the web SVG.
- The background `<rect>` covers the whole pixel canvas (`#0A0B0D`), including the empty space around the window.
- Nothing inside the diagram is scaled. Boxes, arrows and labels keep today's pixel maths.

### Collaborators

- `render::body` and `render::editor` are unchanged.
- `cli::export` keeps calling `SvgRenderer::default().render(...)`, and now gets Full HD.
- `web/src/lib.rs` keeps calling `with_canvas(cols, rows)`.

### Tests

- Export of a tiny diagram and of a huge one both start with `width="1920" height="1080"`, and the background rect is 1920 × 1080.
- Label font size and box sizes are identical between the tiny and huge exports.
- A diagram wider than 240 columns is cut evenly left and right (same overflow on both sides).
- `with_canvas(cols, rows)` emits `width` and `height` equal to `cols * 8` and `rows * 16`, and the footer is still there.
- A `with_canvas` size larger than Full HD extends the canvas without scaling.
- Existing `svg.rs` tests that relied on the fit-to-diagram size are updated to the new canvas.
