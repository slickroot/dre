from dataclasses import dataclass
from typing import List

from .state import Node, State

BOX_SIZE = 3


@dataclass
class Placement:
    node: Node
    x: int
    y: int
    width: int
    height: int


def layout(state: State, cols: int, rows: int) -> List[Placement]:
    return [
        Placement(
            node,
            x=(cols - BOX_SIZE) // 2,
            y=(rows - BOX_SIZE) // 2,
            width=BOX_SIZE,
            height=BOX_SIZE,
        )
        for node in state.nodes
    ]
