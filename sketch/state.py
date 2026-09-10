from dataclasses import dataclass, replace
from typing import List, Literal, Union

PLAIN = -1
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


@dataclass(frozen=True)
class Push:
    pass


@dataclass(frozen=True)
class Pop:
    pass


Node = Union[Box, Space, Arrow, Cursor, Push, Pop]

Mode = Literal["command", "insert"]


@dataclass(frozen=True)
class State:
    nodes: List[Node]
    running: bool = True
    mode: Mode = "command"
    selected: int = -1
    source: int = -1
    pending: str = ""


def next_colour(colour: int) -> int:
    return (colour + 2) % (PALETTE_SIZE + 1) - 1


def move(nodes: List[Node], selected: int, step: int) -> int:
    index = selected + step
    while 0 <= index < len(nodes):
        if isinstance(nodes[index], Box):
            return index
        index += step
    return selected


def beside(nodes: List[Node], selected: int, step: int) -> int:
    slot, target = selected + step, selected + 2 * step
    if not 0 <= target < len(nodes):
        return selected
    if not isinstance(nodes[slot], (Space, Arrow)):
        return selected
    if axis(nodes[slot]) != "col":
        return selected
    if not isinstance(nodes[target], Box):
        return selected
    return target


def below(nodes: List[Node], selected: int) -> int:
    slot, target = selected + 1, selected + 2
    if target >= len(nodes):
        return selected
    if not isinstance(nodes[slot], (Space, Arrow)):
        return selected
    if axis(nodes[slot]) != "row":
        return selected
    if not isinstance(nodes[target], Box):
        return selected
    return target


def outgoing(nodes: List[Node], i: int) -> List[tuple]:
    branches = []
    index = i + 1
    if index < len(nodes) and isinstance(nodes[index], Push):
        separator_index = index + 1
        branches.append((nodes[separator_index].direction, separator_index))
        depth = 1
        index = separator_index
        while depth > 0:
            index += 1
            if isinstance(nodes[index], Push):
                depth += 1
            elif isinstance(nodes[index], Pop):
                depth -= 1
        index += 1
    if index < len(nodes) and isinstance(nodes[index], (Space, Arrow)):
        branches.append((nodes[index].direction, index))
    return branches


def handle_command(state: State, key: str) -> State:
    if state.pending == "b":
        if key in ("j", "l"):
            if not state.nodes:
                return replace(
                    state,
                    nodes=[Box(PAD)],
                    mode="insert",
                    selected=0,
                    source=-1,
                    pending="",
                )
            direction: SpaceDirection = "down" if key == "j" else "right"
            selected = (
                state.selected if state.selected >= 0 else len(state.nodes) - 1
            )
            branches = outgoing(state.nodes, selected)
            match = next((b for b in branches if b[0] == direction), None)
            if match is not None:
                separator_index = match[1]
                if isinstance(state.nodes[separator_index], Arrow):
                    return replace(state, pending="")
                insertion = [Space(direction=direction), Box(PAD)]
                new_index = separator_index + 1
            elif branches:
                insertion = [Push(), Space(direction=direction), Box(PAD), Pop()]
                separator_index = selected + 1
                new_index = separator_index + 2
            else:
                insertion = [Space(direction=direction), Box(PAD)]
                separator_index = selected + 1
                new_index = separator_index + 1
            nodes = (
                state.nodes[:separator_index]
                + insertion
                + state.nodes[separator_index:]
            )
            return replace(
                state,
                nodes=nodes,
                mode="insert",
                selected=new_index,
                source=-1,
                pending="",
            )
        return replace(state, pending="")
    if key in ("h", "l"):
        if not state.nodes:
            return state
        step = 1 if key == "l" else -1
        return replace(state, selected=beside(state.nodes, state.selected, step))
    if key == "b":
        return replace(state, pending="b")
    if key == "q":
        return replace(state, running=False)
    if key == "i":
        if not state.nodes:
            return state
        box = state.nodes[state.selected]
        nodes = edit(state.nodes, state.selected, box.label + PAD)
        return replace(state, nodes=nodes, mode="insert", source=-1)
    if key == "j":
        if not state.nodes:
            return state
        return replace(state, selected=below(state.nodes, state.selected))
    if key == "k":
        if not state.nodes:
            return state
        selected = move(state.nodes, state.selected, -1)
        return replace(state, selected=selected)
    if key == "c":
        if not state.nodes:
            return state
        box = state.nodes[state.selected]
        nodes = (
            state.nodes[: state.selected]
            + [replace(box, colour=next_colour(box.colour))]
            + state.nodes[state.selected + 1 :]
        )
        return replace(state, nodes=nodes)
    if key == "f":
        if not state.nodes:
            return state
        box = state.nodes[state.selected]
        nodes = (
            state.nodes[: state.selected]
            + [replace(box, fill=next_colour(box.fill))]
            + state.nodes[state.selected + 1 :]
        )
        return replace(state, nodes=nodes)
    if key == "a":
        if state.selected < 0:
            return state
        if state.source < 0:
            return replace(state, source=state.selected)
        if abs(state.selected - state.source) != 2:
            return state
        slot = (state.source + state.selected) // 2
        forward = state.selected > state.source
        if axis(state.nodes[slot]) == "col":
            direction = "right" if forward else "left"
        else:
            direction = "down" if forward else "up"
        nodes = state.nodes[:slot] + [Arrow(direction)] + state.nodes[slot + 1 :]
        return replace(state, nodes=nodes, source=-1)
    return state


def edit(nodes: List[Node], index: int, label: str) -> List[Node]:
    return nodes[:index] + [replace(nodes[index], label=label)] + nodes[index + 1 :]


def handle_insert(state: State, key: str) -> State:
    label = state.nodes[state.selected].label
    if key == "\x1b":
        return replace(
            state,
            nodes=edit(state.nodes, state.selected, label[: -len(PAD)]),
            mode="command",
        )
    if key == "\x7f":
        return replace(
            state, nodes=edit(state.nodes, state.selected, label[:-2] + PAD)
        )
    if "\x20" <= key <= "\x7e":
        return replace(
            state, nodes=edit(state.nodes, state.selected, label[:-1] + key + PAD)
        )
    return state


def handle_key(state: State, key: str) -> State:
    if state.mode == "insert":
        return handle_insert(state, key)
    return handle_command(state, key)
