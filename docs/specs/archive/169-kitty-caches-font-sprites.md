# Kitty caches font sprites

This is a refactoring spec. It changes how terminal frames are transported, not what the user sees.

## Problem

`GlyphCache` caches rasterized glyph `Canvas`es in the process, but every render still zlib-compresses, base64-encodes, and transmits every glyph's complete RGBA payload. `Frame::into_bytes` starts each frame with Kitty's uppercase `d=A`, which removes the visible placements and frees their stored image data, and `kitty::show` then sends every image again with `a=T` and no image ID.

Kitty can retain transmitted image data under a numeric image ID and create new placements from it. Repeated font sprites should use that terminal-side cache as the first, deliberately narrow adoption of this capability. Boxes, arrows, glows, LEDs, and carets continue to be freshly transmitted on every frame.

## Acceptance Criteria

- The rendered screen is visually unchanged.
- The first visible occurrence of a glyph identified by character, colour, and weight transmits its RGBA payload to Kitty.
- Further occurrences of that same glyph, including occurrences in later frames, place the existing Kitty image without retransmitting its pixel payload.
- Glyphs that differ by character, colour, or weight do not share a Kitty image.
- Every redraw removes all previous image placements from the visible screen.
- Non-glyph images are still transmitted afresh on every frame, and their previous-frame image data is explicitly freed.
- Terminal-side glyph images remain cached for the lifetime of the alternate-screen session.

## Technical Design

### Two independent caches

The existing CPU-side and new terminal-side caches remain separate:

- `render/font.rs::GlyphCache` continues to own font loading, rasterization, and its cached `Canvas` pixels. It has no dependency on Kitty and its `GlyphSource` interface remains canvas-oriented.
- `render/terminal.rs::TerminalRenderer` owns Kitty image identity and lifetime. It gains a private `GlyphKey { character: char, colour: Option<u8>, bold: bool }`, a `HashMap<GlyphKey, kitty::ImageId>`, a monotonically increasing image-ID allocator, and the IDs of transient images from the most recently rendered frame.
- `kitty.rs` only knows how to encode typed image IDs into protocol commands. It does not know glyph keys or cache policy.

The existing `SpriteKey -> Canvas` cache for boxes, arrows, glows, and LEDs is unchanged. It still avoids rerasterization in Dre, but those canvases are not retained in Kitty in this slice.

### Image IDs

`kitty.rs` introduces an opaque, copyable `ImageId` backed by `NonZeroU32`. Kitty reserves zero to mean “no ID”, so zero cannot be constructed. `TerminalRenderer` allocates IDs sequentially from 1 with checked arithmetic. It never derives an ID by hashing a `GlyphKey`, eliminating hash-collision aliases. Exhausting all 4,294,967,295 non-zero IDs is treated as an impossible session invariant and fails explicitly rather than wrapping and silently reusing a live ID.

All images transmitted by Dre receive an ID:

- A glyph's ID is inserted into the glyph-image map and retained for the session.
- Every non-glyph placement receives a fresh transient ID. Its ID is retained only until the following frame so its terminal data can be hard-deleted.

Dre enters a newly cleared alternate-screen buffer before rendering, so sequential IDs are scoped to that private rendering session. It does not add Kitty's terminal-assigned `I=` image-number response flow.

### Kitty commands

The protocol surface in `kitty.rs` becomes explicit:

- `soft_clear()`: `a=d,d=a,q=2`. This removes all visible placements but deliberately retains their stored image data.
- `delete(id)`: `a=d,d=I,i=<id>,q=2`. This frees the named image's data after its placements have been removed.
- `show(canvas, id, col, row, z)`: the existing compressed and chunked `a=T` transmission, now including `i=<id>`. It transmits and creates the first placement in one operation.
- `place(id, col, row, z)`: `a=p,i=<id>,q=2,z=<z>` with no payload. Like `show`, it first moves the terminal cursor to the requested cell.

Only the first chunk of a `show` transmission carries the action, image ID, dimensions, format, compression, quiet mode, and z-index. Continuation chunks keep the current protocol shape and carry only `m`.

The old uppercase delete-all `clear()` command is removed from the per-frame path. Leaving the alternate screen remains responsible for final session cleanup.

### Frame representation and lifecycle

`Placed` distinguishes a fresh pixel transmission from a cached placement while retaining the common `col`, `row`, and `z` fields:

```rust
enum Image {
    Fresh { id: kitty::ImageId, canvas: Canvas },
    Cached { id: kitty::ImageId },
}

struct Placed {
    image: Image,
    col: i64,
    row: i64,
    z: i32,
}
```

`Frame` also receives the prior frame's transient IDs. After emitting the unchanged character grid, `Frame::into_bytes` emits commands in this exact order:

1. `kitty::soft_clear()`, removing every old graphic placement from view while preserving addressable image data.
2. One `kitty::delete(id)` for each prior-frame transient image, freeing the non-glyph data that will not be reused.
3. Every new `Placed` image in existing scene order: `kitty::show` for `Fresh`, or `kitty::place` for `Cached`.

The first frame has no transient IDs to delete. During construction of the new frame, every fresh non-glyph ID is collected for deletion by the next frame.

There are no anonymous images: soft-clearing an anonymous image would make its retained data unreachable and leave cleanup to terminal storage pressure. Giving transient images IDs makes their cleanup deterministic even though only glyphs are reused.

### Drawing labels

`draw_label` continues its current visibility check before doing cache work. For each visible character it constructs `GlyphKey` from the character plus the label's colour and `bold` flag:

- On a terminal-cache miss, it allocates an ID, asks `GlyphSource` for the canvas, records the key-to-ID mapping, and queues `Image::Fresh`. The first occurrence therefore uses `a=T,i=<id>`.
- On a hit, it does not request or clone the canvas; it queues `Image::Cached`. The occurrence uses `a=p,i=<id>`.
- Inserting the mapping at the miss means a second occurrence of the same glyph in the same frame is already a hit.

Glyph placements are one cell and clipping areas are cell-aligned, so a visible glyph is always placed from its complete cached one-cell image. Existing visibility behavior remains unchanged.

### Cache lifetime and deferred work

The glyph-image map has no application-level eviction in this slice. It lives as long as `TerminalRenderer`, matching the existing unbounded `GlyphCache`; leaving the alternate screen clears the terminal buffer. An LRU or size bound, terminal-eviction detection and `ENOENT` recovery, and extending terminal-side reuse to non-font sprites are separate future changes.

### Tests

`kitty.rs` unit tests assert the exact protocol boundaries:

- `soft_clear` uses lowercase `d=a`, not uppercase `d=A`.
- `delete` uses uppercase `d=I` and the requested non-zero image ID.
- `show` includes `a=T` and the image ID on its first chunk while preserving compression and chunk continuation behavior.
- `place` contains `a=p` and the same image ID, positions the cursor, and contains no pixel payload.

`render/terminal.rs` tests exercise behavior through complete frame bytes:

- Two identical glyphs in one frame produce one payload transmission followed by a placement using the same ID.
- Rendering the same glyph in consecutive frames produces its payload only in the first frame; the later frame soft-clears and uses only `a=p` for that glyph.
- Character, colour, and bold differences each produce distinct IDs and payload transmissions.
- The first frame has no hard-delete commands.
- A non-glyph image is freshly transmitted with a new ID on consecutive frames, and the second frame hard-deletes its first ID.
- Soft clear precedes transient hard deletes, which precede this frame's image commands.
- Existing placement order, clipping, z-index, local shape-cache, resize, and visual pixel tests continue to pass.
