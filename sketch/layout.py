from dataclasses import dataclass
from typing import List

from .state import Arrow, Box, Cursor, Node, Pop, Push, Space, State
from .state import axis as node_axis

BOX_HEIGHT = 3
GAP_HEIGHT = 2
BORDERS = 2


@dataclass
class Placement:
    node: Node
    x: int
    y: int
    width: int
    height: int


@dataclass(frozen=True)
class Coordinate:
    row: int
    col: int


@dataclass(frozen=True)
class Track:
    offset: int
    extent: int


def walk(nodes: List[Node]) -> List[Coordinate]:
    coordinates = [Coordinate(0, 0)]
    row, col, active = 0, 0, "row"
    stack = []
    for node in nodes[1:]:
        if isinstance(node, Push):
            coordinates.append(Coordinate(row, col))
            stack.append((row, col, active))
            continue
        if isinstance(node, Pop):
            coordinates.append(Coordinate(row, col))
            row, col, active = stack.pop()
            continue
        if isinstance(node, (Space, Arrow)):
            active = node_axis(node)
        if active == "row":
            row += 1
        else:
            col += 1
        coordinates.append(Coordinate(row, col))
    top = min(c.row for c in coordinates)
    left = min(c.col for c in coordinates)
    return [Coordinate(c.row - top, c.col - left) for c in coordinates]


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


def height(node: Node) -> int:
    if isinstance(node, Box):
        return BOX_HEIGHT
    if isinstance(node, (Push, Pop)):
        return 0
    return GAP_HEIGHT


def width(node: Node) -> int:
    if isinstance(node, Box):
        return interior(node.label) + BORDERS
    if isinstance(node, Arrow):
        return 4 if node_axis(node) == "col" else 1
    if isinstance(node, Space) and node.direction == "right":
        return 4
    return 0


def cursor(state: State, placements: List[Placement]) -> List[Placement]:
    if state.selected < 0 or not isinstance(state.nodes[state.selected], Box):
        return []
    box = placements[state.selected]
    x = box.x + interior(box.node.label)
    return [Placement(Cursor(), x=x, y=box.y + 1, width=1, height=1)]


def layout(state: State, cols: int, rows: int) -> List[Placement]:
    if not state.nodes:
        return []

    coordinates = walk(state.nodes)
    widths = [width(node) for node in state.nodes]
    heights = [height(node) for node in state.nodes]

    columns = tracks(widths, [c.col for c in coordinates])
    rows_ = tracks(heights, [c.row for c in coordinates])
    total_width, total_height = span(columns), span(rows_)

    placements = [
        Placement(
            node,
            x=centre(columns[at.col], node_width, cols, total_width),
            y=centre(rows_[at.row], node_height, rows, total_height),
            width=node_width,
            height=node_height,
        )
        for node, at, node_width, node_height in zip(
            state.nodes, coordinates, widths, heights
        )
    ]
    return placements + cursor(state, placements)
