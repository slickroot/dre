# Undo the last command

As a sketch user, I want to press `u` to undo my last command so that I can recover from a mistake without redoing my work.

## Acceptance Criteria

- After performing a single box-changing command (`b`, `c`, `f`, `r`, or `C`), pressing `u` restores the state (boxes and selection) to exactly what it was before that command.
- Pressing `u` when there is no previous action to undo (e.g. at the very start, or after already undoing) leaves the state unchanged.
- Undo only applies to command-mode actions; insert-mode typing is out of scope for this story.

## Technical Design
