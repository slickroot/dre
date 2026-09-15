# Migrate layout to Rust

## User Story

As a maintainer learning Rust, I want `dre/layout.py` fully replaced by `dre_rs`, so that the layout engine lives in Rust the same way the document model (`Box`/`State`/`handle_key`) and the Kitty graphics protocol already do, and `dre/layout.py` can be deleted.

## Acceptance Criteria

- `dre/layout.py` is deleted.
- `dre_rs` exposes `layout` and `with_cursor` as two separate `#[pyfunction]`s (not fused into one call), plus `Label`, `Arrow`, and `Cursor` as `#[pyclass]` types alongside the existing `Box`.
- `dre/render.py` and `dre/writer.py` import `layout`, `with_cursor`, `Label`, `Arrow`, `Cursor`, and `Placement` from `dre_rs` instead of `.layout`; `isinstance(placement.node, Box)` / `Label` / `Arrow` / `Cursor` checks in `render.py` are unchanged.
- `Celled`, `Positioned`, and `Track` are not part of `dre_rs`'s Python-visible surface. Neither are `fmap`, `flatten`, `assign`, `forest`, `walk`, `position`, `emit`, `column_tracks`, `tracks`, `span`, `interior`, `width`, `height`, `centre`, or `anchor` — they become private Rust functions, since nothing outside `layout.py` itself ever imported them.
- `Label.path` is `Vec<i64>`, matching `State.selected`'s existing `Vec<i64>` type. `writer.py`'s `tuple(state.selected)` conversion (added in spec 050 solely to satisfy a tuple/list equality mismatch) is removed — `frame()` calls `with_cursor(layout(state.boxes, cols, rows), state.selected)` directly, since the comparison against `Label.path` now happens entirely inside Rust.
- `Arrow.stops` stays a `Tuple[int, ...]` (a custom `PyTuple`-building getter, not the `get_all` default), because `render.py`'s `_key()` puts `(node.stops, node.shaft)` into a `dict` key and a `list` there would be unhashable and break the sprite cache.
- All other numeric fields introduced by this migration (`x`, `y`, `width`, `height`, `cols`, `rows`, `column`, `row`, `shaft`) are `i64`, matching the `i64` convention already used for `colour`/`fill`/`selected` in `dre_rs`.
- `tests/test_layout.py` is deleted. `dre_rs`'s own `#[test]` suite (using `Python::with_gil` for anything crossing the PyO3 boundary) is the only test coverage for `layout` and `with_cursor`'s behavior.
- The new code lives in `dre_rs/src/layout.rs`, declared via `mod layout;` in `lib.rs`, rather than being appended to the existing single-file `lib.rs`.

## Technical Design

### Scope

This migrates all of `dre/layout.py` — the `Celled`/`Positioned`/`Arrow`/`Label`/`Cursor`/`Placement`/`Track` data model and every function in the file (`tracks`, `span`, `interior`, `width`, `height`, `centre`, `fmap`, `flatten`, `anchor`, `assign`, `forest`, `walk`, `position`, `emit`, `with_cursor`, `column_tracks`, `layout`) — into `dre_rs`, and deletes `dre/layout.py` entirely. Unlike spec 050's migration of `state.py`, this module's output feeds directly into `render.py`, a Python module that stays put and relies on `isinstance` dispatch over `Placement.node`.

### Submodule: `dre_rs/src/layout.rs`

`lib.rs` has grown to 1274 lines across specs 043–050. This migration starts splitting it into submodules: layout's types and functions live in `dre_rs/src/layout.rs`, brought in via `mod layout;` and `use layout::*;` (or explicit re-exports) in `lib.rs`, so the `#[pymodule]` registration function in `lib.rs` can still call `m.add_class::<Label>()`, etc. Existing `state`/`kitty` code in `lib.rs` is left untouched by this migration — only the new layout code is organized into its own file.

### `Placement.node`'s polymorphism: a Rust enum wrapping existing pyclasses

`render.py` does `isinstance(placement.node, Box)` / `Label` / `Arrow` / `Cursor` today, and that must keep working unchanged. So `Label`, `Arrow`, and `Cursor` become `#[pyclass]` types (alongside the existing `Box`, i.e. `Node`), and `Placement.node` is represented internally as a Rust enum whose `IntoPy` conversion produces the right concrete pyclass instance:

```rust
enum PlacementNode {
    Node(Node),
    Label(Label),
    Arrow(Arrow),
    Cursor(Cursor),
}
```

The variant is named `Node`, not `Box`, avoiding the same `std::boxed::Box` keyword collision spec 050 already resolved by renaming the domain type. When PyO3 converts a `Placement` to Python, each variant hands back the pyclass instance it wraps, so `isinstance(placement.node, Box)` in `render.py` still passes exactly as before — `render.py` never sees `PlacementNode`, only the underlying pyclass values.

```rust
#[pyclass(get_all)]
#[derive(Clone)]
struct Label {
    text: String,
    path: Vec<i64>,
}

#[pyclass]
#[derive(Clone)]
struct Arrow {
    shaft: i64,
    // stops needs a custom getter — see below.
}

#[pyclass]
#[derive(Clone)]
struct Cursor;

#[pyclass(get_all)]
#[derive(Clone)]
struct Placement {
    node: PlacementNode,
    x: i64,
    y: i64,
    width: i64,
    height: i64,
}
```

### `Label.path` is `Vec<i64>`; the `tuple(state.selected)` shim disappears

Spec 050 left `writer.py` with:

```python
placements = with_cursor(layout(state.boxes, cols, rows), tuple(state.selected))
```

That `tuple(...)` conversion existed only because `layout.with_cursor`'s comparison `placement.node.path == selected` happened in Python, where `Label.path` was a tuple built by `layout.py` and `state.selected` was `dre_rs`'s `Vec<i64>`-backed list. Once `with_cursor` moves into Rust, the comparison happens entirely on the Rust side between two `Vec<i64>` values, so there's no cross-language type mismatch left to paper over. `Label.path` is `Vec<i64>`, and `writer.py`'s `frame()` goes back to:

```python
placements = with_cursor(layout(state.boxes, cols, rows), state.selected)
```

### `Arrow.stops` stays a tuple — the sprite cache needs it hashable

Every other collection introduced by spec 050 (`children`, `boxes`, `selected`) and by this migration (`Label.path`) is a `Vec<i64>` exposed as a Python `list` via `get_all`, for consistency. `Arrow.stops` is the one exception: `render.py`'s `_key()` does

```python
shape = (node.stops, node.shaft)
...
return (type(node), placement.width, ..., shape)
```

and uses the result as a key into `self.cache: Dict[Key, Sprite]`. A `list` inside that tuple would make it unhashable and raise `TypeError` the first time an `Arrow` placement got cached. So `Arrow.stops` keeps producing a Python `tuple`, via a custom getter building a `PyTuple` rather than relying on the `get_all` default:

```rust
#[pymethods]
impl Arrow {
    #[getter]
    fn stops(&self, py: Python<'_>) -> PyResult<Py<PyTuple>> {
        Ok(PyTuple::new(py, &self.stops)?.unbind())
    }
}
```

(`self.stops` itself is stored as `Vec<i64>` on the Rust struct; only the Python-visible getter differs from the `get_all` pattern.)

### `Celled`, `Positioned`, `Track`, and every helper function stay private

Mirroring spec 050 (`Command`, `at`, `rewrite`, `grow`, `colour_row`, `next_colour`, `enter_insert` stayed Rust-internal), none of `Celled`, `Positioned`, `Track`, `tracks`, `span`, `interior`, `width`, `height`, `centre`, `fmap`, `flatten`, `anchor`, `assign`, `forest`, `walk`, `position`, `emit`, or `column_tracks` are `#[pyclass]`/`#[pyfunction]`. `tests/test_layout.py` imports several of these directly today (`assign`, `centre`, `forest`, `tracks`, `span`, `width`, `height`, `walk`, `Track`), which is exactly why it's deleted rather than kept: those names are private implementation details of `dre_rs::layout`, not a contract `dre/` or its tests should depend on.

The only Python-visible surface this migration adds is:

```rust
#[pyfunction]
fn layout(boxes: Vec<Node>, cols: i64, rows: i64) -> Vec<Placement> { ... }

#[pyfunction]
fn with_cursor(placements: Vec<Placement>, selected: Vec<i64>) -> Vec<Placement> { ... }
```

plus the `Label`, `Arrow`, `Cursor`, and `Placement` pyclasses. `layout` and `with_cursor` stay separate calls (not fused into one function) because `writer.py`'s `frame()` chains them, but nothing about the contract requires them to always be called together — keeping them separate keeps `layout()` independently usable and testable, matching today's two-call API.

### Numeric types: `i64` throughout

All numeric fields this migration introduces (`Placement.x/y/width/height`, `Label.path`'s elements, `Arrow.shaft`, `layout`'s `cols`/`rows` parameters, and the internal `Celled`/`Positioned`/`Track` fields `column`/`row`/`offset`/`extent`) are `i64`, matching the `i64` convention `dre_rs` already uses for `Box.colour`/`Box.fill`/`State.selected`.

### Test strategy: same as spec 050

`tests/test_layout.py` is deleted rather than rewritten. `dre_rs`'s own `#[test]` suite covers this in the same two tiers spec 050 established:

- Plain `#[test]` functions for the private helpers (`tracks`, `span`, `centre`, `assign`, `forest`, `column_tracks`, etc.) that never cross the FFI boundary.
- `Python::with_gil` tests for `layout` and `with_cursor` exercised through their actual `#[pyfunction]` entry points, asserting on the resulting `Placement`/`Label`/`Arrow`/`Cursor` values the same way Python code would see them (including `isinstance`-equivalent pattern matching on `PlacementNode`).

`dre/render.py` and `dre/writer.py` import `layout`, `with_cursor`, `Label`, `Arrow`, `Cursor`, and `Placement` from `dre_rs` and trust them without any Python-level test of their own, the same way they already trust `Box`, `State`, and `handle_key`.
