## User Story

Maya is typing a label into a box in Write mode. While she pauses, the footer LED rests dim violet. Each key she presses (a letter, Backspace, an arrow, Enter, or Esc) makes the LED flash fully lit violet and then drop back to dim, just like the lime LED does in MOVE mode. She types a word quickly and sees one violet blink per keystroke, keeping pace with her fingers.

## Acceptance Criteria

- In Write mode, while Maya isn't typing, the footer LED rests dim violet.
- Every key pressed in Write mode (characters, Backspace, ←, →, Enter, Esc) makes the LED go dim → fully lit → dim, staying violet throughout.
- On Esc, the violet flash happens before the LED switches to dim lime for MOVE mode.
- When she types quickly, each key gets its own distinct blink instead of the LED staying lit.
- The LED in Naming mode stays unchanged: amber and always fully lit.

## Technical Design

The flash machinery from spec 166 (`state::flash`, `State.led_flash`, `FooterModel.flash`, the controller's flash frame + `flash_time` sleep) is reused as-is. Today the "which modes flash" rule is spread over three places; this story collapses it so the controller decides *when* a flash happens and the view alone decides *how* each mode shows it.

- **`DreController::run`** (`src/editor/controller/mod.rs`) drops the `*state.mode() == Mode::Command` guard. Every real keystroke (anything but `tty::RESIZE`) renders `state::flash(state.clone())`, sleeps `flash_time`, then reduces as before. Idle polls and resize still never flash.
- **`State::footer()`** (`src/state/mod.rs`) sets `flash: self.led_flash` in every arm (`Insert`, `NamePrompt`, `Command`). `FooterModel` becomes a plain projection of state with no per-mode policy.
- **`view::footer()`** (`src/view.rs`) owns the rule for when the LED is lit: `Move | Write => model.flash`, `Naming => true`. Colours are unchanged (lime / violet / amber).
- **Esc in Write mode** needs no special handling. The flash frame is rendered from the pre-`reduce` state (still `Insert`, so violet), then `reduce` switches to `Command` and the next render shows dim lime.
- **Fast typing** also comes for free. Each keystroke renders its own flash frame and holds it for `flash_time`, and the loop renders the reduced (dim) state before the next key, so every key gets its own dim → lit → dim.

### Test changes

- Controller: `a_keystroke_outside_command_mode_does_not_flash` is replaced by a test that an Insert-mode keystroke flashes before reducing. A NamePrompt keystroke also renders a flash frame, because the controller no longer checks the mode.
- State: `insert_mode_footer_flash_is_always_false` becomes "true after flash". `name_prompt_footer_flash_is_always_false` becomes "true after flash" (pass-through).
- View: `footer_led_is_lit_in_write_mode_regardless_of_flash` splits into "unlit in write mode when not flashing" and "lit in write mode when flashing". The Naming test (`lit regardless of flash`) stays as-is.
