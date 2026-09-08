from dataclasses import dataclass
from typing import List, Protocol

from .layout import Placement
from .state import PLAIN, Arrow, Box, Cursor

TOP_LEFT = "┌"
TOP_RIGHT = "┐"
BOTTOM_LEFT = "└"
BOTTOM_RIGHT = "┘"
HORIZONTAL = "─"
VERTICAL = "│"
BLANK = " "
CURSOR = "\u2588"
ARROW_DOWN = "\u2193"
ARROW_UP = "\u2191"
RESET = "\x1b[0m"

@dataclass(frozen=True)
class Cell:
    character: str = BLANK
    colour: int = PLAIN
    fill: int = PLAIN


Grid = List[List[Cell]]

BLANK_CELL = Cell()


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
            elif isinstance(placement.node, Arrow):
                self._draw_arrow(grid, placement)
        return ["".join(_cell(cell) for cell in row) for row in grid]

    def _draw_box(self, grid: Grid, placement: Placement) -> None:
        left = placement.x
        right = placement.x + placement.width - 1
        top = placement.y
        bottom = placement.y + placement.height - 1
        colour = placement.node.colour
        fill = placement.node.fill
        for y in range(top, bottom + 1):
            for x in range(left, right + 1):
                character = self._box_character(x, y, left, right, top, bottom)
                if character == BLANK:
                    self._put(grid, x, y, Cell(BLANK, fill=fill))
                else:
                    self._put(grid, x, y, Cell(character, colour))
        self._draw_label(grid, placement)

    def _draw_cursor(self, grid: Grid, placement: Placement) -> None:
        self._put(grid, placement.x, placement.y, Cell(CURSOR))

    def _draw_arrow(self, grid: Grid, placement: Placement) -> None:
        glyph = ARROW_DOWN if placement.node.direction == "forward" else ARROW_UP
        self._put(grid, placement.x, placement.y, Cell(glyph))

    def _draw_label(self, grid: Grid, placement: Placement) -> None:
        fill = placement.node.fill
        for offset, character in enumerate(placement.node.label):
            self._put(
                grid,
                placement.x + 1 + offset,
                placement.y + 1,
                Cell(character, fill=fill),
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


def _cell(cell: Cell) -> str:
    codes = []
    if cell.colour != PLAIN:
        codes.append(30 + cell.colour)
    if cell.fill != PLAIN:
        codes.append(40 + cell.fill)
    if not codes:
        return cell.character
    return f"\x1b[{';'.join(str(code) for code in codes)}m{cell.character}{RESET}"
