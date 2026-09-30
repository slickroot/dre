# View scenes feed renderers

## Refactoring

The rendering pipeline currently receives `State` and constructs its own
placements. This makes the renderers responsible for editor composition, and
SVG needs a renderer-side `Mode` plus `without_cursor` to turn an editor state
into an export.

Move composition into a public `view` module. A view builds a `Scene` from a
`State`; renderers consume the resulting scene and only draw it.

## Behaviour

- `view::body(state, area)` builds a one-entry scene containing the centred
  diagram body.
- `view::editor(state, window)` builds a two-entry scene containing the body
  and footer, in that order.
- The body keeps the existing selection/glow/caret behaviour. Export creates a
  `State` without a saved selection, so its body contains no caret; no renderer
  filters cursor placements.
- `TerminalRenderer` renders the editor scene supplied by its caller.
- SVG export renders a body scene; the SVG canvas/editor path renders an editor
  scene. SVG never infers the scene kind from layer indexes.
- Existing pixels, SVG structure, centering, footer layout, and public
  renderer output remain unchanged.

## Technical Design

### Public scene API

Add `src/view.rs` and make the module public from `lib.rs`.

```rust
pub type Scene<'a> = Vec<(Area, Vec<Placement<'a>>)>;

pub fn body(state: &State, area: Area) -> Scene<'_>;
pub fn editor(state: &State, window: Area) -> Scene<'_>;
```

`body` moves the current `render::body` implementation. It returns a
single-entry scene, whose area is the requested body area and whose placements
are the centred layout with selection glow and editing caret as today.

`editor` moves the current `render::editor` implementation. It stacks the
body and footer areas exactly as today, calls `body` for the body entry, and
calls `layout::footer` for the footer entry.

The `view` module re-exports the public scene geometry types:

```rust
pub use crate::composer::Area;
pub use crate::layout::{Placement, PlacementNode};
```

`Area`, `Placement`, `PlacementNode`, and the public data needed to inspect
their variants are made public. Their existing geometry and rendering meaning
does not change.

### Renderer contract

`render::Renderer` changes from taking `&State` to taking `&Scene`:

```rust
pub trait Renderer {
    fn render(&mut self, scene: &Scene<'_>, out: &mut impl Write) -> io::Result<()>;
}
```

The trait remains re-exported at the crate root, as do `SvgRenderer` and the
existing platform-appropriate renderer exports. The render module imports
`crate::view::Scene`; terminal and SVG renderers do not import `State` for
rendering.

### Terminal renderer

`TerminalRenderer::render` receives a scene and loops over every
`(area, placements)` entry, passing each to its existing painting pipeline.
Its window still determines clipping and the caller still supplies
`view::editor(state, whole(window))`.

The terminal screen/controller builds the editor scene before calling the
renderer. Terminal renderer tests that exercise state-to-view composition move
to `view` tests; renderer tests construct scenes directly where they are
testing painting.

### SVG renderer

`SvgRenderer` replaces `mode: Mode` with the existing canvas distinction:

```rust
pub struct SvgRenderer {
    canvas: Option<(i64, i64)>,
}
```

- `Default` uses `None` and retains the Full HD export document framing.
- `with_canvas(columns, rows)` uses `Some((columns * CELL_WIDTH,
  rows * CELL_HEIGHT))` and retains editor-canvas framing.

`render` accepts a scene and passes all its entries to the existing SVG
document builder. It does not select `scene[0]`, inspect layer positions, or
remove caret placements. The caller supplies `view::body` for export and
`view::editor` for the canvas/editor path.

Delete the SVG `Mode` enum and `without_caret` function. `document` and its
helpers no longer accept a mode; they only receive the canvas dimensions and
the scene entries. The existing `Option<canvas>` branch selects document
framing, not scene contents.

### Callers and public paths

- `cli::export` builds the export scene with `view::body` and renders it with
  `SvgRenderer::default()`.
- `TerminalScreen` builds `view::editor` using its current terminal window.
- The public client-facing paths are `dre::view::{Scene, body, editor}` and
  the existing `dre::{Renderer, SvgRenderer}` exports.
- `render::editor` and `render::body` are deleted; all composition helpers
  (`centre`, `shift`, `align_right`) move with the view code or remain private
  helpers of that module.

### Tests

- `view` tests prove `body` returns one area, `editor` returns body then
  footer, and the editor body equals the body scene for the matching area.
- View tests preserve the existing caret, selection, centering, and footer
  expectations.
- Renderer tests prove terminal and SVG render every scene entry they receive.
- SVG tests prove an export scene has no footer and a canvas scene has one,
  without testing any cursor-filtering helper (which no longer exists).
- Public API/build coverage proves external callers can name `Scene`,
  `Area`, `Placement`, and `PlacementNode` and pass a scene to a renderer.
- The full existing test suite remains green.

### Dependencies

```text
state ───────────────→ view
diagram/layout ──────→ view
view::Scene ─────────→ render
render ──────────────→ terminal, svg
cli/editor controller → view, render
```

`view` knows application state and layout composition. `render` knows only
the scene contract plus backend drawing. Neither renderer reconstructs a view
from `State`.
