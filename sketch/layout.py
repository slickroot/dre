from dataclasses import dataclass
from typing import List

from .state import Arrow, Box, Cursor, Node, Space, State

BOX_HEIGHT = 3
GAP_HEIGHT = 2
BORDERS = 2


@dataclass
class Placement:
    node: Node
    x: int
    y: int
    width: int
    height: int


def interior(label: str, editing: bool) -> int:
    if editing:
        return len(label) + 1
    return max(len(label), 1)


def height(node: Node) -> int:
    if isinstance(node, Box):
        return BOX_HEIGHT
    return GAP_HEIGHT


def width(node: Node, editing: bool) -> int:
    if isinstance(node, Box):
        return interior(node.label, editing) + BORDERS
    if isinstance(node, Arrow):
        return 1
    if isinstance(node, Space) and node.direction == "right":
        return 4
    return 0


def layout(state: State, cols: int, rows: int) -> List[Placement]:
    if not state.nodes:
        return []

    node_widths = [
        width(node, index == state.selected and state.mode == "insert")
        for index, node in enumerate(state.nodes)
    ]

    # Walk the flat node list, advancing a (row, col) cursor. Every node past
    # the first moves one step along the "active axis" before being placed.
    # A Space switches the active axis (right -> col, down -> row) before
    # that node's own step is taken; a plain node leaves the axis as-is.
    # This makes every node land in its own cell, exactly like the old
    # single-column flow layout did for a pure vertical stack.
    node_cells = [(0, 0)]
    row, col = 0, 0
    axis = "row"
    for node in state.nodes[1:]:
        if isinstance(node, Space):
            axis = "col" if node.direction == "right" else "row"
        if axis == "row":
            row += 1
        else:
            col += 1
        node_cells.append((row, col))

    n_rows = max(r for r, c in node_cells) + 1
    n_cols = max(c for r, c in node_cells) + 1

    col_widths = [0] * n_cols
    row_heights = [0] * n_rows
    for index, node in enumerate(state.nodes):
        r, c = node_cells[index]
        col_widths[c] = max(col_widths[c], node_widths[index])
        row_heights[r] = max(row_heights[r], height(node))

    col_offsets = [0] * n_cols
    for c in range(1, n_cols):
        col_offsets[c] = col_offsets[c - 1] + col_widths[c - 1]
    row_offsets = [0] * n_rows
    for r in range(1, n_rows):
        row_offsets[r] = row_offsets[r - 1] + row_heights[r - 1]

    total_width = sum(col_widths)
    total_height = sum(row_heights)
    x0 = (cols - total_width) // 2
    y0 = (rows - total_height) // 2

    # A pure vertical stack (n_cols == 1) never actually needs cross-row
    # column alignment, since each row holds exactly one node. Centering
    # such a node inside a column sized to the *widest* node in the stack
    # (as the general grid formula would) stops narrower/wider boxes from
    # each being centered on their own width, which existing behaviour
    # relies on. Fall back to per-node centering in that case; it is
    # mathematically identical to the general formula whenever every node
    # in the column shares the same width, and preserves old behaviour when
    # they don't.
    single_column = n_cols == 1

    placements = []
    for index, node in enumerate(state.nodes):
        r, c = node_cells[index]
        node_width = node_widths[index]
        node_height = height(node)
        if single_column:
            x = (cols - node_width) // 2
        else:
            x = x0 + col_offsets[c] + (col_widths[c] - node_width) // 2
        y = y0 + row_offsets[r] + (row_heights[r] - node_height) // 2
        placements.append(
            Placement(node, x=x, y=y, width=node_width, height=node_height)
        )
    if state.selected >= 0 and isinstance(state.nodes[state.selected], Box):
        box = placements[state.selected]
        label = box.node.label
        if state.mode == "insert":
            x = box.x + 1 + len(label)
        else:
            x = box.x + max(len(label), 1)
        placements.append(Placement(Cursor(), x=x, y=box.y + 1, width=1, height=1))
    return placements
