from typing import List, Protocol, Tuple

from .layout import Placement
from .state import PLAIN, Box, Cursor

TOP_LEFT = "┌"
TOP_RIGHT = "┐"
BOTTOM_LEFT = "└"
BOTTOM_RIGHT = "┘"
HORIZONTAL = "─"
VERTICAL = "│"
BLANK = " "
CURSOR = "\u2588"
RESET = "\x1b[0m"

Cell = Tuple[str, int]
Grid = List[List[Cell]]

BLANK_CELL = (BLANK, PLAIN)


class Renderer(Protocol):
    def render(
        self, placements: List[Placement], cols: int, rows: int
    ) -> List[str]:
        ...


class TerminalRenderer:
    def render(
        self, placements: List[Placement], cols: int, rows: int
    ) -> List[str]:
        grid = [[BLANK_CELL] * cols for _ in range(rows)]
        for placement in placements:
            if isinstance(placement.node, Box):
                self._draw_box(grid, placement)
            elif isinstance(placement.node, Cursor):
                self._draw_cursor(grid, placement)
        return ["".join(_cell(*cell) for cell in row) for row in grid]

    def _draw_box(self, grid: Grid, placement: Placement) -> None:
        left = placement.x
        right = placement.x + placement.width - 1
        top = placement.y
        bottom = placement.y + placement.height - 1
        colour = placement.node.colour
        for y in range(top, bottom + 1):
            for x in range(left, right + 1):
                character = self._box_character(x, y, left, right, top, bottom)
                if character == BLANK:
                    self._put(grid, x, y, BLANK_CELL)
                else:
                    self._put(grid, x, y, (character, colour))
        self._draw_label(grid, placement)

    def _draw_cursor(self, grid: Grid, placement: Placement) -> None:
        self._put(grid, placement.x, placement.y, (CURSOR, PLAIN))

    def _draw_label(self, grid: Grid, placement: Placement) -> None:
        for offset, character in enumerate(placement.node.label):
            self._put(
                grid, placement.x + 1 + offset, placement.y + 1, (character, PLAIN)
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

    def _put(self, grid: Grid, x: int, y: int, cell: Cell) -> None:
        if 0 <= y < len(grid) and 0 <= x < len(grid[y]):
            grid[y][x] = cell


def _cell(character: str, colour: int) -> str:
    if colour == PLAIN:
        return character
    return f"\x1b[{30 + colour}m{character}{RESET}"
