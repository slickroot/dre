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
