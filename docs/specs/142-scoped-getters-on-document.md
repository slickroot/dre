# Scoped getters on Document

This is a technical spec, not a user story: it has no user-facing behaviour.

## Problem

`Document` has `set_colour`, `set_fill` and `set_rounded`, each taking a `Scope` (`Box` or `Siblings`) so a write can target one node or every child of its parent at once. There is no getter with the same shape, so every caller that needs to *read* a field across siblings reaches past `Document` into the raw tree with the free functions `children` and `parent_of`:

- `state/command.rs::next_row_colour` (line 101) — reads every sibling's colour to decide the next palette colour.
- `state/command.rs::toggle_siblings_fill` (line 119) — reads every sibling's colour and fill to decide whether to fill or unfill the row.
- `state/command.rs::toggle_siblings_rounded` (line 170) — reads every sibling's `rounded` to decide whether to round or square the row.

Each site duplicates the same `children(tree, parent_of(path))` lookup that `Document::set`'s `Scope::Siblings` arm already encapsulates for writes, then does its own aggregation (`all`, `any`, first-and-compare) over the raw `Node` values.

`state/command.rs::new_sibling` and `::delete_box` also call `parent_of`/`children` directly, but not to read a scoped field — the former just needs a parent path for `add_child_box`, the latter counts remaining children of an already-known parent after a removal. Those are unaffected by this spec.

## Acceptance Criteria

## Technical Design

- `Document` gains `get_colour`, `get_fill` and `get_rounded`, one per existing setter, each with the signature `(&self, path: &[usize], scope: Scope) -> Vec<T>` (`T` is the field's type: `Option<u8>`, `bool`, `bool`). `Scope::Box` returns a single-element `Vec` (the value at `path`); `Scope::Siblings` returns one element per child of `parent_of(path)`, including the node at `path` itself — the same node set `Scope::Siblings` already writes to.
- The getters hand back the raw per-node values only. They do no aggregation — callers keep doing their own `.all()`/`.any()`/first-and-compare, exactly as they do today, just over `Vec<T>` instead of re-deriving the sibling paths first.
- The getters' scope-resolution logic is not shared with `set`'s private `targets` computation — `get` and `set` each resolve `Box`/`Siblings` independently.
- `state/command.rs` updates its three read sites to call `state.doc.get_colour(&path, Scope::Siblings)`, `get_fill`, and `get_rounded` instead of `children(tree, parent_of(path))` + `tree.value(...)`. `new_sibling` and `delete_box` are left as they are.
- `diagram.rs` gets one focused test per getter (box scope returns one value, siblings scope returns every sibling's value in order, including the target node), rather than growing the file's existing test duplication further.
