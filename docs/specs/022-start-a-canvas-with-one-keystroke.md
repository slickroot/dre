# 022 - Start a canvas with one keystroke

## Story

Bob opens Dre to an empty canvas. He presses `b` and a box appears, cursor
already inside it, so he can start typing his first label without a second
thought. Once that box exists, `b` goes back to asking him which way.

## Acceptance Criteria

- `b` on an empty canvas creates a box, with no direction key needed.
- That box is selected and Dre is in insert mode, so typing goes straight into its label.
- `b` on a canvas that already has a box waits for a direction, as it does today.

## Technical Design

`b` is the only key in `handle_command` that arms `pending`, and the only path
that creates a box. This spec splits the second job off the first: on an empty
canvas the direction is not a question worth asking, because there is nothing
to place the new box relative to. `bj` and `bl` already agree on the answer
there — both append `[Box(PAD)]` and nothing else. The keystroke that follows
`b` only ever discarded a choice that was never open.

### `handle_command` — `b` answers for itself

The emptiness check goes inside the `key == "b"` clause. That is the pattern
every other key already follows: `h`/`l`, `i`, `j`, `k`, `c` and `f` each open
with `if not state.nodes: return state`, and `a` does the same shape with
`if state.selected < 0`. The canvas check is the key's own business, decided
where the key is handled. `b` inverts the guard rather than adding a new kind
of control flow:

```python
if key == "b":
    if not state.nodes:
        return replace(state, nodes=[Box(PAD)], mode="insert", selected=0)
    return replace(state, pending="b")
```

The clause stays where it sits today. Nothing downstream watches `pending` and
fires on its own — the reducer never acts without a key arriving, and giving
`pending` the power to resolve itself would be the first exception to that.

The write names three fields, not five. `pending` is already `""`: this clause
is only reached past the `state.pending == "b"` block at the top, so writing
`pending=""` here would restate the condition we arrived under. `source` is
already `-1`: only `a` sets it, and `a` refuses unless `selected >= 0`, which
needs a box. On an empty canvas both fields are at the value the assignment
would give them, and `replace` says only what changes. `selected=0` is a
constant, not `len(nodes) - 1`, because the list being built has exactly one
element and the index is known at the time it is written.

### The pending branch loses its conditional

```python
nodes = state.nodes + (
    [Space(direction=direction), Box(PAD)] if state.nodes else [Box(PAD)]
)
```

becomes

```python
nodes = state.nodes + [Space(direction=direction), Box(PAD)]
```

That conditional existed for exactly one case — `bj` or `bl` on an empty canvas
— and this spec is that case moving elsewhere. `pending` can no longer be armed
on an empty canvas, so `state.nodes` is always truthy by the time the branch
runs and the `else` arm is unreachable. Keeping it as defensive cover for a
`pending` some future key might arm differently would leave a branch no test
can reach and no reader can trigger. It goes.

The rest of the branch is untouched: the direction lookup, the five-field
`replace`, and the `return replace(state, pending="")` that swallows any other
key after `b`.

### Two creation sites, no shared helper

There are now two places that build a `Box(PAD)` and enter insert mode. They
are not the same statement twice. Site 1 appends a separator and a box to an
existing chain and computes `selected` from the resulting length; site 2 builds
a one-element list and knows the index is `0`. They share a shape and three
field names, and disagree about every value in them. A helper taking `nodes`
and writing the common fields would have to be told the index anyway, leaving
it a rename of `replace` with a shorter argument list.

### Typing the direction key

A consequence the acceptance criteria leave implicit: on an empty canvas `b`
enters insert mode immediately, so the next keystroke reaches `handle_insert`,
not `handle_command`. `bj` on a fresh canvas gives `Box("j ")` — the `j` is the
first character of the label, not a direction. That is the story working as
written: the cursor is already inside the box and typing goes straight into it.
It is left unasserted. It follows from two rules the suite already pins — `b`
creates and enters insert mode, and insert mode types printable keys — and a
test for it would be a test of `handle_insert` wearing a `b` costume.

### Tests

The empty-canvas cases in `HandleKeyTest` currently drive the module-level
`bj(state, key1="b", key2="j")` helper with `State([])`. Two keystrokes no
longer describe what they are checking, and their expectations are wrong under
the new rule — they would see `[Box("j ")]`. They become single-keystroke `b`
tests:

- `b` on an empty canvas appends only a box: `handle_key(State([]), "b").nodes
  == [Box(PAD)]`.
- `b` on an empty canvas enters insert mode.
- `b` on an empty canvas selects the new box — `selected == 0`.
- `b` on an empty canvas keeps the state running, and preserves a stopped one.
- `b` does not mutate the given state.

The `bj` helper stays for the non-empty cases it still honestly describes —
`test_bj_appends_to_existing_nodes`,
`test_bj_separates_the_new_box_from_the_last_one_with_a_space`,
`test_bj_selects_the_new_last_box`,
`test_bj_selects_the_new_box_when_the_selection_was_not_last`,
`test_bj_clears_source` — all of which already start from a canvas with a box.

In `PendingCommandTest`, `test_b_sets_pending_to_b` uses `State([])` and now
asserts the opposite of the truth. It moves to a non-empty canvas, joining the
four `test_b_does_not_*` cases beside it, which already use `State([Box("a")])`
and are unaffected. `test_b_then_i_does_not_enter_insert_mode` also starts from
`State([])`; on an empty canvas `b` now enters insert mode and `i` types the
letter, so it moves to a non-empty canvas too, where it still pins what it
means — `b` then an unrecognised key leaves the mode alone.

Two cases are added for the third acceptance criterion, that a canvas with a
box still waits:

- `b` on a non-empty canvas adds no node and sets `pending` to `"b"` (the
  moved case, now stated where it belongs).
- `bj` on a canvas with one box still appends `[Space("down"), Box(PAD)]` —
  the collapsed conditional changed no behaviour on the path that survives.

### Unchanged

`i`, `c`, `f`, `a`, `j`, `k`, `h`, `l`, `q`, `edit`, `handle_insert`,
`handle_key`, and every node type. `State`'s fields keep their defaults.
Nothing outside `state.py` reads `mode` or `pending` — `writer.py` only calls
`handle_key`, and `render.py`, `layout.py` and `kitty.py` never see either — so
no renderer or main-loop change is needed.

### Relationship to 019

Independent. 019 collapses `beside` and `below` into a direction-taking `move`
and regroups the `h`/`j`/`k`/`l` clauses; this spec touches only the `b` clause
and the `pending == "b"` branch, and no helper is shared between them. Either
can land first. This design is written against `state.py` as it stands today,
before 019.

### Out of scope

- **No direction shortcut on a non-empty canvas.** `b` still waits for `j` or
  `l` the moment there is one box. The third acceptance criterion says so.
- **No first-box direction.** The `Space` before the first box does not exist
  and is not invented; the empty-canvas path appends a bare `Box`, exactly as
  `bj` did.
- **`pending` stays a bare string.** One key arms it and one clause reads it.
  A real prefix table waits for a second prefix.
