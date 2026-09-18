# 072: Count Prefix Navigation

## User Story

I can type a number before `j`, `k`, or `h` to move that many steps at once,
instead of tapping the key over and over.

## Acceptance Criteria

- `j` on its own still moves to the next sibling — it's the same as `1j`. Same for `k` and `h`
- `3j` moves the selection 3 siblings forward; if fewer than 3 siblings remain below, it stops on the last sibling
- `3k` moves the selection 3 siblings back; if fewer than 3 remain above, it stops on the first sibling
- `3h` moves the selection up 3 levels; if there aren't 3 levels, it stops at the top
- Multi-digit counts work: `12j` moves 12 siblings forward
- A count prefix followed by any other command (e.g. `2s`) ignores the count and behaves exactly as it does today

## Technical Design