from dataclasses import dataclass
from functools import partial
from itertools import accumulate
from typing import Generic, Iterator, List, Tuple, TypeVar

from .state import Box, Cursor, Path, State

BOX_HEIGHT = 3
GAP_HEIGHT = 2
GAP_WIDTH = 4
BORDERS = 2
ROW_PITCH = BOX_HEIGHT + GAP_HEIGHT

A = TypeVar("A")


@dataclass(frozen=True)
class Node(Generic[A]):
    value: A
    children: Tuple["Node[A]", ...] = ()


@dataclass(frozen=True)
class Measured:
    box: Box
    width: int
    span: int


@dataclass(frozen=True)
class Celled:
    box: Box
    width: int
    column: int
    row: int
    path: Path


@dataclass(frozen=True)
class Positioned:
    box: Box
    path: Path
    x: int
    y: int
    width: int
    height: int


@dataclass(frozen=True)
class Arrow:
    stops: Tuple[int, ...]


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


def fold_up(f, node) -> Node:
    children = tuple(fold_up(f, child) for child in node.children)
    return Node(f(node, children), children)


def push_down(f, node, context) -> Node:
    value, contexts = f(node, context)
    return Node(
        value,
        tuple(push_down(f, child, ctx) for child, ctx in zip(node.children, contexts)),
    )


def fmap(f, node: Node) -> Node:
    return Node(f(node), tuple(fmap(f, child) for child in node.children))


def flatten(f, node: Node) -> Iterator:
    yield from f(node, node.children)
    for child in node.children:
        yield from flatten(f, child)


def measure(box: Box, children: Tuple[Node, ...]) -> Measured:
    return Measured(box, width(box), sum(c.value.span for c in children) or 1)


def cells(node: Node, context) -> Tuple[Celled, List[Tuple[int, int, Path]]]:
    column, row, path = context
    starts = accumulate((child.value.span for child in node.children), initial=row)
    contexts = [
        (column + 1, start, path + (index,))
        for index, start in zip(range(len(node.children)), starts)
    ]
    return Celled(node.value.box, node.value.width, column, row, path), contexts


def forest(boxes: Tuple[Box, ...]) -> Tuple[Node, ...]:
    measured = [fold_up(measure, box) for box in boxes]
    starts = accumulate((node.value.span for node in measured), initial=0)
    return tuple(
        push_down(cells, node, (0, start, (index,)))
        for index, (node, start) in enumerate(zip(measured, starts))
    )


def walk(nodes: Tuple[Node, ...]) -> Iterator[Node]:
    for node in nodes:
        yield node
        yield from walk(node.children)


def position(node: Node, columns: List[Track], left: int, top: int) -> Positioned:
    cell = node.value
    track = columns[2 * cell.column]
    return Positioned(
        box=cell.box,
        path=cell.path,
        x=left + track.offset,
        y=top + cell.row * ROW_PITCH,
        width=track.extent,
        height=BOX_HEIGHT,
    )


def emit(node: Node, children: Tuple[Node, ...], selected: Path) -> Iterator[Placement]:
    here = node.value
    yield Placement(here.box, here.x, here.y, here.width, here.height)

    if here.path == selected:
        yield Placement(
            Cursor(),
            x=here.x + interior(here.box.label),
            y=here.y + here.height // 2,
            width=1,
            height=1,
        )

    if children:
        shaft = here.y + here.height // 2
        stops = [child.value.y + child.value.height // 2 - shaft for child in children]
        yield Placement(
            Arrow(tuple(stops)),
            x=here.x + here.width,
            y=shaft,
            width=GAP_WIDTH,
            height=stops[-1] + 1,
        )


def column_tracks(nodes: List[Node]) -> List[Track]:
    parents = [node for node in nodes if node.children]
    # Column is doubled into a track index so a gap track can sit between
    # every pair of box tracks: box column c owns track 2c, and the arrow
    # gap after it owns 2c + 1.
    return tracks(
        [node.value.width for node in nodes] + [GAP_WIDTH for _ in parents],
        [2 * node.value.column for node in nodes]
        + [2 * node.value.column + 1 for node in parents],
    )


def layout(state: State, cols: int, rows: int) -> List[Placement]:
    trees = forest(state.boxes)
    nodes = list(walk(trees))
    if not nodes:
        return []

    columns = column_tracks(nodes)
    total_height = max(node.value.row for node in nodes) * ROW_PITCH + BOX_HEIGHT
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
