from dataclasses import dataclass
from typing import List

from .state import Cursor, Node, State

BOX_HEIGHT = 3
BORDERS = 2
GAP = 1


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


def layout(state: State, cols: int, rows: int) -> List[Placement]:
    placements = []
    total = len(state.nodes) * BOX_HEIGHT + (len(state.nodes) - 1) * GAP
    top = (rows - total) // 2
    for index, node in enumerate(state.nodes):
        editing = index == state.selected and state.mode == "insert"
        width = interior(node.label, editing) + BORDERS
        placements.append(
            Placement(
                node,
                x=(cols - width) // 2,
                y=top + index * (BOX_HEIGHT + GAP),
                width=width,
                height=BOX_HEIGHT,
            )
        )
    if state.selected >= 0:
        box = placements[state.selected]
        label = box.node.label
        if state.mode == "insert":
            x = box.x + 1 + len(label)
        else:
            x = box.x + max(len(label), 1)
        placements.append(Placement(Cursor(), x=x, y=box.y + 1, width=1, height=1))
    return placements
