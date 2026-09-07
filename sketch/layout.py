from dataclasses import dataclass
from typing import List, Optional

from .state import Node, State

BOX_HEIGHT = 3
BORDERS_AND_CURSOR = 3


@dataclass
class Placement:
    node: Node
    x: int
    y: int
    width: int
    height: int
    cursor: Optional[int] = None


def layout(state: State, cols: int, rows: int) -> List[Placement]:
    placements = []
    for node in state.nodes:
        width = len(node.label) + BORDERS_AND_CURSOR
        placements.append(
            Placement(
                node,
                x=(cols - width) // 2,
                y=(rows - BOX_HEIGHT) // 2,
                width=width,
                height=BOX_HEIGHT,
                cursor=len(node.label) if state.mode == "insert" else None,
            )
        )
    return placements
