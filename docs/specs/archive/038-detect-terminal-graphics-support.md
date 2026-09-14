# 038 - Detect terminal graphics support

## User Story

As a user, when I launch sketch in a terminal that doesn't support the Kitty graphics protocol, I want to see a clear message telling me my terminal isn't supported, so I don't get stuck with a broken or unusable editor.

## Acceptance Criteria

- Launching sketch in a terminal without Kitty graphics protocol support prints a generic message stating the terminal isn't supported, and the app exits immediately without opening the editor.
- Launching sketch in a terminal that does support the Kitty graphics protocol (e.g. Kitty, Ghostty, WezTerm) continues to work normally, unaffected by this check.

## Technical Design

- A single function, `supports_kitty_graphics()`, lives in `sketch/writer.py` alongside `main()`. No new module or class — this is a one-shot check, not a component with state or collaborators.
- `main()` calls `supports_kitty_graphics()` first, before anything else runs. If it returns `False`, `main()` prints `"sketch requires a terminal with Kitty graphics protocol support."` and exits with `sys.exit(1)`. `run()` (and therefore `terminal_session`, the alternate screen, etc.) is never invoked.
- Detection works by sending the Kitty graphics query escape sequence (`\x1b_Gi=1,a=q;\x1b\\`) to stdout and reading stdin for a reply. A terminal that implements the protocol answers with a response containing `i=1`; a terminal that doesn't ignores the query entirely and sends nothing.
- Reading is a plain blocking read (`stdin.read`) — no `select`, no timeout, no polling. This mirrors the simplicity of the standard shell one-liner for this check.
- Because the response must be read without waiting on Enter/line-buffering, `supports_kitty_graphics()` puts stdin into raw mode for the duration of the write+read, then restores the original `termios` settings — the same save/set-raw/restore shape already used by `terminal_session` in this file, just scoped locally to this function rather than shared with it. This only affects how keystrokes/replies are read; it is unrelated to the alternate screen (`ENTER_ALTERNATE_SCREEN`/`LEAVE_ALTERNATE_SCREEN`), which is never entered when the check fails.
