import unittest

from sketch.layout import Placement
from sketch.render import CURSOR, TerminalRenderer, _cell
from sketch.state import PLAIN, Box, Cursor


class TerminalRendererTest(unittest.TestCase):
    def setUp(self):
        self.renderer = TerminalRenderer()

    def test_empty_canvas_fills_terminal(self):
        self.assertEqual(
            self.renderer.render([], cols=11, rows=5), [" " * 11] * 5
        )

    def test_grid_matches_the_requested_size(self):
        cols, rows = 20, 7
        grid = self.renderer.render([Placement(Box(), 4, 4, 3, 3)], cols, rows)
        self.assertEqual(len(grid), rows)
        self.assertEqual({len(line) for line in grid}, {cols})

    def test_box_is_drawn(self):
        grid = self.renderer.render([Placement(Box(), 4, 4, 3, 3)], 11, 11)
        self.assertEqual(grid[4][4:7], "┌─┐")

    def test_box_sides_and_bottom_are_drawn(self):
        grid = self.renderer.render([Placement(Box(), 4, 4, 3, 3)], 11, 11)
        self.assertEqual(
            [line[4:7] for line in grid[4:7]], ["┌─┐", "│ │", "└─┘"]
        )

    def test_box_interior_is_empty(self):
        grid = self.renderer.render([Placement(Box(), 0, 0, 5, 4)], 5, 4)
        self.assertEqual(grid, ["┌───┐", "│   │", "│   │", "└───┘"])

    def test_nothing_is_drawn_outside_the_box(self):
        grid = self.renderer.render([Placement(Box(), 4, 4, 3, 3)], 11, 11)
        self.assertEqual(grid[3], " " * 11)
        self.assertEqual(grid[4][:4], "    ")
        self.assertEqual(grid[4][7:], "    ")

    def test_a_box_reaching_past_the_edge_is_clipped(self):
        grid = self.renderer.render([Placement(Box(), 3, 1, 3, 3)], 4, 2)
        self.assertEqual(grid, ["    ", "   \u250c"])

    def test_label_is_drawn_inside_the_box(self):
        grid = self.renderer.render([Placement(Box("hi"), 0, 0, 5, 3)], 5, 3)
        self.assertEqual(grid[1], "\u2502hi \u2502")

    def test_cursor_is_drawn_after_the_label(self):
        grid = self.renderer.render(
            [Placement(Box("hi"), 0, 0, 5, 3), Placement(Cursor(), 3, 1, 1, 1)],
            5,
            3,
        )
        self.assertEqual(grid[1], "\u2502hi\u2588\u2502")

    def test_label_and_cursor_past_the_edge_are_clipped(self):
        grid = self.renderer.render(
            [Placement(Box("hi"), 0, 0, 5, 3), Placement(Cursor(), 3, 1, 1, 1)],
            3,
            3,
        )
        self.assertEqual(grid[1], "\u2502hi")

    def test_a_box_does_not_draw_a_cursor(self):
        grid = self.renderer.render([Placement(Box("hi"), 0, 0, 5, 3)], 5, 3)
        self.assertNotIn(CURSOR, "".join(grid))

    def test_cursor_placement_is_drawn_at_its_own_position(self):
        grid = self.renderer.render([Placement(Cursor(), 2, 1, 1, 1)], 4, 3)
        self.assertEqual(grid, ["    ", "  " + CURSOR + " ", "    "])

    def test_a_cursor_outside_the_grid_is_clipped(self):
        grid = self.renderer.render([Placement(Cursor(), 9, 9, 1, 1)], 4, 3)
        self.assertEqual(grid, ["    "] * 3)

    def test_each_placement_is_drawn(self):
        grid = self.renderer.render(
            [Placement(Box(), 0, 0, 3, 3), Placement(Box(), 4, 0, 3, 3)], 11, 3
        )
        self.assertEqual(grid[0], "┌─┐ ┌─┐    ")

    def test_a_plain_box_emits_no_escapes(self):
        grid = self.renderer.render(
            [Placement(Box("hi"), 0, 0, 5, 3), Placement(Cursor(), 3, 1, 1, 1)],
            5,
            3,
        )
        self.assertNotIn("\x1b", "".join(grid))

    def test_a_coloured_box_border_carries_the_colour(self):
        colour = 2
        grid = self.renderer.render([Placement(Box(colour=colour), 0, 0, 5, 3)], 5, 3)
        self.assertTrue(grid[0].startswith(_cell("┌", colour, PLAIN)))

    def test_a_coloured_box_bottom_right_corner_carries_the_colour(self):
        colour = 4
        grid = self.renderer.render([Placement(Box(colour=colour), 0, 0, 5, 3)], 5, 3)
        self.assertTrue(grid[2].endswith(_cell("┘", colour, PLAIN)))

    def test_a_coloured_box_interior_is_plain(self):
        colour = 3
        grid = self.renderer.render([Placement(Box(colour=colour), 0, 0, 5, 3)], 5, 3)
        middle = grid[1][len(_cell("│", colour, PLAIN)) : -len(_cell("│", colour, PLAIN))]
        self.assertEqual(middle, "   ")

    def test_a_label_inside_a_coloured_box_is_plain(self):
        colour = 5
        grid = self.renderer.render(
            [Placement(Box("hi", colour=colour), 0, 0, 5, 3)], 5, 3
        )
        middle = grid[1][len(_cell("│", colour, PLAIN)) : -len(_cell("│", colour, PLAIN))]
        self.assertEqual(middle, "hi ")

    def test_the_cursor_is_plain(self):
        grid = self.renderer.render(
            [Placement(Box(colour=1), 0, 0, 5, 3), Placement(Cursor(), 1, 1, 1, 1)],
            5,
            3,
        )
        after_left_border = grid[1][len(_cell("│", 1, PLAIN)) :]
        self.assertTrue(after_left_border.startswith(CURSOR))

    def test_two_boxes_render_their_own_colours(self):
        first_colour, second_colour = 0, 6
        grid = self.renderer.render(
            [
                Placement(Box(colour=first_colour), 0, 0, 3, 3),
                Placement(Box(colour=second_colour), 4, 0, 3, 3),
            ],
            11,
            3,
        )
        self.assertTrue(grid[0].startswith(_cell("┌", first_colour, PLAIN)))
        gap = grid[0][len(_cell("┌", first_colour, PLAIN)) :]
        self.assertTrue(gap.startswith(_cell("─", first_colour, PLAIN)))
        gap = gap[len(_cell("─", first_colour, PLAIN)) :]
        self.assertTrue(gap.startswith(_cell("┐", first_colour, PLAIN)))
        gap = gap[len(_cell("┐", first_colour, PLAIN)) :]
        self.assertTrue(gap.startswith(" "))
        gap = gap[1:]
        self.assertTrue(gap.startswith(_cell("┌", second_colour, PLAIN)))

    def test_plain_box_is_still_byte_identical(self):
        grid = self.renderer.render([Placement(Box(colour=PLAIN), 0, 0, 5, 4)], 5, 4)
        self.assertEqual(grid, ["┌───┐", "│   │", "│   │", "└───┘"])

    def test_an_unfilled_box_emits_no_escapes(self):
        grid = self.renderer.render([Placement(Box(fill=PLAIN), 0, 0, 5, 4)], 5, 4)
        self.assertNotIn("\x1b", "".join(grid))

    def test_a_filled_box_interior_is_painted(self):
        fill = 2
        grid = self.renderer.render([Placement(Box(fill=fill), 0, 0, 5, 4)], 5, 4)
        for row in grid[1:3]:
            rest = row[len(_cell("│", PLAIN, PLAIN)) :]
            for _ in range(3):
                self.assertTrue(rest.startswith(_cell(" ", PLAIN, fill)))
                rest = rest[len(_cell(" ", PLAIN, fill)) :]
            self.assertTrue(rest.startswith(_cell("│", PLAIN, PLAIN)))

    def test_a_filled_box_border_is_not_painted(self):
        fill = 6
        grid = self.renderer.render([Placement(Box(fill=fill), 0, 0, 5, 4)], 5, 4)
        self.assertTrue(grid[0].startswith(_cell("┌", PLAIN, PLAIN)))
        self.assertTrue(grid[3].startswith(_cell("└", PLAIN, PLAIN)))
        self.assertTrue(grid[1].startswith(_cell("│", PLAIN, PLAIN)))

    def test_a_box_with_border_colour_and_fill_keeps_them_separate(self):
        colour, fill = 1, 4
        grid = self.renderer.render(
            [Placement(Box(colour=colour, fill=fill), 0, 0, 5, 4)], 5, 4
        )
        self.assertTrue(grid[0].startswith(_cell("┌", colour, PLAIN)))
        rest = grid[1][len(_cell("│", colour, PLAIN)) :]
        self.assertTrue(rest.startswith(_cell(" ", PLAIN, fill)))

    def test_a_label_sits_on_top_of_the_fill(self):
        fill = 3
        grid = self.renderer.render(
            [Placement(Box("hi", fill=fill), 0, 0, 5, 3)], 5, 3
        )
        rest = grid[1][len(_cell("│", PLAIN, PLAIN)) :]
        self.assertTrue(rest.startswith(_cell("h", PLAIN, fill)))
        rest = rest[len(_cell("h", PLAIN, fill)) :]
        self.assertTrue(rest.startswith(_cell("i", PLAIN, fill)))
        rest = rest[len(_cell("i", PLAIN, fill)) :]
        self.assertTrue(rest.startswith(_cell(" ", PLAIN, fill)))

    def test_the_cursor_is_plain_over_a_filled_box(self):
        grid = self.renderer.render(
            [
                Placement(Box(fill=5), 0, 0, 5, 3),
                Placement(Cursor(), 1, 1, 1, 1),
            ],
            5,
            3,
        )
        rest = grid[1][len(_cell("│", PLAIN, PLAIN)) :]
        self.assertTrue(rest.startswith(_cell(CURSOR, PLAIN, PLAIN)))

    def test_empty_canvas_with_no_boxes_emits_no_escapes(self):
        grid = self.renderer.render([], cols=11, rows=5)
        self.assertEqual(grid, [" " * 11] * 5)
        self.assertNotIn("\x1b", "".join(grid))


if __name__ == "__main__":
    unittest.main()
