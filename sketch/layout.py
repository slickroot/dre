from dataclasses import dataclass
from itertools import accumulate
from typing import Dict, Generic, Iterator, List, Tuple, TypeVar

from .state import Box, Cursor, Path, State, at

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


def centre(track: Track, extent: int, available: int, total: int) -> int:
    return (available - total + 2 * track.offset + track.extent - extent) // 2


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


def cursor(
    state: State, placements_by_path: Dict[Path, Placement]
) -> List[Placement]:
    if not state.selected:
        return []
    box = at(state.boxes, state.selected)
    placement = placements_by_path[state.selected]
    x = placement.x + interior(box.label)
    return [Placement(Cursor(), x=x, y=placement.y + 1, width=1, height=1)]


def layout(state: State, cols: int, rows: int) -> List[Placement]:
    if not state.boxes:
        return []

    laid = list(walk(forest(state.boxes)))
    placed = [node.value for node in laid]
    arrow_entries = [
        (node.value.column, node.value.row, [c.value.row for c in node.children])
        for node in laid
        if node.children
    ]

    max_row = max(cell.row for cell in placed)

    # Column is doubled into a track index so a gap track can sit between
    # every pair of box tracks: box column c owns track 2c, and the arrow
    # gap after it owns 2c + 1.
    column_extents = [cell.width for cell in placed] + [
        GAP_WIDTH for _ in arrow_entries
    ]
    column_indices = [2 * cell.column for cell in placed] + [
        2 * column + 1 for column, _, _ in arrow_entries
    ]

    columns = tracks(column_extents, column_indices)
    total_width = span(columns)
    total_height = max_row * ROW_PITCH + BOX_HEIGHT
    top = (rows - total_height) // 2

    placements_by_pos: Dict[Tuple[int, int], Placement] = {}
    placements_by_path: Dict[Path, Placement] = {}
    box_placements: List[Placement] = []
    for cell in placed:
        box_width = columns[2 * cell.column].extent
        placement = Placement(
            cell.box,
            x=centre(columns[2 * cell.column], box_width, cols, total_width),
            y=top + cell.row * ROW_PITCH,
            width=box_width,
            height=BOX_HEIGHT,
        )
        box_placements.append(placement)
        placements_by_pos[(cell.column, cell.row)] = placement
        placements_by_path[cell.path] = placement

    arrow_placements: List[Placement] = []
    for column, row, child_rows in arrow_entries:
        parent = placements_by_pos[(column, row)]
        top = parent.y + parent.height // 2
        child_centres = [
            placements_by_pos[(column + 1, r)].y
            + placements_by_pos[(column + 1, r)].height // 2
            for r in child_rows
        ]
        bottom = child_centres[-1]
        stops = tuple(centre_y - top for centre_y in child_centres)
        x = centre(columns[2 * column + 1], GAP_WIDTH, cols, total_width)
        arrow_placements.append(
            Placement(Arrow(stops), x=x, y=top, width=GAP_WIDTH, height=bottom - top + 1)
        )

    return box_placements + arrow_placements + cursor(state, placements_by_path)
