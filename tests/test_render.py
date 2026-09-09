import unittest

from sketch.layout import Placement
from sketch.render import (
    ARROW_DOWN,
    ARROW_UP,
    BLANK,
    BOTTOM_LEFT,
    BOTTOM_RIGHT,
    CURSOR,
    HORIZONTAL,
    TOP_LEFT,
    TOP_RIGHT,
    VERTICAL,
    TerminalRenderer,
    _cell,
)
from sketch.state import PLAIN, Arrow, Box, Cursor


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
        grid = self.renderer.render([Placement(Box("hi"), 0, 0, 5, 3)], 5, 3)
        self.assertEqual(grid[1], BLANK + "hi" + BLANK * 2)

    def test_cursor_is_drawn_after_the_label(self):
        grid = self.renderer.render(
            [Placement(Box("hi"), 0, 0, 5, 3), Placement(Cursor(), 3, 1, 1, 1)],
            5,
            3,
        )
        self.assertEqual(grid[1], BLANK + "hi" + CURSOR + BLANK)

    def test_label_and_cursor_past_the_edge_are_clipped(self):
        grid = self.renderer.render(
            [Placement(Box("hi"), 0, 0, 5, 3), Placement(Cursor(), 3, 1, 1, 1)],
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
            [Placement(Box("hi"), 0, 0, 5, 3), Placement(Cursor(), 3, 1, 1, 1)],
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
            [Placement(Box("hi", colour=5), 0, 0, 5, 3)], 5, 3
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

    def test_a_forward_arrow_leaves_the_gap_blank(self):
        grid = self.renderer.render(
            [Placement(Arrow("down"), 2, 1, 1, 2)], 4, 4
        )
        self.assertEqual(grid, [BLANK * 4] * 4)

    def test_a_backward_arrow_leaves_the_gap_blank(self):
        grid = self.renderer.render(
            [Placement(Arrow("up"), 2, 1, 1, 2)], 4, 4
        )
        self.assertEqual(grid, [BLANK * 4] * 4)

    def test_an_arrow_outside_the_grid_is_clipped(self):
        grid = self.renderer.render([Placement(Arrow("down"), 9, 9, 1, 2)], 4, 3)
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
            [Placement(Box("hi", fill=fill), 0, 0, 5, 3)], 5, 3
        )
        self.assertEqual(
            grid[1],
            _cell(BLANK, PLAIN, fill)
            + _cell("h", PLAIN, fill)
            + _cell("i", PLAIN, fill)
            + _cell(BLANK, PLAIN, fill) * 2,
        )

    def test_the_cursor_is_plain_over_a_filled_box(self):
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
        self.assertTrue(rest.startswith(_cell(CURSOR, PLAIN, PLAIN)))

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


class DrawArrowTest(unittest.TestCase):
    def setUp(self):
        self.renderer = TerminalRenderer()

    def test_a_forward_arrow_draws_the_down_glyph(self):
        grid = [[(" ", PLAIN, PLAIN)] * 4 for _ in range(3)]
        self.renderer._draw_arrow(grid, Placement(Arrow("down"), 2, 1, 1, 2))
        self.assertEqual(grid[1][2], (ARROW_DOWN, PLAIN, PLAIN))

    def test_a_backward_arrow_draws_the_up_glyph(self):
        grid = [[(" ", PLAIN, PLAIN)] * 4 for _ in range(3)]
        self.renderer._draw_arrow(grid, Placement(Arrow("up"), 2, 1, 1, 2))
        self.assertEqual(grid[1][2], (ARROW_UP, PLAIN, PLAIN))


if __name__ == "__main__":
    unittest.main()
