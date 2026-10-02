# A virtual terminal sends only what changed

## Refactoring Goal

Every frame repaints the whole terminal. `Frame::into_bytes` (`src/render/terminal.rs:438`) rewrites a screen full of blank characters (nothing ever writes into `Frame::characters`), deletes every kitty placement with `soft_clear`, and places everything again under placement IDs numbered from 1. Boxes are drawn as one tile per cell (`TileShape::tiles`, `src/render/tiles.rs:84`), including the empty middle, so an 80-column outer flex box is about 240 placements on every keystroke. Whole sprites (arrows, LEDs, untileable boxes) and the legacy caret are re-transmitted under a new image ID every frame.

After this refactor, a frame sends only the difference from what the terminal already shows, like React's reconciler. Nothing the user sees changes, except that it gets faster. This applies to both dre-flex and the legacy editor, which share `TerminalRenderer`.

Prototyped in throwaway scratchpads (not committed): full redraw, append-only, and a move-diff of tiled versus stretched boxes. Both per-cell tiles and stretched slices render correctly in WezTerm. Widening a box costs 6 commands with tiles (3 moves, 3 adds) and 5 with slices (re-places only). Moving a box costs `width × rows` commands with tiles and 8 with slices.

## Technical Design

### Layers

Mirrors React: render, reconcile, commit.

| Layer | Knows | Does |
|---|---|---|
| View (`flex::view::scene`, `view::editor`) | state | `State → Scene`. Unchanged. |
| Painter (`TerminalRenderer`) | glyph source, canvas caches, cell size | `Scene → Vec<Desired>`, and implements `Sprites`. Never sees kitty IDs or bytes. |
| `VirtualTerminal` (new) | the uploaded images and placements the terminal currently shows | `commit(desired, sprites) → Vec<Op>`, `reset() → Vec<Op>` |
| `kitty` | escape syntax | `encode(&[Op]) → bytes`, reusing `show` / `place` / `grow` / `delete` |

`TerminalRenderer::render` paints, commits, encodes, and wraps the bytes in the synchronized update (`BEGIN_SYNCHRONIZED_UPDATE` … `END_SYNCHRONIZED_UPDATE`).

### Vocabulary

```rust
enum ImageKey { Glyph(GlyphKey), Tile(TileKey), Bracket(BracketKey), Sprite(SpriteKey), Caret(CaretKey), Grow(GrowKey) }

struct Desired {
    image: ImageKey,
    col: i64, row: i64, z: i32,
    source: Option<SourceRect>,   // pixels within the image, `x,y,w,h`
    cells: Option<(i64, i64)>,     // stretch over `c,r` cells
}

enum Content { Still(Canvas), Animation { root: Canvas, frames: Vec<Canvas> } }

trait Sprites {
    fn content(&mut self, key: &ImageKey) -> Content;
}

enum Op {
    Upload { image: ImageId, content: Content },
    Place  { image: ImageId, placement: PlacementId, col: i64, row: i64, z: i32, source: Option<SourceRect>, cells: Option<(i64, i64)> },
    Delete { image: ImageId, placement: PlacementId },
    Free   { image: ImageId },
}
```

`Desired` must be `Eq + Hash`. `GrowKey` carries the existing `grow_stamp`, so every grow animation is a distinct image.

### Reconciling: a flat set diff

kitty placements form a flat set with absolute positions and `z` ordering, not a tree. `commit`:

1. **Kept:** a desired placement equal to one already shown claims that placement. No op.
2. **Re-placed:** a remaining desired placement takes any unclaimed placement of the **same image** and re-places it under the old `(image, placement)` ID. In kitty, `a=p` with an existing `i,p` replaces the placement, so moving, re-stretching and re-cropping are all one `Place`. This is React's "update props".
3. **Placed:** only when no placement of that image is left over does it get a new placement ID. If the image isn't uploaded, the `VirtualTerminal` assigns an `ImageId`, calls `sprites.content(key)` and emits `Upload` first.
4. **Deleted:** leftover shown placements get `Delete`.
5. **Freed:** any image with no placements left gets `Free` (`a=d,d=I`) and is forgotten.

Placements of the same image are interchangeable: they carry no state, so any pairing gives the same pixels. That's why there are no React-style `key`s. Op order inside a commit is: deletes, uploads, places, frees.

Placement IDs are allocated per image, and every `Place` carries one. That keeps the WezTerm workaround in `Frame::placement_ids` (`src/render/terminal.rs:458`): WezTerm removes every placement of an image when a new one has no placement ID.

### Images are freed when unused

There is one lifetime rule for every image. It replaces three:
- Glyphs, tiles and brackets were kept forever. Now they're re-uploaded if they come back after being unused. Backspace then retype re-sends one 1-cell glyph, and leaving write mode in flex re-sends the selected-style tiles.
- Whole sprites were re-transmitted each frame (`place_transient`). Now they're uploaded once while in use.
- Grow animations were freed through `previous_transient_images`. Now they're freed the frame after they play, because nothing places them any more.

Deleted: `ImageIds`, `previous_transient_images`, `Image::{Fresh, Cached, Growing}`, `Placed`, `Frame::placement_ids`, `place_sprite`'s ID allocation, and the `glyph_images` / `tile_images` / `bracket_images` ID maps. The painter's canvas caches (`cache`, `tile_canvases`, `GlyphCache`) stay. They're what `Sprites::content` reads from.

The legacy caret/cursor (`draw_caret`) gets a `CaretKey` (its size in cells) so it's re-placed, not re-sent.

### Stretched middles

Tiles stay, and so does spec 170's band machinery (`Band::Start(n)` / `Middle` / `End(n)`, band width from border extent and radius). What changes is that the **middle band is stretched rather than repeated**:
- Start/End band cells stay one `Desired` per cell, so rounded corners, thick borders and glows are unchanged.
- A run of `Middle` cells along an edge becomes one `Desired` of that middle tile with `cells: Some((run, 1))` or `Some((1, run))`.
- The middle-middle region is one `Desired` stretched both ways, or omitted when its tile is fully transparent.

A flex box (square, `FLEX_BORDER = 1`, band 1) becomes 8 placements, whatever its size. `TileShape::tiles` returns runs instead of every cell. `TileKey` is unchanged.

### Clipping is a placement property

Uploaded content is always the **whole** image, so a persistent image is never stored cropped. A partly visible placement says what to show:
- Natural-size sprites (glyphs, brackets, whole sprites): `source` set to the visible pixel rectangle.
- Stretched runs: a shortened `cells`.

`Frame::crop`'s geometry maths moves into the painter, and `Canvas::crop` leaves the transmit path. This is the part expected to be rewritten later; keeping it in `Desired` means a rewrite touches only the painter.

### Startup and resize: reset

`VirtualTerminal::reset()` forgets everything and returns ops that clear the screen (`ESC[2J`) and delete every placement and free every image (`a=d,d=A`). It runs at startup and on every `tty::RESIZE` (`FlexScreen::resize`, and the legacy editor's equivalent). The next commit re-uploads and re-places everything. A resize costs one full redraw, and no terminal-specific resize behaviour is relied on. `ESC[2J` at reset replaces the per-frame blank-grid repaint for good, and `Frame::characters` is deleted.

### Tests

- `VirtualTerminal` (new module, inline `#[test]`, fake `Sprites` returning a 1×1 canvas and recording requested keys), asserting on `Vec<Op>`:
  - `the_first_commit_uploads_each_image_once_and_places_everything`
  - `an_unchanged_frame_commits_no_ops`
  - `a_moved_placement_is_re_placed_under_its_old_id`
  - `a_placement_is_re_placed_from_any_spare_placement_of_the_same_image`
  - `a_new_placement_with_no_spare_gets_a_new_placement_id`
  - `a_placement_no_longer_desired_is_deleted`
  - `an_image_with_no_placements_left_is_freed`
  - `a_freed_image_is_uploaded_again_when_it_comes_back`
  - `content_is_only_asked_for_on_upload`
  - `reset_clears_and_forgets_everything`
  - `deletes_come_before_uploads_and_places_and_frees_come_last`
- `kitty::encode`: one test per `Op` against the escape bytes, next to the existing `src/kitty.rs:432-521` tests. `Place` with `source` emits `x,y,w,h`, and with `cells` emits `c,r`.
- Painter (`src/render/terminal.rs`), asserting on `Vec<Desired>`:
  - `a_tileable_box_stretches_its_middle_band`
  - `an_unfilled_box_omits_its_middle`
  - `a_flex_box_is_eight_placements_whatever_its_size`
  - `a_partly_visible_sprite_carries_a_source_rect`
  - `a_partly_visible_run_is_shortened`
  - `the_caret_is_keyed_by_its_size`
- The existing byte-level tests that use the `composed` / `sprites` helpers (`src/render/terminal.rs:1616-1715`) move to asserting on `Desired` or `Op`. Pixel-identity tests for tiles stay as they are.
- Behaviour that must not change: `flex::view` tests and the `FlexScreen` / `run_loop` mock tests in `src/flex/mod.rs` are untouched.

### Rejected

- **Pure appending**: flex re-centres the stack, so adding a box moves every other box.
- **Diffing in flex from `FlexState`**: only the painter knows the real placements (tiles, glyphs, clipping).
- **Semantic placement keys** (box path plus tile offset): placements have no state, so pairing by image matches at least as much with no identity plumbing.
- **Nine-slice from one image per box style**: cropping from a single atlas works in WezTerm, but stretching the existing middle tiles reaches the same placement count while keeping spec 170's bands, rounded corners and glows.
- **Keeping per-cell middle tiles**: cheap to spread (6 commands), but every flex re-centre moves every cell of every box.
- **`commit` writing bytes**: diff tests would have to parse escape sequences.
- **Desired placements carrying their pixels**: the painter would build or look up content for every placement on every frame, including unchanged ones.
- **Keep-forever images with a transient flag**: size-keyed sprites accumulate for the session, and the painter has to know which keys are transient.
- **Trusting the diff across a resize**: terminals differ in what they do with placements on resize, and a cell-size change invalidates every tile.
