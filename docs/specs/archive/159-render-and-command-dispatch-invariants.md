# Render and command dispatch invariants

## Refactoring

Two renderer and command-dispatch boundaries currently rely on runtime
assumptions. Renderer helpers accept a whole `Placement` and then match its
`PlacementNode`, ending in `unreachable!` when called with another variant.
Command reduction handles four actions before the depth guard, while
`dispatch` still lists those same actions in an `unreachable!` arm.

This refactoring makes the renderer pipeline shared and makes the command
pipeline express the immediate-versus-selection-dependent split in its types.
It preserves rendered output and command behavior, including numeric command
prefixes.

## Technical Design

### Renderer data flow

- Keep `Shape` responsible only for per-pixel colour semantics, and keep
  `Canvas::fill` responsible for rasterising a `Shape` into a `Canvas`.
- Add one renderer-level placement helper that owns the visibility check and
  the final `frame.place` call. Box, arrow, glow, LED, glyph, caret, and cursor
  drawing all use this helper when placing a canvas.
- Add one renderer-level cached-sprite helper for the common sprite path. It
  checks visibility, looks up the supplied cache key, builds and remembers the
  canvas on a miss, and places the cached canvas at the supplied z-layer.
- Keep glyph caching in `GlyphSource`/`GlyphCache`, because glyphs are already
  rasterised independently and are keyed by character. Label rendering still
  creates one-cell placements per character, obtains each glyph from the glyph
  source, and passes each glyph through the shared placement helper.
- Caret and cursor rendering may continue to build a solid canvas directly;
  they must pass placement through the shared placement helper. Cursor remains
  an alias of caret rendering.

### Variant-specific canvas and key builders

- Replace whole-`Placement` matching in `sprite_key`, `glow_key`, and the
  sprite canvas builders with signatures that receive the placement geometry
  and the fields of the already-matched variant.
- The `paint` match is the place that proves the variant. Its box, glow, arrow,
  and LED arms pass their own fields and width/height into the shared cached
  helper and the corresponding canvas builder.
- Box, arrow, glow, and LED builders construct their existing `BoxShape`,
  `ArrowShape`, `GlowShape`, and `LedShape` values and call `Canvas::fill`.
  Their pixel behavior, colour calculations, cell-to-pixel conversion, and
  cache identity remain unchanged.
- Rename the `outline_*` helpers to canvas-building names such as
  `box_canvas`, `arrow_canvas`, `glow_canvas`, and `led_canvas`, or use an
  equivalent naming that does not imply they perform placement. None of these
  helpers matches on `PlacementNode` or uses an unreachable arm.
- `sprite_key` and `glow_key` likewise receive only the fields needed to form
  their key. A future change to a variant’s fields must therefore be handled
  by the compiler at the `paint` match and cannot silently reach an invalid
  helper call.

### Command categories

- Split command actions into two explicit categories:
  - `ImmediateCommand`: `Digit(u8)`, `CancelCount`, `Interrupt`, and
    `OpenNamePrompt`.
  - `SelectionCommand`: every command that proceeds through the depth guard
    and operates on the selected tree position, including `Undo`, `NewBox`,
    `Paste`, navigation, editing, mutations, and `Quit`.
- Update input parsing and action plumbing to construct the appropriate
  category while preserving the existing key bindings and public state
  behavior.
- `command::reduce` handles `ImmediateCommand` before the depth guard. The
  immediate branch continues to accumulate or clear `pending_count`, stop the
  application, or open the name prompt exactly as today.
- The depth guard and pending-count consumption apply only to
  `SelectionCommand`. `SelectionCommand` owns the existing minimum-depth
  mapping, so immediate actions do not need artificial depth values.
- Change `dispatch` to accept `SelectionCommand`, not the broad command
  action type. Its match is exhaustive over selection commands and has no
  `unreachable!` arm for immediate commands.
- Numeric prefixes retain their current semantics: digits accumulate with
  saturation, the next selection command consumes the pending count, commands
  that do not use repetition still consume it, and failed minimum-depth
  commands do not leak it to the next command.

### Tests

- Update renderer tests and helpers to call the variant-specific key and
  canvas builders with their fields rather than constructing a mismatched
  `PlacementNode`.
- Preserve and run the existing box, arrow, glow, LED, glyph, caret, cache,
  clipping, and pixel-output tests. Add focused tests for the shared placement
  helper’s visibility behavior if the existing tests do not cover both visible
  and clipped placements.
- Add or retain a test that every rendered placement variant reaches the
  common placement path, while labels still place one glyph per character.
- Update command tests to construct the new command categories and retain
  coverage for immediate actions, depth rejection, count accumulation,
  count consumption, repeated navigation, paste counts, and interruption/name
  prompt behavior.
- The full Rust test suite must pass. Compilation must prove that renderer
  helpers cannot receive the wrong placement variant and that `dispatch`
  cannot receive an immediate command.
