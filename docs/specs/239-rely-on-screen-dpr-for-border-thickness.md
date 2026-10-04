# Rely on screen DPR when painting dre-flex borders

## Problem

`dre-flex` paints box borders at a fixed thickness: `FLEX_BORDER = 1`
(`src/flex/view.rs:11`), in the same device-pixel units `BoxShape`
rasterizes with (`src/render/shapes.rs`). On a Retina/HiDPI screen, one
device pixel renders near-invisibly thin, since the terminal's
`TIOCGWINSZ` pixel geometry (`src/tty.rs`) reflects the terminal's own
pixel buffer, not the monitor's physical pixel density. The non-flex
diagram view's `BORDER = 4` (`src/view.rs:16`) is unaffected and out of
scope here.

## Technical Design

### Getting the DPR

Add the `display-info` crate (native-only; same
`[target.'cfg(not(target_arch = "wasm32"))'.dependencies]` gate as
`russh`/`tokio` in `Cargo.toml`, since `dre-flex` already depends on
`tty.rs`'s ioctl-based `probe()`, which isn't available under
`wasm32`).

`dre-flex` runs in a terminal process with no window handle, so there
is no way to ask which physical monitor the terminal's own window is
on. `DisplayInfo::from_point` could resolve that given a point, but
getting a reliable point (cursor position, or the terminal's own
on-screen position via XTWINOPS `\x1b[13t`) is its own can of worms and
not every terminal supports it. For now we use the primary display's
`scale_factor` (`DisplayInfo::all()`, filtered to `is_primary`) as the
DPR, which is exact for the common single-monitor case and a known
approximation when `dre` runs on a non-primary secondary monitor with
a different scale factor.

If `DisplayInfo::all()` errors, or no entry has `is_primary == true`
(headless/CI/SSH sessions, unsupported platforms), fall back to
`dpr = 1.0` — today's unscaled behaviour.

### When it's read

Read once, at startup, in `flex::mod::start()` (where `tty::probe()`
already happens). Not re-read on resize or monitor change — moving the
terminal window to a different-DPR monitor mid-session won't rescale
the border until the next launch.

### Threading the scaled value through

Compute the scaled border thickness once in `start()`:

```rust
let border = ((FLEX_BORDER as f64) * dpr).round().max(1.0) as i64;
```

(`round` to the nearest device pixel, `max(1.0)` so it can never
vanish even if `dpr` were degenerate, e.g. `0.0`.)

Store it as a field on `TerminalFlexScreen` (`src/flex/mod.rs:22`), and
thread it as a parameter rather than a global/static:

- `TerminalFlexScreen::render` passes it into `view::scene(state,
  self.renderer.area(), &new, border)`.
- `scene()` passes it into each `paint(state, new, flex_box, arranged,
  border)` call.
- `paint()` uses the `border` parameter instead of the `FLEX_BORDER`
  constant at both call sites: the box border placement and the
  replace-mode highlight placement.

`FLEX_BORDER` stays as the base (1x) constant; existing tests that
assert a placement's border equals `FLEX_BORDER` keep passing by
calling `scene()`/`paint()` with `border: FLEX_BORDER` explicitly
(DPR = 1.0), so no existing test changes behaviour under test.

### Out of scope

- Re-reading DPR on resize or monitor change.
- Detecting the actual monitor the terminal window is on (cursor
  position or XTWINOPS-based heuristics).
- The non-flex diagram view's `BORDER` constant (`src/view.rs`).
