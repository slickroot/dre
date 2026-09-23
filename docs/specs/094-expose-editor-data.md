# Expose editor data

## User Story

As a developer embedding the web editor, I want `WebSession` to expose the current mode and total box count, so that I can build UI around the editor's state without reaching into internal editor logic.

## Acceptance Criteria

- `WebSession` exposes a method `info(key: &str) -> String` that returns editor data by key.
- `info("mode")` returns the current mode as one of `"Command"`, `"Insert"`, or `"SavePrompt"`.
- `info("box_count")` returns the total number of boxes in the diagram (as a string), counting every box including nested children.
- Both values can be read independently via direct calls to `info()` (not only through the `on_change` callback).
- `info()` returns an empty string for any unrecognized key.

## Technical Design

### Overview

Add a selector-style free function `editor_info(state: &State) -> EditorInfo` in `src/state.rs`, inside the platform-agnostic `dre` crate, so both the terminal and web front ends can derive the same render-ready data from `State`. `EditorInfo` holds only strings, since every field is display-ready data for direct rendering — no client-side parsing required.

`WebSession` (in `web/src/lib.rs`) exposes a single `info(&self, key: &str) -> String` method, matching the existing flat-return-type style of `extent()`/`svg()`. It calls `editor_info()` on the current state and picks the field matching `key`, returning `""` for an unrecognized key. This avoids introducing a `#[wasm_bindgen]` struct/getters, and avoids pulling wasm-bindgen into the core `dre` crate.

### `src/state.rs`

```rust
pub struct EditorInfo {
    pub mode: String,
    pub box_count: String,
}

pub fn editor_info(state: &State) -> EditorInfo {
    let mode = match &state.mode {
        Mode::Command => "Command",
        Mode::Insert => "Insert",
        Mode::SavePrompt { .. } => "SavePrompt",
    }
    .to_string();

    fn count(nodes: &[Node]) -> usize {
        nodes.iter().map(|n| 1 + count(&n.children)).sum()
    }
    let box_count = count(&state.doc.boxes).to_string();

    EditorInfo { mode, box_count }
}
```

`EditorInfo` and `editor_info` are `pub` (not `pub(crate)`) so the `web` crate can reach them via `dre::state::{editor_info, EditorInfo}` (or re-exported from the crate root alongside `Document`/`State`).

### `web/src/lib.rs`

```rust
pub fn info(&self, key: &str) -> String {
    let info = dre::editor_info(self.session.borrow().state());
    match key {
        "mode" => info.mode,
        "box_count" => info.box_count,
        _ => String::new(),
    }
}
```

The keys accepted by `info()` are exactly the field names on `EditorInfo` (`"mode"`, `"box_count"`), passed straight through with no renaming — new fields added to `EditorInfo` in the future are automatically reachable via the same key.

### Notes

- `Mode::SavePrompt { filename }`'s `filename` is intentionally ignored here — `editor_info` reports only the mode kind, not the in-progress filename text.
- Box counting is inlined directly in `editor_info` rather than as a `Document`/`Node` method, since it's a single small recursive helper with no other caller today.
- This does not touch `on_change` — `info()` is a separate, independently-callable getter, per the acceptance criteria.
- Out of scope for this spec: reorganizing `state.rs` into a `state/` module split by concern (e.g. separating command-mode/insert-mode key handling into their own files). That restructuring was discussed but deferred as a separate follow-up.
