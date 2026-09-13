from dataclasses import dataclass, replace
from typing import Callable, Literal, Tuple

PLAIN = -1
PALETTE_SIZE = 5
PAD = " "


@dataclass(frozen=True)
class Box:
    label: str = ""
    colour: int = PLAIN
    fill: int = PLAIN
    rounded: bool = False
    children: Tuple["Box", ...] = ()


Mode = Literal["command", "insert"]
Path = Tuple[int, ...]


@dataclass(frozen=True)
class State:
    boxes: Tuple[Box, ...] = ()
    running: bool = True
    mode: Mode = "command"
    selected: Path = ()


def next_colour(colour: int) -> int:
    return (colour + 2) % (PALETTE_SIZE + 1) - 1


def at(boxes: Tuple[Box, ...], path: Path) -> Box:
    box = boxes[path[0]]
    for index in path[1:]:
        box = box.children[index]
    return box


def rewrite(
    boxes: Tuple[Box, ...], path: Path, fn: Callable[[Box], Box]
) -> Tuple[Box, ...]:
    index, rest = path[0], path[1:]
    box = boxes[index]
    if rest:
        box = replace(box, children=rewrite(box.children, rest, fn))
    else:
        box = fn(box)
    return boxes[:index] + (box,) + boxes[index + 1 :]


def colour_row(boxes: Tuple[Box, ...], path: Path) -> Tuple[Box, ...]:
    parent = path[:-1]
    siblings = at(boxes, parent).children if parent else boxes
    if len({box.colour for box in siblings}) == 1:
        new_colour = next_colour(siblings[0].colour)
    else:
        new_colour = 0
    for i in range(len(siblings)):
        boxes = rewrite(boxes, parent + (i,), lambda box: replace(box, colour=new_colour))
    return boxes


def grow(boxes: Tuple[Box, ...], path: Path) -> Tuple[Tuple[Box, ...], Path]:
    if not path:
        return boxes + (Box(PAD),), (len(boxes),)
    new_index = len(at(boxes, path).children)
    grown = rewrite(
        boxes, path, lambda box: replace(box, children=box.children + (Box(PAD),))
    )
    return grown, path + (new_index,)


def handle_command(state: State, key: str) -> State:
    if key == "b":
        boxes, selected = grow(state.boxes, state.selected)
        return replace(state, boxes=boxes, mode="insert", selected=selected)
    if key == "h":
        if len(state.selected) <= 1:
            return state
        return replace(state, selected=state.selected[:-1])
    if key == "l":
        if not state.selected:
            return state
        if not at(state.boxes, state.selected).children:
            return state
        return replace(state, selected=state.selected + (0,))
    if key == "j":
        if not state.selected:
            return state
        parent, index = state.selected[:-1], state.selected[-1]
        siblings = at(state.boxes, parent).children if parent else state.boxes
        if index + 1 >= len(siblings):
            return state
        return replace(state, selected=parent + (index + 1,))
    if key == "k":
        if not state.selected:
            return state
        parent, index = state.selected[:-1], state.selected[-1]
        if index == 0:
            return state
        return replace(state, selected=parent + (index - 1,))
    if key == "i":
        if not state.selected:
            return state
        boxes = rewrite(
            state.boxes, state.selected, lambda box: replace(box, label=box.label + PAD)
        )
        return replace(state, boxes=boxes, mode="insert")
    if key == "c":
        if not state.selected:
            return state
        boxes = rewrite(
            state.boxes,
            state.selected,
            lambda box: replace(box, colour=next_colour(box.colour)),
        )
        return replace(state, boxes=boxes)
    if key == "C":
        if len(state.selected) <= 1:
            return state
        return replace(state, boxes=colour_row(state.boxes, state.selected))
    if key == "f":
        if not state.selected:
            return state
        boxes = rewrite(
            state.boxes,
            state.selected,
            lambda box: replace(box, fill=next_colour(box.fill)),
        )
        return replace(state, boxes=boxes)
    if key == "r":
        if not state.selected:
            return state
        boxes = rewrite(
            state.boxes,
            state.selected,
            lambda box: replace(box, rounded=not box.rounded),
        )
        return replace(state, boxes=boxes)
    if key == "q":
        return replace(state, running=False)
    return state


def handle_insert(state: State, key: str) -> State:
    label = at(state.boxes, state.selected).label
    if key == "\x1b":
        boxes = rewrite(
            state.boxes, state.selected, lambda box: replace(box, label=label[: -len(PAD)])
        )
        return replace(state, boxes=boxes, mode="command")
    if key == "\x7f":
        boxes = rewrite(
            state.boxes, state.selected, lambda box: replace(box, label=label[:-2] + PAD)
        )
        return replace(state, boxes=boxes)
    if "\x20" <= key <= "\x7e":
        boxes = rewrite(
            state.boxes,
            state.selected,
            lambda box: replace(box, label=label[:-1] + key + PAD),
        )
        return replace(state, boxes=boxes)
    return state


def handle_key(state: State, key: str) -> State:
    if state.mode == "insert":
        return handle_insert(state, key)
    return handle_command(state, key)
