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


def layout(state: State, cols: int, rows: int) -> List[Placement]:
    placements = []
    total = len(state.nodes) * BOX_HEIGHT + (len(state.nodes) - 1) * GAP
    top = (rows - total) // 2
    for index, node in enumerate(state.nodes):
        focused = index == len(state.nodes) - 1 and state.mode == "insert"
        width = len(node.label) + BORDERS + (1 if focused else 0)
        placements.append(
            Placement(
                node,
                x=(cols - width) // 2,
                y=top + index * (BOX_HEIGHT + GAP),
                width=width,
                height=BOX_HEIGHT,
            )
        )
    if placements and state.mode == "insert":
        box = placements[-1]
        placements.append(
            Placement(
                Cursor(),
                x=box.x + 1 + len(box.node.label),
                y=box.y + 1,
                width=1,
                height=1,
            )
        )
    return placements
