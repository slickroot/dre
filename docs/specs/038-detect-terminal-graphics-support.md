# 038 - Detect terminal graphics support

## User Story

As a user, when I launch sketch in a terminal that doesn't support the Kitty graphics protocol, I want to see a clear message telling me my terminal isn't supported, so I don't get stuck with a broken or unusable editor.

## Acceptance Criteria

- Launching sketch in a terminal without Kitty graphics protocol support prints a generic message stating the terminal isn't supported, and the app exits immediately without opening the editor.
- Launching sketch in a terminal that does support the Kitty graphics protocol (e.g. Kitty, Ghostty, WezTerm) continues to work normally, unaffected by this check.

## Technical Design

