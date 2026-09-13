from dataclasses import dataclass, replace
from math import cos, hypot, radians, tan
from typing import Dict, List, Protocol, Tuple

from .layout import Arrow, Label, Placement
from .state import PLAIN, Box, Cursor

TOP_LEFT = "┌"
TOP_RIGHT = "┐"
BOTTOM_LEFT = "└"
BOTTOM_RIGHT = "┘"
HORIZONTAL = "─"
VERTICAL = "│"
BLANK = " "
CURSOR = "\u2588"

ARROW_STROKE = 4
ARROWHEAD_ANGLE_DEG = 30
ARROWHEAD_EDGE_LENGTH = 15
# The arrowhead is a fixed shape, so its depth and slope are constants
# rather than trigonometry repeated for every pixel.
ARROWHEAD_DEPTH = ARROWHEAD_EDGE_LENGTH * cos(radians(ARROWHEAD_ANGLE_DEG))
ARROWHEAD_SLOPE = tan(radians(ARROWHEAD_ANGLE_DEG))
RESET = "\x1b[0m"

Cell = Tuple[str, int, int]
Grid = List[List[Cell]]
Key = Tuple

# Distinct shapes on a board are few; this only bounds a pathological run.
CACHE_LIMIT = 512

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
        self.cache: Dict[Key, Sprite] = {}

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
            sprites.append(self._sprite(placement, left, top, right, bottom))
        return sprites

    def _sprite(
        self, placement: Placement, left: int, top: int, right: int, bottom: int
    ) -> Sprite:
        # A keystroke changes one box, so most sprites are pixel for pixel what
        # they were last frame. Only the shape and the colours reach the pixels,
        # never the label or the position on screen.
        key = _key(placement, left, top, right, bottom)
        drawn = self.cache.get(key)
        if drawn is None:
            if isinstance(placement.node, Box):
                drawn = self._outline_box(placement, left, top, right, bottom)
            else:
                drawn = self._outline_arrow(placement, left, top, right, bottom)
            if len(self.cache) >= CACHE_LIMIT:
                self.cache.clear()
            self.cache[key] = drawn
        return replace(drawn, col=left, row=top)

    def _outline_box(
        self, placement: Placement, left: int, top: int, right: int, bottom: int
    ) -> Sprite:
        width = placement.width * self.cell_width
        height = placement.height * self.cell_height
        border = placement.node.border
        edge = _colour(placement.node.colour) + (OPAQUE,)
        fill = _fill_colour(placement.node.fill)
        first_x = (left - placement.x) * self.cell_width
        last_x = (right - placement.x) * self.cell_width
        first_y = (top - placement.y) * self.cell_height
        last_y = (bottom - placement.y) * self.cell_height
        span = last_x - first_x
        radius = placement.node.radius
        if radius:
            pixels = RoundedBox(
                width, height, radius, border, edge, fill
            ).pixels(first_x, last_x, first_y, last_y)
        else:
            pixels = _square_pixels(
                width, height, border, edge, fill,
                first_x, last_x, first_y, last_y,
            )
        return Sprite(
            pixels=pixels,
            width=span,
            height=last_y - first_y,
            col=left,
            row=top,
        )

    def _outline_arrow(
        self, placement: Placement, left: int, top: int, right: int, bottom: int
    ) -> Sprite:
        width = placement.width * self.cell_width
        # A stop is a child centre-row relative to the sprite's top, in the
        # same row units as box placements; the pixel row of its centre is
        # the row's own midpoint.
        stop_rows = [
            stop * self.cell_height + self.cell_height // 2
            for stop in placement.node.stops
        ]
        shaft_row = (
            placement.node.shaft * self.cell_height + self.cell_height // 2
        )
        trunk_top = min(stop_rows)
        trunk_bottom = max(stop_rows)
        midpoint = width // 2
        ink = bytes(_colour(PLAIN) + (OPAQUE,))
        first_x = (left - placement.x) * self.cell_width
        last_x = (right - placement.x) * self.cell_width
        first_y = (top - placement.y) * self.cell_height
        last_y = (bottom - placement.y) * self.cell_height
        # The arrow is a handful of lines on a transparent field, so lay the
        # field down once and stroke only the lit pixels.
        canvas = Canvas(first_x, last_x, first_y, last_y, ink)
        canvas.horizontal(shaft_row, 0, midpoint, width=ARROW_STROKE)
        canvas.vertical(midpoint, trunk_top, trunk_bottom, width=ARROW_STROKE)
        for stop_row in stop_rows:
            canvas.horizontal(stop_row, midpoint, width - 1, width=ARROW_STROKE)
            self._arrowhead(canvas, stop_row, midpoint, width - 1)
        return Sprite(
            pixels=canvas.pixels(),
            width=last_x - first_x,
            height=last_y - first_y,
            col=left,
            row=top,
        )

    def _arrowhead(
        self, canvas: "Canvas", stop_row: int, midpoint: int, right_edge: int
    ) -> None:
        for distance in range(int(ARROWHEAD_DEPTH) + 1):
            if distance >= ARROWHEAD_DEPTH:
                break
            x = right_edge - distance
            if x < midpoint:
                break
            spread = round(distance * ARROWHEAD_SLOPE)
            canvas.point(x, stop_row - spread, width=ARROW_STROKE)
            canvas.point(x, stop_row + spread, width=ARROW_STROKE)


class RoundedBox:
    """A box whose corners are cut from a rounded-rectangle distance field."""

    def __init__(
        self,
        width: int,
        height: int,
        radius: int,
        border: int,
        edge: Tuple[int, int, int, int],
        fill: Tuple[int, int, int, int],
    ) -> None:
        self.width = width
        self.height = height
        self.border = border
        self.radius = radius
        self.outer = min(radius + border, width // 2, height // 2)
        self.edge = bytes(edge)
        self.fill = bytes(fill)
        self.clear = bytes(TRANSPARENT)

    def pixels(
        self, first_x: int, last_x: int, first_y: int, last_y: int
    ) -> bytes:
        buffer = bytearray()
        straight_row = None
        for y in range(first_y, last_y):
            if self.outer <= y < self.height - self.outer:
                if straight_row is None:
                    straight_row = _body_row(
                        self.width, self.border, self.edge, self.fill,
                        first_x, last_x,
                    )
                buffer.extend(straight_row)
            else:
                buffer.extend(self._corner_row(y, first_x, last_x))
        return bytes(buffer)

    def _corner_row(self, y: int, first_x: int, last_x: int) -> bytes:
        row = bytearray()
        for x in range(first_x, min(self.outer, last_x)):
            row.extend(self._pixel(x, y))
        middle = min(self.width - self.outer, last_x) - max(
            self.outer, first_x
        )
        if middle > 0:
            straight = (
                self.edge
                if y < self.border or y >= self.height - self.border
                else self.fill
            )
            row.extend(straight * middle)
        for x in range(max(self.width - self.outer, first_x), last_x):
            row.extend(self._pixel(x, y))
        return bytes(row)

    def _pixel(self, x: int, y: int) -> bytes:
        px = x + 0.5
        py = y + 0.5
        outer_coverage = self._coverage(
            px, py, self.width, self.height, self.outer
        )
        inner_coverage = self._coverage(
            px - self.border,
            py - self.border,
            self.width - 2 * self.border,
            self.height - 2 * self.border,
            self.radius,
        )
        edge_coverage = outer_coverage - inner_coverage
        alpha = edge_coverage * self.edge[3] + inner_coverage * self.fill[3]
        if not alpha:
            return self.clear
        channels = [
            round(
                (
                    self.edge[channel] * edge_coverage * self.edge[3]
                    + self.fill[channel] * inner_coverage * self.fill[3]
                )
                / alpha
            )
            for channel in range(3)
        ]
        return bytes(channels + [round(alpha)])

    @staticmethod
    def _coverage(
        px: float, py: float, width: int, height: int, radius: int
    ) -> float:
        half_x = width / 2
        half_y = height / 2
        qx = abs(px - half_x) - (half_x - radius)
        qy = abs(py - half_y) - (half_y - radius)
        distance = (
            hypot(max(qx, 0), max(qy, 0))
            + min(max(qx, qy), 0)
            - radius
        )
        return min(max(0.5 - distance, 0), 1)


def _centered_span(c: int, width: int) -> range:
    start = c - (width - 1) // 2
    return range(start, start + width)


class Canvas:
    """A clipped, transparent pixel field that lines are stroked onto."""

    def __init__(
        self, first_x: int, last_x: int, first_y: int, last_y: int, ink: bytes
    ) -> None:
        self.first_x = first_x
        self.last_x = last_x
        self.first_y = first_y
        self.last_y = last_y
        self.ink = ink
        self.span = last_x - first_x
        self.buffer = bytearray(
            bytes(TRANSPARENT) * (self.span * (last_y - first_y))
        )

    def point(self, x: int, y: int, width: int) -> None:
        for px in _centered_span(x, width):
            for py in _centered_span(y, width):
                if (
                    self.first_x <= px < self.last_x
                    and self.first_y <= py < self.last_y
                ):
                    start = self._offset(px, py)
                    self.buffer[start : start + 4] = self.ink

    def horizontal(self, y: int, x0: int, x1: int, width: int) -> None:
        start_x = max(x0, self.first_x)
        stop_x = min(x1 + 1, self.last_x)
        if start_x >= stop_x:
            return
        for py in _centered_span(y, width):
            if not self.first_y <= py < self.last_y:
                continue
            start = self._offset(start_x, py)
            self.buffer[start : start + (stop_x - start_x) * 4] = self.ink * (
                stop_x - start_x
            )

    def vertical(self, x: int, y0: int, y1: int, width: int) -> None:
        for px in _centered_span(x, width):
            if not self.first_x <= px < self.last_x:
                continue
            for y in range(max(y0, self.first_y), min(y1 + 1, self.last_y)):
                start = self._offset(px, y)
                self.buffer[start : start + 4] = self.ink

    def pixels(self) -> bytes:
        return bytes(self.buffer)

    def _offset(self, x: int, y: int) -> int:
        return ((y - self.first_y) * self.span + (x - self.first_x)) * 4


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
            elif isinstance(placement.node, Label):
                self._draw_label(grid, placement)
        return ["".join(_cell(*cell) for cell in row) for row in grid]

    def _draw_box(self, grid: Grid, placement: Placement) -> None:
        for y in range(placement.y, placement.y + placement.height):
            for x in range(placement.x, placement.x + placement.width):
                self._put(grid, x, y, BLANK_CELL)

    def _draw_cursor(self, grid: Grid, placement: Placement) -> None:
        self._stamp(grid, placement.x, placement.y, CURSOR)

    def _draw_label(self, grid: Grid, placement: Placement) -> None:
        for offset, character in enumerate(placement.node.text):
            self._stamp(grid, placement.x + offset, placement.y, character)

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

    def _stamp(self, grid: Grid, x: int, y: int, character: str) -> None:
        if 0 <= y < len(grid) and 0 <= x < len(grid[y]):
            _, colour, fill = grid[y][x]
            grid[y][x] = (character, colour, fill)


def _square_pixels(
    width: int,
    height: int,
    border: int,
    edge: Tuple[int, int, int, int],
    fill: Tuple[int, int, int, int],
    first_x: int,
    last_x: int,
    first_y: int,
    last_y: int,
) -> bytes:
    # A square box has only two kinds of row, so build each once and repeat it
    # rather than deciding pixel by pixel.
    edge_row = bytes(edge) * (last_x - first_x)
    body_row = _body_row(width, border, edge, fill, first_x, last_x)
    pixels = bytearray()
    if height <= 2 * border:
        pixels.extend(edge_row * (last_y - first_y))
    else:
        top_edge = max(0, border - first_y)
        pixels.extend(edge_row * top_edge)
        bottom_edge = max(0, border - (height - last_y))
        body_count = (last_y - first_y) - top_edge - bottom_edge
        pixels.extend(body_row * body_count)
        pixels.extend(edge_row * bottom_edge)
    return bytes(pixels)


def _body_row(
    width: int,
    border: int,
    edge: Tuple[int, int, int, int],
    fill: Tuple[int, int, int, int],
    first_x: int,
    last_x: int,
) -> bytes:
    span = last_x - first_x
    if width <= 2 * border:
        return bytes(edge) * span
    row = bytearray()
    left_edge = max(0, border - first_x)
    row.extend(bytes(edge) * left_edge)
    right_edge = max(0, border - (width - last_x))
    row.extend(bytes(fill) * (span - left_edge - right_edge))
    row.extend(bytes(edge) * right_edge)
    return bytes(row)


def _key(
    placement: Placement, left: int, top: int, right: int, bottom: int
) -> Key:
    node = placement.node
    shape = (
        (node.colour, node.fill, node.border, node.radius)
        if isinstance(node, Box)
        else (node.stops, node.shaft)
    )
    return (
        type(node),
        placement.width,
        placement.height,
        left - placement.x,
        top - placement.y,
        right - placement.x,
        bottom - placement.y,
        shape,
    )


def _colour(colour: int) -> Tuple[int, int, int]:
    if colour == PLAIN:
        return PLAIN_COLOUR
    return PALETTE[colour]


def _fill_colour(fill: int) -> Tuple[int, int, int, int]:
    if fill == PLAIN:
        return TRANSPARENT
    return tuple(
        round(channel * FILL_ALPHA / OPAQUE) for channel in PALETTE[fill]
    ) + (OPAQUE,)


def _cell(character: str, colour: int, fill: int) -> str:
    codes = []
    if colour != PLAIN:
        codes.append(30 + colour)
    if fill != PLAIN:
        codes.append(40 + fill)
    if not codes:
        return character
    return f"\x1b[{';'.join(str(code) for code in codes)}m{character}{RESET}"
