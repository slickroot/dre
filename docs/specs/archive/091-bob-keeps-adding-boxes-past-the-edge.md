# Bob keeps adding boxes past the edge

## User Story

Bob opens dre and keeps adding boxes to a diagram. When a new box would land past the right edge of the terminal, the view automatically scrolls so the new box is fully visible, instead of being cropped or hidden — so he can keep building his diagram wider than the terminal without losing track of what he just added.

## Acceptance Criteria

- When Bob adds a box that would land past the right edge of the terminal, the view shifts left just enough so that new box is fully visible.

## Technical Design

### Overview

Today `Screen::centre_on` (render/terminal.rs) recomputes the horizontal origin from scratch on every render by centering the whole diagram's bounding box in the terminal. That has two problems: it redistributes any overflow across *both* edges (so content that already fit can get newly clipped when the diagram grows on one side), and it doesn't guarantee the box that was just added is actually visible.

We introduce a persisted, incrementally-adjusted horizontal scroll offset that only moves when something would go off-screen, and by exactly the amount needed — instead of recentering everything on every keystroke.

### `State.scroll_x`

- A new `pub(crate) scroll_x: i64` field on `State` (state.rs), defaulting to `0`.
- It is a plain integer, opaque to terminal geometry. `state.rs` never needs to know about `Terminal`, `TerminalRenderer`, or column counts.
- It is mutated only inside `handle_key`, via a new `Command::ScrollBy(i64)` handled in `command_mode::reduce`, keeping every `State` mutation — including any future manual scroll key — funneled through the same single reducer. There is exactly one mechanism that changes the view; automatic follow-on-overflow and a future manual scroll command both drive it.

### Triggering an automatic scroll

- `editor.rs`'s `edit()` loop is the only place that has both the current `State` and the terminal's column count (via the renderer). After reducing a real key (editor.rs:58), it calls a small pure function — plain integers in, plain integer out: the selected box's placement edges, the current `scroll_x`, and the terminal's column count go in; an `Option<i64>` overflow delta comes out.
- If it returns `Some(delta)`, the loop synthesizes a sentinel key string (`"\x1bSCROLL" + delta`, following the same pattern `terminal::RESIZE` already uses for synthesized input) and calls `handle_key` again with it, in the same loop iteration, before the next paint. `command_mode::parse` recognizes the `\x1bSCROLL` prefix and produces `Command::ScrollBy(delta)`. (This needs an unambiguous prefix — a bare digit string would otherwise be swallowed by `state::handle_key`'s existing single-digit `pending_count` accumulation in `Mode::Command`.)
- Because this all happens before the loop's next `render` call, Bob never sees an intermediate unscrolled frame.

### Rendering with the offset

- `Renderer::render` changes signature from `fn render(&mut self, doc: &Document, out: &mut impl Write)` to `fn render(&mut self, state: &State, out: &mut impl Write)`. `State` becomes `pub` (the type only; its fields stay `pub(crate)`, opaque outside the crate) so it can appear in the public trait used by the wasm `web/` crate.
  - `SvgRenderer::render` (render/svg.rs) is updated to read `state.doc` instead of a bare `doc`, and ignores `scroll_x` — SVG output isn't viewport-constrained.
  - `web/src/lib.rs`'s `WebSession::svg` call site updates to pass `session.state()` (a new accessor alongside the existing `document()`) instead of `session.document()`.
- `TerminalRenderer::render` (render/terminal.rs) computes the horizontal origin as: if the diagram's full span fits within the terminal's columns, centre it as `Screen::centre_on` does today (dynamic, recomputed every frame — small diagrams keep today's centered UX). If it doesn't fit, use `state.scroll_x` directly instead of centering, so the view only moves in response to an explicit `ScrollBy`, never as a side effect of unrelated edits.

### Summary of collaborators

- `State` — owns `scroll_x`; the single source of truth for view position, mutated only through `handle_key`.
- `command_mode` — gains `Command::ScrollBy(i64)`, parsed from either a synthesized overflow key or (later) a real manual-scroll keystroke.
- `editor.rs` (`edit` loop) — the only place that knows both current state and terminal width; detects overflow after each real key and synthesizes the follow-up scroll key.
- `TerminalRenderer` — reads `state.scroll_x` to position the diagram; no longer owns any view state itself.
- `Renderer` trait / `SvgRenderer` / `web/` — updated mechanically to pass `&State` through instead of `&Document`, with no behavior change for SVG export.
