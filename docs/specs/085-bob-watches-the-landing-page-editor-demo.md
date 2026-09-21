# 085: Bob Watches The Landing Page Editor Demo

Bob opens the Dre landing page and watches Dre automatically demonstrate editing, so he can quickly understand that Dre draws and changes diagrams without installing anything.

## Acceptance Criteria

- When Bob opens the Dre landing page, an editor demo plays automatically.
- Bob sees boxes being drawn in front of him.
- Bob sees boxes being labeled.
- Bob sees a box change color.
- Bob does not need to interact for the demo to complete.

## Technical Design

Add a static browser demo under `examples/landing-page-editor-demo/`. The page is not a hand-drawn imitation of Dre: it drives the Dre reducer and SVG renderer through WebAssembly.

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
}
```

`Session` is intentionally just a public boundary around the internal `State`. It must hide `State` fields, modes, history, selection, and save details from callers. `press_key` delegates to the existing `state::handle_key` reducer, so the browser demo uses the same editing behavior as the terminal editor.

Keep rendering separate from the session. The existing renderer contract remains buffer-based:

```rust
pub trait Renderer {
    fn render(&mut self, doc: &Document, out: &mut impl Write) -> io::Result<()>;
}
```

Expose only the pieces needed by the web adapter: `Session`, `Renderer`, and `SvgRenderer`. Lower-level reducers and data-structure modules should stay private or crate-private unless the compiler forces a narrower visibility change for this API.

### Web adapter

The `web/` crate exports a JavaScript-friendly `WebSession` with `wasm-bindgen`:

```rust
#[wasm_bindgen]
pub struct WebSession {
    session: dre::Session,
}

#[wasm_bindgen]
impl WebSession {
    #[wasm_bindgen(constructor)]
    pub fn new() -> WebSession;

    pub fn press_key(&mut self, key: &str);

    pub fn svg(&self) -> String;
}
```

`WebSession::svg()` renders `self.session.document()` with `dre::SvgRenderer` into a byte buffer and returns SVG markup as a `String`. The SVG string convenience lives in the web adapter, not in the core session API.

### Demo page

Create `examples/landing-page-editor-demo/index.html`. It loads the generated WASM package from `examples/landing-page-editor-demo/pkg/`.

The page owns one `WebSession`, then plays a timed list of Dre key presses. After every key press, including label characters, the page calls `svg()` and replaces the displayed SVG markup. Bob should see the diagram appear one editing step at a time without clicking or typing.

Use a script that demonstrates:

- creating boxes with existing Dre keys,
- typing labels one character at a time,
- committing labels with Escape,
- changing a box color with the existing color command,
- optionally toggling fill after a color exists.

Do not add movement or cursor/selection rendering for this story. The earlier movement acceptance criterion is intentionally removed because the current SVG renderer does not show editor selection, and this story should not expand SVG rendering to include cursor state.

### Build tooling

- Update `flake.nix` so the development shell includes the `wasm32-unknown-unknown` target and `wasm-bindgen-cli`.
- Add a Makefile target, named `demo`, that builds the web crate for `wasm32-unknown-unknown` and writes/bundles the generated JS/WASM package into `examples/landing-page-editor-demo/pkg/`.
- `examples/landing-page-editor-demo/index.html` should work as a static example after `make demo` has produced `pkg/`.

### Tests

Keep tests focused on new contracts rather than retesting all existing reducers:

- Add Rust tests for `Session` proving `press_key("b")` creates a renderable box through the existing reducer.
- Add a Rust test proving a `Session` document can be rendered by `SvgRenderer`.
- Add a build check for the WASM/demo artifact through the new `make demo` path.
- Do not add browser automation for this story.
