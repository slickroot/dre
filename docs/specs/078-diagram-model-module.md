# 078: Diagram model module

Refactoring spec — no user story. Placeholder, to be designed.

## Goal

Move `Node`, `Path`, `Document` and the tree helpers (`at`, `children_at`, `grow`, …) out of `state.rs` into `src/diagram.rs`, so renderers and layout no longer depend on the editor's state. `State` keeps only editor concerns: mode, undo history, running, save target.

## Technical Design

### Principle

The diagram is the data being drawn; the selection and the cursor belong to the editor. After this spec nothing on the render side (`render`, `svg`, `layout`) and nothing on the file side (`dre_format`, `file_document`) imports from `state`.

### `src/diagram.rs` — the model (new)

Knows the tree; knows nothing about modes, selection, undo or the insert cursor.

- `Node { label, colour: Option<u8>, filled, rounded, children }` — moved as-is, with its `Default`.
- `Path { ancestors, index }` — moved as-is (shared vocabulary: the editor selects with it, `layout::Label` carries it).
- `Diagram { boxes: Vec<Node> }` — newtype replacing `Document`. **No methods**; behaviour is free functions.
- Free functions over sibling lists:
  - `at(boxes: &mut Vec<Node>, path: &Path) -> &mut Node` — moved.
  - `children_at(boxes: &mut Vec<Node>, ancestors: &[usize]) -> &mut Vec<Node>` — moved.
  - `append(siblings: &mut Vec<Node>, node: Node) -> usize` — replaces `grow`. Pushes the node and returns its index; knows nothing about labels or `PAD`.
- Palette — single source of truth:
  - Private `PALETTE: [(u8, u8, u8); 5]` moved from `render.rs`.
  - `palette(index: u8) -> Option<(u8, u8, u8)>` — the only way to turn a colour index into a colour; `None` means "not in the palette".
  - `PALETTE_SIZE` is deleted. Validation (`dre_format`: `palette(i).is_some()`), cycling (`next_colour`: advance while `palette(i + 1).is_some()`) and rendering (`palette(i).unwrap()` for `Some(i)`; the plain grey for `None` stays a render concern) all go through `palette`. Spec 081's theme may wrap it, never copy the table.
- `#[cfg(test)] pub(crate) fn node(label)` and `node_with_children(label, children)` — the one set of model test builders. Local copies in `state`, `render`, `svg`, `layout`, `command_mode`, `insert_mode` tests are deleted in favour of `use crate::diagram::{node, node_with_children}`.

### `src/state.rs` — editor state only

- `State { diagram: Diagram, selected: Option<Path>, history: Vec<Snapshot>, mode, running, save_to, new_file, pending_count }` — `doc` is gone; selection lives here.
- Private `Snapshot { diagram: Diagram, selected: Option<Path> }`. `snapshot` pushes both, `undo` restores both — behaviour unchanged (undo still restores the cursor position and can never leave a dangling path).
- Stays here (moves with spec 080's editor module): `Mode`, `KeyBinding`, `DEFAULT_FILENAME`, `PAD`, `next_colour`, `colour_row`, `add_child_box`, `handle_key`, `new_state` (test helper, builds `Diagram` + `selected`).
- New editor helper `blank_box() -> Node` = `Node { label: PAD.to_string(), ..Default::default() }`; `add_child_box` and `command_mode.rs` call `append(siblings, blank_box())` instead of `grow`.

### Rendering — draws the diagram, nothing else

- `Renderer::render(&mut self, diagram: &Diagram, out) -> io::Result<()>` — no selection, no cursor. Both `TerminalRenderer` and `SvgRenderer` take `&Diagram`.
- `layout`:
  - `with_cursor`, `Cursor` and `PlacementNode::Cursor` are deleted.
  - New pure `centre(placements, cols, rows) -> (left, top)` — the offset currently computed inline in `TerminalRenderer::draw`; the renderer calls it, and so does the editor, so they cannot disagree.
  - New pure `label_end(placements, path) -> Option<(x, y)>` — where the cursor goes for a label (replaces the lookup inside `with_cursor`).
- `TerminalRenderer` loses `draw_cursor`/cursor stamping; `CURSOR` leaves `render.rs`.
- Tests: `render.rs` cursor tests move to the editor side; `svg.rs`'s "selection does not change the SVG" test is deleted (selection no longer exists there).

### The cursor — editor chrome

- `writer.rs` owns `CURSOR` and `draw_cursor(diagram: &Diagram, selected: &Path, cols, rows, out)`: runs `layout::layout`, `layout::centre`, `layout::label_end`, then writes `\x1b[row;colH` + `█` in the terminal's default colour. No box colour, no node lookup.
- `writer::frame` = `renderer.render(&state.diagram, out)` then, if `state.selected` is `Some`, `draw_cursor(...)`. Layout runs twice per frame — accepted: stateless, clean boundaries over performance.
- Moves into the editor module with spec 080.

### Loading and saving

- `file_document`:
  - `to_state(FileDoc) -> State` becomes `to_diagram(FileDoc) -> Diagram`.
  - `from_state(&State) -> FileDoc` becomes `from_diagram(&Diagram) -> FileDoc`.
  - No longer imports `state`.
- "Select the first top-level box if there is one" is editor policy: `writer::load_state` builds `State` from `to_diagram(...)` and applies it.
- `cli::export` calls `to_diagram` directly and renders it with `SvgRenderer`.
- `dre_format` validates colours with `diagram::palette` instead of `state::PALETTE_SIZE`.

### Dependencies after this spec

```
diagram      ← (nothing)
layout       → diagram
render, svg  → diagram, layout
dre_format   → diagram
file_document→ diagram, dre_format
state, command_mode, insert_mode, save_prompt_mode → diagram
writer       → state, diagram, layout, render, file_document
```
