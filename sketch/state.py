from dataclasses import dataclass
from typing import List, Union


@dataclass(frozen=True)
class Box:
    pass


Node = Union[Box]


@dataclass
class State:
    nodes: List[Node]


def handle_key(state: State, key: str) -> State:
    if key == "b":
        return State(state.nodes + [Box()])
    return state
