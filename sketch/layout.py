from dataclasses import dataclass
from typing import Dict, List, Tuple

from .state import Box, Cursor, Path, State, at

BOX_HEIGHT = 3
GAP_HEIGHT = 2
GAP_WIDTH = 4
BORDERS = 2


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


def _place(
    box: Box,
    depth: int,
    row: int,
    path: Path,
    box_entries: list,
    arrow_entries: list,
) -> int:
    if not box.children:
        box_entries.append((box, depth, row, path))
        return 1

    child_row = row
    child_rows = []
    for index, child in enumerate(box.children):
        child_rows.append(child_row)
        child_row += _place(
            child, depth + 1, child_row, path + (index,), box_entries, arrow_entries
        )

    box_entries.append((box, depth, row, path))
    arrow_entries.append((depth, row, child_rows))
    return child_row - row


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

    box_entries: List[Tuple[Box, int, int, Path]] = []
    arrow_entries: List[Tuple[int, int, List[int]]] = []

    row = 0
    for index, box in enumerate(state.boxes):
        row += _place(box, 0, row, (index,), box_entries, arrow_entries)

    max_row = max(row for _, _, row, _ in box_entries)

    # Depth and row are doubled into track indices so a gap track can sit
    # between every pair of box tracks: box column d / row r own tracks
    # 2d / 2r, and the arrow-gap / sibling-gap after it owns 2d+1 / 2r+1.
    column_extents = [width(box) for box, _, _, _ in box_entries] + [
        GAP_WIDTH for _ in arrow_entries
    ]
    column_indices = [2 * depth for _, depth, _, _ in box_entries] + [
        2 * depth + 1 for depth, _, _ in arrow_entries
    ]
    row_extents = [BOX_HEIGHT for _ in box_entries] + [
        GAP_HEIGHT for _ in range(max_row)
    ]
    row_indices = [2 * r for _, _, r, _ in box_entries] + [
        2 * r + 1 for r in range(max_row)
    ]

    columns = tracks(column_extents, column_indices)
    rows_ = tracks(row_extents, row_indices)
    total_width, total_height = span(columns), span(rows_)

    placements_by_pos: Dict[Tuple[int, int], Placement] = {}
    placements_by_path: Dict[Path, Placement] = {}
    box_placements: List[Placement] = []
    for box, depth, row, path in box_entries:
        box_width = width(box)
        placement = Placement(
            box,
            x=centre(columns[2 * depth], box_width, cols, total_width),
            y=centre(rows_[2 * row], BOX_HEIGHT, rows, total_height),
            width=box_width,
            height=BOX_HEIGHT,
        )
        box_placements.append(placement)
        placements_by_pos[(depth, row)] = placement
        placements_by_path[path] = placement

    arrow_placements: List[Placement] = []
    for depth, row, child_rows in arrow_entries:
        parent = placements_by_pos[(depth, row)]
        top = parent.y + parent.height // 2
        child_centres = [
            placements_by_pos[(depth + 1, r)].y
            + placements_by_pos[(depth + 1, r)].height // 2
            for r in child_rows
        ]
        bottom = child_centres[-1]
        stops = tuple(centre_y - top for centre_y in child_centres)
        x = centre(columns[2 * depth + 1], GAP_WIDTH, cols, total_width)
        arrow_placements.append(
            Placement(Arrow(stops), x=x, y=top, width=GAP_WIDTH, height=bottom - top + 1)
        )

    return box_placements + arrow_placements + cursor(state, placements_by_path)
