# Expose editor data

## User Story

As a developer embedding the web editor, I want `WebSession` to expose the current mode and total box count, so that I can build UI around the editor's state without reaching into internal editor logic.

## Acceptance Criteria

- `WebSession` exposes a method returning the current mode as one of `"Command"`, `"Insert"`, or `"SavePrompt"`.
- `WebSession` exposes a method returning the total number of boxes in the diagram, counting every box including nested children.
- Both values can be read independently via direct method calls (not only through the `on_change` callback).

## Technical Design
