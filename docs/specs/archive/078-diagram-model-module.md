# 078: Diagram model module

Refactoring spec — no user story.

## Goal

Move `Node`, `Path`, `Document` and the tree helpers (`at`, `children_at`, `grow`, …) out of `state.rs` into `src/diagram.rs`, so renderers, layout and the file format no longer depend on the editor's state. `State` keeps only editor concerns: mode, undo history, running, save target.

## Technical Design

### Principle

After this spec nothing on the render side (`render`, `svg`, `layout`) and nothing on the file side (`dre_format`, `file_document`) imports from `state`.

The selection stays part of `Document`: the cursor is drawn by the renderer as part of the grid, as today. An earlier draft pulled the selection out into `State` and drew the cursor from the editor; that needed a shared centring function, a label-end lookup, a second layout per frame, and lost the renderer's clipping of an off-screen cursor — more machinery than it removed.

### `src/diagram.rs` — the model (new)

- `Node { label, colour: Option<u8>, filled, rounded, children }` — moved as-is, with its `Default`.
- `Path { ancestors, index }` — moved as-is.
- `Document { boxes: Vec<Node>, selected: Option<Path> }` — moved as-is.
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

- `State { doc: Document, history: Vec<Document>, mode, running, save_to, new_file, pending_count }` — unchanged apart from `Document` now coming from `diagram`. Undo keeps restoring the whole `Document`, selection included.
- Stays here (moves with spec 080's editor module): `Mode`, `KeyBinding`, `DEFAULT_FILENAME`, `PAD`, `next_colour`, `colour_row`, `add_child_box`, `handle_key`, `new_state`.
- New editor helper `blank_box() -> Node` = `Node { label: PAD.to_string(), ..Default::default() }`; `add_child_box` and `command_mode.rs` call `append(siblings, blank_box())` instead of `grow`.

### Rendering — unchanged

- `Renderer::render(&mut self, doc: &Document, out)` as today; `TerminalRenderer` still draws the cursor for `doc.selected` via `layout::with_cursor`; `SvgRenderer` ignores the selection.
- Only the imports change (`crate::diagram` instead of `crate::state`).

### Loading and saving

- `file_document`:
  - `to_state(FileDoc) -> State` becomes `to_document(FileDoc) -> Document`, with `selected: None`.
  - `from_state(&State) -> FileDoc` becomes `from_document(&Document) -> FileDoc`.
  - No longer imports `state`.
- "Select the first top-level box if there is one" is editor policy: `writer::load_state` builds `State` from `to_document(...)` and applies it.
- `cli::export` calls `to_document` directly and renders it with `SvgRenderer`.
- `dre_format` validates colours with `diagram::palette` instead of `state::PALETTE_SIZE`.

### Dependencies after this spec

```
diagram      ← (nothing)
layout       → diagram
render, svg  → diagram, layout
dre_format   → diagram
file_document→ diagram, dre_format
state, command_mode, insert_mode, save_prompt_mode → diagram
writer       → state, diagram, render, file_document
```
