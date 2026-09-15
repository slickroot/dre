# 048 - Abstract I/O plumbing in Rust

## User Story

As a maintainer learning Rust, I want `dre_rs/src/lib.rs` to hide infallible plumbing (buffer setup, `Write` calls, error propagation for operations that can't actually fail) the same way the original Python `kitty.py` hid it, so that the Rust code reads as an expression of intent rather than a manual walkthrough of its own mechanics.

## Technical Design

### Scope

This story establishes a general convention for `dre_rs/src/lib.rs`: hide plumbing for operations that are only fallible in a formal sense (their type signature says `Result`, but they can never actually return `Err` given how they're used here). It's not about `encode` alone — every function in the file is held to the same standard.

### Starting point: spec 045 is merged

Spec 045 landed (`origin/main` @ `7fb28d8`) and already turned `encode`/`chunks`/`more`/`escape` into private internal functions — no `#[pyfunction]`, called only via `transmission`, which is in turn called only by `KittyGraphics::draw`. That removes the PyO3-boundary constraint this spec was originally waiting on: there's no `#[pyfunction]` return-type coupling left to navigate. This spec proceeds against that merged state, not the pre-045 file.

What's left of the problem: `encode` still returns `std::io::Result<String>` (commit `d44403b` decoupled it from `PyResult` but kept a `Result`), and `chunks` still has a bare, message-less `.unwrap()`.

### `encode` returns `String`, not `std::io::Result<String>`

`write_all`/`finish` on `ZlibEncoder<Vec<u8>>` return `io::Result` only because `Write` is a generic trait — writing to an in-memory `Vec<u8>` can never actually fail. The current code propagates that formal fallibility outward via `?`, which is exactly the kind of I/O ceremony the Python version never had to show.

`encode` changes to:

```rust
fn encode(pixels: &[u8]) -> String {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(pixels).expect("writes to an in-memory Vec<u8> can't fail");
    let compressed = encoder.finish().expect("writes to an in-memory Vec<u8> can't fail");
    STANDARD.encode(compressed)
}
```

### `transmission` returns `String`, not `PyResult<String>`

`transmission`'s only fallible call was `encode(pixels)?`. Once `encode` is infallible, `chunks`/`more`/`escape` were never fallible to begin with, so `transmission` has nothing left to propagate:

```rust
fn transmission(pixels: &[u8], width: i64, height: i64) -> String {
    let payload = encode(pixels);
    let chunk_list = chunks(&payload, CHUNK_SIZE);
    let header = format!(
        "a=T,f=32,s={width},v={height},o=z,q=2,z=-1,m={}",
        more(&chunk_list, 0)
    );
    let mut escapes = vec![escape(&header, &chunk_list[0])];
    for (index, chunk) in chunk_list.iter().enumerate().skip(1) {
        let keys = format!("m={}", more(&chunk_list, index));
        escapes.push(escape(&keys, chunk));
    }
    escapes.join("")
}
```

`KittyGraphics::draw` stays `PyResult<String>` — `getattr`/`extract` on arbitrary Python objects are genuinely fallible — but its call site drops a `?`:

```rust
out.push_str(&transmission(&pixels, width, height));
```

### `.expect()` messages are the documentation, not a comment

The convention: when collapsing a formally-fallible call that can't actually fail, the string passed to `.expect(...)` must state the invariant that guarantees success (matching Rust API guidelines / clippy's `expect_used` convention). No separate comment line is added on top — the message *is* the comment.

This same treatment applies to `chunks`' existing bare `.unwrap()`:

```rust
.map(|chunk| std::str::from_utf8(chunk).expect("base64 payload is single-byte ASCII, so byte chunks are always valid UTF-8").to_owned())
```

## Acceptance Criteria

- `encode` returns `String` (not `std::io::Result<String>`); its two internal `.expect(...)` calls each state the invariant that guarantees the write can't fail.
- `chunks`' `std::str::from_utf8(chunk).unwrap()` becomes `.expect("base64 payload is single-byte ASCII, so byte chunks are always valid UTF-8")`.
- `transmission` returns `String` (not `PyResult<String>`); it no longer uses `?`.
- `KittyGraphics::draw` keeps `PyResult<String>` (its `getattr`/`extract` calls remain genuinely fallible) but no longer uses `?` on its call to `transmission`.
- No behavior change: `dre_rs`'s native `#[test]` suite (`chunks_splits_at_boundaries` and friends in `dre_rs/src/lib.rs`) and `tests/test_kitty.py` pass unmodified.
- No other function in `dre_rs/src/lib.rs` (`more`, `escape`, the `#[pymethods]`/`#[pymodule]` blocks) changes — they either have no fallibility to hide or their `Result` is genuine (module/class registration).
