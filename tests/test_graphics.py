import unittest
from math import cos, radians, tan

from sketch.layout import Arrow, Placement
from sketch.render import (
    ARROWHEAD_ANGLE_DEG,
    ARROWHEAD_DEPTH,
    ARROWHEAD_EDGE_LENGTH,
    ARROWHEAD_SLOPE,
    BORDER,
    CACHE_LIMIT,
    PALETTE,
    OPAQUE,
    PLAIN_COLOUR,
    GraphicsRenderer,
)
from sketch.state import PLAIN, Box, Cursor


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
        sprite = self.only_sprite(
            Placement(Box("hi"), x=0, y=0, width=3, height=3), cell=(4, 4)
        )
        for x in range(sprite.width):
            self.assertEqual(self.pixel(sprite, x, 0)[3], OPAQUE)
            self.assertEqual(self.pixel(sprite, x, sprite.height - 1)[3], OPAQUE)
        for y in range(sprite.height):
            self.assertEqual(self.pixel(sprite, 0, y)[3], OPAQUE)
            self.assertEqual(self.pixel(sprite, sprite.width - 1, y)[3], OPAQUE)
        for y in range(BORDER, sprite.height - BORDER):
            for x in range(BORDER, sprite.width - BORDER):
                self.assertEqual(self.pixel(sprite, x, y), (0, 0, 0, 0))

    def test_border_takes_the_colour_of_its_palette_index(self):
        for index, rgb in enumerate(PALETTE):
            sprite = self.only_sprite(
                Placement(Box("hi", colour=index), x=0, y=0, width=2, height=2)
            )
            self.assertEqual(self.pixel(sprite, 0, 0), rgb + (OPAQUE,))

    def test_a_plain_border_is_grey(self):
        sprite = self.only_sprite(
            Placement(Box("hi", colour=PLAIN), x=0, y=0, width=2, height=2)
        )
        self.assertEqual(self.pixel(sprite, 0, 0), PLAIN_COLOUR + (OPAQUE,))

    def test_an_arrow_sprite_covers_the_placement_in_pixels(self):
        sprite = self.only_sprite(
            Placement(Arrow((0,), 0), x=1, y=1, width=2, height=1), cell=(4, 5)
        )
        self.assertEqual((sprite.width, sprite.height), (8, 5))

    def test_a_single_stop_arrow_is_a_straight_line_across_every_column(self):
        # stops == (0,) is the degenerate case: shaft and trunk collapse onto
        # one row, so the sprite is exactly today's straight "-->" arrow.
        sprite = self.only_sprite(
            Placement(Arrow((0,), 0), x=1, y=1, width=2, height=1), cell=(4, 5)
        )
        shaft_row = sprite.height // 2
        for x in range(sprite.width):
            self.assertEqual(self.pixel(sprite, x, shaft_row)[3], OPAQUE)

    def test_a_single_stop_arrow_tip_is_a_single_pixel_at_the_right_edge(self):
        sprite = self.only_sprite(
            Placement(Arrow((0,), 0), x=1, y=1, width=2, height=1), cell=(4, 5)
        )
        shaft_row = sprite.height // 2
        tip = sprite.width - 1
        for y in range(sprite.height):
            expected = OPAQUE if y == shaft_row else 0
            self.assertEqual(self.pixel(sprite, tip, y)[3], expected)

    def test_a_single_stop_arrowhead_diagonals_are_symmetric_about_the_shaft(self):
        sprite = self.only_sprite(
            Placement(Arrow((0,), 0), x=1, y=1, width=2, height=1), cell=(4, 5)
        )
        shaft_row = sprite.height // 2
        # Base column of the head: furthest from the tip, widest spread.
        base = sprite.width - 4
        opaque_ys = [
            y
            for y in range(sprite.height)
            if y != shaft_row and self.pixel(sprite, base, y)[3] == OPAQUE
        ]
        self.assertEqual(len(opaque_ys), 2)
        top, bottom = opaque_ys
        self.assertEqual(shaft_row - top, bottom - shaft_row)
        self.assertGreater(bottom, shaft_row)

    def test_arrowhead_shape_matches_the_thirty_degree_geometry(self):
        sprite = self.only_sprite(
            Placement(Arrow((0,), 0), x=1, y=1, width=3, height=1), cell=(40, 20)
        )
        shaft_row = sprite.height // 2
        tip = sprite.width - 1

        def spread_at(distance_from_tip):
            opaque = [
                y
                for y in range(sprite.height)
                if y != shaft_row
                and self.pixel(sprite, tip - distance_from_tip, y)[3] == OPAQUE
            ]
            return abs(opaque[0] - shaft_row) if opaque else 0

        depth = int(ARROWHEAD_EDGE_LENGTH * cos(radians(ARROWHEAD_ANGLE_DEG)))
        expected = [
            round(d * tan(radians(ARROWHEAD_ANGLE_DEG))) for d in range(depth)
        ]
        self.assertEqual([spread_at(d) for d in range(depth)], expected)

    def test_arrow_off_shape_pixels_are_transparent(self):
        sprite = self.only_sprite(
            Placement(Arrow((0,), 0), x=1, y=1, width=2, height=1), cell=(4, 5)
        )
        self.assertEqual(self.pixel(sprite, 0, 0), (0, 0, 0, 0))

    def test_an_arrow_is_plain_grey(self):
        sprite = self.only_sprite(
            Placement(Arrow((0,), 0), x=1, y=1, width=2, height=1), cell=(4, 5)
        )
        shaft_row = sprite.height // 2
        self.assertEqual(
            self.pixel(sprite, 0, shaft_row), PLAIN_COLOUR + (OPAQUE,)
        )

    def test_an_arrow_overhanging_the_top_is_cropped(self):
        sprite = self.only_sprite(
            Placement(Arrow((0, 3), 0), x=1, y=-1, width=2, height=4), cell=(4, 5)
        )
        self.assertEqual(sprite.row, 0)
        self.assertEqual(sprite.height, 15)
        self.assertEqual(len(sprite.pixels), sprite.width * sprite.height * 4)

    def test_an_arrow_overhanging_the_bottom_is_cropped(self):
        sprite = self.only_sprite(
            Placement(Arrow((0, 3), 0), x=1, y=1, width=2, height=4),
            cols=40,
            rows=2,
            cell=(4, 5),
        )
        self.assertEqual(sprite.row, 1)
        self.assertEqual(sprite.height, 5)
        self.assertEqual(len(sprite.pixels), sprite.width * sprite.height * 4)

    def test_a_branching_arrow_trunk_spans_from_the_shaft_to_the_last_stop(self):
        sprite = self.only_sprite(
            Placement(Arrow((0, 3), 0), x=1, y=1, width=2, height=4), cell=(4, 5)
        )
        midpoint = sprite.width // 2
        shaft_row = 5 // 2  # centre of the parent's own row
        last_stop_row = 3 * 5 + 5 // 2
        for y in range(shaft_row, last_stop_row + 1):
            self.assertEqual(self.pixel(sprite, midpoint, y)[3], OPAQUE)

    def test_a_branching_arrow_has_a_stub_at_every_stop(self):
        sprite = self.only_sprite(
            Placement(Arrow((0, 3), 0), x=1, y=1, width=2, height=4), cell=(4, 5)
        )
        midpoint = sprite.width // 2
        for stop in (0, 3):
            row = stop * 5 + 5 // 2
            for x in range(midpoint, sprite.width):
                self.assertEqual(self.pixel(sprite, x, row)[3], OPAQUE)

    def test_a_branching_arrow_rows_between_stops_are_blank_past_the_trunk(self):
        sprite = self.only_sprite(
            Placement(Arrow((0, 3), 0), x=1, y=1, width=2, height=4), cell=(4, 5)
        )
        midpoint = sprite.width // 2
        row_between_stops = 1 * 5 + 5 // 2
        for x in range(midpoint + 1, sprite.width):
            self.assertEqual(self.pixel(sprite, x, row_between_stops)[3], 0)

    def test_three_stubs_each_end_in_their_own_arrowhead(self):
        sprite = self.only_sprite(
            Placement(Arrow((0, 1, 3), 0), x=1, y=1, width=2, height=4), cell=(4, 5)
        )
        tip = sprite.width - 1
        stop_rows = {stop * 5 + 5 // 2 for stop in (0, 1, 3)}
        for y in range(sprite.height):
            expected = OPAQUE if y in stop_rows else 0
            self.assertEqual(self.pixel(sprite, tip, y)[3], expected)

    def test_a_cursor_has_no_sprite(self):
        self.assertEqual(
            self.sprites([Placement(Cursor(), x=1, y=1, width=1, height=1)]), []
        )

    def test_a_box_overhanging_the_left_is_cropped(self):
        sprite = self.only_sprite(
            Placement(Box("hi"), x=-2, y=1, width=5, height=3),
            cols=40,
            rows=20,
            cell=(4, 4),
        )
        self.assertEqual(sprite.col, 0)
        self.assertEqual(sprite.width, 3 * 4)
        self.assertEqual(len(sprite.pixels), sprite.width * sprite.height * 4)
        self.assertEqual(self.pixel(sprite, 0, sprite.height // 2)[3], 0)

    def test_a_box_overhanging_the_top_is_cropped(self):
        sprite = self.only_sprite(
            Placement(Box("hi"), x=1, y=-2, width=4, height=5), cell=(4, 4)
        )
        self.assertEqual(sprite.row, 0)
        self.assertEqual(sprite.height, 3 * 4)
        self.assertEqual(len(sprite.pixels), sprite.width * sprite.height * 4)
        self.assertEqual(self.pixel(sprite, sprite.width // 2, 0)[3], 0)

    def test_a_box_overhanging_the_right_is_cropped(self):
        sprite = self.only_sprite(
            Placement(Box("hi"), x=1, y=0, width=6, height=3),
            cols=4,
            rows=20,
            cell=(4, 4),
        )
        self.assertEqual(sprite.col, 1)
        self.assertEqual(sprite.width, 3 * 4)
        self.assertEqual(len(sprite.pixels), sprite.width * sprite.height * 4)
        self.assertEqual(
            self.pixel(sprite, sprite.width - 1, sprite.height // 2)[3], 0
        )

    def test_a_box_overhanging_the_bottom_is_cropped(self):
        sprite = self.only_sprite(
            Placement(Box("hi"), x=0, y=1, width=3, height=6),
            cols=40,
            rows=4,
            cell=(4, 4),
        )
        self.assertEqual(sprite.row, 1)
        self.assertEqual(sprite.height, 3 * 4)
        self.assertEqual(len(sprite.pixels), sprite.width * sprite.height * 4)
        self.assertEqual(
            self.pixel(sprite, sprite.width // 2, sprite.height - 1)[3], 0
        )

    def test_a_box_off_screen_has_no_sprite(self):
        self.assertEqual(
            self.sprites(
                [Placement(Box("hi"), x=10, y=0, width=4, height=3)], cols=5, rows=20
            ),
            [],
        )


class ArrowheadConstantsTest(unittest.TestCase):
    def test_depth_matches_the_edge_length_and_angle(self):
        self.assertEqual(
            ARROWHEAD_DEPTH,
            ARROWHEAD_EDGE_LENGTH * cos(radians(ARROWHEAD_ANGLE_DEG)),
        )

    def test_slope_matches_the_angle(self):
        self.assertEqual(ARROWHEAD_SLOPE, tan(radians(ARROWHEAD_ANGLE_DEG)))


class SpriteCacheTest(unittest.TestCase):
    def setUp(self):
        self.renderer = GraphicsRenderer(
            FakeText(["line"]), FakeGraphics(), cell_width=2, cell_height=4
        )

    def draw(self, placement, cols=40, rows=20):
        return self.renderer._sprites([placement], cols, rows)[0]

    def test_an_unchanged_box_is_not_redrawn(self):
        placement = Placement(Box("hi", colour=1, fill=2), x=0, y=0, width=4, height=3)
        first = self.draw(placement)
        self.assertEqual(len(self.renderer.cache), 1)
        self.assertEqual(self.draw(placement).pixels, first.pixels)
        self.assertEqual(len(self.renderer.cache), 1)

    def test_a_recoloured_box_is_redrawn(self):
        plain = self.draw(Placement(Box("hi"), x=0, y=0, width=4, height=3))
        blue = self.draw(Placement(Box("hi", colour=4), x=0, y=0, width=4, height=3))
        self.assertNotEqual(plain.pixels, blue.pixels)
        self.assertEqual(len(self.renderer.cache), 2)

    def test_a_refilled_box_is_redrawn(self):
        plain = self.draw(Placement(Box("hi"), x=0, y=0, width=6, height=3))
        filled = self.draw(Placement(Box("hi", fill=3), x=0, y=0, width=6, height=3))
        self.assertNotEqual(plain.pixels, filled.pixels)

    def test_rounded_and_square_are_cached_distinctly(self):
        for rounded in (False, True):
            self.draw(
                Placement(Box("hi", rounded=rounded), x=0, y=0, width=4, height=3)
            )
        self.assertEqual(len(self.renderer.cache), 2)

    def test_a_relabelled_box_of_the_same_size_reuses_its_pixels(self):
        first = self.draw(Placement(Box("hi"), x=0, y=0, width=4, height=3))
        second = self.draw(Placement(Box("ok"), x=0, y=0, width=4, height=3))
        self.assertEqual(second.pixels, first.pixels)
        self.assertEqual(len(self.renderer.cache), 1)

    def test_a_cached_sprite_moves_to_its_own_position(self):
        self.draw(Placement(Box("hi"), x=0, y=0, width=4, height=3))
        moved = self.draw(Placement(Box("hi"), x=5, y=2, width=4, height=3))
        self.assertEqual((moved.col, moved.row), (5, 2))

    def test_a_differently_cropped_box_is_redrawn(self):
        whole = self.draw(Placement(Box("hi"), x=0, y=0, width=4, height=3))
        cropped = self.draw(
            Placement(Box("hi"), x=0, y=0, width=4, height=3), cols=2, rows=20
        )
        self.assertNotEqual(whole.width, cropped.width)
        self.assertEqual(len(self.renderer.cache), 2)

    def test_arrows_with_different_stops_are_redrawn(self):
        one = self.draw(Placement(Arrow((0,), 0), x=0, y=0, width=4, height=6))
        two = self.draw(Placement(Arrow((0, 2), 0), x=0, y=0, width=4, height=6))
        self.assertNotEqual(one.pixels, two.pixels)

    def test_arrows_with_different_shafts_are_redrawn(self):
        one = self.draw(Placement(Arrow((0, 2), 0), x=0, y=0, width=4, height=6))
        two = self.draw(Placement(Arrow((0, 2), 1), x=0, y=0, width=4, height=6))
        self.assertNotEqual(one.pixels, two.pixels)

    def test_the_cache_is_bounded(self):
        for width in range(CACHE_LIMIT + 2):
            self.renderer._sprites(
                [Placement(Box("hi"), x=0, y=0, width=width + 1, height=3)], 4000, 20
            )
        self.assertLessEqual(len(self.renderer.cache), CACHE_LIMIT)
