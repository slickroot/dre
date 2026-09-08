import unittest

from sketch.layout import Placement
from sketch.render import (
    ANSI_COLOURS,
    OPAQUE,
    PLAIN_COLOUR,
    GraphicsRenderer,
)
from sketch.state import PLAIN, Arrow, Box, Cursor


class FakeText:
    def __init__(self, lines):
        self.lines = lines

    def render(self, placements, cols, rows):
        return list(self.lines)


class FakeGraphics:
    def __init__(self, payload="<payload>"):
        self.payload = payload
        self.sprites = None

    def draw(self, sprites):
        self.sprites = sprites
        return self.payload


class GraphicsRendererTest(unittest.TestCase):
    def test_payload_is_appended_to_the_last_line(self):
        graphics = FakeGraphics()
        renderer = GraphicsRenderer(
            FakeText(["one", "two"]), graphics, cell_width=2, cell_height=4
        )
        self.assertEqual(
            renderer.render([], cols=3, rows=2), ["one", "two<payload>"]
        )

    def sprites(self, placements, cols=40, rows=20, cell=(2, 4)):
        graphics = FakeGraphics()
        renderer = GraphicsRenderer(
            FakeText(["line"]), graphics, cell_width=cell[0], cell_height=cell[1]
        )
        renderer.render(placements, cols, rows)
        return graphics.sprites

    def only_sprite(self, placement, cols=40, rows=20, cell=(2, 4)):
        sprites = self.sprites([placement], cols, rows, cell)
        self.assertEqual(len(sprites), 1)
        return sprites[0]

    def pixel(self, sprite, x, y):
        start = (y * sprite.width + x) * 4
        return tuple(sprite.pixels[start : start + 4])

    def test_line_count_is_unchanged(self):
        renderer = GraphicsRenderer(
            FakeText(["one", "two", "three"]),
            FakeGraphics(),
            cell_width=2,
            cell_height=4,
        )
        self.assertEqual(len(renderer.render([], cols=3, rows=3)), 3)

    def test_sprite_covers_the_placement_in_pixels(self):
        sprite = self.only_sprite(
            Placement(Box("hi"), x=1, y=2, width=4, height=3), cell=(6, 12)
        )
        self.assertEqual((sprite.width, sprite.height), (24, 36))

    def test_sprite_sits_at_the_placement_cell(self):
        sprite = self.only_sprite(Placement(Box("hi"), x=1, y=2, width=4, height=3))
        self.assertEqual((sprite.col, sprite.row), (1, 2))

    def test_edges_are_opaque_and_the_inside_is_transparent(self):
        sprite = self.only_sprite(Placement(Box("hi"), x=0, y=0, width=3, height=2))
        for x in range(sprite.width):
            self.assertEqual(self.pixel(sprite, x, 0)[3], OPAQUE)
            self.assertEqual(self.pixel(sprite, x, sprite.height - 1)[3], OPAQUE)
        for y in range(sprite.height):
            self.assertEqual(self.pixel(sprite, 0, y)[3], OPAQUE)
            self.assertEqual(self.pixel(sprite, sprite.width - 1, y)[3], OPAQUE)
        for y in range(1, sprite.height - 1):
            for x in range(1, sprite.width - 1):
                self.assertEqual(self.pixel(sprite, x, y), (0, 0, 0, 0))

    def test_border_takes_the_colour_of_its_ansi_index(self):
        for index, rgb in enumerate(ANSI_COLOURS):
            sprite = self.only_sprite(
                Placement(Box("hi", colour=index), x=0, y=0, width=2, height=2)
            )
            self.assertEqual(self.pixel(sprite, 0, 0), rgb + (OPAQUE,))

    def test_a_plain_border_is_grey(self):
        sprite = self.only_sprite(
            Placement(Box("hi", colour=PLAIN), x=0, y=0, width=2, height=2)
        )
        self.assertEqual(self.pixel(sprite, 0, 0), PLAIN_COLOUR + (OPAQUE,))

    def test_an_arrow_has_no_sprite(self):
        self.assertEqual(
            self.sprites([Placement(Arrow(), x=1, y=1, width=1, height=1)]), []
        )

    def test_a_cursor_has_no_sprite(self):
        self.assertEqual(
            self.sprites([Placement(Cursor(), x=1, y=1, width=1, height=1)]), []
        )

    def test_a_box_overhanging_the_left_is_cropped(self):
        sprite = self.only_sprite(
            Placement(Box("hi"), x=-2, y=1, width=5, height=3), cols=40, rows=20
        )
        self.assertEqual(sprite.col, 0)
        self.assertEqual(sprite.width, 3 * 2)
        self.assertEqual(len(sprite.pixels), sprite.width * sprite.height * 4)
        self.assertEqual(self.pixel(sprite, 0, 1)[3], 0)

    def test_a_box_overhanging_the_top_is_cropped(self):
        sprite = self.only_sprite(Placement(Box("hi"), x=1, y=-1, width=4, height=3))
        self.assertEqual(sprite.row, 0)
        self.assertEqual(sprite.height, 2 * 4)
        self.assertEqual(len(sprite.pixels), sprite.width * sprite.height * 4)
        self.assertEqual(self.pixel(sprite, 1, 0)[3], 0)

    def test_a_box_overhanging_the_right_is_cropped(self):
        sprite = self.only_sprite(
            Placement(Box("hi"), x=3, y=0, width=4, height=2), cols=5, rows=20
        )
        self.assertEqual(sprite.col, 3)
        self.assertEqual(sprite.width, 2 * 2)
        self.assertEqual(len(sprite.pixels), sprite.width * sprite.height * 4)
        self.assertEqual(self.pixel(sprite, sprite.width - 1, 1)[3], 0)

    def test_a_box_overhanging_the_bottom_is_cropped(self):
        sprite = self.only_sprite(
            Placement(Box("hi"), x=0, y=1, width=2, height=4), cols=40, rows=3
        )
        self.assertEqual(sprite.row, 1)
        self.assertEqual(sprite.height, 2 * 4)
        self.assertEqual(len(sprite.pixels), sprite.width * sprite.height * 4)
        self.assertEqual(self.pixel(sprite, 1, sprite.height - 1)[3], 0)

    def test_a_box_off_screen_has_no_sprite(self):
        self.assertEqual(
            self.sprites(
                [Placement(Box("hi"), x=10, y=0, width=4, height=3)], cols=5, rows=20
            ),
            [],
        )
