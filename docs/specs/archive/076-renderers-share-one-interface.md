# 076: Renderers share one interface

Refactoring spec — no user story. Behaviour on screen and in exported SVGs is
unchanged.

## Goal

The terminal and the SVG output are both renderers: the same pipeline
(document → layout → drawing) with a different target. Today they share no
interface — `TerminalRenderer::render(&mut self, placements, cols, rows) ->
Vec<String>` and `SvgRenderer::render(&self, placements) -> String` — and each
caller does its own layout and then post-processes and writes the result. After
this spec both implement one `Renderer` trait, take the `Document`, and write
straight into a stream they are given, like `println!`.

## Technical Design

### Components

- `trait Renderer` — new, in `src/render.rs`.
- `TerminalRenderer` (src/render.rs) — implements `Renderer`.
- `SvgRenderer` (src/svg.rs) — implements `Renderer`.
- Callers: `writer::frame` / `writer::run` (the editor) and `cli::export`.

### The interface

```rust
pub(crate) trait Renderer {
    fn render(&mut self, doc: &Document, out: &mut impl Write) -> io::Result<()>;
}
```

- Input is the `Document` (`crate::state::Document` until the model moves out
  in 078). Layout happens inside the renderer; callers never call `layout` or
  `with_cursor` again.
- Output is written into `out`; nothing is returned but `io::Result<()>`.
  `out` is stdout for the terminal, the `.svg` `File` for export, and a
  `Vec<u8>` in tests.
- `&mut self` because `TerminalRenderer` owns a sprite cache; `SvgRenderer`
  simply doesn't use the mutability.

### Responsibilities / State

`TerminalRenderer`
- Knows: Kitty graphics, cell pixel size, sprite cache, and the viewport
  `cols` / `rows` (new fields).
- `resize(&mut self, cols, rows)` — the editor calls it each frame before
  `render`, so the trait signature carries no terminal-only parameters.
- `render` = `layout(&doc.boxes)` → `with_cursor(.., doc.selected)` → the
  existing centring, grid and sprites → writes `HOME_CURSOR`, the lines joined
  by `\r\n`, then the Kitty payload. (`HOME_CURSOR` and the painting currently
  in `writer::paint` move here.)
- Does not flush — the caller owns the stream.

`SvgRenderer`
- `render` = `layout(&doc.boxes)` → the existing SVG building → writes the
  document into `out`. Ignores `doc.selected` (no cursor in exports).

Both keep a private placement-level drawing function (the current body of
`render`, taking `&[Placement]`) that `render` calls after layout. The
existing fine-grained tests keep driving that function with hand-built
placements, so test churn stays small.

### Editor side (src/writer.rs)

- `frame` becomes: `renderer.resize(cols, rows)`, `renderer.render(&state.doc,
  stream)`, then — only in `Mode::SavePrompt` — the editor writes the prompt
  on top: move to the last row (`\x1b[{rows};1H`) and write
  `prompt_line(filename, cols)`. Then flush.
- The renderer never learns about modes; the save prompt is the editor's
  overlay, drawn after the frame instead of splicing the renderer's last line.
- `paint` is deleted (moved into `TerminalRenderer`).

### Export side (src/cli.rs)

- `cli::export` loads the document as today, then
  `SvgRenderer {}.render(&doc, &mut File::create(output_path(&input))?)`
  instead of `layout` + `fs::write`. (Removing `export` as a separate pipeline
  is spec 079's concern.)

### Dependencies / Collaborators

- `render.rs` and `svg.rs` depend on `layout` and `state::Document`.
- `writer.rs` depends on `Renderer` and `TerminalRenderer` only — no more
  `layout` import.
- `cli.rs` depends on `Renderer` and `SvgRenderer` — no more `layout` import.

### Tests

- New: a `Document` rendered by `SvgRenderer` into a `Vec<u8>` equals the
  placement-level output for `layout(&doc.boxes)` — proves `render` is just
  layout + drawing.
- New: `TerminalRenderer` renders into a `Vec<u8>` starting with `HOME_CURSOR`,
  with rows joined by `\r\n`, sized by the last `resize`.
- New: `TerminalRenderer` draws the cursor for `doc.selected`; `SvgRenderer`
  output is identical with and without a selection.
- Move writer's `paint` tests (home cursor first, no trailing newline) to
  `TerminalRenderer`.
- Update writer's prompt tests: in `SavePrompt` the output ends with the
  last-row move followed by `prompt_line`; in `Command` mode there is no
  `Save as:`.
- Existing SVG golden tests and `cli::export` tests pass unchanged.
