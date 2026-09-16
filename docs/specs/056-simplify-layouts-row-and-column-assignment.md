# Simplify layout's row and column assignment

## User Story

As a maintainer, I want `layout.rs` rewritten in Rust-native terms instead of carrying `layout.py`'s shape forward, so that opening the file explains the layout algorithm the way `state.rs`'s mode split explains command handling, and so that a frame's placements borrow from `state.doc.boxes` instead of deep-cloning it every keystroke.

## Acceptance Criteria

- `Celled` and `Positioned`, and the functions that build and flatten them (`forest`, `walk`, `assign`, `tracks`, `span`, `column_tracks`, `position`, `emit`, `emit_tree`), are deleted. The algorithm recurses over the existing `Node` tree directly, twice, instead of building intermediate tree types.
- `measure_columns(nodes: &[Node]) -> Vec<i64>` replaces `forest`/`walk`/`tracks`/`column_tracks`: one recursive pass over `nodes` that tracks column (tree depth) as it descends and accumulates each column's max width — including a reserved gap-width slot for any node with children — into a flat `Vec<i64>`, one entry per column.
- `place(nodes: &[Node], offsets: &[i64]) -> Vec<Placement>` replaces `position`/`emit`/`emit_tree`: one recursive pass over `nodes` that recurses into children first, assigns this node's row as the median of its children's rows (the existing `anchor` rule, folded into `place` rather than living as a separate function over a separate type), resolves `x` from `offsets[col]` and `y` from `row * HALF_PITCH`, and emits this node's box/label/arrow placements directly — using `node.children` for arrow targets rather than looking children up by path.
- `layout(nodes: &[Node], cols: i64, rows: i64) -> Vec<Placement>` converts column widths to offsets (a prefix sum over `measure_columns`'s result) and calls `place`.
- All layout-internal vocabulary is renamed from `box`/`boxes` to `node`/`nodes`: function parameters, the `Node` field inside `Celled`/`Positioned` (now gone), and `PlacementNode::Node`'s payload binding.
- `layout`, `measure_columns`, `place`, `Placement`, `PlacementNode`, and `Label` carry a lifetime `'a`. `PlacementNode::Node` holds `&'a Node`; `Label.text` holds `&'a str` (borrowed from `node.label`), both borrowed from the input slice rather than cloned. `with_cursor` also carries `'a` since it takes and returns `Vec<Placement<'a>>`.
- `layout`'s signature changes from `fn layout(boxes: Vec<Node>, ...)` (owned) to `fn layout<'a>(nodes: &'a [Node], ...)` (borrowed). `writer.rs:113`'s `state.doc.boxes.clone()` is removed; `frame` passes `&state.doc.boxes` directly.
- `row`, `col`, and every path element (`Label.path`, the path built during `place`'s recursion) are `usize`, not `i64` — they are indices/counts, never negative, and are already cast `as usize` for array indexing today. `x`, `y`, `width`, `height`, column `offset`/`extent`, and arrow `stops`/`shaft` stay `i64`, since they participate in arithmetic that can go negative (centering math, off-screen placement when content is wider than the terminal, relative arrow stops).
- `Label.path` becomes `Vec<usize>`, matching `State.selected: Vec<usize>` directly. `writer.rs`'s `state.doc.selected.iter().map(|&i| i as i64).collect()` cast is deleted; `frame` passes `state.doc.selected.clone()` to `with_cursor` unchanged in type.
- The file stays a single `layout.rs` (no `layout/` folder) — organized top to bottom as: constants, measurement helpers (`interior`, `width`, `height`, `centre`), `measure_columns`, `place`, the output types (`Label`, `Arrow`, `Cursor`, `PlacementNode`, `Placement`), `layout`, `with_cursor`.
- Behaviour is unchanged: every existing placement, pixel-for-pixel, for every case the current test suite covers.
- The test suite is rewritten against `measure_columns`/`place` in place of the deleted `Celled`/`Positioned`-era tests (`tracks_*`, `span_*`, `anchor_*`, `assign_*`, `forest_*`, `walk_*`, `column_tracks_*`, `position_*`, `emit_*`), preserving the same coverage: column widths and gap reservation, the median-of-children row rule, box/label/arrow emission and ordering, and `with_cursor` behaviour. No assertion is weakened or deleted, only re-targeted at the new functions.

## Technical Design

### Why this file, and not another split like spec 055

Spec 055 split `state.rs` because it had grown to hold two unrelated vocabularies (command mode's verbs and insert mode's verbs) crammed into one file. `layout.rs` doesn't have that problem — it does one job, laying out a diagram — but it's still hard to read, for a different reason: it's `layout.py` carried over with Rust syntax, not Rust structure. Two Python habits survived the port and are what make the file feel opaque rather than self-explanatory:

1. **A new tree type per pipeline stage.** Python passes objects by reference for free, so `layout.py` could freely build `Celled` (row/col added) and then `Positioned` (x/y added) as fresh trees without a cost model forcing a second thought. In Rust, each of those is a full recursive rebuild — a real allocation and copy — of a shape that already existed as `Node`. Three tree *types* (`Node`, `Celled`, `Positioned`) exist to carry the same conceptual tree through three stages of the same computation.
2. **`.clone()` as the default way to share data.** Every stage clones the box/node it's wrapping (`box_: box_.clone()` in both `Celled` and `Positioned`, `here.box_.clone()` in `emit`), and the caller in `writer.rs` clones the entire `state.doc.boxes` tree before handing it to `layout` at all. Since `Node.children: Vec<Node>`, cloning a `Node` clones its whole subtree — this happens on every keystroke, for every node, at every stage, even though `render.rs` never reads a placement's `.children`.

Neither habit is a Rust idiom; both are exactly what you'd write if you mentally modelled `Node` the way Python modelled `Box` — as a handle you copy around cheaply. This spec removes both: one tree type, walked twice instead of rebuilt twice, and no cloning where a borrow will do.

### The algorithm: two passes over one tree, not three tree types

The design question this spec had to answer wasn't "how do we split the file" but "does the two-tree-type structure encode something real about the layout problem, or is it accidental." It turned out to be a mix of both.

**Row and column assignment don't need a new tree.** A node's column is simply its depth in the tree — free information during any recursive walk, no bookkeeping required. A node's row follows one rule: a leaf claims the next free row (rows 0, 2, 4, … via `LEAF_STRIDE`, left to right across the whole forest); a parent's row is the *median* of its children's rows, computed after they're placed (the existing `anchor` rule). Both facts are pure properties of the `Node` tree's shape — they don't need widths, and they don't need a new tree type to be computed, because the recursion that computes them can walk `Node` directly and return `(row)` to its caller as it unwinds, exactly as `assign` does today over `Celled`.

**Sizing does need a whole-tree view, but not a whole-tree copy.** A column's pixel width is the widest node in that column across the *entire* forest — including nodes in branches visited later than the one currently being examined. That's a genuine ordering constraint: a node's `x` position can't be finalized until every node that might share its column has been seen. But "seen" doesn't require holding onto the node — it only requires remembering one number, its width, indexed by column. So this pass doesn't need a `Celled` tree either; it needs a `Vec<i64>`, one slot per column, updated in place as the walk visits each node (`widths[col] = max(widths[col], width(node))`), plus a reserved slot after any column that has children (today's gap-track doubling, `2 * col` / `2 * col + 1`).

That leaves exactly two passes, both over the same `Node` tree, connected by one flat array:

```rust
fn measure_columns(nodes: &[Node]) -> Vec<i64> {
    let mut widths = Vec::new();
    fn visit(node: &Node, col: usize, widths: &mut Vec<i64>) {
        if widths.len() <= col {
            widths.resize(col + 1, 0);
        }
        widths[col] = widths[col].max(width(node));
        if !node.children.is_empty() {
            let gap_col = col + 1;
            if widths.len() <= gap_col {
                widths.resize(gap_col + 1, 0);
            }
            widths[gap_col] = widths[gap_col].max(GAP_WIDTH);
            for child in &node.children {
                visit(child, gap_col + 1, widths);
            }
        }
    }
    for node in nodes {
        visit(node, 0, &mut widths);
    }
    widths
}
```

```rust
fn place<'a>(nodes: &'a [Node], offsets: &[i64], free: usize) -> (Vec<Placement<'a>>, usize) {
    // recurses depth-first; for a leaf, row = free, free += LEAF_STRIDE;
    // for a parent, recurse into children first, then row = median(children's rows);
    // emits this node's box/label placements, and (if it has children) an arrow
    // placement built directly from node.children, no path lookup needed.
}
```

The second pass takes `offsets`, not `widths` — turning the flat width array into offsets is a prefix sum, `O(columns)`, no tree walk:

```rust
fn layout<'a>(nodes: &'a [Node], cols: i64, rows: i64) -> Vec<Placement<'a>> {
    let widths = measure_columns(nodes);
    let mut offsets = Vec::with_capacity(widths.len());
    let mut offset = 0;
    for w in &widths {
        offsets.push(offset);
        offset += w;
    }
    let (placements, _) = place(nodes, &offsets, 0);
    placements
}
```

This is why two passes are necessary and not a leftover of the old design: an early node's `x` depends on a later node's width, so nothing can hand back a final position before the whole tree has been measured. What's not necessary is what the old code did with that constraint — rebuilding the entire tree twice to carry the answer. One array is enough.

### Ownership: borrow the tree, don't clone it

`render.rs` reads four scalar fields off a placement's node (`label`, `colour`, `fill`, `rounded`) and never its `children`. Every `.clone()` in the current pipeline — `writer.rs`'s `state.doc.boxes.clone()`, `Celled`/`Positioned`'s `box_: box_.clone()`, `emit`'s `here.box_.clone()` — exists solely to get those four fields to `render.rs`, and each one clones the full subtree beneath the node because `Node.children: Vec<Node>`. On a deep diagram this is `O(depth)` redundant subtree copies, once per frame, i.e. once per keystroke.

Since `layout`'s output (`Vec<Placement>`) is consumed immediately within the same `frame` call that owns `state`, a borrow is sound: `PlacementNode::Node(&'a Node)` and `Label.text: &'a str` (borrowed from `node.label`) instead of owned clones, with `'a` tying every `Placement` back to the `&'a [Node]` slice `layout` was called with. `layout`'s signature changes to take `&'a [Node]` instead of owning `Vec<Node>`, so `writer.rs` drops its clone and calls `layout(&state.doc.boxes, cols, rows)` directly. `path: Vec<usize>` stays owned on `Label` regardless — a path is freshly built by appending an index at each recursion level, not borrowed from anywhere in `Node`.

The `'a` threads through every type and function that mentions `Placement`/`PlacementNode`/`Label`, in `layout.rs` and in `render.rs`. That's mechanical — the compiler enforces it can't be forgotten anywhere — and it buys a correctness property beyond the performance win: a `Placement` can't outlive the `State` it was computed from, so there's no way to accidentally render a stale layout against a tree that's since changed.

### Types: usize for indices, i64 for signed arithmetic

`layout.rs`'s numeric fields were all `i64` because spec 051 matched the `i64` convention PyO3 needed for round-tripping Python ints through `dre_rs` pyclasses. Spec 053 removed Python packaging and PyO3 entirely; nothing in the crate uses `pyclass` today. That reason is gone, and its absence already shows: `state.rs` moved `State.selected` to `Vec<usize>`, but `Label.path` stayed `Vec<i64>`, so `writer.rs` casts on every frame (`state.doc.selected.iter().map(|&i| i as i64).collect()`) to bridge the two.

Splitting by what a number means, not by inherited habit:

- **Indices and counts — `row`, `col`, path elements — become `usize`.** They're never negative, they're used to index arrays (`offsets[col]`), and they're already cast `as usize` at every use site today. `Label.path` becomes `Vec<usize>`, matching `State.selected` directly, and the per-frame cast in `writer.rs` is deleted.
- **Signed pixel arithmetic — `x`, `y`, `width`, `height`, column `offset`/`extent`, arrow `stops`/`shaft` — stays `i64`.** These can legitimately go negative: `left`/`top` in `layout` (`(cols - span) / 2`) go negative when the diagram is wider than the terminal, and arrow stops are relative offsets (`child.y - origin`) that are negative for any child above the arrow's origin. Keeping these signed avoids introducing a third numeric type (`i32`) purely to look smaller; `i64` is already the width `terminal_size`'s `libc::winsize` values are converted to elsewhere in `writer.rs`.

### Out of scope

Per-mode state, the text cursor moving into `Mode::Insert`, deleting `PAD`, and making `with_cursor` conditional are **spec 057** (renumbered from spec 056 to make room for this spec). This spec's `with_cursor` keeps its current unconditional behaviour and signature shape, only gaining the `'a` lifetime and `selected: Vec<usize>` (dropping the now-pointless `as i64` cast) to match the rest of the file.

`layout::height`'s `#[allow(dead_code)]` stub and the unused `GAP_HEIGHT`/`ROW_PITCH` constants are untouched — they exist for the multi-line-label story spec 055 flagged as a later constraint on 057, and this spec doesn't touch label content, only row/column assignment and ownership.
