# 033 - Pack siblings into the next free row

## Story

Bob has three boxes hanging off one parent, and gives the middle one two
children of its own. Today that shoves the outer two siblings far apart, leaving
a big empty margin above and below. Instead, the first and third child slide in
as close as they can get to the middle child without bumping into its children,
so the picture stays tight and no row is left empty.

## Acceptance Criteria

- Each gap between two adjacent siblings is sized on its own, from just those
  two neighbours' subtrees — one tall child no longer pushes all of its siblings
  apart.
- Siblings sit as close together as they can without any two boxes colliding.
- The vertical gap between boxes is never less than 3 rows, anywhere boxes are
  stacked.
- With an odd number of children, the parent sits level with the middle child,
  whatever the gaps around it look like.
- With an even number of children, the parent sits one normal half-gap below the
  first of the two middle children.
- A parent with plain children looks exactly as it does today.

## Technical Design
Rows are counted in half-slots, as they are today: a half-slot is `HALF_PITCH`
rows, so two boxes stacked in a column sit 2 half-slots apart and no closer.

Today `measure` picks one `pitch` for a whole row of siblings — the widest gap
any adjacent pair needs, rounded up to even — and `cells` re-derives every child
row from it. That shared stride is what leaves the empty margin: one tall child
sets the spacing for all of them.

## Start at the leaves

The contour bookkeeping was a roundabout way of talking about leaves. The
topmost row in a subtree belongs to a leaf, as does the bottommost, so
`below(a) + above(b) + 2` only ever meant *a's last leaf and b's first leaf must
be 2 apart*. Say that directly and there is nothing left to measure:

```
leaves, in order, take the next free row — always 2 apart
every parent is then centred on its own children
```

One counter runs through the entire forest in depth-first order, so it never
learns who a leaf's parent was. Siblings, cousins and separate roots are all
spaced by the same rule.

Nothing collides. Only nodes in the same column can, and two distinct nodes at
the same depth always have disjoint subtrees. Disjoint subtrees take disjoint
runs of leaves, and consecutive runs are 2 apart because the counter put them
there. Every node sits inside its own subtree's span — a parent lands between
its first and last child, inductively within the leaves beneath it — so two
nodes in one column are never closer than their leaf runs.

## Assigning rows

A plain recursion replaces the measure-then-place pair, taking the next free row
in and giving it back out. It builds `Celled` straight from `Box`:

```python
def assign(box, column, path, free) -> Tuple[Celled, int]:
    if not box.children:
        return Celled(box, width(box), column, free, path), free + 2
    children = []
    for index, child in enumerate(box.children):
        node, free = assign(child, column + 1, path + (index,), free)
        children.append(node)
    row = anchor(children)
    return Celled(box, width(box), column, row, path, tuple(children)), free
```

`anchor` is the centring rule, read off the children's rows rather than computed
from a stride. With an odd number of children it is the middle child's row; with
an even number it is the first of the two middle children plus 1 — one half-slot
below, which is half a normal gap.

`forest` threads the counter across the roots and returns the trees. Because the
counter starts at 0 and only ever moves down, every row is non-negative from the
start: the normalising pass that subtracted the minimum row goes away.

## What this removes

`assign` produces `Celled` directly, so the middle layer is orphaned. Delete
`Measured`, `measure`, `fold_up` and `push_down` — `forest` was the only caller
of the last two. `fmap` and `flatten` stay; `layout` still uses both. The tests
that exercise the deleted functions go with them.

Everything downstream is untouched. `position`, `total_height`, `emit` and the
arrow's `stops`/`shaft` all read rows off `Celled` and never knew about `pitch`.
The even-child parent still lands between its children's stops, so the arrow
sprite is unaffected.

Plain trees do not move. Three leaf children give rows 0, 2, 4 with the parent
at 2, and two leaf roots give 0 and 2 — the same as a `pitch` of 2. The story's
case goes from rows 0, 4, 8 to 0, 3, 6.

## Follow-up

`layout.py`'s vocabulary — `cells`, `tracks`, `span`, and now the leftovers of
the half-slot scheme — deserves a renaming pass of its own once this lands. Out
of scope here.
