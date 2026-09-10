# Insert box at selection

As a user, when I have a box selected that isn't the last one on the canvas, I want `bj`/`bl` to insert the new box right next to my selected box, so the new box appears where I'm actually working instead of at the end of the canvas.

## Acceptance Criteria

- Given a box is selected that is not the last box on the canvas, when I press `bj`, the new box is inserted directly below the selected box (spliced in at that position, not appended to the end).
- Given a box is selected that is not the last box on the canvas, when I press `bl`, the new box is inserted directly to the right of the selected box (spliced in at that position, not appended to the end).
- Boxes and connections that came after the selected box remain in place relative to the selected box after the insertion.
- After insertion, the newly created box becomes the selected box and the app enters insert mode, same as today.

## Technical Design
The flat `nodes` list is a turtle walk: each `Space`/`Arrow` separator sets the
active axis, and every following node advances one cell along it. A turtle has
exactly one slot after each box, so it cannot branch — and branching is what
this story needs. `[A, Space(right), B]` with `A` selected and `bj` pressed must
put the new box *below* `A` while `B` stays *right* of `A`, which is two
outgoing edges from one box.

We keep the turtle and borrow the L-system solution: bracket operators that
push and pop the turtle's state.

### Model

Two new node types, carrying no data:

```python
@dataclass(frozen=True)
class Push: pass    # save (row, col, active axis)
@dataclass(frozen=True)
class Pop: pass     # restore it
```

`Node` gains both. A bracketed flat list is a tree in serialised form — `Push`
and `Pop` are parentheses — so this is the nested-frame model written inline,
and it can be parsed into real frame objects later if frames ever need identity.

### `walk` — owns the turtle

Gains a stack. `Push` appends the current coordinate without advancing and
pushes `(row, col, active)`; `Pop` appends the current coordinate without
advancing and restores the triple. Both must *not* advance, so they share a
track with their neighbour.

Tracing `[A, Push, Space(down), New, Pop, Space(right), B]`: `A` at (0,0), `New`
at (2,0), `B` back at (0,2) — exactly where `B` sat before the insertion. The
existing `Space(right)` is never touched and still describes A→B.

One coordinate per node is still returned, so `layout` keeps zipping the
parallel lists and `cursor` keeps indexing `placements[state.selected]`. No
filtering, no re-indexing.

### `width` / `height`

`width` already returns `0` for the new types via its fallthrough. `height`
needs a branch so `Push`/`Pop` return `0` instead of claiming `GAP_HEIGHT`.
At zero extent they are absorbed by `tracks`'s `max()`.

Neither renderer changes: both dispatch on `isinstance` and ignore node types
they don't know.

### `outgoing(nodes, i)` — new helper in `state`

Reports the directions already emanating from the box at `i`. Scanning forward
from `i + 1`: a `Push` opens a bracketed branch whose direction is the
separator just inside it, then skip to the matching `Pop`; a bare separator is
the unbracketed tail branch; a `Pop` or the end of the list means no more.
Returns `(direction, separator_index)` pairs.

Since only two directions exist, a box has at most two outgoing branches, so
at most one bracket group per box — the group is always the first branch and
the tail is always the second.

### Insertion — `handle_command`, `b` prefix

With direction `d` and `selected` resolved to a real index first (`-1` works
for `nodes[-1]` but not for splice arithmetic):

1. **`d` already emanates from the box.** Splice `[Space(d), Box(PAD)]` at that
   branch's separator index, pushing the existing chain along.
   `[A, Push, Space(down), C, Pop, Space(right), B]` + `bj` on `A` →
   `[A, Push, Space(down), New, Space(down), C, Pop, Space(right), B]`: `New` at
   (2,0), `C` pushed to (4,0), `B` still at (0,2).
   **Guard:** if the separator at that index is an `Arrow`, the keystroke is a
   no-op — clear `pending`, stay in command mode, create no box, leave the
   selection alone. An arrow is a connection the user deliberately drew, and
   whether splitting it should move it, keep it, or duplicate it onto both
   halves is a product question this story does not answer.
2. **`d` is new and the box has no other outgoing branch.** Plain splice at
   `selected + 1`. This is today's append behaviour, and it still holds when
   `Pop`s follow, since they remain after the new box.
3. **`d` is new and the box already has another branch.** Bracketed splice
   `[Push, Space(d), Box(PAD), Pop]` at `selected + 1`. The existing tail branch
   stays unbracketed.

In all three cases the new box becomes `selected` and the mode becomes
`insert`, as today.

Cases 2 and 3 never inspect an existing separator, and case 1 inspects one only
to decide whether to proceed — so the emitted separator is always a plain
`Space`.

### Unchanged

`move` already skips non-`Box` nodes, so `j`/`k` step over `Push`/`Pop` and
document order stays preorder. `edit`, `i`, `c`, `f`, `Box`, `Space`, `Arrow`,
`render`, `writer`, and `kitty` are untouched.

### Out of scope

- **`a` cannot reach a newly branched box.** Adjacency is
  `abs(selected - source) == 2`, which brackets defeat: in
  `[A, Push, Space(down), New, Pop, Space(right), B]` the boxes sit at 0, 3 and
  6, so `a` refuses A↔New and A↔B even though both are spatially adjacent. The
  fix is to skip a bracket group when the target is past it and descend into it
  when the target is inside, but it is deferred. Deferring is safe rather than
  merely postponed: no reachable arrangement puts a `Push`/`Pop` at the
  midpoint slot, so `a` silently refuses instead of crashing on `axis(Push)`.
- **No collision resolver.** The three insertion rules never overlap two boxes,
  but contrived hand-built lists still can. Out of scope.
- **No convergence.** Two boxes cannot point at a third. The current model
  cannot express this either, so it is not a regression.
- **No user-facing frames.** Brackets are internal only.
