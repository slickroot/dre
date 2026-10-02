Doug is typing a label in dre-flex. As soon as he enters typing mode a `|` blinks at the end of his text, right where the next character would land. It blinks once per second and sits in the empty cell after the text, so nothing moves. When Doug backspaces to empty, the `|` is still there, blinking. The moment he leaves typing mode, it's gone.

## Acceptance Criteria

1. Pressing the key that starts typing on a box with no text shows the `|` immediately, before any character is typed.
2. The `|` sits in the cell immediately after the last typed character, and typing another character leaves the `|` one cell further along, with no other text shifting.
3. The `|` is visible for roughly half a second and hidden for roughly half a second, on that repeating cycle.
4. Backspacing to an empty label leaves the `|` visible in that position, still blinking.
5. The `|` blinks only in typing mode; once Doug leaves typing mode it is not drawn.
6. The `|` does not occupy a cell — nothing after it moves.

## Technical Design
