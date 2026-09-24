# 105: Fix the resize test that depends on a real TTY

## Problem

`editor::tests::a_resize_key_re_probes_the_terminal_and_propagates_a_failed_probe`
fails when `cargo test` runs in an interactive terminal:

```
panicked at src/editor.rs:289:9:
terminal::probe performs a real ioctl against stdout, which is not a TTY in the
test process, so the RESIZE arm's re-probe is expected to fail here
```

The `RESIZE` arm of `edit` calls `terminal::probe()`, which runs an ioctl on
the real fd 1. The test asserts that this fails, assuming fd 1 is never a TTY.
In a terminal the probe succeeds, so the test fails. Under CI or a pipe it
passes. The test result depends on where it is run.

## Acceptance Criteria

- The resize tests pass whether or not fd 1 is a TTY.
- A failed probe on `RESIZE` is propagated as the error from `edit`.
- A successful probe on `RESIZE` is called exactly once and the loop carries on.
- No production behaviour changes: `open` still probes with `terminal::probe`.

## Technical Design

### Inject the probe into `edit` (`src/editor.rs`)

`edit` already receives input as an injected closure (`next_key`). The probe is
the same kind of seam and gets the same treatment:

```rust
fn edit(
    state: State,
    mut next_key: impl FnMut() -> io::Result<Option<String>>,
    mut probe: impl FnMut() -> io::Result<Terminal>,
    output: &mut impl Write,
    renderer: &mut TerminalRenderer,
) -> io::Result<State>
```

- The `RESIZE` arm becomes `renderer.on_resize(probe()?)`.
- `open` passes `terminal::probe` as the probe. Nothing else in production changes.

### Test helpers (`#[cfg(test)]` in `src/editor.rs`)

- `run_script(state, script)` keeps its signature, so the existing call sites
  don't change. It passes a probe that panics with an explanatory message:
  `"unexpected terminal probe: only a RESIZE key makes edit() re-probe the terminal, and this test's script has none. Use run_script_with_probe to supply a probe stub for tests that send RESIZE."`
  A test that never meant to resize fails loudly if it ever probes.
- New helper `run_script_with_probe(state, script, probe)` holds the body that
  `run_script` used to have. `run_script` delegates to it with the panicking probe.
- `run(state, key)` is unchanged.

### Tests

Both tests assert only on the probe stub. They do not inspect the renderer.
What `on_resize` does with a new size belongs to the renderer's own tests.

1. `a_resize_key_propagates_a_failed_probe`
   Stub returns `Err(io::Error::other(..))`. Script is `[RESIZE, "q"]`.
   Assert `edit` returns that error. The error must come from the stub, not
   from the script running out (`UnexpectedEof`).
2. `a_resize_key_re_probes_the_terminal_once`
   Stub counts calls and returns a `Terminal`. Script is `[RESIZE, "q"]`.
   Assert `Ok`, `running == false`, and exactly one probe call.

The old test `a_resize_key_re_probes_the_terminal_and_propagates_a_failed_probe`
is removed and replaced by these two.

### Out of scope

- Injecting the probe into `open`. It is glue and is exercised by running the app.
- Renderer behaviour after `on_resize`.
