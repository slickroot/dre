import unittest
from math import cos, radians, tan

from dre_rs import Arrow as LayoutArrow
from dre_rs import Cursor, Label, Placement
from dre.render import (
    ARROW_STROKE,
    ARROWHEAD_ANGLE_DEG,
    ARROWHEAD_DEPTH,
    ARROWHEAD_EDGE_LENGTH,
    ARROWHEAD_SLOPE,
    BLANK,
    BORDER,
    CACHE_LIMIT,
    CURSOR,
    FILL_ALPHA,
    OPAQUE,
    PALETTE,
    PLAIN_COLOUR,
    ROUNDED_RADIUS,
    TRANSPARENT,
    Canvas,
    Sprite,
    TerminalRenderer,
    _centered_span,
    _colour,
    _fill_colour,
)
from dre_rs import PLAIN, Box


class FakeGraphics:
    def __init__(self, payload="<payload>"):
        self.payload = payload
        self.sprites = None

    def draw(self, sprites):
        self.sprites = sprites
        return self.payload


class TerminalRendererTest(unittest.TestCase):
    def setUp(self):
        self.renderer = TerminalRenderer(
            graphics=FakeGraphics(), cell_width=1, cell_height=1
        )

    def render(self, placements, cols, rows):
        return self.renderer._grid(placements, cols, rows)

    def test_empty_canvas_fills_terminal(self):
        self.assertEqual(
            self.render([], cols=11, rows=5), [BLANK * 11] * 5
        )

    def test_grid_matches_the_requested_size(self):
        cols, rows = 20, 7
        grid = self.render([Placement(Box(), 4, 4, 3, 3)], cols, rows)
        self.assertEqual(len(grid), rows)
        self.assertEqual({len(line) for line in grid}, {cols})

    def test_a_box_claims_its_cells_without_border_characters(self):
        grid = self.render([Placement(Box(), 4, 4, 3, 3)], 11, 11)
        self.assertEqual(grid[4][4:7], BLANK * 3)

    def test_every_row_of_a_box_is_blank(self):
        grid = self.render([Placement(Box(), 4, 4, 3, 3)], 11, 11)
        self.assertEqual([line[4:7] for line in grid[4:7]], [BLANK * 3] * 3)

    def test_box_interior_is_empty(self):
        grid = self.render([Placement(Box(), 0, 0, 5, 4)], 5, 4)
        self.assertEqual(grid, [BLANK * 5] * 4)

    def test_nothing_is_drawn_outside_the_box(self):
        fill = 2
        grid = self.render([Placement(Box(fill=fill), 4, 4, 3, 3)], 11, 11)
        self.assertEqual(grid[3], BLANK * 11)
        self.assertEqual(grid[4][: len(BLANK * 4)], BLANK * 4)
        self.assertTrue(grid[4].endswith(BLANK * 4))

    def test_a_box_reaching_past_the_edge_is_clipped(self):
        fill = 3
        grid = self.render([Placement(Box(fill=fill), 3, 1, 3, 3)], 4, 2)
        self.assertEqual(grid, [BLANK * 4, BLANK * 4])

    def test_label_is_drawn_inside_the_box(self):
        grid = self.render(
            [
                Placement(Box("hi"), 0, 0, 5, 3),
                Placement(Label("hi"), 1, 1, 2, 1),
            ],
            5,
            3,
        )
        self.assertEqual(grid[1], BLANK + "hi" + BLANK * 2)

    def test_cursor_is_drawn_after_the_label(self):
        grid = self.render(
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
        grid = self.render(
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
        grid = self.render([Placement(Box("hi"), 0, 0, 5, 3)], 5, 3)
        self.assertNotIn(CURSOR, "".join(grid))

    def test_cursor_placement_is_drawn_at_its_own_position(self):
        grid = self.render([Placement(Cursor(), 2, 1, 1, 1)], 4, 3)
        self.assertEqual(
            grid, [BLANK * 4, BLANK * 2 + CURSOR + BLANK, BLANK * 4]
        )

    def test_a_cursor_outside_the_grid_is_clipped(self):
        grid = self.render([Placement(Cursor(), 9, 9, 1, 1)], 4, 3)
        self.assertEqual(grid, [BLANK * 4] * 3)

    def test_each_placement_is_drawn(self):
        first_fill, second_fill = 1, 2
        grid = self.render(
            [
                Placement(Box(fill=first_fill), 0, 0, 3, 3),
                Placement(Box(fill=second_fill), 4, 0, 3, 3),
            ],
            11,
            3,
        )
        self.assertEqual(grid[0], BLANK * 11)

    def test_a_plain_box_emits_no_escapes(self):
        grid = self.render(
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
        grid = self.render([Placement(Box(colour=2), 0, 0, 5, 3)], 5, 3)
        self.assertEqual(grid, [BLANK * 5] * 3)

    def test_a_coloured_box_bottom_row_is_plain(self):
        grid = self.render([Placement(Box(colour=4), 0, 0, 5, 3)], 5, 3)
        self.assertEqual(grid[2], BLANK * 5)

    def test_a_label_inside_a_coloured_box_is_plain(self):
        grid = self.render(
            [
                Placement(Box("hi", colour=5), 0, 0, 5, 3),
                Placement(Label("hi"), 1, 1, 2, 1),
            ],
            5,
            3,
        )
        self.assertEqual(grid[1], BLANK + "hi" + BLANK * 2)

    def test_the_cursor_is_plain(self):
        grid = self.render(
            [Placement(Box(colour=1), 0, 0, 5, 3), Placement(Cursor(), 1, 1, 1, 1)],
            5,
            3,
        )
        self.assertEqual(grid[1], BLANK + CURSOR + BLANK * 3)

    def test_a_box_claims_cells_drawn_by_an_earlier_placement(self):
        grid = self.render(
            [
                Placement(Label("hi"), 1, 1, 2, 1),
                Placement(Box(fill=2), 0, 0, 5, 3),
            ],
            5,
            3,
        )
        self.assertEqual(grid, [BLANK * 5] * 3)

    def test_an_arrow_leaves_the_gap_blank(self):
        grid = self.render(
            [Placement(LayoutArrow((0,), 0), 2, 1, 1, 2)], 4, 4
        )
        self.assertEqual(grid, [BLANK * 4] * 4)

    def test_an_arrow_outside_the_grid_is_clipped(self):
        grid = self.render(
            [Placement(LayoutArrow((0,), 0), 9, 9, 1, 2)], 4, 3
        )
        self.assertEqual(grid, [BLANK * 4] * 3)

    def test_a_filled_rounded_box_emits_no_escapes(self):
        grid = self.render(
            [Placement(Box(fill=2, rounded=True), 0, 0, 5, 4)], 5, 4
        )
        self.assertNotIn("\x1b", "".join(grid))

    def test_a_filled_box_claims_every_cell_as_blank(self):
        fill = 2
        grid = self.render([Placement(Box(fill=fill), 0, 0, 5, 4)], 5, 4)
        self.assertEqual(grid, [BLANK * 5] * 4)

    def test_a_box_with_border_colour_and_fill_emits_plain_cells(self):
        fill = 4
        grid = self.render(
            [Placement(Box(colour=1, fill=fill), 0, 0, 5, 4)], 5, 4
        )
        self.assertEqual(grid, [BLANK * 5] * 4)

    def test_a_label_sits_on_top_of_a_filled_box(self):
        fill = 3
        grid = self.render(
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
        grid = self.render(
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
        grid = self.render(
            [
                Placement(Box(fill=fill, rounded=True), 0, 0, 5, 3),
                Placement(Label("hi"), 1, 1, 2, 1),
                Placement(Cursor(), 3, 1, 1, 1),
            ],
            5,
            3,
        )
        self.assertEqual(grid[1], BLANK + "hi" + CURSOR + BLANK)

    def test_empty_canvas_with_no_boxes_emits_no_escapes(self):
        grid = self.render([], cols=11, rows=5)
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


class TerminalRendererFillTest(unittest.TestCase):
    def setUp(self):
        self.renderer = TerminalRenderer(
            graphics=None, cell_width=1, cell_height=1
        )

    def outline(self, box, width=2 * BORDER + 3, height=2 * BORDER + 3):
        placement = Placement(box, x=0, y=0, width=width, height=height)
        return self.renderer._outline_box(
            placement, left=0, top=0, right=width, bottom=height
        )

    def pixel(self, sprite, x, y):
        offset = (y * sprite.width + x) * 4
        return tuple(sprite.pixels[offset : offset + 4])

    def test_plain_fill_renders_transparent_interior(self):
        sprite = self.outline(Box(fill=PLAIN))
        self.assertEqual(self.pixel(sprite, BORDER + 1, BORDER + 1), TRANSPARENT)

    def test_a_fill_colour_is_composited_over_black_and_made_opaque(self):
        sprite = self.outline(Box(fill=2))
        composited = tuple(
            round(channel * FILL_ALPHA / OPAQUE) for channel in PALETTE[2]
        ) + (OPAQUE,)
        self.assertEqual(self.pixel(sprite, BORDER + 1, BORDER + 1), composited)

    def test_border_pixels_are_unaffected_by_fill(self):
        sprite = self.outline(Box(colour=3, fill=2))
        self.assertEqual(self.pixel(sprite, 0, 0), _colour(3) + (OPAQUE,))
        self.assertEqual(
            self.pixel(sprite, BORDER + 1, BORDER + 1), _fill_colour(2)
        )


class TerminalRendererBorderTest(unittest.TestCase):
    def setUp(self):
        self.renderer = TerminalRenderer(
            graphics=None, cell_width=4, cell_height=4
        )

    def outline(self, box, width=3, height=3):
        placement = Placement(box, x=0, y=0, width=width, height=height)
        return self.renderer._outline_box(
            placement, left=0, top=0, right=width, bottom=height
        )

    def pixel(self, sprite, x, y):
        offset = (y * sprite.width + x) * 4
        return tuple(sprite.pixels[offset : offset + 4])

    def test_a_border_is_bold_at_every_edge(self):
        sprite = self.outline(Box(colour=1, fill=2))
        edge = _colour(1) + (OPAQUE,)
        fill = _fill_colour(2)
        # Top and bottom edges, checked across a middle column.
        for offset in range(BORDER):
            self.assertEqual(self.pixel(sprite, 5, offset), edge)
            self.assertEqual(
                self.pixel(sprite, 5, sprite.height - 1 - offset), edge
            )
        self.assertEqual(self.pixel(sprite, 5, BORDER), fill)
        self.assertEqual(
            self.pixel(sprite, 5, sprite.height - 1 - BORDER), fill
        )
        # Left and right edges, checked across a middle row.
        for offset in range(BORDER):
            self.assertEqual(self.pixel(sprite, offset, 5), edge)
            self.assertEqual(
                self.pixel(sprite, sprite.width - 1 - offset, 5), edge
            )
        self.assertEqual(self.pixel(sprite, BORDER, 5), fill)
        self.assertEqual(
            self.pixel(sprite, sprite.width - 1 - BORDER, 5), fill
        )


class TerminalRendererArrowThicknessTest(unittest.TestCase):
    def setUp(self):
        self.renderer = TerminalRenderer(
            graphics=None, cell_width=10, cell_height=10
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


class TerminalRendererCornerRadiusTest(unittest.TestCase):
    def setUp(self):
        self.renderer = TerminalRenderer(
            graphics=None,
            cell_width=2 * ROUNDED_RADIUS // 5,
            cell_height=2 * ROUNDED_RADIUS // 5,
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
        border = BORDER
        sprite = self.outline(Box(colour=1, fill=2, rounded=False))
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
        sprite = self.outline(Box(colour=1, fill=2, rounded=True))
        last_x, last_y = sprite.width - 1, sprite.height - 1
        self.assertEqual(self.pixel(sprite, 0, 0), TRANSPARENT)
        self.assertEqual(self.pixel(sprite, last_x, 0), TRANSPARENT)
        self.assertEqual(self.pixel(sprite, 0, last_y), TRANSPARENT)
        self.assertEqual(self.pixel(sprite, last_x, last_y), TRANSPARENT)

    def test_straight_edges_stay_as_crisp_as_a_square_box(self):
        square = self.outline(Box(colour=1, fill=2, rounded=False))
        rounded = self.outline(Box(colour=1, fill=2, rounded=True))
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
            self.outline(Box(colour=1, fill=2, rounded=rounded))
            for rounded in (False, True)
        ]
        sizes = {(sprite.width, sprite.height) for sprite in sprites}
        positions = {(sprite.col, sprite.row) for sprite in sprites}
        self.assertEqual(len(sizes), 1)
        self.assertEqual(len(positions), 1)

    def test_the_arc_is_anti_aliased(self):
        square = self.outline(Box(colour=1, fill=PLAIN, rounded=False))
        rounded = self.outline(Box(colour=1, fill=PLAIN, rounded=True))
        self.assertFalse(
            any(0 < alpha < OPAQUE for alpha in square.pixels[3::4])
        )
        self.assertTrue(
            any(0 < alpha < OPAQUE for alpha in rounded.pixels[3::4])
        )

    def test_arc_coverage_is_continuous_at_the_pixel_centre(self):
        sprite = self.outline(
            Box(colour=1, fill=PLAIN, rounded=True)
        )
        self.assertEqual(self.pixel(sprite, 14, 2), _colour(1) + (254,))

    def test_border_coverage_is_composed_over_the_opaque_fill(self):
        sprite = self.outline(Box(colour=1, fill=2, rounded=True))
        self.assertEqual(self.pixel(sprite, 20, 4), (131, 27, 25, OPAQUE))

    def test_a_rounded_box_cuts_away_more_than_a_square_one(self):
        square = self.outline(Box(colour=1, fill=2, rounded=False))
        rounded = self.outline(Box(colour=1, fill=2, rounded=True))
        self.assertLess(self.alpha_total(rounded), self.alpha_total(square))

    def test_the_fringe_keeps_the_edge_colour_instead_of_fading_to_black(self):
        sprite = self.outline(Box(colour=1, fill=PLAIN, rounded=True))
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
        box = Box(colour=1, fill=2, rounded=True)
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


class TerminalRendererSmallBoxTest(unittest.TestCase):
    """A box no larger than twice its corner radius, where the two corner
    bands of a row would otherwise overlap."""

    def setUp(self):
        cell = ROUNDED_RADIUS // 2
        self.renderer = TerminalRenderer(
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
        sprite = self.outline(Box(colour=1, fill=2, rounded=True))
        self.assertEqual(
            len(sprite.pixels), sprite.width * sprite.height * 4
        )

    def test_clipping_a_small_box_is_a_pure_crop_of_the_whole_box(self):
        box = Box(colour=1, fill=2, rounded=True)
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


class TerminalRendererSpriteTest(unittest.TestCase):
    def test_payload_is_appended_to_the_last_line(self):
        graphics = FakeGraphics()
        renderer = TerminalRenderer(graphics, cell_width=2, cell_height=4)
        self.assertEqual(
            renderer.render([], cols=3, rows=2),
            [BLANK * 3, BLANK * 3 + "<payload>"],
        )

    def sprites(self, placements, cols=40, rows=20, cell=(2, 4)):
        graphics = FakeGraphics()
        renderer = TerminalRenderer(
            graphics, cell_width=cell[0], cell_height=cell[1]
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
        renderer = TerminalRenderer(FakeGraphics(), cell_width=2, cell_height=4)
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
            Placement(LayoutArrow((0,), 0), x=1, y=1, width=2, height=1), cell=(4, 5)
        )
        self.assertEqual((sprite.width, sprite.height), (8, 5))

    def test_a_single_stop_arrow_is_a_straight_line_across_every_column(self):
        # stops == (0,) is the degenerate case: shaft and trunk collapse onto
        # one row, so the sprite is exactly today's straight "-->" arrow.
        sprite = self.only_sprite(
            Placement(LayoutArrow((0,), 0), x=1, y=1, width=2, height=1), cell=(4, 5)
        )
        shaft_row = sprite.height // 2
        for x in range(sprite.width):
            self.assertEqual(self.pixel(sprite, x, shaft_row)[3], OPAQUE)

    def test_a_single_stop_arrow_tip_is_arrow_stroke_pixels_at_the_right_edge(self):
        # With a 1px stroke the tip would be a single lit pixel; at
        # ARROW_STROKE thickness the nearby stamps overlap and widen the
        # opaque band a little past the raw stroke width, so the sprite is
        # tall enough here to keep that band clear of the top/bottom edges.
        sprite = self.only_sprite(
            Placement(LayoutArrow((0,), 0), x=1, y=1, width=2, height=1), cell=(4, 40)
        )
        shaft_row = sprite.height // 2
        tip = sprite.width - 1
        low = shaft_row - ARROW_STROKE // 2
        high = shaft_row + ARROW_STROKE // 2 + 2
        expected_rows = set(range(low, high))
        for y in range(sprite.height):
            expected = OPAQUE if y in expected_rows else 0
            self.assertEqual(self.pixel(sprite, tip, y)[3], expected)

    def test_a_single_stop_arrowhead_diagonals_are_symmetric_about_the_shaft(self):
        sprite = self.only_sprite(
            Placement(LayoutArrow((0,), 0), x=1, y=1, width=2, height=1), cell=(4, 40)
        )
        shaft_row = sprite.height // 2
        # Base column of the head: furthest from the tip, widest spread.
        base = sprite.width - 4
        opaque_ys = [
            y
            for y in range(sprite.height)
            if y != shaft_row and self.pixel(sprite, base, y)[3] == OPAQUE
        ]
        self.assertTrue(opaque_ys)
        top, bottom = min(opaque_ys), max(opaque_ys)
        # A 4px stroke can't be perfectly centred on a single row, so allow
        # the off-by-one bias that _centered_span's rounding introduces.
        self.assertAlmostEqual(shaft_row - top, bottom - shaft_row, delta=1)
        self.assertGreater(bottom, shaft_row)

    def test_arrowhead_shape_matches_the_thirty_degree_geometry(self):
        sprite = self.only_sprite(
            Placement(LayoutArrow((0,), 0), x=1, y=1, width=3, height=1), cell=(40, 20)
        )
        tip = sprite.width - 1

        def half_span_at(distance_from_tip):
            opaque = [
                y
                for y in range(sprite.height)
                if self.pixel(sprite, tip - distance_from_tip, y)[3] == OPAQUE
            ]
            return (max(opaque) - min(opaque)) / 2

        depth = int(ARROWHEAD_EDGE_LENGTH * cos(radians(ARROWHEAD_ANGLE_DEG)))
        spans = [half_span_at(d) for d in range(depth)]
        # The angle and depth are unchanged: the head still widens
        # monotonically towards its base, by at least as much as the old,
        # unthickened 30-degree geometry would.
        self.assertEqual(spans, sorted(spans))
        self.assertGreaterEqual(spans[0], (ARROW_STROKE - 1) / 2)
        self.assertGreaterEqual(
            spans[-1] - spans[0],
            round((depth - 1) * tan(radians(ARROWHEAD_ANGLE_DEG))),
        )

    def test_arrow_off_shape_pixels_are_transparent(self):
        sprite = self.only_sprite(
            Placement(LayoutArrow((0,), 0), x=1, y=1, width=2, height=1), cell=(4, 5)
        )
        self.assertEqual(self.pixel(sprite, 0, 0), (0, 0, 0, 0))

    def test_an_arrow_is_plain_grey(self):
        sprite = self.only_sprite(
            Placement(LayoutArrow((0,), 0), x=1, y=1, width=2, height=1), cell=(4, 5)
        )
        shaft_row = sprite.height // 2
        self.assertEqual(
            self.pixel(sprite, 0, shaft_row), PLAIN_COLOUR + (OPAQUE,)
        )

    def test_an_arrow_overhanging_the_top_is_cropped(self):
        sprite = self.only_sprite(
            Placement(LayoutArrow((0, 3), 0), x=1, y=-1, width=2, height=4), cell=(4, 5)
        )
        self.assertEqual(sprite.row, 0)
        self.assertEqual(sprite.height, 15)
        self.assertEqual(len(sprite.pixels), sprite.width * sprite.height * 4)

    def test_an_arrow_overhanging_the_bottom_is_cropped(self):
        sprite = self.only_sprite(
            Placement(LayoutArrow((0, 3), 0), x=1, y=1, width=2, height=4),
            cols=40,
            rows=2,
            cell=(4, 5),
        )
        self.assertEqual(sprite.row, 1)
        self.assertEqual(sprite.height, 5)
        self.assertEqual(len(sprite.pixels), sprite.width * sprite.height * 4)

    def test_a_branching_arrow_trunk_spans_from_the_shaft_to_the_last_stop(self):
        sprite = self.only_sprite(
            Placement(LayoutArrow((0, 3), 0), x=1, y=1, width=2, height=4), cell=(4, 5)
        )
        midpoint = sprite.width // 2
        shaft_row = 5 // 2  # centre of the parent's own row
        last_stop_row = 3 * 5 + 5 // 2
        for y in range(shaft_row, last_stop_row + 1):
            self.assertEqual(self.pixel(sprite, midpoint, y)[3], OPAQUE)

    def test_a_branching_arrow_has_a_stub_at_every_stop(self):
        sprite = self.only_sprite(
            Placement(LayoutArrow((0, 3), 0), x=1, y=1, width=2, height=4), cell=(4, 5)
        )
        midpoint = sprite.width // 2
        for stop in (0, 3):
            row = stop * 5 + 5 // 2
            for x in range(midpoint, sprite.width):
                self.assertEqual(self.pixel(sprite, x, row)[3], OPAQUE)

    def test_a_branching_arrow_rows_between_stops_are_blank_past_the_trunk(self):
        sprite = self.only_sprite(
            Placement(LayoutArrow((0, 3), 0), x=1, y=1, width=2, height=4), cell=(4, 5)
        )
        midpoint = sprite.width // 2
        # The trunk itself is ARROW_STROKE pixels wide, so it occupies more
        # than just the midpoint column now.
        trunk_columns = set(_centered_span(midpoint, ARROW_STROKE))
        row_between_stops = 1 * 5 + 5 // 2
        for x in range(sprite.width):
            expected = OPAQUE if x in trunk_columns else 0
            self.assertEqual(self.pixel(sprite, x, row_between_stops)[3], expected)

    def test_three_stubs_each_end_in_their_own_arrowhead(self):
        # Cells are large enough here that the three arrowheads' opaque
        # bands, widened by ARROW_STROKE, don't merge into each other.
        sprite = self.only_sprite(
            Placement(LayoutArrow((0, 1, 3), 0), x=1, y=1, width=2, height=4), cell=(4, 60)
        )
        tip = sprite.width - 1
        stop_rows = {stop * 60 + 60 // 2 for stop in (0, 1, 3)}
        expected_rows = set()
        for stop_row in stop_rows:
            low = stop_row - ARROW_STROKE // 2
            high = stop_row + ARROW_STROKE // 2 + 2
            expected_rows |= set(range(low, high))
        for y in range(sprite.height):
            expected = OPAQUE if y in expected_rows else 0
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
        self.renderer = TerminalRenderer(FakeGraphics(), cell_width=2, cell_height=4)

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
        one = self.draw(Placement(LayoutArrow((0,), 0), x=0, y=0, width=4, height=6))
        two = self.draw(Placement(LayoutArrow((0, 2), 0), x=0, y=0, width=4, height=6))
        self.assertNotEqual(one.pixels, two.pixels)

    def test_arrows_with_different_shafts_are_redrawn(self):
        one = self.draw(Placement(LayoutArrow((0, 2), 0), x=0, y=0, width=4, height=6))
        two = self.draw(Placement(LayoutArrow((0, 2), 1), x=0, y=0, width=4, height=6))
        self.assertNotEqual(one.pixels, two.pixels)

    def test_the_cache_is_bounded(self):
        for width in range(CACHE_LIMIT + 2):
            self.renderer._sprites(
                [Placement(Box("hi"), x=0, y=0, width=width + 1, height=3)], 4000, 20
            )
        self.assertLessEqual(len(self.renderer.cache), CACHE_LIMIT)


if __name__ == "__main__":
    unittest.main()
