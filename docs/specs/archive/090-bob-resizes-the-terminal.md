# Bob resizes the terminal

Bob opened `dre` and drew a diagram. He resized his terminal window, and the
diagram redrew live to fit the new size — centered correctly, with no
corrupted or leftover content on screen.

## Acceptance Criteria

- When the terminal window is resized, `dre` detects the size change without
  requiring a keypress or other action.
- The diagram redraws live, using the new terminal dimensions, re-centered to
  fit the new space (in both growing and shrinking directions).
- The status line is redrawn at the correct row for the new terminal size.
- No stale/leftover content remains on screen outside the new bounds after a
  resize.

## Technical Design

Resize detection uses `SIGWINCH` via the standard self-pipe trick, since a
signal handler can't safely call `ioctl` or touch shared program state
directly.

- `terminal::install_resize_pipe() -> io::Result<RawFd>` creates a pipe,
  stashes the write-end fd in a static (the only thing the signal handler is
  allowed to touch), installs a minimal `SIGWINCH` handler that writes one
  byte to it, and returns the read-end fd. This is a plain function, not a
  guard/`Drop` type like `RawScreen` — there is nothing to restore on exit;
  the OS reclaims the pipe and handler when the process ends.
- `poll_read` gains a second parameter: `poll_read(fd: RawFd, resize_fd:
  RawFd, timeout_ms: u16) -> io::Result<Option<String>>`. It polls both fds.
  When the resize fd becomes readable, it drains the byte and returns
  `Some(RESIZE.to_string())` instead of a real key.
- `RESIZE` is a sentinel constant (a multi-character string), following the
  existing `INTERRUPT` pattern in `editor.rs`. A real keypress from stdin is
  always a single byte turned into a single-character string, so `RESIZE`
  can never collide with actual input — no new `Event` enum is needed, and
  `next_key`'s existing signature and test-script shape stay intact.
- `editor::edit`'s match gains an inline arm:
  `Some(key) if key == RESIZE => renderer.on_resize(terminal::probe()?)`.
  A failed re-probe propagates via `?` and ends the session, same as every
  other I/O error in this loop — no special-cased recovery.
- `TerminalRenderer::on_resize(&mut self, terminal: Terminal)` overwrites
  `self.terminal`. The sprite cache (keyed on box/arrow size in cells, not
  pixels) is left untouched: cell pixel size is assumed constant for the
  life of a terminal session, so cached sprites stay valid across a resize.
- No extra clearing logic is needed for "no stale content": every frame
  already does a full redraw (`HOME_CURSOR` + a full `terminal.cols` x
  `terminal.rows` character grid) and `kitty::clear()` deletes all images
  before showing the current frame's sprites. Once `on_resize` updates
  `self.terminal`, the very next `render()` call recomputes `centre_on`
  against the new dimensions and `status_line` writes to the new
  `self.terminal.rows` — both are already driven off that one field.
- The write end of the self-pipe is left blocking (no `O_NONBLOCK`) for now;
  a burst of rapid `SIGWINCH` signals filling the pipe before it's drained
  is treated as an acceptable, unhandled edge case rather than something to
  engineer around up front.

Testing boundary: `install_resize_pipe` and the signal handler itself are
not unit-testable (they need a real OS signal) and are verified manually by
resizing an actual terminal. Everything downstream is covered by unit
tests: `poll_read` against a real pipe fd, `TerminalRenderer::on_resize`
re-centering on the next render, and `editor::edit`'s `RESIZE` arm via a
scripted `Some(RESIZE)` entry in the existing test harness.
