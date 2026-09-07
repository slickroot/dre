from typing import List, Protocol

from .layout import Placement
from .state import Box, Cursor

TOP_LEFT = "┌"
TOP_RIGHT = "┐"
BOTTOM_LEFT = "└"
BOTTOM_RIGHT = "┘"
HORIZONTAL = "─"
VERTICAL = "│"
BLANK = " "
CURSOR = "\u2588"


class Renderer(Protocol):
    def render(
        self, placements: List[Placement], cols: int, rows: int
    ) -> List[str]:
        ...


class TerminalRenderer:
    def render(
        self, placements: List[Placement], cols: int, rows: int
    ) -> List[str]:
        grid = [[BLANK] * cols for _ in range(rows)]
        for placement in placements:
            if isinstance(placement.node, Box):
                self._draw_box(grid, placement)
            elif isinstance(placement.node, Cursor):
                self._draw_cursor(grid, placement)
        return ["".join(line) for line in grid]

    def _draw_box(self, grid: List[List[str]], placement: Placement) -> None:
        left = placement.x
        right = placement.x + placement.width - 1
        top = placement.y
        bottom = placement.y + placement.height - 1
        for y in range(top, bottom + 1):
            for x in range(left, right + 1):
                self._put(
                    grid,
                    x,
                    y,
                    self._box_character(x, y, left, right, top, bottom),
                )
        self._draw_label(grid, placement)

    def _draw_cursor(self, grid: List[List[str]], placement: Placement) -> None:
        self._put(grid, placement.x, placement.y, CURSOR)

    def _draw_label(self, grid: List[List[str]], placement: Placement) -> None:
        for offset, character in enumerate(placement.node.label):
            self._put(grid, placement.x + 1 + offset, placement.y + 1, character)
        if placement.cursor is not None:
            self._put(
                grid, placement.x + 1 + placement.cursor, placement.y + 1, CURSOR
            )

    def _box_character(
        self, x: int, y: int, left: int, right: int, top: int, bottom: int
    ) -> str:
        if y == top:
            if x == left:
                return TOP_LEFT
            if x == right:
                return TOP_RIGHT
            return HORIZONTAL
        if y == bottom:
            if x == left:
                return BOTTOM_LEFT
            if x == right:
                return BOTTOM_RIGHT
            return HORIZONTAL
        if x in (left, right):
            return VERTICAL
        return BLANK

    def _put(self, grid: List[List[str]], x: int, y: int, character: str) -> None:
        if 0 <= y < len(grid) and 0 <= x < len(grid[y]):
            grid[y][x] = character
