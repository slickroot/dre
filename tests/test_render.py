import unittest
from typing import get_args, get_type_hints

from sketch.layout import Arrow as LayoutArrow
from sketch.layout import Label, Placement
from sketch.render import (
    BLANK,
    BOTTOM_LEFT,
    BOTTOM_RIGHT,
    CURSOR,
    FILL_ALPHA,
    HORIZONTAL,
    OPAQUE,
    PALETTE,
    TOP_LEFT,
    TOP_RIGHT,
    TRANSPARENT,
    VERTICAL,
    GraphicsRenderer,
    Sprite,
    TerminalRenderer,
    _cell,
    _colour,
    _fill_colour,
)
from sketch.state import PLAIN, Box, Cursor

SQUARE, SMALL_RADIUS, LARGE_RADIUS = get_args(get_type_hints(Box)["radius"])


class TerminalRendererTest(unittest.TestCase):
    def setUp(self):
        self.renderer = TerminalRenderer()

    def test_empty_canvas_fills_terminal(self):
        self.assertEqual(
            self.renderer.render([], cols=11, rows=5), [BLANK * 11] * 5
        )

    def test_grid_matches_the_requested_size(self):
        cols, rows = 20, 7
        grid = self.renderer.render([Placement(Box(), 4, 4, 3, 3)], cols, rows)
        self.assertEqual(len(grid), rows)
        self.assertEqual({len(line) for line in grid}, {cols})

    def test_a_box_claims_its_cells_without_border_characters(self):
        grid = self.renderer.render([Placement(Box(), 4, 4, 3, 3)], 11, 11)
        self.assertEqual(grid[4][4:7], BLANK * 3)

    def test_every_row_of_a_box_is_blank(self):
        grid = self.renderer.render([Placement(Box(), 4, 4, 3, 3)], 11, 11)
        self.assertEqual([line[4:7] for line in grid[4:7]], [BLANK * 3] * 3)

    def test_box_interior_is_empty(self):
        grid = self.renderer.render([Placement(Box(), 0, 0, 5, 4)], 5, 4)
        self.assertEqual(grid, [BLANK * 5] * 4)

    def test_nothing_is_drawn_outside_the_box(self):
        fill = 2
        grid = self.renderer.render([Placement(Box(fill=fill), 4, 4, 3, 3)], 11, 11)
        self.assertEqual(grid[3], BLANK * 11)
        self.assertEqual(grid[4][: len(BLANK * 4)], BLANK * 4)
        self.assertTrue(grid[4].endswith(BLANK * 4))

    def test_a_box_reaching_past_the_edge_is_clipped(self):
        fill = 3
        grid = self.renderer.render([Placement(Box(fill=fill), 3, 1, 3, 3)], 4, 2)
        self.assertEqual(
            grid, [BLANK * 4, BLANK * 3 + _cell(BLANK, PLAIN, fill)]
        )

    def test_label_is_drawn_inside_the_box(self):
        grid = self.renderer.render(
            [
                Placement(Box("hi"), 0, 0, 5, 3),
                Placement(Label("hi"), 1, 1, 2, 1),
            ],
            5,
            3,
        )
        self.assertEqual(grid[1], BLANK + "hi" + BLANK * 2)

    def test_cursor_is_drawn_after_the_label(self):
        grid = self.renderer.render(
            [
                Placement(Box("hi"), 0, 0, 5, 3),
                Placement(Label("hi"), 1, 1, 2, 1),
                Placement(Cursor(), 3, 1, 1, 1),
            ],
            5,
            3,
        )
        self.assertEqual(grid[1], BLANK + "hi" + CURSOR + BLANK)

    def test_label_and_cursor_past_the_edge_are_clipped(self):
        grid = self.renderer.render(
            [
                Placement(Box("hi"), 0, 0, 5, 3),
                Placement(Label("hi"), 1, 1, 2, 1),
                Placement(Cursor(), 3, 1, 1, 1),
            ],
            3,
            3,
        )
        self.assertEqual(grid[1], BLANK + "hi")

    def test_a_box_does_not_draw_a_cursor(self):
        grid = self.renderer.render([Placement(Box("hi"), 0, 0, 5, 3)], 5, 3)
        self.assertNotIn(CURSOR, "".join(grid))

    def test_cursor_placement_is_drawn_at_its_own_position(self):
        grid = self.renderer.render([Placement(Cursor(), 2, 1, 1, 1)], 4, 3)
        self.assertEqual(
            grid, [BLANK * 4, BLANK * 2 + CURSOR + BLANK, BLANK * 4]
        )

    def test_a_cursor_outside_the_grid_is_clipped(self):
        grid = self.renderer.render([Placement(Cursor(), 9, 9, 1, 1)], 4, 3)
        self.assertEqual(grid, [BLANK * 4] * 3)

    def test_each_placement_is_drawn(self):
        first_fill, second_fill = 1, 2
        grid = self.renderer.render(
            [
                Placement(Box(fill=first_fill), 0, 0, 3, 3),
                Placement(Box(fill=second_fill), 4, 0, 3, 3),
            ],
            11,
            3,
        )
        self.assertEqual(
            grid[0],
            _cell(BLANK, PLAIN, first_fill) * 3
            + BLANK
            + _cell(BLANK, PLAIN, second_fill) * 3
            + BLANK * 4,
        )

    def test_a_plain_box_emits_no_escapes(self):
        grid = self.renderer.render(
            [
                Placement(Box("hi"), 0, 0, 5, 3),
                Placement(Label("hi"), 1, 1, 2, 1),
                Placement(Cursor(), 3, 1, 1, 1),
            ],
            5,
            3,
        )
        self.assertNotIn("\x1b", "".join(grid))

    def test_a_coloured_box_puts_no_colour_in_the_grid(self):
        grid = self.renderer.render([Placement(Box(colour=2), 0, 0, 5, 3)], 5, 3)
        self.assertEqual(grid, [BLANK * 5] * 3)

    def test_a_coloured_box_bottom_row_is_plain(self):
        grid = self.renderer.render([Placement(Box(colour=4), 0, 0, 5, 3)], 5, 3)
        self.assertEqual(grid[2], BLANK * 5)

    def test_a_label_inside_a_coloured_box_is_plain(self):
        grid = self.renderer.render(
            [
                Placement(Box("hi", colour=5), 0, 0, 5, 3),
                Placement(Label("hi"), 1, 1, 2, 1),
            ],
            5,
            3,
        )
        self.assertEqual(grid[1], BLANK + "hi" + BLANK * 2)

    def test_the_cursor_is_plain(self):
        grid = self.renderer.render(
            [Placement(Box(colour=1), 0, 0, 5, 3), Placement(Cursor(), 1, 1, 1, 1)],
            5,
            3,
        )
        self.assertEqual(grid[1], BLANK + CURSOR + BLANK * 3)

    def test_two_boxes_render_their_own_fills(self):
        first_fill, second_fill = 0, 6
        grid = self.renderer.render(
            [
                Placement(Box(fill=first_fill), 0, 0, 3, 3),
                Placement(Box(fill=second_fill), 4, 0, 3, 3),
            ],
            11,
            3,
        )
        rest = grid[0]
        for _ in range(3):
            self.assertTrue(rest.startswith(_cell(BLANK, PLAIN, first_fill)))
            rest = rest[len(_cell(BLANK, PLAIN, first_fill)) :]
        self.assertTrue(rest.startswith(BLANK))
        rest = rest[1:]
        self.assertTrue(rest.startswith(_cell(BLANK, PLAIN, second_fill)))

    def test_an_arrow_leaves_the_gap_blank(self):
        grid = self.renderer.render(
            [Placement(LayoutArrow((0,), 0), 2, 1, 1, 2)], 4, 4
        )
        self.assertEqual(grid, [BLANK * 4] * 4)

    def test_an_arrow_outside_the_grid_is_clipped(self):
        grid = self.renderer.render(
            [Placement(LayoutArrow((0,), 0), 9, 9, 1, 2)], 4, 3
        )
        self.assertEqual(grid, [BLANK * 4] * 3)

    def test_an_unfilled_box_emits_no_escapes(self):
        grid = self.renderer.render([Placement(Box(fill=PLAIN), 0, 0, 5, 4)], 5, 4)
        self.assertNotIn("\x1b", "".join(grid))

    def test_a_filled_box_paints_every_cell(self):
        fill = 2
        grid = self.renderer.render([Placement(Box(fill=fill), 0, 0, 5, 4)], 5, 4)
        self.assertEqual(grid, [_cell(BLANK, PLAIN, fill) * 5] * 4)

    def test_a_filled_box_paints_the_cells_that_used_to_be_border(self):
        fill = 6
        grid = self.renderer.render([Placement(Box(fill=fill), 0, 0, 5, 4)], 5, 4)
        painted = _cell(BLANK, PLAIN, fill)
        self.assertTrue(grid[0].startswith(painted))
        self.assertTrue(grid[0].endswith(painted))
        self.assertTrue(grid[3].startswith(painted))
        self.assertTrue(grid[1].startswith(painted))

    def test_a_box_with_border_colour_and_fill_paints_only_the_fill(self):
        fill = 4
        grid = self.renderer.render(
            [Placement(Box(colour=1, fill=fill), 0, 0, 5, 4)], 5, 4
        )
        self.assertEqual(grid, [_cell(BLANK, PLAIN, fill) * 5] * 4)

    def test_a_label_sits_on_top_of_the_fill(self):
        fill = 3
        grid = self.renderer.render(
            [
                Placement(Box("hi", fill=fill), 0, 0, 5, 3),
                Placement(Label("hi"), 1, 1, 2, 1),
            ],
            5,
            3,
        )
        self.assertEqual(
            grid[1],
            _cell(BLANK, PLAIN, fill)
            + _cell("h", PLAIN, fill)
            + _cell("i", PLAIN, fill)
            + _cell(BLANK, PLAIN, fill) * 2,
        )

    def test_the_cursor_keeps_the_fill_of_a_filled_box(self):
        fill = 5
        grid = self.renderer.render(
            [
                Placement(Box(fill=fill), 0, 0, 5, 3),
                Placement(Cursor(), 1, 1, 1, 1),
            ],
            5,
            3,
        )
        rest = grid[1][len(_cell(BLANK, PLAIN, fill)) :]
        self.assertTrue(rest.startswith(_cell(CURSOR, PLAIN, fill)))

    def test_the_fill_survives_under_the_label_and_the_cursor(self):
        fill = 4
        grid = self.renderer.render(
            [
                Placement(Box(fill=fill), 0, 0, 5, 3),
                Placement(Label("hi"), 1, 1, 2, 1),
                Placement(Cursor(), 3, 1, 1, 1),
            ],
            5,
            3,
        )
        self.assertEqual(
            grid[1],
            _cell(BLANK, PLAIN, fill)
            + _cell("h", PLAIN, fill)
            + _cell("i", PLAIN, fill)
            + _cell(CURSOR, PLAIN, fill)
            + _cell(BLANK, PLAIN, fill),
        )

    def test_empty_canvas_with_no_boxes_emits_no_escapes(self):
        grid = self.renderer.render([], cols=11, rows=5)
        self.assertEqual(grid, [BLANK * 11] * 5)
        self.assertNotIn("\x1b", "".join(grid))


class BoxCharacterTest(unittest.TestCase):
    def setUp(self):
        self.renderer = TerminalRenderer()

    def character(self, x, y):
        return self.renderer._box_character(x, y, left=0, right=4, top=0, bottom=3)

    def test_corners(self):
        self.assertEqual(self.character(0, 0), TOP_LEFT)
        self.assertEqual(self.character(4, 0), TOP_RIGHT)
        self.assertEqual(self.character(0, 3), BOTTOM_LEFT)
        self.assertEqual(self.character(4, 3), BOTTOM_RIGHT)

    def test_top_and_bottom_edges_are_horizontal(self):
        self.assertEqual(self.character(2, 0), HORIZONTAL)
        self.assertEqual(self.character(2, 3), HORIZONTAL)

    def test_left_and_right_edges_are_vertical(self):
        self.assertEqual(self.character(0, 1), VERTICAL)
        self.assertEqual(self.character(4, 1), VERTICAL)

    def test_the_interior_is_blank(self):
        self.assertEqual(self.character(2, 1), BLANK)


class GraphicsRendererFillTest(unittest.TestCase):
    def setUp(self):
        self.renderer = GraphicsRenderer(
            text=TerminalRenderer(), graphics=None, cell_width=1, cell_height=1
        )

    def outline(self, box, width=3, height=3):
        placement = Placement(box, x=0, y=0, width=width, height=height)
        return self.renderer._outline_box(
            placement, left=0, top=0, right=width, bottom=height
        )

    def pixel(self, sprite, x, y):
        offset = (y * sprite.width + x) * 4
        return tuple(sprite.pixels[offset : offset + 4])

    def test_plain_fill_renders_transparent_interior(self):
        sprite = self.outline(Box(fill=PLAIN))
        self.assertEqual(self.pixel(sprite, 1, 1), TRANSPARENT)

    def test_a_fill_colour_renders_the_palette_colour_at_thirty_percent_opacity(self):
        sprite = self.outline(Box(fill=2))
        self.assertEqual(self.pixel(sprite, 1, 1), PALETTE[2] + (FILL_ALPHA,))

    def test_border_pixels_are_unaffected_by_fill(self):
        sprite = self.outline(Box(colour=3, fill=2))
        self.assertEqual(self.pixel(sprite, 0, 0), _colour(3) + (OPAQUE,))
        self.assertEqual(self.pixel(sprite, 1, 1), _fill_colour(2))


class GraphicsRendererBorderThicknessTest(unittest.TestCase):
    def setUp(self):
        self.renderer = GraphicsRenderer(
            text=TerminalRenderer(), graphics=None, cell_width=4, cell_height=4
        )

    def outline(self, box, width=3, height=3):
        placement = Placement(box, x=0, y=0, width=width, height=height)
        return self.renderer._outline_box(
            placement, left=0, top=0, right=width, bottom=height
        )

    def pixel(self, sprite, x, y):
        offset = (y * sprite.width + x) * 4
        return tuple(sprite.pixels[offset : offset + 4])

    def test_a_thin_border_is_one_pixel_at_each_edge(self):
        sprite = self.outline(Box(colour=1, fill=2, border=1))
        edge = _colour(1) + (OPAQUE,)
        fill = _fill_colour(2)
        # Top and bottom edges, checked across a middle column.
        self.assertEqual(self.pixel(sprite, 5, 0), edge)
        self.assertEqual(self.pixel(sprite, 5, 1), fill)
        self.assertEqual(self.pixel(sprite, 5, sprite.height - 1), edge)
        self.assertEqual(self.pixel(sprite, 5, sprite.height - 2), fill)
        # Left and right edges, checked across a middle row.
        self.assertEqual(self.pixel(sprite, 0, 5), edge)
        self.assertEqual(self.pixel(sprite, 1, 5), fill)
        self.assertEqual(self.pixel(sprite, sprite.width - 1, 5), edge)
        self.assertEqual(self.pixel(sprite, sprite.width - 2, 5), fill)

    def test_a_thick_border_is_two_pixels_at_each_edge(self):
        sprite = self.outline(Box(colour=1, fill=2, border=2))
        edge = _colour(1) + (OPAQUE,)
        fill = _fill_colour(2)
        # Top edge: two edge rows, then fill.
        self.assertEqual(self.pixel(sprite, 5, 0), edge)
        self.assertEqual(self.pixel(sprite, 5, 1), edge)
        self.assertEqual(self.pixel(sprite, 5, 2), fill)
        # Bottom edge: two edge rows, then fill.
        self.assertEqual(self.pixel(sprite, 5, sprite.height - 1), edge)
        self.assertEqual(self.pixel(sprite, 5, sprite.height - 2), edge)
        self.assertEqual(self.pixel(sprite, 5, sprite.height - 3), fill)
        # Left edge: two edge columns, then fill.
        self.assertEqual(self.pixel(sprite, 0, 5), edge)
        self.assertEqual(self.pixel(sprite, 1, 5), edge)
        self.assertEqual(self.pixel(sprite, 2, 5), fill)
        # Right edge: two edge columns, then fill.
        self.assertEqual(self.pixel(sprite, sprite.width - 1, 5), edge)
        self.assertEqual(self.pixel(sprite, sprite.width - 2, 5), edge)
        self.assertEqual(self.pixel(sprite, sprite.width - 3, 5), fill)

    def test_a_border_of_three_is_three_pixels_at_each_edge(self):
        sprite = self.outline(Box(colour=1, fill=2, border=3))
        edge = _colour(1) + (OPAQUE,)
        fill = _fill_colour(2)
        # Top edge: three edge rows, then fill.
        self.assertEqual(self.pixel(sprite, 5, 0), edge)
        self.assertEqual(self.pixel(sprite, 5, 1), edge)
        self.assertEqual(self.pixel(sprite, 5, 2), edge)
        self.assertEqual(self.pixel(sprite, 5, 3), fill)
        # Bottom edge: three edge rows, then fill.
        self.assertEqual(self.pixel(sprite, 5, sprite.height - 1), edge)
        self.assertEqual(self.pixel(sprite, 5, sprite.height - 2), edge)
        self.assertEqual(self.pixel(sprite, 5, sprite.height - 3), edge)
        self.assertEqual(self.pixel(sprite, 5, sprite.height - 4), fill)
        # Left edge: three edge columns, then fill.
        self.assertEqual(self.pixel(sprite, 0, 5), edge)
        self.assertEqual(self.pixel(sprite, 1, 5), edge)
        self.assertEqual(self.pixel(sprite, 2, 5), edge)
        self.assertEqual(self.pixel(sprite, 3, 5), fill)
        # Right edge: three edge columns, then fill.
        self.assertEqual(self.pixel(sprite, sprite.width - 1, 5), edge)
        self.assertEqual(self.pixel(sprite, sprite.width - 2, 5), edge)
        self.assertEqual(self.pixel(sprite, sprite.width - 3, 5), edge)
        self.assertEqual(self.pixel(sprite, sprite.width - 4, 5), fill)

    def test_a_border_of_four_is_four_pixels_at_each_edge(self):
        sprite = self.outline(Box(colour=1, fill=2, border=4))
        edge = _colour(1) + (OPAQUE,)
        fill = _fill_colour(2)
        # Top edge: four edge rows, then fill.
        self.assertEqual(self.pixel(sprite, 5, 0), edge)
        self.assertEqual(self.pixel(sprite, 5, 1), edge)
        self.assertEqual(self.pixel(sprite, 5, 2), edge)
        self.assertEqual(self.pixel(sprite, 5, 3), edge)
        self.assertEqual(self.pixel(sprite, 5, 4), fill)
        # Bottom edge: four edge rows, then fill.
        self.assertEqual(self.pixel(sprite, 5, sprite.height - 1), edge)
        self.assertEqual(self.pixel(sprite, 5, sprite.height - 2), edge)
        self.assertEqual(self.pixel(sprite, 5, sprite.height - 3), edge)
        self.assertEqual(self.pixel(sprite, 5, sprite.height - 4), edge)
        self.assertEqual(self.pixel(sprite, 5, sprite.height - 5), fill)
        # Left edge: four edge columns, then fill.
        self.assertEqual(self.pixel(sprite, 0, 5), edge)
        self.assertEqual(self.pixel(sprite, 1, 5), edge)
        self.assertEqual(self.pixel(sprite, 2, 5), edge)
        self.assertEqual(self.pixel(sprite, 3, 5), edge)
        self.assertEqual(self.pixel(sprite, 4, 5), fill)
        # Right edge: four edge columns, then fill.
        self.assertEqual(self.pixel(sprite, sprite.width - 1, 5), edge)
        self.assertEqual(self.pixel(sprite, sprite.width - 2, 5), edge)
        self.assertEqual(self.pixel(sprite, sprite.width - 3, 5), edge)
        self.assertEqual(self.pixel(sprite, sprite.width - 4, 5), edge)
        self.assertEqual(self.pixel(sprite, sprite.width - 5, 5), fill)

    def test_border_thickness_does_not_change_sprite_size(self):
        thin = self.outline(Box(border=1))
        thick = self.outline(Box(border=2))
        thicker = self.outline(Box(border=3))
        thickest = self.outline(Box(border=4))
        self.assertEqual((thin.width, thin.height), (thick.width, thick.height))
        self.assertEqual((thin.width, thin.height), (thicker.width, thicker.height))
        self.assertEqual((thin.width, thin.height), (thickest.width, thickest.height))

    def test_interior_still_fills_correctly_with_a_thick_border(self):
        sprite = self.outline(Box(fill=2, border=2))
        fill = _fill_colour(2)
        for y in range(2, sprite.height - 2):
            for x in range(2, sprite.width - 2):
                self.assertEqual(self.pixel(sprite, x, y), fill)

    def test_interior_still_fills_correctly_with_a_border_of_four(self):
        sprite = self.outline(Box(fill=2, border=4))
        fill = _fill_colour(2)
        for y in range(4, sprite.height - 4):
            for x in range(4, sprite.width - 4):
                self.assertEqual(self.pixel(sprite, x, y), fill)


class GraphicsRendererCornerRadiusTest(unittest.TestCase):
    def setUp(self):
        self.renderer = GraphicsRenderer(
            text=TerminalRenderer(), graphics=None, cell_width=4, cell_height=4
        )

    def outline(self, box, width=10, height=10):
        placement = Placement(box, x=0, y=0, width=width, height=height)
        return self.renderer._outline_box(
            placement, left=0, top=0, right=width, bottom=height
        )

    def pixel(self, sprite, x, y):
        offset = (y * sprite.width + x) * 4
        return tuple(sprite.pixels[offset : offset + 4])

    def alpha_total(self, sprite):
        return sum(sprite.pixels[3::4])

    def test_a_square_box_is_built_from_flat_edge_and_body_rows(self):
        border = 2
        sprite = self.outline(Box(colour=1, fill=2, border=border, radius=SQUARE))
        edge = bytes(_colour(1) + (OPAQUE,))
        fill = bytes(_fill_colour(2))
        edge_row = edge * sprite.width
        body_row = (
            edge * border + fill * (sprite.width - 2 * border) + edge * border
        )
        self.assertEqual(
            sprite.pixels,
            edge_row * border
            + body_row * (sprite.height - 2 * border)
            + edge_row * border,
        )

    def test_a_rounded_box_cuts_away_its_extreme_corners(self):
        sprite = self.outline(Box(colour=1, fill=2, radius=SMALL_RADIUS))
        last_x, last_y = sprite.width - 1, sprite.height - 1
        self.assertEqual(self.pixel(sprite, 0, 0), TRANSPARENT)
        self.assertEqual(self.pixel(sprite, last_x, 0), TRANSPARENT)
        self.assertEqual(self.pixel(sprite, 0, last_y), TRANSPARENT)
        self.assertEqual(self.pixel(sprite, last_x, last_y), TRANSPARENT)

    def test_straight_edges_stay_as_crisp_as_a_square_box(self):
        square = self.outline(Box(colour=1, fill=2, border=2, radius=SQUARE))
        rounded = self.outline(
            Box(colour=1, fill=2, border=2, radius=LARGE_RADIUS)
        )
        middle_y = square.height // 2
        middle_x = square.width // 2
        for x in range(square.width):
            self.assertEqual(
                self.pixel(rounded, x, middle_y), self.pixel(square, x, middle_y)
            )
        for y in range(square.height):
            self.assertEqual(
                self.pixel(rounded, middle_x, y), self.pixel(square, middle_x, y)
            )

    def test_the_radius_leaves_the_sprite_size_and_position_alone(self):
        sprites = [
            self.outline(Box(colour=1, fill=2, radius=radius))
            for radius in (SQUARE, SMALL_RADIUS, LARGE_RADIUS)
        ]
        sizes = {(sprite.width, sprite.height) for sprite in sprites}
        positions = {(sprite.col, sprite.row) for sprite in sprites}
        self.assertEqual(len(sizes), 1)
        self.assertEqual(len(positions), 1)

    def test_the_arc_is_anti_aliased(self):
        square = self.outline(Box(colour=1, fill=PLAIN, radius=SQUARE))
        rounded = self.outline(Box(colour=1, fill=PLAIN, radius=LARGE_RADIUS))
        self.assertFalse(
            any(0 < alpha < OPAQUE for alpha in square.pixels[3::4])
        )
        self.assertTrue(
            any(0 < alpha < OPAQUE for alpha in rounded.pixels[3::4])
        )

    def test_a_larger_radius_cuts_away_more_than_a_smaller_one(self):
        square = self.outline(Box(colour=1, fill=2, radius=SQUARE))
        small = self.outline(Box(colour=1, fill=2, radius=SMALL_RADIUS))
        large = self.outline(Box(colour=1, fill=2, radius=LARGE_RADIUS))
        self.assertLess(self.alpha_total(small), self.alpha_total(square))
        self.assertLess(self.alpha_total(large), self.alpha_total(small))

    def test_the_fringe_keeps_the_edge_colour_instead_of_fading_to_black(self):
        sprite = self.outline(Box(colour=1, fill=PLAIN, radius=LARGE_RADIUS))
        edge = _colour(1)
        partial = [
            self.pixel(sprite, x, y)
            for y in range(sprite.height)
            for x in range(sprite.width)
            if 0 < self.pixel(sprite, x, y)[3] < OPAQUE
        ]
        self.assertTrue(partial)
        for red, green, blue, _ in partial:
            self.assertEqual((red, green, blue), edge)

    def test_clipping_a_rounded_box_is_a_pure_crop_of_the_whole_box(self):
        box = Box(colour=1, fill=2, radius=LARGE_RADIUS)
        whole = self.outline(box)
        hidden_cols = 2
        clipped = self.renderer._outline_box(
            Placement(box, x=-hidden_cols, y=0, width=10, height=10),
            left=0,
            top=0,
            right=10 - hidden_cols,
            bottom=10,
        )
        offset = hidden_cols * self.renderer.cell_width
        for y in range(clipped.height):
            for x in range(clipped.width):
                self.assertEqual(
                    self.pixel(clipped, x, y), self.pixel(whole, x + offset, y)
                )


if __name__ == "__main__":
    unittest.main()
