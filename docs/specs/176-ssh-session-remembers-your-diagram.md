# Doug comes back to his diagram

Doug types `ssh dre.sh` from his laptop. He builds a small diagram and leaves. A week later he types `ssh dre.sh` again and finds his diagram just as he left it, ready to keep working on.

## Acceptance Criteria

- A visitor who offers no SSH key sees this message and is disconnected: "dre needs an SSH key to remember your diagram. Run `ssh-keygen`, then try `ssh dre.sh` again."
- A key connecting for the first time gets an empty canvas.
- Every change Doug makes is kept straight away, so a dropped connection loses nothing.
- When Doug connects again with the same key, he sees his diagram with its boxes, labels, colours, fills and rounded corners. The selection starts fresh.
- Each SSH key has its own single diagram. Different keys never see each other's diagrams.
- If Doug connects with a key that already has a live connection, the new connection takes over and the old one is closed.

## Technical Design

### Approach

Builds on story 175 (`dre serve`, one `dre` child per visitor on a PTY). The editor already loads a diagram from a path and saves after every change that has a save path, so 176 changes nothing in the editor. The server gives each SSH key its own file and starts that visitor's child as `dre <path>`. Insert mode saves exactly as local `dre` does.

The file is `<data_dir>/<fingerprint>/diagram.dre`. The footer shows the file stem, so the visitor always sees `diagram`, however long the fingerprint is. A first-time key has no file yet: `FileStateStore::load` already returns an empty canvas that saves on the first change.

### Components

What 176 adds:

- **`DiagramDir { root }`** in `src/serve/`.
  - `for_key(fingerprint) -> io::Result<String>` creates `<root>/<fingerprint>/` with mode `0700` if it is missing and returns `<root>/<fingerprint>/diagram.dre`.
  - It goes through `filesystem`, so it is testable like `HostKey`.
  - The server creates the directory, not the editor: `filesystem::write` stays as it is, so a local `dre nope/x.dre` does not start creating directories.
- **`Sessions`**, one shared registry held by `SshServer`, mapping fingerprint to a close handle for the live connection.
  - `take_over(fingerprint)` removes any existing entry and closes it, without waiting.
  - `register(fingerprint, handle)` stores the new connection.
  - `unregister` removes an entry only if it is still the current one, so an old connection closing late cannot evict its replacement.
- **`--data-dir`** flag on `cli::Command::Serve`, defaulting to `~/.local/share/dre/diagrams` (next to the host key).

What 176 extends from 175:

- **`VisitorHandler`**:
  - `auth_publickey` stores the SHA-256 fingerprint and accepts (175 already noted it as needed here).
  - `auth_none` is rejected. Otherwise OpenSSH would be let in before offering its key.
  - `auth_keyboard_interactive` accepts with no prompts and no fingerprint. A client with no key falls back to it.
  - `shell_request` with no fingerprint writes the no-key message (one constant, `\r\n` line endings), closes the channel and never spawns.
  - `shell_request` with a fingerprint calls `Sessions::take_over`, then `DiagramDir::for_key`, then `Spawner::spawn(window, path)`. It then registers the new session and relays.
  - When the connection ends it unregisters.
- **`Spawner::spawn(Window, path)`**: `PtyProcess` runs `current_exe() <path>` instead of `current_exe()`.

### Collaborations

```
VisitorHandler ── Sessions      (take over a live connection with the same key)
               ── DiagramDir    (fingerprint → <root>/<fingerprint>/diagram.dre)
               ── Spawner       (spawn(window, path) → Session running `dre <path>`)
```

The new session starts immediately and the old one is closed in parallel. Input is relayed only to the newest connection, and the old child exits on hangup without saving, so it makes no further changes.

### Tests

- `cli`: `--data-dir` parses and defaults.
- `DiagramDir`: creates the directory with mode `0700`; the path ends in `diagram.dre`; two keys give two different directories; calling it again is fine.
- `Sessions`: `take_over` closes the previous entry; a stale `unregister` is ignored.
- `VisitorHandler` with mock `Spawner` and `Session`:
  - `auth_none` is rejected; `auth_keyboard_interactive` is accepted without a fingerprint;
  - a keyless visitor gets the exact message, the channel closes, and `spawn` is never called;
  - a key spawns with that key's path;
  - the same key connecting twice closes the first session and spawns a second;
  - different keys never close each other.
- `PtyProcess` integration: the child receives the path argument.
- Manual:
  - Connect, draw, disconnect, reconnect: the diagram is back with boxes, labels, colours, fills and rounded corners, and the selection is fresh.
  - `ssh -o IdentitiesOnly=yes -i /dev/null` shows the no-key message.
  - Two connections with the same key: the first one closes.
  - A different key sees an empty canvas.

### Out of scope

- Saving on every keystroke in Insert mode (local behaviour is kept).
- Limits on the number or size of stored diagrams.
- Deployment.
