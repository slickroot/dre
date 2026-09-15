from dataclasses import dataclass, replace
from enum import Enum
from typing import Callable, Literal, Optional, Tuple

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


class Command(Enum):
    UNDO = "u"
    NEW_BOX = "b"
    NEW_SIBLING = "s"
    SELECT_PARENT = "h"
    SELECT_CHILD = "l"
    SELECT_NEXT = "j"
    SELECT_PREVIOUS = "k"
    EDIT_LABEL = "i"
    RENAME_LABEL = "I"
    CYCLE_COLOUR = "c"
    CYCLE_SIBLINGS_COLOUR = "C"
    CYCLE_FILL = "f"
    TOGGLE_ROUNDED = "r"
    QUIT = "q"


@dataclass(frozen=True)
class State:
    boxes: Tuple[Box, ...] = ()
    running: bool = True
    mode: Mode = "command"
    selected: Path = ()
    before: Optional["State"] = None


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


UNDOABLE_COMMANDS = {
    Command.NEW_BOX,
    Command.CYCLE_COLOUR,
    Command.CYCLE_FILL,
    Command.TOGGLE_ROUNDED,
    Command.CYCLE_SIBLINGS_COLOUR,
    Command.RENAME_LABEL,
}


def enter_insert(state: State, base_label: str) -> State:
    boxes = rewrite(
        state.boxes, state.selected, lambda box: replace(box, label=base_label + PAD)
    )
    return replace(state, boxes=boxes, mode="insert")


def handle_command(state: State, key: str) -> State:
    try:
        command = Command(key)
    except ValueError:
        return state
    if command in UNDOABLE_COMMANDS:
        state = replace(state, before=replace(state, before=None))
    if command == Command.UNDO:
        return state.before if state.before is not None else state
    if command == Command.NEW_BOX:
        boxes, selected = grow(state.boxes, state.selected)
        return replace(state, boxes=boxes, mode="insert", selected=selected)
    if command == Command.NEW_SIBLING:
        if not state.selected:
            return state
        boxes, selected = grow(state.boxes, state.selected[:-1])
        return replace(state, boxes=boxes, mode="insert", selected=selected)
    if command == Command.SELECT_PARENT:
        if len(state.selected) <= 1:
            return state
        return replace(state, selected=state.selected[:-1])
    if command == Command.SELECT_CHILD:
        if not state.selected:
            return state
        if not at(state.boxes, state.selected).children:
            return state
        return replace(state, selected=state.selected + (0,))
    if command == Command.SELECT_NEXT:
        if not state.selected:
            return state
        parent, index = state.selected[:-1], state.selected[-1]
        siblings = at(state.boxes, parent).children if parent else state.boxes
        if index + 1 >= len(siblings):
            return state
        return replace(state, selected=parent + (index + 1,))
    if command == Command.SELECT_PREVIOUS:
        if not state.selected:
            return state
        parent, index = state.selected[:-1], state.selected[-1]
        if index == 0:
            return state
        return replace(state, selected=parent + (index - 1,))
    if command == Command.EDIT_LABEL:
        if not state.selected:
            return state
        return enter_insert(state, at(state.boxes, state.selected).label)
    if command == Command.RENAME_LABEL:
        if not state.selected:
            return state
        return enter_insert(state, "")
    if command == Command.CYCLE_COLOUR:
        if not state.selected:
            return state
        boxes = rewrite(
            state.boxes,
            state.selected,
            lambda box: replace(box, colour=next_colour(box.colour)),
        )
        return replace(state, boxes=boxes)
    if command == Command.CYCLE_SIBLINGS_COLOUR:
        if len(state.selected) <= 1:
            return state
        return replace(state, boxes=colour_row(state.boxes, state.selected))
    if command == Command.CYCLE_FILL:
        if not state.selected:
            return state
        boxes = rewrite(
            state.boxes,
            state.selected,
            lambda box: replace(box, fill=next_colour(box.fill)),
        )
        return replace(state, boxes=boxes)
    if command == Command.TOGGLE_ROUNDED:
        if not state.selected:
            return state
        boxes = rewrite(
            state.boxes,
            state.selected,
            lambda box: replace(box, rounded=not box.rounded),
        )
        return replace(state, boxes=boxes)
    if command == Command.QUIT:
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
