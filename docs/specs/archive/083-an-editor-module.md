# 083: An editor module

Refactoring spec — no user story. Behaviour is unchanged.

## Goal

`main.rs` is not a dispatcher. It is a dispatcher plus the editor: a loop, a
path-to-`State` loader, a mode-to-status-text policy, two constants, and eight
tests. `main` itself is nine lines of the file's 215.

Spec 082 put the loop there on purpose, so that the program would be visible in
one place, and that was the right move at the time — it is how the five unrelated
responsibilities inside `writer.rs` got separated. But `main.rs` is the one
module that cannot be a component: it is the binary's root, it owns the `mod`
list, and nothing can call into it. Whatever lives there is unreachable from a
test.

That cost is now paid in full, and there is a test that shows the bill:

```rust
fn an_interrupt_leaves_nothing_to_save_so_the_file_is_never_written() {
    let mut state = load(Some(path.clone())).unwrap();
    let key = (INTERRUPT.as_bytes()[0] as char).to_string();
    assert_eq!(key, INTERRUPT);
    state.save_to = None;
    if let Some(path) = &state.save_to { filesystem::write(path, "").unwrap(); }
    assert!(!std::path::Path::new(&path).exists());
}
```

It never calls `edit`. It re-enacts `edit`'s body inside the test and asserts the
re-enactment — it would still pass if `edit` were deleted. And the rule it is
pretending to cover, *interrupt discards the save*, is this loop's own invention:
`command_mode` has never heard of `\x03`.

After this spec the loop lives in `src/editor.rs`, where it can be called, and
`main.rs` is the `match` and the `eprintln!`.

## Technical Design

### Principle

The editor splits at the line where a terminal stops being necessary. Above it,
acquiring a terminal — the Kitty probe, raw mode, the alternate screen. Below it,
a loop that needs only bytes in and bytes out. That line is the module's whole
design: it is what makes the loop callable, and everything else follows from it.

```rust
pub(crate) fn open(file: Option<String>) -> io::Result<ExitCode>
fn edit(state: State, input: &mut impl Read, output: &mut impl Write,
        renderer: &mut TerminalRenderer) -> io::Result<State>
fn load(file: Option<String>) -> io::Result<State>
fn status(state: &State) -> Option<String>
```

`open` acquires, as `RawScreen::open` does. `edit` is the editing. Only `open` is
`pub(crate)`; the other three are private, and their tests sit beside them in the
module's own `#[cfg(test)] mod tests`.

### `edit` — the loop

```rust
fn edit(state, input, output, renderer) -> io::Result<State> {
    let mut state = state;
    while state.running {
        let status = status(&state);
        renderer.render(&state.doc, output)?;
        renderer.status_line(status.as_deref(), output)?;
        output.flush()?;

        let mut key = [0u8; 1];
        input.read_exact(&mut key)?;
        let key = (key[0] as char).to_string();
        if key == INTERRUPT {
            state.save_to = None;
            break;
        }
        state = handle_key(state, &key);
    }
    Ok(state)
}
```

Three decisions are load-bearing.

**The renderer is a concrete `&mut TerminalRenderer`, not `&mut impl Renderer`.**
`status_line` is an inherent method on `TerminalRenderer`, not a trait method,
because 076 settled that the trait holds only what both renderers do and
`SvgRenderer` has no concept of a status line. A generic parameter here would
force `status_line` onto the trait and hand `SvgRenderer` a no-op, undoing that.
The concrete type costs nothing in testability: `TerminalRenderer::new(Terminal
{ .. })` is a plain struct with no tty behind it, which is how `render/terminal.rs`'s
own tests already build one. The `Renderer` trait exists so that `main` and `cli`
can choose a renderer per command, not so that the editor can be polymorphic. The
editor is the terminal editor.

**`edit` returns the final `State` and never writes a file.** The save stays in
`open`. A save inside the loop would put every loop test on disk with a temp path
and cleanup, and would send the interrupt test straight back to asserting a
file's absence instead of the rule. Nor can the destination be injected as a
pre-opened sink: `save_to` is chosen *during* the loop — the save prompt sets it
when the user types a filename at keystroke 40 — and `None`, which is what an
interrupt produces, is not a sink at all. A callback seam (`&mut impl FnMut(&str,
&str)`) would carry the path but make every test build a closure and assert on
captured arguments, to cover a two-line `if let`. Returning the state gets the
same coverage for free: because the write is a total function of `save_to`,
`save_to: None` *is* the assertion that nothing is written. `editor` needs
neither `dre_format` nor `file_document` in scope.

**EOF stays an error.** `read_exact` on an exhausted `Read` returns
`UnexpectedEof`, which propagates. A tty never EOFs, so production behaviour is
exactly today's, and `dre` with stdin closed failing loudly is correct. Breaking
the loop on EOF would invent a production behaviour whose only motivation is test
convenience — and would then require inventing a save rule for it, discard or
keep, with nothing to decide it. The constraint this puts on tests is that every
fixture ends in a key that ends the loop, which makes them exercise the real exit
path.

### `open` — the session

```rust
pub(crate) fn open(file: Option<String>) -> io::Result<ExitCode> {
    let state = load(file)?;
    let mut stdout = io::stdout();
    let mut stdin = io::stdin();
    kitty::require(&mut stdout, stdin.as_raw_fd())?;
    let mut renderer = TerminalRenderer::new(terminal::probe()?);
    let _screen = RawScreen::open(stdin.as_raw_fd())?;

    let state = edit(state, &mut stdin, &mut stdout, &mut renderer)?;

    if let Some(path) = &state.save_to {
        filesystem::write(path, &dre_format::write(&file_document::from_document(&state.doc)))?;
    }
    Ok(ExitCode::SUCCESS)
}
```

Everything here needs a real terminal — `kitty::require` waits up to 500ms for a
reply, `RawScreen::open` calls `tcgetattr` on a real fd — which is exactly why it
is separated from `edit` rather than inlined with it. `open` is not tested, and
should not be: faking a tty to verify that we call two setup functions buys
nothing.

082's constraints are preserved. `stdout` is a plain `Stdout`, never a
`StdoutLock`, or `RawScreen`'s `drop` would deadlock taking the lock at exit. The
binding is `let _screen`, not `let _`, or raw mode would be turned on and dropped
on the spot. And `ExitCode` is kept rather than `()` because both arms of `main`'s
match must agree in type and `cli::export` returns `io::Result<ExitCode>`.

`load` and `status` move verbatim from `main.rs`, along with `CURSOR` and
`INTERRUPT`. `load` belongs here because path-to-`State` is a step of the edit
path; `cli::export`'s near-duplicate of it stays as 082 left it, deferred to the
spec that will collapse both into one text-to-`Document` function.

### `src/main.rs`

```rust
fn main() -> ExitCode {
    let result = match cli::parse_args() {
        cli::Command::Edit(file) => editor::open(file),
        cli::Command::Export { input } => cli::export(input),
    };
    match result {
        Ok(code) => code,
        Err(e) => { eprintln!("{e}"); ExitCode::FAILURE }
    }
}
```

That plus the `mod` list, with `mod editor;` added. No other functions, no
constants, no tests.

### Dependencies after this spec

```
editor  → kitty, terminal, render, state, filesystem, dre_format, file_document
main    → cli, editor
```

### Tests

Moved from `main.rs` to `editor.rs`, unchanged: the two `status` tests, and the
six temp-file `load` tests (no argument, a valid file, a zero-byte file, a
malformed file, an invalid file's message, a missing file).

Deleted: `an_interrupt_leaves_nothing_to_save_so_the_file_is_never_written`,
which tests a copy of the loop rather than the loop.

New, on `edit` — three, each pinning a rule that exists in no other module. All
three build a `TerminalRenderer` from a `Terminal` literal, read from a byte
slice, and write to a `Vec<u8>`; none touches the filesystem or a tty.

- **An interrupt clears `save_to`.** Feed `"\x03"` to a state with a path set;
  the returned `save_to` is `None`. This is the rule the deleted test was
  impersonating.
- **A quit preserves `save_to`.** Feed `"q"` to the same state; the returned
  `save_to` still holds the path and `running` is false. Together with the above:
  the loop clears `save_to` on interrupt and only on interrupt. It also proves
  the loop terminates.
- **A frame is painted before the first key is read.** Feed `"\x03"`, which
  breaks before any reduction; the output buffer is already non-empty. Pure
  ordering, invisible to every other module, and a blank first screen is a real
  bug that nothing else would catch.

Deliberately not tested: that each key is reduced in order. The loop's only
contribution there is "once, in sequence", and asserting it means either
duplicating `command_mode`'s tests through a keyhole or stubbing `handle_key`,
which asserts our own call graph back to us.

Every `render`, `layout`, `svg`, `dre_format`, `file_document`, `filesystem`,
`state`, `cli` and mode test is untouched.
