import unittest
from typing import get_args, get_type_hints

from sketch.layout import Arrow as LayoutArrow
from sketch.layout import Label, Placement
from sketch.render import (
    ARROW_STROKE,
    ARROWHEAD_SLOPE,
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
    Canvas,
    GraphicsRenderer,
    Sprite,
    TerminalRenderer,
    _cell,
    _centered_span,
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
        self.assertEqual(grid, [BLANK * 4, BLANK * 4])

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
        self.assertEqual(grid[0], BLANK * 11)

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

    def test_a_box_claims_cells_drawn_by_an_earlier_placement(self):
        grid = self.renderer.render(
            [
                Placement(Label("hi"), 1, 1, 2, 1),
                Placement(Box(fill=2), 0, 0, 5, 3),
            ],
            5,
            3,
        )
        self.assertEqual(grid, [BLANK * 5] * 3)

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

    def test_a_filled_rounded_box_emits_no_escapes(self):
        grid = self.renderer.render(
            [Placement(Box(fill=2, radius=SMALL_RADIUS), 0, 0, 5, 4)], 5, 4
        )
        self.assertNotIn("\x1b", "".join(grid))

    def test_a_filled_box_claims_every_cell_as_blank(self):
        fill = 2
        grid = self.renderer.render([Placement(Box(fill=fill), 0, 0, 5, 4)], 5, 4)
        self.assertEqual(grid, [BLANK * 5] * 4)

    def test_a_box_with_border_colour_and_fill_emits_plain_cells(self):
        fill = 4
        grid = self.renderer.render(
            [Placement(Box(colour=1, fill=fill), 0, 0, 5, 4)], 5, 4
        )
        self.assertEqual(grid, [BLANK * 5] * 4)

    def test_a_label_sits_on_top_of_a_filled_box(self):
        fill = 3
        grid = self.renderer.render(
            [
                Placement(Box("hi", fill=fill), 0, 0, 5, 3),
                Placement(Label("hi"), 1, 1, 2, 1),
            ],
            5,
            3,
        )
        self.assertEqual(grid[1], BLANK + "hi" + BLANK * 2)

    def test_the_cursor_sits_on_top_of_a_filled_box(self):
        fill = 5
        grid = self.renderer.render(
            [
                Placement(Box(fill=fill), 0, 0, 5, 3),
                Placement(Cursor(), 1, 1, 1, 1),
            ],
            5,
            3,
        )
        self.assertEqual(grid[1], BLANK + CURSOR + BLANK * 3)

    def test_label_and_cursor_stamp_over_a_filled_rounded_box(self):
        fill = 4
        grid = self.renderer.render(
            [
                Placement(Box(fill=fill, radius=LARGE_RADIUS), 0, 0, 5, 3),
                Placement(Label("hi"), 1, 1, 2, 1),
                Placement(Cursor(), 3, 1, 1, 1),
            ],
            5,
            3,
        )
        self.assertEqual(grid[1], BLANK + "hi" + CURSOR + BLANK)

    def test_empty_canvas_with_no_boxes_emits_no_escapes(self):
        grid = self.renderer.render([], cols=11, rows=5)
        self.assertEqual(grid, [BLANK * 11] * 5)
        self.assertNotIn("\x1b", "".join(grid))


class CanvasTest(unittest.TestCase):
    def setUp(self):
        self.ink = _colour(1) + (OPAQUE,)

    def canvas(self, first_x=0, last_x=20, first_y=0, last_y=20):
        return Canvas(first_x, last_x, first_y, last_y, bytes(self.ink))

    def pixel(self, canvas, x, y):
        offset = ((y - canvas.first_y) * canvas.span + (x - canvas.first_x)) * 4
        return tuple(canvas.buffer[offset : offset + 4])

    def blank(self):
        return tuple(TRANSPARENT)

    def test_point_with_width_one_stamps_a_single_pixel(self):
        canvas = self.canvas()
        canvas.point(10, 10, width=1)
        self.assertEqual(self.pixel(canvas, 10, 10), self.ink)
        for x, y in [(9, 10), (11, 10), (10, 9), (10, 11)]:
            self.assertEqual(self.pixel(canvas, x, y), self.blank())

    def test_point_with_width_four_stamps_a_four_by_four_block(self):
        canvas = self.canvas()
        canvas.point(10, 10, width=4)
        for x in (9, 10, 11, 12):
            for y in (9, 10, 11, 12):
                self.assertEqual(self.pixel(canvas, x, y), self.ink)
        for x, y in [(8, 10), (13, 10), (10, 8), (10, 13)]:
            self.assertEqual(self.pixel(canvas, x, y), self.blank())

    def test_horizontal_with_width_four_paints_four_rows(self):
        canvas = self.canvas()
        canvas.horizontal(10, 2, 6, width=4)
        for y in (9, 10, 11, 12):
            for x in range(2, 7):
                self.assertEqual(self.pixel(canvas, x, y), self.ink)
        for y in (8, 13):
            for x in range(2, 7):
                self.assertEqual(self.pixel(canvas, x, y), self.blank())
        self.assertEqual(self.pixel(canvas, 1, 10), self.blank())
        self.assertEqual(self.pixel(canvas, 7, 10), self.blank())

    def test_vertical_with_width_four_paints_four_columns(self):
        canvas = self.canvas()
        canvas.vertical(10, 2, 6, width=4)
        for x in (9, 10, 11, 12):
            for y in range(2, 7):
                self.assertEqual(self.pixel(canvas, x, y), self.ink)
        for x in (8, 13):
            for y in range(2, 7):
                self.assertEqual(self.pixel(canvas, x, y), self.blank())
        self.assertEqual(self.pixel(canvas, 10, 1), self.blank())
        self.assertEqual(self.pixel(canvas, 10, 7), self.blank())

    def test_a_thick_point_near_the_edge_is_clipped(self):
        canvas = self.canvas(first_x=0, last_x=20, first_y=0, last_y=20)
        canvas.point(0, 0, width=4)
        for x in (0, 1):
            for y in (0, 1):
                self.assertEqual(self.pixel(canvas, x, y), self.ink)

    def test_a_thick_horizontal_near_the_edge_is_clipped(self):
        canvas = self.canvas(first_x=0, last_x=20, first_y=0, last_y=20)
        canvas.horizontal(0, 2, 6, width=4)
        for x in range(2, 7):
            self.assertEqual(self.pixel(canvas, x, 0), self.ink)
            self.assertEqual(self.pixel(canvas, x, 1), self.ink)

    def test_a_thick_vertical_near_the_edge_is_clipped(self):
        canvas = self.canvas(first_x=0, last_x=20, first_y=0, last_y=20)
        canvas.vertical(0, 2, 6, width=4)
        for y in range(2, 7):
            self.assertEqual(self.pixel(canvas, 0, y), self.ink)
            self.assertEqual(self.pixel(canvas, 1, y), self.ink)


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

    def test_a_fill_colour_is_composited_over_black_and_made_opaque(self):
        sprite = self.outline(Box(fill=2))
        composited = tuple(
            round(channel * FILL_ALPHA / OPAQUE) for channel in PALETTE[2]
        ) + (OPAQUE,)
        self.assertEqual(self.pixel(sprite, 1, 1), composited)

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


class GraphicsRendererArrowThicknessTest(unittest.TestCase):
    def setUp(self):
        self.renderer = GraphicsRenderer(
            text=TerminalRenderer(), graphics=None, cell_width=10, cell_height=10
        )
        self.ink = _colour(PLAIN) + (OPAQUE,)
        self.blank = tuple(TRANSPARENT)

    def outline(self, arrow, width=4, height=3):
        placement = Placement(arrow, x=0, y=0, width=width, height=height)
        return self.renderer._outline_arrow(
            placement, left=0, top=0, right=width, bottom=height
        )

    def pixel(self, sprite, x, y):
        offset = (y * sprite.width + x) * 4
        return tuple(sprite.pixels[offset : offset + 4])

    def test_the_shaft_is_arrow_stroke_pixels_thick(self):
        sprite = self.outline(LayoutArrow(stops=(0, 2), shaft=1))
        shaft_row = 1 * 10 + 5
        rows = list(_centered_span(shaft_row, ARROW_STROKE))
        for y in rows:
            self.assertEqual(self.pixel(sprite, 5, y), self.ink)
        self.assertEqual(self.pixel(sprite, 5, rows[0] - 1), self.blank)
        self.assertEqual(self.pixel(sprite, 5, rows[-1] + 1), self.blank)

    def test_the_trunk_is_arrow_stroke_pixels_thick(self):
        sprite = self.outline(LayoutArrow(stops=(0, 2), shaft=1))
        midpoint = (4 * 10) // 2
        columns = list(_centered_span(midpoint, ARROW_STROKE))
        for x in columns:
            self.assertEqual(self.pixel(sprite, x, 10), self.ink)
        self.assertEqual(self.pixel(sprite, columns[0] - 1, 10), self.blank)
        self.assertEqual(self.pixel(sprite, columns[-1] + 1, 10), self.blank)

    def test_each_stops_run_is_arrow_stroke_pixels_thick(self):
        sprite = self.outline(LayoutArrow(stops=(0, 2), shaft=1))
        stop_row = 0 * 10 + 5
        rows = list(_centered_span(stop_row, ARROW_STROKE))
        for y in rows:
            self.assertEqual(self.pixel(sprite, 25, y), self.ink)
        self.assertEqual(self.pixel(sprite, 25, rows[0] - 1), self.blank)
        self.assertEqual(self.pixel(sprite, 25, rows[-1] + 1), self.blank)

    def test_the_arrowhead_stamps_are_arrow_stroke_squares(self):
        sprite = self.outline(LayoutArrow(stops=(0, 2), shaft=1))
        stop_row = 5
        right_edge = 4 * 10 - 1
        distance = 6
        spread = round(distance * ARROWHEAD_SLOPE)
        x = right_edge - distance
        y = stop_row - spread
        columns = list(_centered_span(x, ARROW_STROKE))
        rows = list(_centered_span(y, ARROW_STROKE))
        for cx in columns:
            for cy in rows:
                self.assertEqual(self.pixel(sprite, cx, cy), self.ink)
        self.assertEqual(self.pixel(sprite, 25, 2), self.blank)

    def test_the_arrowhead_tip_sits_at_the_stop_row(self):
        sprite = self.outline(LayoutArrow(stops=(0, 2), shaft=1))
        right_edge = 4 * 10 - 1
        for stop_row in (5, 25):
            self.assertEqual(self.pixel(sprite, right_edge, stop_row), self.ink)
            self.assertEqual(
                self.pixel(sprite, right_edge - 1, stop_row), self.ink
            )


class GraphicsRendererCornerRadiusTest(unittest.TestCase):
    def setUp(self):
        self.renderer = GraphicsRenderer(
            text=TerminalRenderer(),
            graphics=None,
            cell_width=2 * LARGE_RADIUS // 5,
            cell_height=2 * LARGE_RADIUS // 5,
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

    def test_arc_coverage_is_continuous_at_the_pixel_centre(self):
        sprite = self.outline(
            Box(colour=1, fill=PLAIN, border=1, radius=LARGE_RADIUS)
        )
        self.assertEqual(self.pixel(sprite, 6, 6), _colour(1) + (253,))

    def test_border_coverage_is_composed_over_the_opaque_fill(self):
        sprite = self.outline(
            Box(colour=1, fill=2, border=1, radius=LARGE_RADIUS)
        )
        self.assertEqual(self.pixel(sprite, 6, 7), (131, 27, 25, OPAQUE))

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


class GraphicsRendererSmallBoxTest(unittest.TestCase):
    """A box no larger than twice its corner radius, where the two corner
    bands of a row would otherwise overlap."""

    def setUp(self):
        cell = LARGE_RADIUS // 2
        self.renderer = GraphicsRenderer(
            text=TerminalRenderer(),
            graphics=None,
            cell_width=cell,
            cell_height=cell,
        )
        self.cells = 2

    def outline(self, box, hidden_cols=0):
        placement = Placement(
            box,
            x=-hidden_cols,
            y=0,
            width=self.cells,
            height=self.cells,
        )
        return self.renderer._outline_box(
            placement,
            left=0,
            top=0,
            right=self.cells - hidden_cols,
            bottom=self.cells,
        )

    def pixel(self, sprite, x, y):
        offset = (y * sprite.width + x) * 4
        return tuple(sprite.pixels[offset : offset + 4])

    def test_the_sprite_holds_exactly_one_pixel_per_cell_of_its_area(self):
        for radius in (SMALL_RADIUS, LARGE_RADIUS):
            with self.subTest(radius=radius):
                sprite = self.outline(Box(colour=1, fill=2, radius=radius))
                self.assertEqual(
                    len(sprite.pixels), sprite.width * sprite.height * 4
                )

    def test_clipping_a_small_box_is_a_pure_crop_of_the_whole_box(self):
        box = Box(colour=1, fill=2, radius=LARGE_RADIUS)
        whole = self.outline(box)
        hidden_cols = 1
        clipped = self.outline(box, hidden_cols=hidden_cols)
        offset = hidden_cols * self.renderer.cell_width
        self.assertEqual(clipped.width, whole.width - offset)
        for y in range(clipped.height):
            for x in range(clipped.width):
                self.assertEqual(
                    self.pixel(clipped, x, y), self.pixel(whole, x + offset, y)
                )


if __name__ == "__main__":
    unittest.main()
