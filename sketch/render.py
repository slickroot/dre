from typing import List, Protocol

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


class Renderer(Protocol):
    def render(
        self, placements: List[Placement], cols: int, rows: int
    ) -> List[str]:
        ...


class TerminalRenderer:
    def render(
        self, placements: List[Placement], cols: int, rows: int
    ) -> List[str]:
        chars = [[BLANK] * cols for _ in range(rows)]
        colours = [[PLAIN] * cols for _ in range(rows)]
        for placement in placements:
            if isinstance(placement.node, Box):
                self._draw_box(chars, colours, placement)
            elif isinstance(placement.node, Cursor):
                self._draw_cursor(chars, colours, placement)
        return [
            "".join(
                _cell(character, colour)
                for character, colour in zip(row, colour_row)
            )
            for row, colour_row in zip(chars, colours)
        ]

    def _draw_box(
        self,
        chars: List[List[str]],
        colours: List[List[int]],
        placement: Placement,
    ) -> None:
        left = placement.x
        right = placement.x + placement.width - 1
        top = placement.y
        bottom = placement.y + placement.height - 1
        box_colour = placement.node.colour
        for y in range(top, bottom + 1):
            for x in range(left, right + 1):
                character = self._box_character(x, y, left, right, top, bottom)
                colour = box_colour if character != BLANK else PLAIN
                self._put(chars, colours, x, y, character, colour)
        self._draw_label(chars, colours, placement)

    def _draw_cursor(
        self,
        chars: List[List[str]],
        colours: List[List[int]],
        placement: Placement,
    ) -> None:
        self._put(chars, colours, placement.x, placement.y, CURSOR, PLAIN)

    def _draw_label(
        self,
        chars: List[List[str]],
        colours: List[List[int]],
        placement: Placement,
    ) -> None:
        for offset, character in enumerate(placement.node.label):
            self._put(
                chars,
                colours,
                placement.x + 1 + offset,
                placement.y + 1,
                character,
                PLAIN,
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

    def _put(
        self,
        chars: List[List[str]],
        colours: List[List[int]],
        x: int,
        y: int,
        character: str,
        colour: int,
    ) -> None:
        if 0 <= y < len(chars) and 0 <= x < len(chars[y]):
            chars[y][x] = character
            colours[y][x] = colour


def _cell(character: str, colour: int) -> str:
    if colour == PLAIN:
        return character
    return f"\x1b[{30 + colour}m{character}{RESET}"
