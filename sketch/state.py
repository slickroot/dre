from dataclasses import dataclass, replace
from typing import List, Literal, Union

PLAIN = -1
CYCLE = 9
PALETTE_SIZE = 5
PAD = " "


@dataclass(frozen=True)
class Box:
    label: str = ""
    colour: int = PLAIN
    fill: int = PLAIN


SpaceDirection = Literal["down", "right"]


@dataclass(frozen=True)
class Space:
    direction: SpaceDirection = "down"


@dataclass(frozen=True)
class Cursor:
    pass


Direction = Literal["up", "down", "left", "right"]

HORIZONTAL = ("left", "right")


@dataclass(frozen=True)
class Arrow:
    direction: Direction = "down"


def axis(node: Union[Space, Arrow]) -> Literal["row", "col"]:
    return "col" if node.direction in HORIZONTAL else "row"


Node = Union[Box, Space, Arrow, Cursor]

Mode = Literal["command", "insert"]


@dataclass
class State:
    nodes: List[Node]
    running: bool = True
    mode: Mode = "command"
    selected: int = -1
    source: int = -1
    pending: str = ""


def next_colour(colour: int) -> int:
    return (colour + 2) % (PALETTE_SIZE + 1) - 1


def next_fill(colour: int) -> int:
    return (colour + 2) % CYCLE - 1


def move(nodes: List[Node], selected: int, step: int) -> int:
    index = selected + step
    while 0 <= index < len(nodes):
        if isinstance(nodes[index], Box):
            return index
        index += step
    return selected


def handle_command(state: State, key: str) -> State:
    if state.pending == "b":
        if key in ("j", "l"):
            direction: SpaceDirection = "down" if key == "j" else "right"
            nodes = state.nodes + (
                [Space(direction=direction), Box(PAD)] if state.nodes else [Box(PAD)]
            )
            return State(
                nodes=nodes,
                running=state.running,
                mode="insert",
                selected=len(nodes) - 1,
                source=-1,
                pending="",
            )
        return State(
            nodes=state.nodes,
            running=state.running,
            mode=state.mode,
            selected=state.selected,
            source=state.source,
            pending="",
        )
    if key == "b":
        return State(
            nodes=state.nodes,
            running=state.running,
            mode=state.mode,
            selected=state.selected,
            source=state.source,
            pending="b",
        )
    if key == "q":
        return State(
            nodes=state.nodes,
            running=False,
            mode=state.mode,
            selected=state.selected,
            source=state.source,
            pending=state.pending,
        )
    if key == "i":
        if not state.nodes:
            return state
        box = state.nodes[state.selected]
        nodes = edit(state.nodes, state.selected, box.label + PAD)
        return State(
            nodes=nodes,
            running=state.running,
            mode="insert",
            selected=state.selected,
            source=-1,
            pending=state.pending,
        )
    if key in ("j", "k"):
        if not state.nodes:
            return state
        step = 1 if key == "j" else -1
        selected = move(state.nodes, state.selected, step)
        return State(
            nodes=state.nodes,
            running=state.running,
            mode=state.mode,
            selected=selected,
            source=state.source,
            pending=state.pending,
        )
    if key == "c":
        if not state.nodes:
            return state
        box = state.nodes[state.selected]
        nodes = (
            state.nodes[: state.selected]
            + [replace(box, colour=next_colour(box.colour))]
            + state.nodes[state.selected + 1 :]
        )
        return State(
            nodes=nodes,
            running=state.running,
            mode=state.mode,
            selected=state.selected,
            source=state.source,
            pending=state.pending,
        )
    if key == "f":
        if not state.nodes:
            return state
        box = state.nodes[state.selected]
        nodes = (
            state.nodes[: state.selected]
            + [replace(box, fill=next_fill(box.fill))]
            + state.nodes[state.selected + 1 :]
        )
        return State(
            nodes=nodes,
            running=state.running,
            mode=state.mode,
            selected=state.selected,
            source=state.source,
            pending=state.pending,
        )
    if key == "a":
        if state.selected < 0:
            return state
        if state.source < 0:
            return State(
                nodes=state.nodes,
                running=state.running,
                mode=state.mode,
                selected=state.selected,
                source=state.selected,
                pending=state.pending,
            )
        if abs(state.selected - state.source) != 2:
            return state
        slot = (state.source + state.selected) // 2
        forward = state.selected > state.source
        if axis(state.nodes[slot]) == "col":
            direction = "right" if forward else "left"
        else:
            direction = "down" if forward else "up"
        nodes = state.nodes[:slot] + [Arrow(direction)] + state.nodes[slot + 1 :]
        return State(
            nodes=nodes,
            running=state.running,
            mode=state.mode,
            selected=state.selected,
            source=-1,
            pending=state.pending,
        )
    return state


def edit(nodes: List[Node], index: int, label: str) -> List[Node]:
    return nodes[:index] + [replace(nodes[index], label=label)] + nodes[index + 1 :]


def handle_insert(state: State, key: str) -> State:
    label = state.nodes[state.selected].label
    if key == "\x1b":
        return State(
            nodes=edit(state.nodes, state.selected, label[: -len(PAD)]),
            running=state.running,
            mode="command",
            selected=state.selected,
            source=state.source,
            pending=state.pending,
        )
    if key == "\x7f":
        return State(
            nodes=edit(state.nodes, state.selected, label[:-2] + PAD),
            running=state.running,
            mode=state.mode,
            selected=state.selected,
            source=state.source,
            pending=state.pending,
        )
    if "\x20" <= key <= "\x7e":
        return State(
            nodes=edit(state.nodes, state.selected, label[:-1] + key + PAD),
            running=state.running,
            mode=state.mode,
            selected=state.selected,
            source=state.source,
            pending=state.pending,
        )
    return state


def handle_key(state: State, key: str) -> State:
    if state.mode == "insert":
        return handle_insert(state, key)
    return handle_command(state, key)
