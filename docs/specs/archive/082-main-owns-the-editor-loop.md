# 082: Main owns the editor loop

Refactoring spec — no user story. Behaviour is unchanged, with one deliberate
exception noted under *Quitting*, where the bytes out are identical but the rule
producing them stops being accidental.

## Goal

`src/writer.rs` is not a component. It is five unrelated responsibilities that
happened to be on the path between `main` and the screen:

1. **`RawModeGuard`** — raw mode, the alternate screen, and the hidden cursor.
2. **`run`** — the loop: frame, read a key, reduce, save on exit.
3. **`frame`** — render, then overlay the save prompt.
4. **`load_state`** — a path becomes a `State`.
5. **`write`** — the Kitty capability probe and the failure exit.

Nothing binds them together except the call chain. The name says so: `writer`
neither writes files nor writes to the terminal — `filesystem` and the renderer
do that — and `writer::write` is the editor's entry point, not a write.

The cost is that `main` cannot see the program. `main` dispatches into
`writer::write`, and the loop that is the whole editor — render a frame, read a
key, reduce — is buried three functions deep behind a save prompt, a termios
guard and a file loader. After this spec that loop is in `main.rs`, in full
view, and each of the other four responsibilities sits with the thing it is
actually about.

`src/writer.rs` is deleted.

## Technical Design

### Principle

`main` owns the loop and nothing else. Every collaborator it calls answers one
question at one layer, so the loop body reads as the program: render this
document, place this status line, take a key, reduce.

Two things follow. Terminal *state* — which is a lifetime, not a step — is a
single RAII type, so every exit path restores it. And reading a key is not a
terminal concern at all: raw mode is one-time setup, after which a key is a byte
on stdin and nothing more.

### Reading a key

The current four steps buy nothing:

```rust
let stdin = unsafe { BorrowedFd::borrow_raw(stdin_fd) };
read(stdin, &mut key_buffer).map_err(io::Error::from)?;
let key = std::str::from_utf8(&key_buffer).unwrap_or("").to_string();
```

The `unsafe` and the `BorrowedFd` exist only because the code reached for
`nix::unistd::read`, which takes one — a borrow it already had for `tcsetattr`.
`from_utf8(..).unwrap_or("")` silently discards any byte ≥ `0x80`. `.to_string()`
heap-allocates once per keystroke.

In `std` it is:

```rust
let mut key = [0u8; 1];
io::stdin().read_exact(&mut key)?;
```

No `unsafe`, no `nix`, no allocation. `Stdin` implements `Read`, and its internal
buffer is not a problem: in raw mode the underlying `read` returns as soon as at
least one byte is available rather than waiting to fill, so a keystroke arrives
immediately and a pasted burst is buffered instead of costing one syscall per
byte.

One byte stays adequate because the entire key vocabulary is single-byte ASCII
plus a lone `\x1b` for Esc — `command_mode::parse`, `insert_mode::parse` and
`save_prompt_mode::parse` match nothing else. Multi-byte input (arrow keys,
non-ASCII labels) is no more broken than it is today and is not this spec's
business.

### `src/terminal.rs` — `RawScreen`

`RawModeGuard` moves here, beside the `Terminal` this module already probes, and
is renamed for what it guards. It keeps all three responsibilities — termios, the
alternate screen, the cursor — because all three must be undone and one
destructor is how you guarantee that:

```rust
pub(crate) struct RawScreen { fd: RawFd, saved: Termios }

impl RawScreen {
    pub(crate) fn open(fd: RawFd) -> io::Result<Self>
}
```

`open` saves termios, writes `ENTER_ALTERNATE_SCREEN` and `HIDE_CURSOR`, then
`cfmakeraw` + `tcsetattr`. `Drop` restores termios, then `SHOW_CURSOR`, then
`LEAVE_ALTERNATE_SCREEN` — exactly the reverse, errors swallowed with `let _`
as they must be, since `drop` cannot report.

Two changes from today:

- **The `stream: &'a mut W` field is gone.** The guard needs stdout only at
  construction and at destruction, but the borrow checker cannot know that, so
  the field froze the caller's stream for the guard's whole lifetime — which is
  why the loop has to write through `&mut *guard.stream`. It calls
  `io::stdout()` itself at both ends instead, owns no reference, and the
  reborrow disappears. The type loses its lifetime and its `W` parameter.
- **Key reading leaves it.** The guard establishes the conditions under which a
  read means anything; it does not perform the read.

`main` must hold a plain `Stdout`, never a `StdoutLock`. `io::stdout()` takes
the lock per call, so a lock held across `drop` would deadlock at exit. Today's
code already uses an unlocked `Stdout`; this is a constraint to preserve.

The binding in `edit` is load-bearing: `let _screen = RawScreen::open(fd)?;`.
`let _ = ` would drop it on the spot, turning raw mode on and immediately off.

### `src/kitty.rs` — the probe becomes fallible

`supported` is self-contained: it saves termios, queries, reads the reply under
its own temporary raw mode, and restores before returning. That is what makes it
safe to call from `main` ahead of `RawScreen`. Only its signature changes:

```rust
pub(crate) fn require<W: Write>(stream: &mut W, stdin_fd: RawFd) -> io::Result<()>
```

Unsupported → write `CLEAR_LINE` to `stream`, flush, and return
`io::Error::new(ErrorKind::Unsupported, NOT_SUPPORTED_MESSAGE)`.

`main` grows no branch for it: the existing `Err` arm already prints and returns
`FAILURE`. The message moves from stdout to stderr, which is where a diagnostic
accompanying a non-zero exit belongs. `CLEAR_LINE` stays in `kitty`, next to the
query whose echoed garbage it wipes.

The probe sits on the `Edit` arm, **not** before dispatch. `cli::export` touches
no terminal, and `dre --svg x.dre` works today when redirected, piped, or run in
CI with no tty; probing first would cost it a 500ms timeout and then fail it.

`is_supported(&reply)` is untouched, so its three tests are untouched.

### `src/render/terminal.rs` — the status line

Spec 076 settled that *the renderer never learns about modes; the save prompt is
the editor's overlay*. That holds. But `prompt_line` is geometry, not policy — it
pads or truncates to `cols` and is positioned at `rows` — which is why `frame`
takes a `Terminal` parameter it uses for nothing else, while `TerminalRenderer`
already owns `self.terminal`.

So the renderer places the line and `main` decides its text:

```rust
pub(crate) fn status_line(&mut self, text: Option<&str>, out: &mut impl Write) -> io::Result<()>
```

`Some(text)` → move to the last row (`\x1b[{rows};1H`) and write it padded or cut
to `cols`. `None` → `Ok(())` immediately.

It is called unconditionally, every frame, so the loop body carries no branch.
No erase is needed on the `None` path: `render` repaints the full grid including
the last row, so leaving `SavePrompt` clears the prompt on the next frame, as it
does today.

The renderer is handed a line of text. It is not told why.

### `src/filesystem.rs` — a new module, path ↔ text

The only module besides `main` that touches the filesystem. A leaf over
`std::fs`, dealing in text:

```rust
pub(crate) fn read(path: &str) -> io::Result<String>
pub(crate) fn write(path: &str, contents: &str) -> io::Result<()>
pub(crate) fn missing(path: &str) -> io::Error   // "no such file: {path}"
pub(crate) fn invalid(path: &str) -> io::Error   // "{path}: not a valid diagram"
```

`read` propagates `NotFound` untouched, because the two callers disagree about
what it means: editing a missing path starts a new file, exporting one is an
error. Policy stays with the caller; the module owns only the wording, which
both callers share and neither should spell out.

It is **not** `file_document.rs`, and must not be named anything near it.
`file_document` is a pure `FileDoc` ↔ `Document` mapping with no `io` at all.
The two sit at different stages of one chain:

```
path ←─ filesystem ─→ text ←─ dre_format ─→ FileDoc ←─ file_document ─→ Document
```

### `src/state.rs` — construction, not parsing

`state.rs` is the `State` and its reducer. It stays pure: it imports `diagram`
and the three mode modules, and gains no knowledge that a file format exists.

`load_state` splits along that line. Parsing was never state's business; it was
bundled in by accident of the old call site. What *is* state's business is the
construction policy, and that moves here as two constructors taking a parsed
`Document`:

```rust
pub(crate) fn load(doc: Document, save_to: Option<String>) -> State
pub(crate) fn new_file(path: String) -> State
```

`load` sets `doc`, `save_to`, and `selected` to the first box when there is one.
`new_file` is an empty `State` with `save_to` set and `new_file` true. Both are
unit-testable from a `Document` literal, with no temp files.

### `src/main.rs` — the loop

`main` keeps its two-arm match and its `eprintln!`. The `Edit` arm calls a local
`edit`, which exists because the body needs `?` over `io::Result` while `main`
returns `ExitCode`:

```rust
fn edit(file: Option<String>) -> io::Result<ExitCode> {
    let mut state = load(file)?;
    let mut stdout = io::stdout();
    kitty::require(&mut stdout, io::stdin().as_raw_fd())?;
    let mut renderer = TerminalRenderer::new(terminal::probe()?);
    let _screen = RawScreen::open(io::stdin().as_raw_fd())?;

    while state.running {
        let status = status(&state);
        renderer.render(&state.doc, &mut stdout)?;
        renderer.status_line(status.as_deref(), &mut stdout)?;
        stdout.flush()?;

        let mut key = [0u8; 1];
        io::stdin().read_exact(&mut key)?;
        let key = (key[0] as char).to_string();
        if key == INTERRUPT {
            state.save_to = None;
            break;
        }
        state = handle_key(state, &key);
    }

    if let Some(path) = &state.save_to {
        filesystem::write(path, &dre_format::write(&file_document::from_document(&state.doc)))?;
    }
    Ok(ExitCode::SUCCESS)
}
```

Loading keeps today's order — the file is read and parsed before the probe, so
an invalid file fails without paying the 500ms timeout:

```rust
fn load(file: Option<String>) -> io::Result<State> {
    let Some(path) = file else { return Ok(State::default()) };
    let text = match filesystem::read(&path) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(state::new_file(path)),
        result => result?,
    };
    let doc = dre_format::read(&text)
        .map(file_document::to_document)
        .ok_or_else(|| filesystem::invalid(&path))?;
    Ok(state::load(doc, Some(path)))
}
```

`status` is the mode-to-text policy, and the only place that knows the prompt's
wording:

```rust
fn status(state: &State) -> Option<String> {
    match &state.mode {
        Mode::SavePrompt { filename } => Some(format!("Save as: {filename}{CURSOR}")),
        _ => None,
    }
}
```

`INTERRUPT` and `CURSOR` move here. `ENTER_ALTERNATE_SCREEN`, `LEAVE_ALTERNATE_SCREEN`,
`HIDE_CURSOR` and `SHOW_CURSOR` move to `terminal.rs`; `CLEAR_LINE` and
`NOT_SUPPORTED_MESSAGE` to `kitty.rs`. `mod writer;` is replaced by
`mod filesystem;`.

### Quitting

The save moves out of the loop, which makes an existing asymmetry explicit.

Today the save sits inside the loop after `handle_key`, while Ctrl-C is an early
`return Ok(())` a few lines above it — so Ctrl-C skips the save as a side effect
of which keyword ended the loop, a rule stated nowhere.

With the save after the loop, `break` would fall into it and Ctrl-C would start
writing the file. So Ctrl-C clears `save_to` first. "Quit without saving" becomes
a fact about the state rather than an accident of control flow, and the bytes on
disk are what they are today: Ctrl-C writes nothing, every other quit path saves
when a path is set.

### Dependencies after this spec

```
filesystem   ← (nothing but std::fs)
terminal     ← (nothing in-crate)
state        → diagram, command_mode, insert_mode, save_prompt_mode
main         → cli, kitty, terminal, render, state, filesystem,
               dre_format, file_document
```

`main` composing `dre_format` + `file_document` in both directions is the one
known wart, and it is deliberate. `cli::export` keeps its own near-identical copy
of the load path. Collapsing both into a single text ↔ `Document` function — most
naturally by letting `file_document` absorb `dre_format` — is a separate spec, so
that this one stays a pure move.

### Tests

`writer.rs`'s tests are redistributed; none are lost.

- **To `render/terminal.rs`**: the three `prompt_line` tests (shown, padded,
  cut) become `status_line` tests, plus one that it writes to the last row and
  one that `None` writes nothing.
- **To `main.rs`**: `status` returns `Some` in `SavePrompt` and `None` in
  `Command` — two small tests replacing the two frame-level prompt tests.
- **To `state.rs`**: the selection and `save_to` rules, driven by a `Document`
  literal — `load` selects the first box, `load` of an empty document selects
  nothing, `load` records `save_to`, `load` is not a new file, `new_file` is
  empty with `save_to` set and `new_file` true. The temp files these needed are
  gone.
- **To `filesystem.rs`**: `read` round-trips what `write` wrote; `read` of a
  missing path is `NotFound`; `missing` and `invalid` have the exact messages
  today's tests assert.
- **To `main.rs`** (temp files, as today): no argument gives an empty state;
  a valid file loads with the first box selected and saves back to that path; a
  zero-byte file and a malformed file are `InvalidData`; an invalid file's
  message is `"{path}: not a valid diagram"`; a missing file gives an empty
  state saved to that path with `new_file` set.
- **Deleted**: `the_renderer_is_given_the_terminal_size` and
  `what_the_renderer_returned_is_painted`. Both assert through `frame` what
  `TerminalRenderer`'s own tests have asserted directly since 076.
- **New**: Ctrl-C leaves `save_to` as `None`, so nothing is written.

Every `render`, `layout`, `svg`, `dre_format`, `file_document`, `cli` and mode
test is untouched.
