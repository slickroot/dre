# 085: Bob Watches The Landing Page Editor Demo

Bob opens the Dre landing page and watches Dre automatically demonstrate editing, so he can quickly understand that Dre draws and changes diagrams without installing anything.

## Acceptance Criteria

- When Bob opens the Dre landing page, an editor demo plays automatically.
- Bob sees boxes being drawn in front of him.
- Bob sees boxes being labeled.
- Bob sees a box change color.
- Bob does not need to interact for the demo to complete.

## Technical Design

Dre ships a WebAssembly package that drives the Dre reducer and SVG renderer. The landing page demo itself (the page, its key script and styling) lives in the landing page project, which copies the generated `pkg/` from this repository. This repository does not contain the demo page.

### Crate shape

- Keep the root package named `dre`, but add `src/lib.rs` so the package has both a library target and the existing binary target.
- Move module ownership declarations that need to be shared by the binary and library from `src/main.rs` into `src/lib.rs`.
- Keep `src/main.rs` thin: it imports the library modules and continues to run the current CLI/editor behavior.
- Add a `web/` crate for the `wasm-bindgen` adapter. This crate depends on the root `dre` library by path.
- Convert the repository to a Cargo workspace only as much as needed for the root package and `web/` crate to build together.

### Public library API

Expose a small opaque session facade from the root `dre` library:

```rust
pub struct Session {
    state: state::State,
}

impl Session {
    pub fn new() -> Self;
    pub fn press_key(&mut self, key: &str);
    pub fn is_running(&self) -> bool;
    pub fn document(&self) -> &diagram::Document;
    pub fn extent(&self) -> (i64, i64); // [width, height] in cells of the laid-out document, (0, 0) when empty
}
```

`Session` is intentionally just a public boundary around the internal `State`. It must hide `State` fields, modes, history, selection, and save details from callers. `press_key` delegates to the existing `state::handle_key` reducer, so the browser uses the same editing behavior as the terminal editor. `extent` is the maximum right and bottom edge of the document's layout.

Keep rendering separate from the session. The existing renderer contract remains buffer-based:

```rust
pub trait Renderer {
    fn render(&mut self, doc: &Document, out: &mut impl Write) -> io::Result<()>;
}
```

Expose only the pieces needed by the web adapter: `Session`, `Renderer`, and `SvgRenderer`. Lower-level reducers and data-structure modules stay private or crate-private.

### SVG renderer

- `SvgRenderer::with_canvas(cols, rows)` produces a fixed `viewBox="0 0 cols*8 rows*16"`.
- `SvgRenderer::centered_on(width, height)`, where width and height are a diagram extent in cells, translates the drawing by `((cols - width) / 2, (rows - height) / 2)` cells, rounded down and clamped at zero, so the extent sits in the middle of the canvas. Without `centered_on`, rendering is unchanged. It is unit tested for an extent centered in a larger canvas, equal to the canvas (zero offset), and larger than the canvas (clamped to zero).
- Label text is drawn with a font size derived from the cell width (`CELL_WIDTH / 0.6`, the monospace advance ratio), so a glyph advances one cell.
- Label `<text>` elements carry `xml:space="preserve"`. Labels typed in the editor end with a trailing space that counts toward `textLength`; without preserved whitespace the visible glyphs were stretched over one extra cell and changed width as characters were typed.

### Web adapter

The `web/` crate exports a JavaScript-friendly `WebSession` with `wasm-bindgen`:

```rust
#[wasm_bindgen]
impl WebSession {
    #[wasm_bindgen(constructor)]
    pub fn new() -> WebSession;

    pub fn press_key(&mut self, key: &str);

    pub fn extent(&self) -> Vec<i32>; // [width, height] in cells of the current document

    pub fn svg(&self, cols: i32, rows: i32, extent_width: i32, extent_height: i32) -> String;
}
```

`svg` renders the session's document with `dre::SvgRenderer::with_canvas(cols, rows).centered_on(extent_width, extent_height)` into a byte buffer and returns SVG markup as a `String`. The SVG string convenience lives in the web adapter, not in the core session API. The adapter has no hardcoded canvas size: the caller owns the grid.

### How the landing page uses it

This is guidance for the consumer, not code in this repository:

- Own one `WebSession` and play a timed list of Dre key presses (create boxes, type labels one character at a time, commit with Escape, change a box colour with the existing colour command, optionally toggle fill). After every key press, call `svg()` and replace the displayed markup.
- Size the grid from the window: an integer `SCALE` of the renderer's native 8x16 cell, `cols = floor(width / (8 * SCALE))`, `rows = floor(height / (16 * SCALE))`, and size the SVG element to exactly `cols*8*SCALE x rows*16*SCALE` px, centered on a page with the same background.
- For stable placement, run the whole key script once on a throwaway `WebSession`, read `extent()` once, and pass that fixed extent on every render, including after a resize.
- No movement or cursor/selection rendering: the SVG renderer does not show editor selection and this story does not expand it.

### Build tooling

- `flake.nix` includes the `wasm32-unknown-unknown` target and `wasm-bindgen-cli` in the development shell.
- `make wasm` builds the `web` crate for `wasm32-unknown-unknown` and runs `wasm-bindgen --target web --out-name dre_web` into `web/pkg/` (gitignored). `make wasm RELEASE=1` does the same with a release build, which is what gets copied into the landing page project.

### Tests

- Rust unit tests for `centered_on`, the label font size, and preserved label whitespace.
- A Rust test for `WebSession::extent()` after `press_key("b")`, and for `svg` with the new signature.
- The WASM artifact is checked by building it through `make wasm`.
- No browser automation for this story.
