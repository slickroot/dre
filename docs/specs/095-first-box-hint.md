# First box hint

## User Story

As a first-time dre user, I want to see a hint on the empty canvas telling me how to add my first box, so that I know how to get started without reading documentation.

## Acceptance Criteria

- Whenever the canvas has no boxes, a faded hint box reading "press b to add a box" is shown where the first box would appear.
- As soon as a box is added, the hint disappears. If the canvas becomes empty again (e.g. after deleting or undoing the last box), the hint reappears.

## Technical Design

**Visibility is purely derived, not tracked.** The hint is not a session flag on `State` — it shows whenever `state.doc.boxes.is_empty()`, full stop. No new field on `State` (src/state.rs:27) is needed.

**Positioning reuses the real box layout.** A synthetic hint box is expressed as an ordinary `Node`:

```rust
Node {
    label: "press b to add a box".to_string(),
    hint: true,
    ..Default::default()
}
```

(plain border, not filled, not rounded — the same default appearance as a freshly added box via `state::blank_box()`). Passing `&[hint_node]` through the existing `crate::layout::layout` gives back a `Node` placement and a `Label` placement, so it gets centred on screen exactly like a real box would, with zero new layout code.

**A new `hint: bool` field on `Node`** (src/diagram.rs), defaulting to `false`, marks this box as a hint rather than encoding "light gray" as a real palette entry (`PALETTE`, src/diagram.rs:34, is user-facing box colour and shouldn't gain a UI-only entry). `layout::place`'s `emit()` (src/layout.rs:70) copies this flag onto the `Label` it creates, since `PlacementNode::Label` doesn't otherwise carry a `Node` reference.

**Rendering fades rather than recolours.** Both the box edge/fill (`outline_box`, src/render/terminal.rs:352) and the glyph ink (`GlyphCache::rasterize`, src/render/font.rs:61) use the same grey (`PLAIN_COLOUR`) as normal plain boxes and text, but at `OPAQUE / 4` alpha when `hint` is true — reading as a faded, muted version of normal chrome rather than introducing a new colour concept. `GlyphCache`'s cache key extends from `char` to `(char, bool)` to hold both the full-opacity and faded rasterizations of each glyph.

**The renderer stays in charge of the branch.** `TerminalRenderer::render_diagram` (src/render/terminal.rs:267) decides which placements to draw:

```rust
let placements = if state.doc.boxes.is_empty() {
    layout(&[hint_node()])
} else {
    with_cursor(layout(&state.doc.boxes), state.doc.selected.clone())
};
```

No selection cursor applies to the hint (there's nothing to select), so `with_cursor` is skipped on that branch. `state`/`editor` remain unaware that hints exist at all.
