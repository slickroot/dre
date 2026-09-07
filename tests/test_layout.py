import unittest

from sketch.layout import BOX_SIZE, Placement, layout
from sketch.state import Box, State


class LayoutTest(unittest.TestCase):
    def test_empty_state_has_no_placements(self):
        self.assertEqual(layout(State([]), cols=11, rows=11), [])

    def test_box_is_centered(self):
        placements = layout(State([Box()]), cols=11, rows=11)
        self.assertEqual(
            placements, [Placement(Box(), x=4, y=4, width=3, height=3)]
        )

    def test_box_is_sized_by_the_layout(self):
        placement = layout(State([Box()]), cols=11, rows=11)[0]
        self.assertEqual((placement.width, placement.height), (BOX_SIZE, BOX_SIZE))

    def test_box_stays_centered_in_a_larger_terminal(self):
        placement = layout(State([Box()]), cols=81, rows=25)[0]
        self.assertEqual(
            (placement.x + BOX_SIZE / 2, placement.y + BOX_SIZE / 2),
            (81 / 2, 25 / 2),
        )

    def test_every_node_gets_a_placement(self):
        self.assertEqual(len(layout(State([Box(), Box()]), cols=11, rows=11)), 2)


class PlacementTest(unittest.TestCase):
    def test_positional_and_keyword_construction_agree(self):
        self.assertEqual(
            Placement(Box(), 4, 4, 3, 3),
            Placement(Box(), x=4, y=4, width=3, height=3),
        )


if __name__ == "__main__":
    unittest.main()
