# 043 - Migrate from Python to Rust

## User Story

As a maintainer who wants to learn Rust, I want to start moving this codebase from Python to Rust one deliberate step at a time, so that I build real Rust fluency while the tool keeps working throughout the migration.

## Acceptance Criteria

- `dre/kitty.py` no longer defines `_encode`, `_chunks`, `_more`, or `_escape`; `KittyGraphics` calls a new `dre_rs` Rust module instead.
- A new Rust crate at `dre_rs/` (built with PyO3 + maturin) exposes `encode`, `chunks`, `more`, and `escape` as plain functions over primitive types only (`bytes`, `str`, `int`, `list[str]`) — it does not know about `Sprite` or any other Python dataclass.
- `CHUNK_SIZE` stays defined in `kitty.py` and is passed as an argument into `dre_rs.chunks(...)`.
- The existing `tests/test_kitty.py` suite passes unmodified against the Rust-backed implementation (it only exercises behavior through `KittyGraphics.draw`, never the removed private functions directly).
- No Rust-native (`#[test]`) tests are added in this first milestone — verification relies solely on the existing Python test suite.

## Technical Design

### Overall migration strategy

This is the first step of an incremental migration: the Python app (`dre/`) stays the thing that runs, and internal pieces get replaced one at a time by Rust functions called from Python via **PyO3** (a Rust crate that lets Rust code be compiled into a native Python extension module). Each step should be small enough to be a real, understandable Rust learning exercise on its own — this spec covers exactly one such step.

### Why `kitty.py` first

`dre/kitty.py` formats sprite pixel data into escape sequences for the [Kitty terminal graphics protocol](https://sw.kovidgoyal.net/kitty/graphics-protocol/), which lets a supporting terminal draw images inline instead of just text:

- **Why base64?** An escape sequence is transmitted as text, delimited by control bytes like `\x1b_G...\x1b\\`. Raw pixel bytes can contain any byte value, which could collide with the terminal's parser. Base64 re-encodes arbitrary binary data into a safe, restricted ASCII alphabet so it can be embedded in a text-based escape sequence.
- **Why zlib compress first?** Raw pixel data (`width * height * 4` bytes) can be large, and base64 itself inflates size by ~33%. Compressing first shrinks what has to be written to the terminal, which may be over a slow connection (e.g. SSH). The header's `o=z` flag tells the terminal "decompress this after base64-decoding it."
- **Why chunk?** The protocol caps how much payload a single escape sequence can carry, so the base64 string is split into 4096-byte pieces, each sent as its own escape sequence carrying an `m=` flag (`m=1` = more chunks coming, `m=0` = last chunk).

`kitty.py` is a good first Rust target because its leaf functions are pure (no I/O, no shared state) and operate only on primitive types (`bytes`, `str`, `int`, `list[str]`) — the simplest possible surface for a first PyO3 exercise. `state.py` was considered but rejected for this first step: it uses recursive, self-referential dataclasses (`Box.children: Tuple["Box", ...]`, `State.before: Optional["State"]`), which would require exposing custom types to Rust (`#[pyclass]`) — a bigger leap than a first exercise should be.

### Scope: which functions move, which stay

Only the four pure leaf functions move to Rust:

- `_encode(pixels: bytes) -> str`
- `_chunks(payload: str) -> List[str]`
- `_more(chunks: List[str], index: int) -> int`
- `_escape(keys: str, payload: str) -> str`

`KittyGraphics.draw` and `_sprite` **stay in Python** — they orchestrate `Sprite` objects (a Python dataclass), and teaching PyO3 about a custom Python type is out of scope for this first step. Instead of passing a `Sprite` across the FFI boundary, Python keeps pulling primitive fields (`sprite.pixels`, `sprite.width`, ...) out of it before calling into Rust.

### Crate layout and tooling

A new Cargo crate lives at the repo root: `dre_rs/`. It's built with [maturin](https://www.maturin.rs/), the standard tool for building a PyO3 extension and making it importable from Python (`maturin develop` builds the crate and installs it into the active virtualenv as the `dre_rs` module).

`dre_rs/Cargo.toml`:

```toml
[package]
name = "dre_rs"
version = "0.1.0"
edition = "2021"

[lib]
name = "dre_rs"
crate-type = ["cdylib"]

[dependencies]
pyo3 = { version = "0.22", features = ["extension-module"] }
base64 = "0.22"
flate2 = "1"
```

`dre_rs/src/lib.rs`:

```rust
use pyo3::prelude::*;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use flate2::write::ZlibEncoder;
use flate2::Compression;
use std::io::Write;

#[pyfunction]
fn encode(pixels: &[u8]) -> PyResult<String> {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(pixels)?;
    let compressed = encoder.finish()?;
    Ok(STANDARD.encode(compressed))
}

#[pyfunction]
fn chunks(payload: &str, chunk_size: usize) -> Vec<String> {
    // The base64 alphabet is single-byte ASCII, so splitting on byte
    // offsets never lands in the middle of a multi-byte character.
    payload
        .as_bytes()
        .chunks(chunk_size)
        .map(|chunk| std::str::from_utf8(chunk).unwrap().to_owned())
        .collect()
}

#[pyfunction]
fn more(chunks: Vec<String>, index: usize) -> i32 {
    if index == chunks.len() - 1 { 0 } else { 1 }
}

#[pyfunction]
fn escape(keys: &str, payload: &str) -> String {
    format!("\x1b_G{keys};{payload}\x1b\\")
}

#[pymodule]
fn dre_rs(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(encode, m)?)?;
    m.add_function(wrap_pyfunction!(chunks, m)?)?;
    m.add_function(wrap_pyfunction!(more, m)?)?;
    m.add_function(wrap_pyfunction!(escape, m)?)?;
    Ok(())
}
```

A couple of Rust-specific things worth noting since this is a first exercise:

- `&[u8]` is "a borrowed slice of `u8`" — Rust's name for an unsigned 8-bit integer (0-255), which is exactly what a byte of pixel data is. PyO3 automatically converts Python `bytes` to `&[u8]` and back.
- `PyResult<String>` is used for `encode` because writing to the zlib encoder can technically fail; PyO3 turns an `Err` into a raised Python exception automatically. `chunks`, `more`, and `escape` can't fail, so they return plain values with no `Result` wrapper.

### `kitty.py` after the change

```python
import dre_rs

CHUNK_SIZE = 4096
DELETE_ALL = "\x1b_Ga=d,d=A,q=2;\x1b\\"


class KittyGraphics:
    def draw(self, sprites: List[Sprite]) -> str:
        return DELETE_ALL + "".join(self._sprite(sprite) for sprite in sprites)

    def _sprite(self, sprite: Sprite) -> str:
        return f"\x1b[{sprite.row + 1};{sprite.col + 1}H" + self._transmission(
            sprite
        )

    def _transmission(self, sprite: Sprite) -> str:
        chunks = dre_rs.chunks(dre_rs.encode(sprite.pixels), CHUNK_SIZE)
        header = (
            f"a=T,f=32,s={sprite.width},v={sprite.height},o=z,q=2,z=-1,"
            f"m={dre_rs.more(chunks, 0)}"
        )
        escapes = [dre_rs.escape(header, chunks[0])]
        for index, chunk in enumerate(chunks[1:], start=1):
            escapes.append(dre_rs.escape(f"m={dre_rs.more(chunks, index)}", chunk))
        return "".join(escapes)
```

`_encode`, `_chunks`, `_more`, and `_escape` are deleted entirely — nothing calls them anymore. `CHUNK_SIZE` stays in Python and is passed explicitly into `dre_rs.chunks`, rather than being duplicated or hardcoded on the Rust side.

### Why no test changes are needed

`tests/test_kitty.py` only calls `KittyGraphics.draw(...)` — it never imports or calls `_encode`/`_chunks`/`_more`/`_escape` directly. Since `draw`'s behavior is unchanged (same escape sequences produced, same chunking), the existing test suite verifies the Rust-backed implementation with no modifications required.

### Development workflow

Building this crate requires a Rust toolchain (`rustup`) and `maturin` installed. After `dre_rs/src/lib.rs` changes, run `maturin develop` from within `dre_rs/` (with the project's virtualenv active) to rebuild the extension and make the updated `dre_rs` module importable again before running the Python test suite.
