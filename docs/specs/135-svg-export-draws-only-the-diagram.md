## Story

Doug has `plans.dre` open. He runs `dre --svg plans.dre` and opens `plans.svg`. He sees his diagram and nothing else: no name, no `• dre`, no empty strip at the bottom. Happy, he can drop the picture straight into a document.

## Acceptance Criteria

- The exported SVG contains only the diagram. It has no name and no `• dre`.
- The picture is exactly the size of the diagram, with no extra row at the bottom.
- The live editor's bottom-right corner is unchanged. It still reads `<name> • dre`, or `[no name] • dre`.

## Technical Design

### Decisions

- The mode follows the canvas, so there is no new public API and `web/src/lib.rs` is untouched.
  - `SvgRenderer::default()` has no canvas. This is the `--svg` export. It draws only the diagram.
  - `SvgRenderer::with_canvas(cols, rows)` keeps drawing the full editor screen, footer included. This is the web editor demo.
- `render::editor` splits its body half into `pub(crate) fn body(state: &State, area: Area) -> Vec<Placement<'_>>`. It is the diagram with the cursor, centred in `area`. `editor` calls `body` for the body area, so the terminal and the canvas path behave exactly as before.
- `SvgRenderer::window`: with no canvas, the window is the diagram's extent, with no `+ FOOTER_ROWS`. With a canvas, it is unchanged.
- `SvgRenderer::render`:
  - No canvas: `document(window, &[(window, body(state, window))])`. One area, the whole picture.
  - Canvas: `document(window, &editor(state, window))`, as today.
- `document` is unchanged. It already takes a list of areas.
- `cli::export` is unchanged. It keeps `SvgRenderer::default()`.
- `FOOTER_ROWS` is no longer imported for the no-canvas window. The import stays for the tests that still use it.

### Tests

- `render/svg`: without a canvas, the picture is exactly `extent` in size (`height * CELL_HEIGHT`, no extra row). This replaces the existing `(height + FOOTER_ROWS) * CELL_HEIGHT` assertion at `svg.rs:1266` and the window at `svg.rs:1278`.
- `render/svg`: without a canvas, the output contains no footer text. A named state (`docs/plans.dre`) and an unnamed one both give no `plans`, no `[no name]` and no `• dre`.
- `render/svg`: with a canvas, the footer is still drawn and the viewBox is still `cols*8 × rows*16`. The existing canvas tests stay as they are.
- `render`: `body(state, area)` equals `editor(state, window)[0].1` for the matching body area. This guards the extraction.
- `cli`: `export` on a diagram writes an SVG that does not contain `• dre`.
- The live editor's bottom-right corner is covered by the existing `render` editor tests. They must stay green with no change.
