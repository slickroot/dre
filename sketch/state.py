from dataclasses import dataclass
from typing import List, Literal, Union


@dataclass(frozen=True)
class Box:
    label: str = ""


@dataclass(frozen=True)
class Cursor:
    pass


Node = Union[Box, Cursor]

Mode = Literal["command", "insert"]


@dataclass
class State:
    nodes: List[Node]
    running: bool = True
    mode: Mode = "command"
    selected: int = -1


def handle_command(state: State, key: str) -> State:
    if key == "b":
        nodes = state.nodes + [Box("")]
        return State(nodes, state.running, "insert", len(nodes) - 1)
    if key == "q":
        return State(state.nodes, False, state.mode, state.selected)
    if key == "i":
        if not state.nodes:
            return state
        return State(state.nodes, state.running, "insert", state.selected)
    if key in ("j", "k"):
        if not state.nodes:
            return state
        step = 1 if key == "j" else -1
        selected = min(max(state.selected + step, 0), len(state.nodes) - 1)
        return State(state.nodes, state.running, state.mode, selected)
    return state


def edit(nodes: List[Node], index: int, label: str) -> List[Node]:
    return nodes[:index] + [Box(label)] + nodes[index + 1 :]


def handle_insert(state: State, key: str) -> State:
    if key == "\x1b":
        return State(state.nodes, state.running, "command", state.selected)
    label = state.nodes[state.selected].label
    if key == "\x7f":
        return State(
            edit(state.nodes, state.selected, label[:-1]),
            state.running,
            state.mode,
            state.selected,
        )
    if "\x20" <= key <= "\x7e":
        return State(
            edit(state.nodes, state.selected, label + key),
            state.running,
            state.mode,
            state.selected,
        )
    return state


def handle_key(state: State, key: str) -> State:
    if state.mode == "insert":
        return handle_insert(state, key)
    return handle_command(state, key)
