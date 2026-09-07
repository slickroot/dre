from dataclasses import dataclass
from typing import List, Literal, Union


@dataclass(frozen=True)
class Box:
    label: str = ""


Node = Union[Box]

Mode = Literal["command", "insert"]


@dataclass
class State:
    nodes: List[Node]
    running: bool = True
    mode: Mode = "command"


def handle_command(state: State, key: str) -> State:
    if key == "b":
        return State(state.nodes + [Box("")], state.running, "insert")
    if key == "q":
        return State(state.nodes, False, state.mode)
    return state


def relabel_last(nodes: List[Node], label: str) -> List[Node]:
    return nodes[:-1] + [Box(label)]


def handle_insert(state: State, key: str) -> State:
    if key == "\x1b":
        return State(state.nodes, state.running, "command")
    label = state.nodes[-1].label
    if key == "\x7f":
        return State(relabel_last(state.nodes, label[:-1]), state.running, state.mode)
    if "\x20" <= key <= "\x7e":
        return State(relabel_last(state.nodes, label + key), state.running, state.mode)
    return state


def handle_key(state: State, key: str) -> State:
    if state.mode == "insert":
        return handle_insert(state, key)
    return handle_command(state, key)
