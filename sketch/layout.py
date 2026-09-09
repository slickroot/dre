from dataclasses import dataclass
from typing import List

from .state import Arrow, Box, Cursor, Node, State

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
    return 0


def layout(state: State, cols: int, rows: int) -> List[Placement]:
    placements = []
    total = sum(height(node) for node in state.nodes)
    y = (rows - total) // 2
    for index, node in enumerate(state.nodes):
        editing = index == state.selected and state.mode == "insert"
        node_width = width(node, editing)
        placements.append(
            Placement(
                node,
                x=(cols - node_width) // 2,
                y=y,
                width=node_width,
                height=height(node),
            )
        )
        y += height(node)
    if state.selected >= 0 and isinstance(state.nodes[state.selected], Box):
        box = placements[state.selected]
        label = box.node.label
        if state.mode == "insert":
            x = box.x + 1 + len(label)
        else:
            x = box.x + max(len(label), 1)
        placements.append(Placement(Cursor(), x=x, y=box.y + 1, width=1, height=1))
    return placements
