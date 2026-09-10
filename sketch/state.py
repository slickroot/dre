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


Node = Union[Box, Space, Arrow, Cursor]

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


# A box's "parent" is inferred from adjacency: it has a parent iff the node right before it is an Arrow.
def has_parent(nodes: List[Node], selected: int) -> bool:
    return selected > 0 and isinstance(nodes[selected - 1], Arrow)


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


def handle_command(state: State, key: str) -> State:
    if state.pending == "b":
        if key in ("j", "l"):
            direction: SpaceDirection = "down" if key == "j" else "right"
            nodes = state.nodes + [Space(direction=direction), Box(PAD)]
            return replace(
                state,
                nodes=nodes,
                mode="insert",
                selected=len(nodes) - 1,
                source=-1,
                pending="",
            )
        return replace(state, pending="")
    if state.pending == "s":
        if key in ("j", "l") and not has_parent(state.nodes, state.selected):
            direction: SpaceDirection = "down" if key == "j" else "right"
            nodes = state.nodes + [Space(direction=direction), Box(PAD)]
            return replace(
                state,
                nodes=nodes,
                mode="insert",
                selected=len(nodes) - 1,
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
        if not state.nodes:
            return replace(state, nodes=[Box(PAD)], mode="insert", selected=0)
        return replace(state, pending="b")
    if key == "s":
        if not state.nodes:
            return state
        return replace(state, pending="s")
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
