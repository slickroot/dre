# 045 - Fully migrate kitty.py to Rust

## User Story

As a maintainer who wants to learn Rust, I want to finish moving `dre/kitty.py` to Rust — not just its leaf functions, but the `KittyGraphics` class itself — so that another whole module is retired from the Python codebase and I get practice exposing a stateful class across the PyO3 boundary.

## Technical Design

### Starting point

Spec 043 moved `kitty.py`'s four pure leaf functions (`_encode`, `_chunks`, `_more`, `_escape`) into `dre_rs` as public `#[pyfunction]`s, but `KittyGraphics.draw` and `_sprite` — the code that orchestrates `Sprite` objects and calls those functions — stayed in Python. This spec finishes the job: `KittyGraphics` itself moves to Rust, and `dre/kitty.py` is deleted.

### `Sprite` stays a Python dataclass

`Sprite` (`dre/render.py`) remains an ordinary `@dataclass` with plain fields (`pixels: bytes`, `width: int`, `height: int`, `col: int`, `row: int`). It does **not** become a Rust `#[pyclass]`.

This was a real fork: making `Sprite` a `#[pyclass]` was considered, but `render.py:_sprite` does `dataclasses.replace(drawn, col=left, row=top)` to reposition a cached sprite, which depends on `__dataclass_fields__` — something a `#[pyclass]` doesn't have. Rather than replace that call with a new Rust method or manual reconstruction, we keep `Sprite` as-is in Python and have Rust reach into it via attribute access instead.

### `KittyGraphics` becomes a Rust `#[pyclass]`

`dre_rs` gains a stateless `#[pyclass] struct KittyGraphics` with a single `#[pymethods]` method:

```rust
#[pymethods]
impl KittyGraphics {
    #[new]
    fn new() -> Self {
        KittyGraphics
    }

    fn draw(&self, sprites: Vec<Bound<'_, PyAny>>) -> PyResult<String> {
        let mut out = DELETE_ALL.to_string();
        for sprite in sprites {
            let row: i64 = sprite.getattr("row")?.extract()?;
            let col: i64 = sprite.getattr("col")?.extract()?;
            let width: i64 = sprite.getattr("width")?.extract()?;
            let height: i64 = sprite.getattr("height")?.extract()?;
            let pixels: Vec<u8> = sprite.getattr("pixels")?.extract()?;
            out.push_str(&format!("\x1b[{};{}H", row + 1, col + 1));
            out.push_str(&transmission(&pixels, width, height)?);
        }
        Ok(out)
    }
}
```

`draw` takes a `Vec<Bound<'_, PyAny>>` — any Python object with `.pixels`/`.width`/`.height`/`.col`/`.row` attributes, extracted via `getattr` — rather than requiring a specific Python type. This mirrors what `KittyGraphics._sprite` used to do by directly accessing `sprite.pixels` etc.

`transmission` (and the old `_encode`/`_chunks`/`_more`/`_escape` logic) become **plain internal Rust functions** — no `#[pyfunction]`, not exposed to Python at all. Nothing in Python calls them directly anymore now that `draw` is the sole Python-facing entry point, so keeping them as public PyO3 functions would be dead API surface.

### `CHUNK_SIZE` and `DELETE_ALL` move into Rust

Both constants move from `kitty.py` into `dre_rs` as Rust constants, registered as module attributes in `#[pymodule]`:

```rust
const CHUNK_SIZE: usize = 4096;
const DELETE_ALL: &str = "\x1b_Ga=d,d=A,q=2;\x1b\\";

#[pymodule]
fn dre_rs(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<KittyGraphics>()?;
    m.add("CHUNK_SIZE", CHUNK_SIZE)?;
    m.add("DELETE_ALL", DELETE_ALL)?;
    Ok(())
}
```

They're internal to the Rust implementation now — nothing in Python needs to pass `CHUNK_SIZE` in as an argument anymore, since `draw` uses the Rust constant directly. They're exposed as module attributes solely so `tests/test_kitty.py` can keep asserting against them.

### `dre/kitty.py` is deleted

The file is removed entirely — no wrapper class, no re-exports. Callers import `KittyGraphics` directly from `dre_rs`.

### `dre/writer.py` update

```python
from dre_rs import KittyGraphics
...
renderer = TerminalRenderer(KittyGraphics(), *cell_size())
```

Only the import line changes (`from .kitty import KittyGraphics` → `from dre_rs import KittyGraphics`); the construction and call site are unchanged.

### `render.py` is unaffected

`TerminalRenderer` calls `self.graphics.draw(sprites)` against `GraphicsProtocol`, a structural `Protocol`. A `KittyGraphics` instance from `dre_rs` satisfies it the same way the old Python class did — no changes needed to `render.py`, `GraphicsProtocol`, or `Sprite`.

### Test strategy: split by which side of the FFI boundary the code is on

- **Pure Rust internals** (`transmission` and the old encode/chunks/more/escape logic) get native `#[test]` functions in `dre_rs/src/lib.rs`, exercised directly against primitive Rust types (`&[u8]`, `String`, `Vec<String>`, `usize`) with no Python interpreter involved.
- **`KittyGraphics::draw`**, the one function that crosses the PyO3 boundary (accepting Python objects via `getattr`, returning a Python `str`), keeps its test coverage in Python: `tests/test_kitty.py` is unchanged except for its import line, which becomes `from dre_rs import CHUNK_SIZE, DELETE_ALL, KittyGraphics`. It continues to exercise `draw` only through its public behavior, exactly as before.

This supersedes spec 043's "no Rust-native tests in this milestone" note — now that a chunk of logic is purely internal to Rust with no Python-facing surface, it's tested where it lives.

## Acceptance Criteria

- `dre/kitty.py` is deleted.
- `dre_rs` exposes a `#[pyclass] KittyGraphics` with a `draw(sprites) -> str` method that produces identical output to the old Python `KittyGraphics.draw` (delete-all sequence, cursor positioning, header, chunking, `m=` flags).
- `dre_rs.encode`/`chunks`/`more`/`escape` are no longer exposed as `#[pyfunction]`s; their logic exists only as internal Rust functions called by `KittyGraphics::draw`.
- `dre_rs` exposes `CHUNK_SIZE` and `DELETE_ALL` as module-level attributes.
- `Sprite` (`dre/render.py`) is unchanged — still a plain `@dataclass`, still repositioned via `dataclasses.replace`.
- `dre/writer.py` imports `KittyGraphics` from `dre_rs` instead of `.kitty`.
- `tests/test_kitty.py` passes with only its import line changed (`from dre_rs import CHUNK_SIZE, DELETE_ALL, KittyGraphics`); no test bodies change.
- New native `#[test]` functions exist in `dre_rs/src/lib.rs` covering the internal encode/chunk/more/escape logic directly in Rust.
