# flex/mod.rs splits into a kitty module

## Problem

`src/flex/mod.rs` is one flat file mixing two unrelated concerns: the
Kitty graphics protocol (capability check, image transmission,
placement, chunking/encoding) and the app's own startup/loop
orchestration. `require` alone hand-rolls a termios save/raw/restore
sequence with a closure just to guarantee cleanup on every path,
duplicating a pattern `tty::RawMode` already solves with `Drop`, and
repeats the same write-then-flush pairing twice for no reason.

This is a refactoring spec: no behaviour the user can see changes.

## Target

`flex/mod.rs` keeps one file on disk (future layers are not split out
yet), but gains a real internal boundary: a nested `mod kitty` holding
everything that speaks the Kitty graphics protocol, with the
top-level `run`/`start` left as pure orchestration that calls into it.

## Technical Design

### `mod kitty` (nested inside `flex/mod.rs`)

Holds `require`, `is_supported`, `transmit`, `place`, `chunked`,
`encode`, `zlib`, `base64`, `chunks`, `more`, `escape`, and their
constants (`QUERY`, `CLEAR_LINE`, `NOT_SUPPORTED_MESSAGE`,
`REPLY_TIMEOUT_MICROS`, `CHUNK_SIZE`). Nothing in here knows about the
app loop, the background buffer, or `ExitCode`.

Two simplifications land inside this module:

1. **A local RAII raw-mode guard.** `require` currently saves
   termios, flips to raw mode by hand, and restores it after an
   IIFE whose only job is to guarantee the restore runs on every
   path. Replace this with a small guard type (enter = save + make
   raw, `Drop` = restore), the same pattern `tty::RawMode` already
   uses, but kept local to `mod kitty` rather than shared from
   `tty.rs` — flex still depends on nothing from `tty` but `probe`
   and `RawMode`, per spec 244. `require` becomes: write the query,
   read the reply inside the guard's scope, restore happens
   automatically via `Drop`, then interpret the reply.
2. **A `send` helper.** `write_all` followed immediately by `flush`
   happens twice (sending `QUERY`, sending `CLEAR_LINE`), only
   because `io::Stdout` is buffered and the terminal must actually
   receive the bytes before `require` waits for or depends on them.
   Replace both pairs with one `fn send<W: Write>(stream: &mut W,
   bytes: &[u8]) -> io::Result<()>`.

### Top level (`run`, `start`)

Unchanged in behaviour. Calls `kitty::require`, `kitty::transmit`,
`kitty::place` instead of free functions. `BACKGROUND_RGBA` stays
top-level since it belongs to the pixel buffer `start` builds, not to
the protocol.

### Migration

One slice, no behaviour change:

1. Add `mod kitty` inside `flex/mod.rs`, move the protocol functions
   and constants into it, update call sites in `start`.
2. Extract the local raw-mode guard inside `mod kitty`, rewrite
   `require` to use it.
3. Add `send`, rewrite the two write+flush pairs to use it.
4. Confirm `dre-flex` still starts, paints, and Ctrl-C exits cleanly.

## Acceptance Criteria

- `flex/mod.rs` is still the only file under `src/flex/`.
- `mod kitty` contains every function and constant that speaks the
  Kitty graphics protocol; `run`/`start` contain no protocol detail
  beyond calling into `mod kitty`.
- `require` has no manual termios save/restore bookkeeping — cleanup
  happens via a guard's `Drop`.
- No repeated write+flush pairs; both call through `send`.
- `flex` still imports nothing from `tty` beyond `probe` and
  `RawMode`.

## Out of scope

- Splitting `mod kitty` (or any other area) into its own file — this
  spec only draws the boundary inside the one file.
- Any change to `run`/`start`'s visible behaviour, timing, or the
  Ctrl-C exit path.
- Sharing the raw-mode guard with `tty::RawMode` — they stay separate
  implementations of the same pattern until something forces them
  together.
