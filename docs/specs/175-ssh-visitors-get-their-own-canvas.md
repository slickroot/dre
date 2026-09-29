# Doug tries dre over SSH

Doug types `ssh dre.sh`. dre opens for him with an empty canvas, as if he'd run `dre` on his own machine. He builds a few boxes with the usual keys, presses `Ctrl-C` and the connection closes. Pleased, he tells a friend to try it.

## Acceptance Criteria

- Anyone can run `ssh dre.sh` and get in, with no password or sign-up, as long as they offer an SSH key.
- On connecting, they see an empty dre canvas.
- The usual dre keys work for building boxes.
- `Ctrl-C` closes the connection.
- Two people connected at the same time each get their own canvas, and neither sees or affects the other's boxes.

## Technical Design

### Approach

Process per visitor. `dre serve` is an SSH server built on `russh`. For every connection it starts a plain `dre` (the current executable, no arguments) on a fresh PTY and relays bytes between the SSH channel and the PTY. The editor code is untouched: its `tty`, `kitty` and resize statics keep working because each visitor has a separate process. Isolation between simultaneous visitors comes from the process boundary.

`Ctrl-C` is already "Quit without saving", and with no save path it emits no effects. The child exits, the server sees it and closes the channel. No `--hosted` flag is needed. The `q` save-on-quit prompt is out of scope.

### Components

New module `src/serve/`, compiled out on wasm like `editor`. `tokio` is confined to it; the rest of the editor stays synchronous.

- **`cli::Command::Serve { listen, host_key }`** (in `src/cli.rs`, following the `parse_args_from` pattern).
  - Invoked as `dre serve [--listen <addr>] [--host-key <path>]`.
  - `listen` defaults to `0.0.0.0:2222` and `host_key` to `~/.local/share/dre/host_key`.
  - `lib::run` dispatches it to `serve::run`.
- **`HostKey`** loads the ed25519 key at the path, or generates one and writes it with mode `0600` if it is missing. It reads and writes through `filesystem`, so it is testable like `.dre` files.
- **`SshServer`** implements the russh server trait and creates one `VisitorHandler` per connection.
- **`VisitorHandler`** (one per connection). It knows:
  - the public key fingerprint offered at auth (unused by this story, needed by 176),
  - the requested PTY size in cells and pixels,
  - its `Session`, once started.

  It does:
  - `auth_publickey` accepts any offered key; no password, no `authorized_keys`;
  - `pty_request` records the size;
  - `shell_request` asks the `Spawner` for a `Session` and starts relaying;
  - `data` writes to the session;
  - `window_change_request` calls `session.resize`;
  - when the session's output ends, it closes the channel; when the connection drops, it drops the session.
- **`trait Spawner`** with `spawn(Window) -> io::Result<Box<dyn Session>>`.
- **`trait Session`** with `write(&[u8])`, `read() -> bytes` (`None` at end), `resize(Window)` and `wait()`. Both traits are `mockall::automock`ed, as `Editor`'s collaborators are.
- **`PtyProcess`** (real `Spawner`/`Session`).
  - `nix::pty::openpty`, with `ws_col`, `ws_row`, `ws_xpixel` and `ws_ypixel` set from the SSH `pty-req`, because `tty::probe` derives the cell size from the pixel fields.
  - It re-executes `std::env::current_exe()` with the slave as stdin, stdout and stderr, in a new session (`setsid` and `TIOCSCTTY`).
  - `resize` sets the winsize on the master and sends `SIGWINCH` to the child.
  - Dropping the master hangs up the child.
  - Blocking PTY reads run on `spawn_blocking`.

### Collaborations

```
ssh client ⇄ russh ⇄ VisitorHandler ⇄ Session (PtyProcess) ⇄ PTY ⇄ dre child
                          │
                          └─ Spawner creates the Session
```

The child does its own kitty-graphics probe through the PTY to the visitor's real terminal. A visitor whose terminal lacks kitty support gets the same error a local user gets.

### Testing locally

- `dre serve` with the defaults listens on `0.0.0.0:2222`, so `ssh -p 2222 localhost` works with no setup. `--listen 127.0.0.1:2222` keeps it local-only. `--host-key ./tmp/host_key` avoids touching `~/.local/share/dre`.
- A `make serve` target runs it with a throwaway host key.
- The README and CONTRIBUTING document `ssh -p 2222 -o UserKnownHostsFile=/dev/null localhost`, so a regenerated throwaway host key gives no warning.
- The local terminal must support kitty graphics.

### Tests

- `cli`: `serve` parses with and without `--listen` and `--host-key`, and applies the defaults.
- `HostKey`: generated on first use with mode `0600`; loaded unchanged on the second call.
- `VisitorHandler` with a mock `Spawner` and `Session`:
  - any public key is accepted;
  - `shell_request` spawns with the size from `pty_request`, pixels included;
  - channel data reaches `Session::write`;
  - session output reaches the channel;
  - `window_change_request` reaches `Session::resize`;
  - end of session output closes the channel;
  - two connections get two separate sessions.
- `PtyProcess` integration test: spawn a child (`cat`, or `dre`) on a PTY, write bytes and read them back, resize, and confirm hangup when dropped.
- Manual: `ssh -p 2222 localhost` from a kitty-capable terminal; build a few boxes; `Ctrl-C` closes the connection. Repeat from two terminals at once and check that the canvases are independent.

### Out of scope

- The no-key message, per-key persistence and takeover of a live connection (story 176).
- The `q` save prompt.
- Getting port 22 on `dre.sh` (deployment).
