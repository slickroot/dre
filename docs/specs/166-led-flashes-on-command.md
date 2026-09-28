## User Story

Doug is in Command mode with the footer LED sitting dim lime. He runs a command — moving with h/j/k/l, changing color, changing fill, or anything else available in that mode. The instant dre receives the command, the LED flashes fully lit lime, then goes back to dim as soon as the command finishes — like a remote control's light blinking to confirm the button press. He fires off commands rapidly one after another, and sees the LED blink once for each individual command, keeping pace with him rather than just staying lit.

## Acceptance Criteria

- Whenever a command executes in Command mode, the footer LED goes dim → fully lit → dim again, staying the same lime color throughout.
- The LED turns dim again immediately once that command completes (no extra hold time).
- Rapid, back-to-back commands each get their own distinct flash — one blink per command, not one sustained lit state.
- This story only covers Command mode; Write and Naming mode LED behavior is unchanged.

## Technical Design
