from dataclasses import dataclass, replace
from functools import partial
from typing import Iterator, List, Tuple

from .state import Box, Cursor, Path, State

BOX_HEIGHT = 3
GAP_HEIGHT = 3
GAP_WIDTH = 4
BORDERS = 2
ROW_PITCH = BOX_HEIGHT + GAP_HEIGHT
HALF_PITCH = BOX_HEIGHT


@dataclass(frozen=True)
class Measured:
    box: Box
    width: int
    above: int
    below: int
    pitch: int
    children: Tuple["Measured", ...] = ()


@dataclass(frozen=True)
class Celled:
    box: Box
    width: int
    column: int
    row: int
    path: Path
    children: Tuple["Celled", ...] = ()


@dataclass(frozen=True)
class Positioned:
    box: Box
    path: Path
    x: int
    y: int
    width: int
    height: int
    children: Tuple["Positioned", ...] = ()


@dataclass(frozen=True)
class Arrow:
    stops: Tuple[int, ...]
    shaft: int


@dataclass(frozen=True)
class Label:
    text: str


@dataclass
class Placement:
    node: object
    x: int
    y: int
    width: int
    height: int


@dataclass(frozen=True)
class Track:
    offset: int
    extent: int


def tracks(extents: List[int], indices: List[int]) -> List[Track]:
    sizes = [0] * (max(indices) + 1)
    for extent, index in zip(extents, indices):
        sizes[index] = max(sizes[index], extent)
    offset = 0
    laid = []
    for size in sizes:
        laid.append(Track(offset=offset, extent=size))
        offset += size
    return laid


def span(laid: List[Track]) -> int:
    return sum(track.extent for track in laid)


def interior(label: str) -> int:
    return max(len(label), 1)


def width(box: Box) -> int:
    return interior(box.label) + BORDERS


def height(box: Box) -> int:
    return BOX_HEIGHT


def centre(width: int, label: str) -> int:
    leftover = width - BORDERS - interior(label)
    return 1 + leftover - leftover // 2


def fold_up(f, node):
    children = tuple(fold_up(f, child) for child in node.children)
    return replace(f(node, children), children=children)


def push_down(f, node, context):
    value, contexts = f(node, context)
    return replace(
        value,
        children=tuple(
            push_down(f, child, ctx) for child, ctx in zip(node.children, contexts)
        ),
    )


def fmap(f, node):
    return replace(f(node), children=tuple(fmap(f, child) for child in node.children))


def flatten(f, node) -> Iterator:
    yield from f(node, node.children)
    for child in node.children:
        yield from flatten(f, child)


def measure(box: Box, children: Tuple[Measured, ...]) -> Measured:
    if not children:
        return Measured(box, width(box), 0, 0, 0)
    required = max(
        (a.below + b.above + 2 for a, b in zip(children, children[1:])),
        default=0,
    )
    pitch = max(2, required)
    if pitch % 2:
        pitch += 1
    half = pitch * (len(children) - 1) // 2
    return Measured(
        box, width(box), half + children[0].above, half + children[-1].below, pitch
    )


def cells(node: Measured, context) -> Tuple[Celled, List[Tuple[int, int, Path]]]:
    column, row, path = context
    half = node.pitch * (len(node.children) - 1) // 2
    contexts = [
        (column + 1, row - half + index * node.pitch, path + (index,))
        for index in range(len(node.children))
    ]
    return Celled(node.box, node.width, column, row, path), contexts


def forest(boxes: Tuple[Box, ...]) -> Tuple[Celled, ...]:
    measured = [fold_up(measure, box) for box in boxes]
    starts: List[int] = []
    for index, node in enumerate(measured):
        if index == 0:
            starts.append(0)
        else:
            starts.append(starts[-1] + measured[index - 1].below + 2 + node.above)
    trees = tuple(
        push_down(cells, node, (0, start, (index,)))
        for index, (node, start) in enumerate(zip(measured, starts))
    )
    minimum = min((node.row for node in walk(trees)), default=0)
    if minimum == 0:
        return trees
    return tuple(
        fmap(lambda node: replace(node, row=node.row - minimum), tree)
        for tree in trees
    )


def walk(nodes: Tuple[Celled, ...]) -> Iterator[Celled]:
    for node in nodes:
        yield node
        yield from walk(node.children)


def position(node: Celled, columns: List[Track], left: int, top: int) -> Positioned:
    track = columns[2 * node.column]
    return Positioned(
        box=node.box,
        path=node.path,
        x=left + track.offset,
        y=top + node.row * HALF_PITCH,
        width=track.extent,
        height=BOX_HEIGHT,
    )


def emit(
    here: Positioned, children: Tuple[Positioned, ...], selected: Path
) -> Iterator[Placement]:
    yield Placement(here.box, here.x, here.y, here.width, here.height)

    start = here.x + centre(here.width, here.box.label)
    middle = here.y + here.height // 2
    yield Placement(Label(here.box.label), x=start, y=middle,
                     width=interior(here.box.label), height=1)

    if here.path == selected:
        yield Placement(
            Cursor(),
            x=start + interior(here.box.label) - 1,
            y=middle,
            width=1,
            height=1,
        )

    if children:
        origin = children[0].y + children[0].height // 2
        stops = tuple(child.y + child.height // 2 - origin for child in children)
        shaft = here.y + here.height // 2 - origin
        yield Placement(
            Arrow(stops, shaft),
            x=here.x + here.width,
            y=origin,
            width=GAP_WIDTH,
            height=stops[-1] - stops[0] + 1,
        )


def column_tracks(nodes: List[Celled]) -> List[Track]:
    parents = [node for node in nodes if node.children]
    # Column is doubled into a track index so a gap track can sit between
    # every pair of box tracks: box column c owns track 2c, and the arrow
    # gap after it owns 2c + 1.
    return tracks(
        [node.width for node in nodes] + [GAP_WIDTH for _ in parents],
        [2 * node.column for node in nodes]
        + [2 * node.column + 1 for node in parents],
    )


def layout(state: State, cols: int, rows: int) -> List[Placement]:
    trees = forest(state.boxes)
    nodes = list(walk(trees))
    if not nodes:
        return []

    columns = column_tracks(nodes)
    total_height = max(node.row for node in nodes) * HALF_PITCH + BOX_HEIGHT
    left = (cols - span(columns)) // 2
    top = (rows - total_height) // 2

    place = partial(position, columns=columns, left=left, top=top)
    show = partial(emit, selected=state.selected)
    placements = [
        placement
        for tree in trees
        for placement in flatten(show, fmap(place, tree))
    ]

    # Boxes are opaque, so they are drawn before the arrows and cursor that
    # must show on top of them.
    return [p for p in placements if isinstance(p.node, Box)] + [
        p for p in placements if not isinstance(p.node, Box)
    ]
