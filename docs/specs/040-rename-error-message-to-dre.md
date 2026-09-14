# 040 - Rename error message to Dre

## User Story

As a user launching Dre in an unsupported terminal, I want the error message to refer to the tool by its correct name, so the message is consistent with the product I'm actually using.

## Acceptance Criteria

- Launching in a terminal without Kitty graphics protocol support prints: `"Dre requires a terminal with Kitty graphics protocol support."`
- The app still exits immediately without opening the editor, unchanged from current behavior.

## Technical Design

