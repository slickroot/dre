# 064 - Install dre with a one-line command

## Story

Bob has an Apple Silicon Mac. He runs
`curl -fsSL https://raw.githubusercontent.com/slickroot/dre/main/install.sh | bash`,
and `dre` is installed and ready to run from his terminal.

## Acceptance Criteria

- Running
  `curl -fsSL https://raw.githubusercontent.com/slickroot/dre/main/install.sh | bash`
  on an Apple Silicon Mac installs `dre` to `~/.local/bin/dre`, with no `sudo`.
- It installs the newest release, pre-releases included (today that's
  `v0.1.0-dev`).
- When the install works, the script prints a confirmation, e.g.
  `dre v0.1.0-dev installed to ~/.local/bin/dre`.
- If `~/.local/bin` isn't on Bob's PATH, the script says so and prints the
  exact line to add to his shell config.
- After that, typing `dre` in his terminal runs it.

## Technical Design
