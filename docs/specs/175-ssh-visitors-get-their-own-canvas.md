# Doug tries dre over SSH

Doug types `ssh dre.sh`. dre opens for him with an empty canvas, as if he'd run `dre` on his own machine. He builds a few boxes with the usual keys, presses `Ctrl-C` and the connection closes. Pleased, he tells a friend to try it.

## Acceptance Criteria

- Anyone can run `ssh dre.sh` and get in, with no password or sign-up, as long as they offer an SSH key.
- On connecting, they see an empty dre canvas.
- The usual dre keys work for building boxes.
- `Ctrl-C` closes the connection.
- Two people connected at the same time each get their own canvas, and neither sees or affects the other's boxes.

## Technical Design
