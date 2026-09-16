# 057 - Prompt to save new diagram on quit

## User Story

As a `dre` user, I want to be prompted to save my new diagram when I quit, so that I don't lose my work by accident.

## Acceptance Criteria

- Pressing `q` on a diagram that has never been saved to a file shows a filename prompt.
- The prompt is pre-filled with a default filename of `diagram.dre`.
- Typing a filename and confirming saves the diagram to that file (in the current diagram format) and then quits `dre`.
- If the filename I type doesn't end in `.dre`, `.dre` is appended automatically.
- Pressing Escape at the prompt quits `dre` without saving anything.
- After a successful save, a real `.dre` file exists on disk containing the diagram, and `dre` has exited.

## Technical Design
