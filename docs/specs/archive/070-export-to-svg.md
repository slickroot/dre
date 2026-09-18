# 070: Export to SVG

## User Story

As a user, I can run `dre --svg diagram.dre` and get a crisp, vector `diagram.svg` that looks
just like my diagram in the terminal — same boxes, labels, colours, fills, rounded corners,
and arrows — without ever opening the editor.

## Acceptance Criteria

- Running `dre --svg diagram.dre` creates `diagram.svg` next to the `.dre` file and exits
- The SVG matches the terminal view: boxes, labels, colours, fills, rounded corners, and arrows
- The SVG has no cursor or selection overlay — it's the saved document, not the editing session
- A missing file prints `no such file: <path>` and exits with a failure code
- A corrupted file prints `<path>: not a valid diagram` and exits with a failure code

## Technical Design

### Prerequisite

Spec 069 delivers origin-based `layout(&[Node]) -> Vec<Placement>` with centering owned by
renderers. This spec builds on it; the SVG renderer never sees a canvas.

### Modules

**`svg.rs` (new)** — `SvgRenderer`, a second, independent renderer that mirrors
`TerminalRenderer`'s *contract* (placements in, frame out) without sharing its pipeline:
`struct SvgRenderer {}` with `fn render(&self, placements: &[Placement]) -> String` returning
the complete SVG document. Stateless: no sprite cache, no TTY metrics.

**`cli.rs` (new)** — owns argument parsing and the headless export flow.
`parse_args() -> Command::Edit(Option<String>) | Export { input: String }`.
`writer.rs` keeps the interactive loop only, minus its arg parsing.

### Responsibilities

| Component | Knows | Does |
|---|---|---|
| `layout` | boxes only (069) | emits origin-based placements (boxes first) |
| `SvgRenderer` | `CELL_WIDTH`, `CELL_HEIGHT`, the shared pixel constants | computes bbox + margin viewBox, emits boxes → arrows → labels as SVG, returns the document |
| `cli.parse_args` | argv | returns `Edit` or `Export` |
| `cli.export` | filesystem, `dre_format`, `layout`, `SvgRenderer` | loads/validates the file, renders, writes `…svg`, returns success/failure |
| `writer.rs` | the terminal, `TerminalRenderer` | runs the interactive loop only |

### Geometry & colours

- **Coordinates**: layout cells → SVG units via canonical metrics: `x × CELL_WIDTH`,
  `y × CELL_HEIGHT`, with `CELL_WIDTH: i64 = 8` and `CELL_HEIGHT: i64 = 16` added
  pub(crate) in `render.rs` beside the existing pixel constants (`BORDER=4`,
  `ARROW_STROKE=4`, `ROUNDED_RADIUS=20`, arrowhead constants, `PALETTE`, `PLAIN_COLOUR`).
- **ViewBox**: placements' bounding box padded by one `CELL_HEIGHT` on every side (margin
  derived from metrics, not a magic number). Transparent background — no page rect.
- **Draw order**: boxes first, then arrows, then labels (labels sit on top of the fill).
- **Boxes**: `<rect>` per box; `stroke` = palette colour (or `PLAIN_COLOUR` for colourless),
  `stroke-width="{BORDER}"`, `rx="{ROUNDED_RADIUS}"` when rounded; fill only when
  `filled && colour.is_some()`.
- **Fills (design-idiom)**: `PALETTE[i]` at `fill-opacity = FILL_ALPHA/255` — translucent so
  it tints any viewer background; deliberately *not* the terminal's opaque
  composited-over-black value.
- **Labels**: one `<text>` per label — `font-family="monospace"`, `font-size="{CELL_HEIGHT}"`,
  `text-anchor="start"` at `x = label.x × CELL_WIDTH`, baseline from `label.y`, and
  **`textLength="{char_count × CELL_WIDTH}"` + `lengthAdjust="spacingAndGlyphs"`** so the
  string spans exactly its cell width regardless of the viewer's font — the glyph-per-cell
  guarantee. Dark foreground.
- **Arrows (starter)**: SVG `<marker>` arrowheads on arrow paths — see Deferred.

### Export flow (`cli.rs`)

1. `parse_args` → `Export { input }`.
2. Load the file:
   - not found → `no such file: <path>` (failure);
   - content fails `dre_format::read` → `<path>: not a valid diagram` (failure, existing message).
3. Convert via the `file_document` mapping to boxes. **No selection, no `with_cursor`** — the
   SVG is the saved document, never the editing session.
4. `layout(boxes)` → placements → `SvgRenderer::render()` → `fs::write`.
5. Output path: swap the final extension for `.svg` (`x.dre` → `x.svg`); a path with no
   extension gets `.svg` appended (`report` → `report.svg`).
6. Errors surface as before: `main()` prints to stderr and returns `ExitCode::FAILURE`.

`Edit` keeps the existing entry path through `writer.rs`, minus its own argument parsing.

### Deferred (recorded, not forgotten)

- **Shared shape layer for arrows**: with two renderers, arrow *appearance* is defined per
  backend. A later restyle (e.g. smooth bezier arrows) must not require edits in two places;
  the decision of *how* to share (baked geometry vs semantic primitives) is parked. The
  `<marker>` approach is the accepted starter.
- **No `Renderer` trait**: the renderers share a placements-in/frame-out *shape* but keep
  natural signatures; introduce a trait only when a second shared method actually appears.

### Test notes

- `svg.rs`: a known box set renders the expected SVG string — viewBox with the
  `CELL_HEIGHT` margin, rects (border, `rx`, fill-opacity), labels (monospace +
  `textLength`), marker arrows — golden over the spec-example diagram; empty placements
  render an empty document.
- `cli.rs`: arg parsing branches; missing-file and corrupt-file messages; output-name
  derivation (`.dre` swap, extensionless append); success/failure exit codes.