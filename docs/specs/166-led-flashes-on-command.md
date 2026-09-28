## User Story

Doug is in Command mode with the footer LED sitting dim lime. He runs a command — moving with h/j/k/l, changing color, changing fill, or anything else available in that mode. The instant dre receives the command, the LED flashes fully lit lime, then goes back to dim as soon as the command finishes — like a remote control's light blinking to confirm the button press. He fires off commands rapidly one after another, and sees the LED blink once for each individual command, keeping pace with him rather than just staying lit.

## Acceptance Criteria

- Whenever a command executes in Command mode, the footer LED goes dim → fully lit → dim again, staying the same lime color throughout.
- The LED turns dim again immediately once that command completes (no extra hold time).
- Rapid, back-to-back commands each get their own distinct flash — one blink per command, not one sustained lit state.
- This story only covers Command mode; Write and Naming mode LED behavior is unchanged.

## Technical Design

The control loop (`DreController::run` in `src/editor/controller/mod.rs`) is synchronous and key-driven — there's no timer or animation-frame ticker anywhere in the codebase, so the flash is produced by rendering twice around a single keystroke rather than by any duration-based mechanism.

- **`State` gets a transient `led_flash: bool` field** (default `false`), alongside the existing fields in `src/state/mod.rs`.
- **`state::flash(state: State) -> State`** is a new, small, state-module-owned pure function that returns a copy of `state` with `led_flash: true`. It's the only place the flag is ever set. The controller calls it directly — this mirrors the existing precedent of `State::set_save_to`, a direct mutator called from the imperative shell outside the `reduce`/`Action` pipeline, since `led_flash` is render-timing state, not domain state.
- **`reduce()`** (`src/state/mod.rs`) unconditionally resets `led_flash` to `false` as part of its existing contract. `flash()` turns it on; the very next `reduce()` call always turns it back off. No other code needs to know the field exists — in particular, none of the ~1500 lines of command handlers in `src/state/command.rs` need to manage it.
- **`FooterModel`** (`src/state/mod.rs`) gets a `flash: bool` field, populated from `state.led_flash` in the `Mode::Command` arm of `State::footer()`; the `Mode::Insert` and `Mode::NamePrompt` arms always set it to `false` (this story only covers Command mode).
- **`view::footer()`** (`src/view.rs`) changes the LED's `lit` computation from `!matches!(model.mode, FooterMode::Move)` to: lit is `true` for `Write`/`Naming` as before, and equal to `model.flash` for `Move`. The LED colour is unchanged (still lime for `Move`).
- **`DreController::run`** changes to render an extra lit frame around real keystrokes received while in `Mode::Command`: when `next_key()` returns `Some(key)` (not the resize key) and `state.mode()` is `Mode::Command`, the controller calls `state::flash(state.clone())` and renders that before calling `reduce` as normal. `reduce`'s result (with `led_flash` reset to `false`) is rendered again afterward, same as today. Idle `None` polls and the resize key never trigger a flash.

This gives the required sequence — dim → lit → dim, one blink per keystroke, no extra hold time — using the existing render call and reduce cycle, with no new timer, trait, or public API surface beyond the one small `state::flash` function.
