# Command Status Line

Marouane is editing a diagram in dre. He presses `b` to create a new box. At the bottom-center of the screen, on the same row as the footer, he sees `["b" Create New Box 20ms]` appear — confirming exactly which command just ran and how long it took. He presses another key, and the line updates to reflect that new command. Before he's pressed anything, that space is simply empty.

## Acceptance Criteria

- The command-status line sits at the bottom-center of the screen, on the same row as the footer.
- Before any command has been executed, the space is empty.
- After every command — not just box actions, but any dispatched command including selection/navigation — the line updates to show the one just executed.
- Only the most recent command is shown (no history/log).
- Format is exactly: `["<key>" <Human-Readable Command Name> <duration>ms]`

## Technical Design
