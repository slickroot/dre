from dataclasses import dataclass, replace
from functools import partial
from typing import Iterator, List, Tuple

from .state import Box, Path

BOX_HEIGHT = 3
GAP_HEIGHT = 3
GAP_WIDTH = 8
BORDERS = 2
ROW_PITCH = BOX_HEIGHT + GAP_HEIGHT
HALF_PITCH = BOX_HEIGHT
LEAF_STRIDE = 2


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
    path: Path = ()


@dataclass(frozen=True)
class Cursor:
    pass


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


def fmap(f, node):
    return replace(f(node), children=tuple(fmap(f, child) for child in node.children))


def flatten(f, node) -> Iterator:
    yield from f(node, node.children)
    for child in node.children:
        yield from flatten(f, child)


def anchor(children: List[Celled]) -> int:
    middle = len(children) // 2
    if len(children) % 2:
        return children[middle].row
    return children[middle - 1].row + 1


def assign(box: Box, column: int, path: Path, free: int) -> Tuple[Celled, int]:
    if not box.children:
        return Celled(box, width(box), column, free, path), free + LEAF_STRIDE
    children = []
    for index, child in enumerate(box.children):
        node, free = assign(child, column + 1, path + (index,), free)
        children.append(node)
    return Celled(box, width(box), column, anchor(children), path, tuple(children)), free


def forest(boxes: Tuple[Box, ...]) -> Tuple[Celled, ...]:
    trees = []
    free = 0
    for index, box in enumerate(boxes):
        tree, free = assign(box, 0, (index,), free)
        trees.append(tree)
    return tuple(trees)


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
    here: Positioned, children: Tuple[Positioned, ...]
) -> Iterator[Placement]:
    yield Placement(here.box, here.x, here.y, here.width, here.height)

    start = here.x + centre(here.width, here.box.label)
    middle = here.y + here.height // 2
    yield Placement(Label(here.box.label, here.path), x=start, y=middle,
                     width=interior(here.box.label), height=1)

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


def with_cursor(placements: List[Placement], selected: Path) -> List[Placement]:
    for placement in placements:
        if isinstance(placement.node, Label) and placement.node.path == selected:
            return placements + [
                Placement(
                    Cursor(),
                    x=placement.x + placement.width - 1,
                    y=placement.y,
                    width=1,
                    height=1,
                )
            ]
    return placements


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


def layout(boxes: Tuple[Box, ...], cols: int, rows: int) -> List[Placement]:
    trees = forest(boxes)
    nodes = list(walk(trees))
    if not nodes:
        return []

    columns = column_tracks(nodes)
    total_height = max(node.row for node in nodes) * HALF_PITCH + BOX_HEIGHT
    left = (cols - span(columns)) // 2
    top = (rows - total_height) // 2

    place = partial(position, columns=columns, left=left, top=top)
    placements = [
        placement
        for tree in trees
        for placement in flatten(emit, fmap(place, tree))
    ]

    # Boxes are opaque, so they are drawn before the arrows and cursor that
    # must show on top of them.
    return [p for p in placements if isinstance(p.node, Box)] + [
        p for p in placements if not isinstance(p.node, Box)
    ]
