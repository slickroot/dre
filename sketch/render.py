from dataclasses import dataclass
from math import cos, radians, tan
from typing import List, Protocol, Tuple

from .layout import Placement
from .state import PLAIN, Arrow, Box, Cursor, axis

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
ARROW_LEFT = "\u2190"
ARROW_RIGHT = "\u2192"

ARROW_GLYPHS = {
    "down": ARROW_DOWN,
    "up": ARROW_UP,
    "left": ARROW_LEFT,
    "right": ARROW_RIGHT,
}
ARROWHEAD_ANGLE_DEG = 30
ARROWHEAD_EDGE_LENGTH = 15
RESET = "\x1b[0m"

Cell = Tuple[str, int, int]
Grid = List[List[Cell]]

BLANK_CELL = (BLANK, PLAIN, PLAIN)

OPAQUE = 255
FILL_ALPHA = 77
TRANSPARENT = (0, 0, 0, 0)
PLAIN_COLOUR = (128, 128, 128)
PALETTE = (
    (255, 190, 11),
    (251, 86, 7),
    (255, 0, 110),
    (131, 56, 236),
    (58, 134, 255),
)


@dataclass(frozen=True)
class Sprite:
    pixels: bytes
    width: int
    height: int
    col: int
    row: int


class Renderer(Protocol):
    def render(
        self, placements: List[Placement], cols: int, rows: int
    ) -> List[str]:
        ...


class GraphicsProtocol(Protocol):
    def draw(self, sprites: List[Sprite]) -> str:
        ...


class GraphicsRenderer:
    def __init__(
        self,
        text: Renderer,
        graphics: GraphicsProtocol,
        cell_width: int,
        cell_height: int,
    ) -> None:
        self.text = text
        self.graphics = graphics
        self.cell_width = cell_width
        self.cell_height = cell_height

    def render(
        self, placements: List[Placement], cols: int, rows: int
    ) -> List[str]:
        lines = self.text.render(placements, cols, rows)
        payload = self.graphics.draw(self._sprites(placements, cols, rows))
        return lines[:-1] + [lines[-1] + payload]

    def _sprites(
        self, placements: List[Placement], cols: int, rows: int
    ) -> List[Sprite]:
        sprites = []
        for placement in placements:
            if not isinstance(placement.node, (Box, Arrow)):
                continue
            left = max(placement.x, 0)
            top = max(placement.y, 0)
            right = min(placement.x + placement.width, cols)
            bottom = min(placement.y + placement.height, rows)
            if left >= right or top >= bottom:
                continue
            if isinstance(placement.node, Box):
                sprites.append(self._outline_box(placement, left, top, right, bottom))
            elif isinstance(placement.node, Arrow):
                sprites.append(
                    self._outline_arrow(placement, left, top, right, bottom)
                )
        return sprites

    def _outline_box(
        self, placement: Placement, left: int, top: int, right: int, bottom: int
    ) -> Sprite:
        width = placement.width * self.cell_width
        height = placement.height * self.cell_height
        edge = _colour(placement.node.colour) + (OPAQUE,)
        fill = _fill_colour(placement.node.fill)
        first_x = (left - placement.x) * self.cell_width
        last_x = (right - placement.x) * self.cell_width
        first_y = (top - placement.y) * self.cell_height
        last_y = (bottom - placement.y) * self.cell_height
        pixels = bytearray()
        for y in range(first_y, last_y):
            for x in range(first_x, last_x):
                on_edge = x in (0, width - 1) or y in (0, height - 1)
                pixels.extend(edge if on_edge else fill)
        return Sprite(
            pixels=bytes(pixels),
            width=last_x - first_x,
            height=last_y - first_y,
            col=left,
            row=top,
        )

    def _outline_arrow(
        self, placement: Placement, left: int, top: int, right: int, bottom: int
    ) -> Sprite:
        width = placement.width * self.cell_width
        height = placement.height * self.cell_height
        direction = placement.node.direction
        vertical = axis(placement.node) == "row"
        if vertical:
            length = height
            centre = width // 2
            head_at_far_end = direction == "down"
        else:
            length = width
            centre = height // 2
            head_at_far_end = direction == "right"
        ink = _colour(PLAIN) + (OPAQUE,)
        first_x = (left - placement.x) * self.cell_width
        last_x = (right - placement.x) * self.cell_width
        first_y = (top - placement.y) * self.cell_height
        last_y = (bottom - placement.y) * self.cell_height
        pixels = bytearray()
        for y in range(first_y, last_y):
            for x in range(first_x, last_x):
                along, across = (y, x) if vertical else (x, y)
                on_arrow = self._on_arrow(
                    along, across, length, centre, head_at_far_end
                )
                pixels.extend(ink if on_arrow else TRANSPARENT)
        return Sprite(
            pixels=bytes(pixels),
            width=last_x - first_x,
            height=last_y - first_y,
            col=left,
            row=top,
        )

    def _on_arrow(
        self,
        along: int,
        across: int,
        length: int,
        centre: int,
        head_at_far_end: bool,
    ) -> bool:
        if across == centre:
            return True
        if head_at_far_end:
            distance = (length - 1) - along
        else:
            distance = along
        depth = ARROWHEAD_EDGE_LENGTH * cos(radians(ARROWHEAD_ANGLE_DEG))
        if distance < 0 or distance >= depth:
            return False
        spread = round(distance * tan(radians(ARROWHEAD_ANGLE_DEG)))
        return abs(across - centre) == spread


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
        fill = placement.node.fill
        for y in range(placement.y, placement.y + placement.height):
            for x in range(placement.x, placement.x + placement.width):
                self._put(grid, x, y, (BLANK, PLAIN, fill))
        self._draw_label(grid, placement)

    def _draw_cursor(self, grid: Grid, placement: Placement) -> None:
        self._put(grid, placement.x, placement.y, (CURSOR, PLAIN, PLAIN))

    def _draw_arrow(self, grid: Grid, placement: Placement) -> None:
        glyph = ARROW_GLYPHS[placement.node.direction]
        self._put(grid, placement.x, placement.y, (glyph, PLAIN, PLAIN))

    def _draw_label(self, grid: Grid, placement: Placement) -> None:
        fill = placement.node.fill
        for offset, character in enumerate(placement.node.label):
            self._put(
                grid,
                placement.x + 1 + offset,
                placement.y + 1,
                (character, PLAIN, fill),
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


def _colour(colour: int) -> Tuple[int, int, int]:
    if colour == PLAIN:
        return PLAIN_COLOUR
    return PALETTE[colour]


def _fill_colour(fill: int) -> Tuple[int, int, int, int]:
    return TRANSPARENT if fill == PLAIN else PALETTE[fill] + (FILL_ALPHA,)


def _cell(character: str, colour: int, fill: int) -> str:
    codes = []
    if colour != PLAIN:
        codes.append(30 + colour)
    if fill != PLAIN:
        codes.append(40 + fill)
    if not codes:
        return character
    return f"\x1b[{';'.join(str(code) for code in codes)}m{character}{RESET}"
