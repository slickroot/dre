# 039 - Rename graphics renderer to terminal renderer

## Refactoring

`GraphicsRenderer` in `sketch/render.py` composites a text renderer with a graphics protocol to draw sprites via Kitty graphics. Since sketch no longer supports plain terminals — every supported terminal must implement the Kitty graphics protocol — the "graphics renderer" *is* the terminal renderer; there is no other kind. Rename it to reflect that it's the only rendering path, not an optional graphical enhancement.

## Technical Design

- `GraphicsRenderer` (in `sketch/render.py`) is renamed to `TerminalRenderer`. It's the only renderer sketch has — the "graphics" qualifier no longer distinguishes it from anything.
- The existing `TerminalRenderer` class — the plain-text grid renderer it used to wrap — is deleted, not renamed. Its responsibilities (blanking a box's cells, stamping the cursor, stamping label text into the character grid) fold into the new `TerminalRenderer` as private methods and state: `render()` builds the grid itself (via a private `_grid()` method replacing the old collaborator's `render()`) before compositing the Kitty payload onto the last line, and `_draw_box`, `_draw_cursor`, `_draw_label`, `_put`, `_stamp` move over as private methods.
- The constructor drops the `text` parameter, since there's no longer a separate text-rendering collaborator: `TerminalRenderer(graphics: GraphicsProtocol, cell_width: int, cell_height: int)`.
- `_box_character()` is dropped as dead code — it was never called; boxes are drawn entirely through sprites, and the grid only blanks the cell for a `Box` placement. The constants that existed solely for it (`TOP_LEFT`, `TOP_RIGHT`, `BOTTOM_LEFT`, `BOTTOM_RIGHT`, `HORIZONTAL`, `VERTICAL`) are removed too.
- The `Renderer` protocol is unchanged and keeps its name — it's still the public type `writer.py`'s `frame()` depends on, now with `TerminalRenderer` as its sole implementation.
- `GraphicsProtocol` and `KittyGraphics` are unchanged — `GraphicsProtocol` still names the Kitty-specific drawing capability, a distinct concept from the terminal renderer that uses it.
- `sketch/writer.py` updates its instantiation from `GraphicsRenderer(TerminalRenderer(), KittyGraphics(), *cell_size())` to `TerminalRenderer(KittyGraphics(), *cell_size())`.
- `tests/test_graphics.py` is merged into `tests/test_render.py`: its `FakeText`-based tests (which faked the now-gone text collaborator) are dropped, and the merged renderer is exercised directly with real placements. `BoxCharacterTest` in `tests/test_render.py` is deleted along with the method it tested. `tests/test_kitty.py` is unaffected.
- `sketch/render.py` keeps its filename — it already generically houses the rendering module (`Sprite`, `Canvas`, `RoundedBox`, etc.), not just the renderer class.

