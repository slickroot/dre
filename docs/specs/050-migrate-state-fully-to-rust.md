# Migrate state.py fully to Rust

## User Story

As a maintainer learning Rust, I want `dre/state.py` fully replaced by `dre_rs`, so that the document model and its command logic live in Rust the same way the Kitty graphics protocol already does, and `dre/state.py` can be deleted.

## Acceptance Criteria

- `dre/state.py` is deleted.
- `dre_rs` exposes `Box`, `State`, and `handle_key` to Python; `dre/layout.py`, `dre/writer.py`, and `tests/test_state.py` import all three from `dre_rs` instead of `dre.state`.
- `Command`, `at`, `rewrite`, `grow`, `colour_row`, `next_colour`, and `enter_insert` are not part of `dre_rs`'s Python-visible surface — they become private Rust functions/enum, since nothing outside `state.py` itself ever imported them.
- `tests/test_state.py` is deleted. `dre_rs`'s own `#[test]` suite (some using `Python::with_gil` to exercise the PyO3 boundary directly) is the only test coverage for `Box`, `State`, and `handle_key`'s behavior, including undo, insert-mode editing, and command dispatch.
- `Box.children`, `State.boxes`, and `State.selected` are exposed as Python lists (PyO3's default `Vec<T>` conversion), not tuples.
- `dre/writer.py`'s call `with_cursor(layout(state.boxes, cols, rows), state.selected)` becomes `with_cursor(layout(state.boxes, cols, rows), tuple(state.selected))` — the one place a list would otherwise fail an equality check against a tuple built in `layout.py`.
- No other line in `dre/layout.py` changes: every other use of `.children` there (iteration, `enumerate`, truthiness checks) is unaffected by lists replacing tuples.
- `State.before` (used only by the `UNDO` command) is a private Rust-native field (`before: Option<Box<State>>`, using Rust's `Box` smart pointer for indirection) with no PyO3 getter — Python code never reads or writes it.

## Technical Design

### Scope

This migrates all of `dre/state.py` — the `Box`/`State` data model, the `Command` enum, and every function in the file (`at`, `rewrite`, `grow`, `colour_row`, `next_colour`, `enter_insert`, `handle_command`, `handle_insert`, `handle_key`) — into `dre_rs`, and deletes `dre/state.py` entirely. This is a bigger step than the `kitty.py` migration (specs 043/045/048): `Box.children: Tuple["Box", ...]` and `State.before: Optional["State"]` are self-referential, so this is the first time `dre_rs` needs `#[pyclass]` types instead of only functions over primitives.

### Naming: `Box` collides with Rust's `Box<T>`

The domain type is renamed `Node` in Rust and exposed to Python under its existing name via `#[pyclass(name = "Box")]`, so `dre_rs.Box` is unchanged from the caller's perspective. This avoids writing `std::boxed::Box` fully-qualified everywhere a real smart pointer is needed (notably `State.before`).

```rust
#[pyclass(name = "Box", get_all)]
#[derive(Clone, PartialEq)]
struct Node {
    label: String,
    colour: i64,
    fill: i64,
    rounded: bool,
    children: Vec<Node>,
}
```

`children: Vec<Node>` needs no smart-pointer indirection — a `Vec` already heap-allocates its elements, so a plain recursive `Vec<Node>` field compiles and requires no `Py<Node>`/reference-counting scheme. Equality (`Box(children=(Box("a"),)) != Box(children=(Box("b"),))` today) is structural, so `#[derive(PartialEq)]` plus `#[pyclass(eq)]` covers it directly since `Node: PartialEq` implies deep, recursive comparison through `Vec<Node>`'s own `PartialEq`.

### `State` and undo

```rust
#[pyclass(get_all)]
#[derive(Clone)]
struct State {
    boxes: Vec<Node>,
    running: bool,
    mode: String,
    selected: Vec<i64>,
    before: Option<Box<State>>,
}
```

`before` has no `#[pyo3(get)]` — nothing outside the old `handle_command`'s `UNDO` branch ever read `state.before` from Python, so it stays a Rust-internal implementation detail of the undo mechanism, not part of the promised contract. `Option<Box<State>>` uses Rust's real smart pointer here (no naming collision, since the domain type is `Node`).

### Lists, not tuples, at the Python boundary

`children`, `boxes`, and `selected` are exposed via `get_all`'s default `Vec<T>` → Python `list` conversion — no custom `PyTuple`-building getters. This changes the observable type from the old dataclass-based tuples, but only one line outside the deleted `state.py`/`test_state.py` actually depends on tuple identity for equality: `dre/writer.py`'s

```python
placements = with_cursor(layout(state.boxes, cols, rows), state.selected)
```

`layout.with_cursor` compares `placement.node.path == selected`, and `placement.node.path` is a tuple built inside `layout.py` (`path + (index,)`); a list would never compare equal to it. That line becomes:

```python
placements = with_cursor(layout(state.boxes, cols, rows), tuple(state.selected))
```

Every other consumer of `.children` (all in `layout.py`: `fmap`, `flatten`, `assign`, `walk`, `column_tracks`) only iterates or checks truthiness, so it's unaffected by the list/tuple change.

### `Command` and the pure helper functions stay Rust-internal

`Command`, `at`, `rewrite`, `grow`, `colour_row`, `next_colour`, and `enter_insert` are not exposed via PyO3. A grep of `dre/` and `tests/` confirms `Command` is referenced only inside `state.py`/`test_state.py` today, and the five helper functions the same — nothing is made public just so it can be unit-tested from Python. `Command` becomes a plain Rust enum (e.g. `TryFrom<char>` for parsing a key press), and the helpers become private functions called only by `handle_command`/`handle_insert`, mirroring how `kitty.py`'s `_encode`/`_chunks`/`_more`/`_escape` became private internals of `transmission` in spec 045.

The only two things `dre_rs` exposes for this module are `Box` (`Node`) as a `#[pyclass]`, `State` as a `#[pyclass]`, and `handle_key` as a `#[pyfunction]`:

```rust
#[pyfunction]
fn handle_key(state: &State, key: &str) -> State { ... }
```

### Test strategy: the contract is Rust's to keep

`tests/test_state.py` is deleted rather than rewritten. The reasoning: `dre_rs` is a library with a promised contract, and that contract — including the PyO3 binding shape itself (argument marshalling, `#[pyclass(eq)]` giving real Python `==`, `handle_key`'s exact signature) — should be certified by `dre_rs`'s own test suite, not re-verified from the Python side. If a binding bug slipped through, that's a `dre_rs` bug to catch in `dre_rs`'s tests, not a gap for `dre/` to patch over.

Rust's `#[test]` suite covers this in two tiers:

- Plain `#[test]` functions for the private helpers (`at`, `rewrite`, `grow`, `colour_row`, `next_colour`) that never cross the FFI boundary — ordinary Rust unit tests, no Python interpreter involved.
- `#[test]` functions using `Python::with_gil` for everything that does cross the boundary: constructing `Box`/`State` as real Python objects, calling `handle_key` through the same `#[pyfunction]` entry point Python calls, and asserting on the resulting `State`'s Python-visible attributes (`.boxes`, `.mode`, `.selected`, equality). This is what replaces `test_state.py`'s command-dispatch and undo tests (`test_b_on_an_empty_canvas_...`, `test_a_second_b_on_the_same_parent_...`, etc.) — same behavior under test, but exercised and owned entirely inside `dre_rs`.

`dre/layout.py` and `dre/writer.py` import `Box`, `State`, and `handle_key` from `dre_rs` and trust them without any Python-level test of their own, the same way they'd trust any other well-tested external library dependency.
