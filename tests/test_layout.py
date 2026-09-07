import unittest

from sketch.layout import BORDERS_AND_CURSOR, BOX_HEIGHT, Placement, layout
from sketch.state import Box, State


class LayoutTest(unittest.TestCase):
    def test_empty_state_has_no_placements(self):
        self.assertEqual(layout(State([]), cols=11, rows=11), [])

    def test_box_is_centered(self):
        placements = layout(State([Box()]), cols=11, rows=11)
        self.assertEqual(
            placements,
            [Placement(Box(), x=4, y=4, width=3, height=3, cursor=None)],
        )

    def test_box_is_sized_by_the_layout(self):
        placement = layout(State([Box()]), cols=11, rows=11)[0]
        self.assertEqual(
            (placement.width, placement.height),
            (BORDERS_AND_CURSOR, BOX_HEIGHT),
        )

    def test_box_stays_centered_in_a_larger_terminal(self):
        placement = layout(State([Box()]), cols=81, rows=25)[0]
        self.assertEqual(
            (
                placement.x + BORDERS_AND_CURSOR / 2,
                placement.y + BOX_HEIGHT / 2,
            ),
            (81 / 2, 25 / 2),
        )

    def test_every_node_gets_a_placement(self):
        self.assertEqual(len(layout(State([Box(), Box()]), cols=11, rows=11)), 2)

    def test_box_widens_to_fit_the_label(self):
        placements = layout(State([Box("hi")], mode="insert"), cols=11, rows=11)
        self.assertEqual(
            placements,
            [Placement(Box("hi"), x=3, y=4, width=5, height=3, cursor=2)],
        )

    def test_box_stays_centered_as_it_grows(self):
        def x(label):
            return layout(State([Box(label)], mode="insert"), 21, 11)[0].x

        self.assertEqual([x(""), x("ab"), x("abcd")], [9, 8, 7])

    def test_box_never_shrinks_below_3x3(self):
        placement = layout(State([Box("")], mode="insert"), cols=11, rows=11)[0]
        self.assertEqual((placement.width, placement.height), (3, 3))

    def test_no_cursor_in_command_mode(self):
        self.assertIsNone(layout(State([Box("hi")]), 11, 11)[0].cursor)


class PlacementTest(unittest.TestCase):
    def test_positional_and_keyword_construction_agree(self):
        self.assertEqual(
            Placement(Box(), 4, 4, 3, 3, 0),
            Placement(Box(), x=4, y=4, width=3, height=3, cursor=0),
        )

    def test_cursor_defaults_to_none(self):
        self.assertIsNone(Placement(Box(), 4, 4, 3, 3).cursor)


if __name__ == "__main__":
    unittest.main()
