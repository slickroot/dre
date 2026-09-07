import unittest

from sketch.layout import Placement
from sketch.render import CURSOR, TerminalRenderer
from sketch.state import Box, Cursor


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


if __name__ == "__main__":
    unittest.main()
