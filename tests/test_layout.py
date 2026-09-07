import unittest

from sketch.layout import BORDERS, BOX_HEIGHT, GAP, Placement, layout
from sketch.state import Box, Cursor, State


class LayoutTest(unittest.TestCase):
    def test_empty_state_has_no_placements(self):
        self.assertEqual(layout(State([]), cols=11, rows=11), [])

    def test_box_is_centered(self):
        placements = layout(State([Box()]), cols=11, rows=11)
        self.assertEqual(
            placements,
            [Placement(Box(), x=4, y=4, width=2, height=3)],
        )

    def test_box_is_sized_by_the_layout(self):
        placement = layout(State([Box()]), cols=11, rows=11)[0]
        self.assertEqual(
            (placement.width, placement.height),
            (BORDERS, BOX_HEIGHT),
        )

    def test_box_stays_centered_in_a_larger_terminal(self):
        placement = layout(State([Box()]), cols=80, rows=25)[0]
        self.assertEqual(
            (
                placement.x + BORDERS / 2,
                placement.y + BOX_HEIGHT / 2,
            ),
            (80 / 2, 25 / 2),
        )

    def test_every_node_gets_a_placement(self):
        self.assertEqual(len(layout(State([Box(), Box()]), cols=11, rows=11)), 2)

    def test_box_widens_to_fit_the_label(self):
        placements = layout(State([Box("hi")], mode="insert"), cols=11, rows=11)
        self.assertEqual(
            placements[0],
            Placement(Box("hi"), x=3, y=4, width=5, height=3),
        )

    def test_box_stays_centered_as_it_grows(self):
        def x(label):
            return layout(State([Box(label)], mode="insert"), 21, 11)[0].x

        self.assertEqual([x(""), x("ab"), x("abcd")], [9, 8, 7])

    def test_insert_mode_puts_a_cursor_inside_the_focused_box(self):
        box, cursor = layout(State([Box("hi")], mode="insert"), 11, 11)
        self.assertIsInstance(cursor.node, Cursor)
        self.assertTrue(box.x < cursor.x < box.x + box.width)
        self.assertTrue(box.y < cursor.y < box.y + box.height)

    def test_command_mode_emits_no_cursor(self):
        placements = layout(State([Box("hi")]), 11, 11)
        self.assertEqual([type(p.node) for p in placements], [Box])

    def test_the_cursor_widens_the_box_it_sits_in(self):
        focused = layout(State([Box("hi")], mode="insert"), 11, 11)[0]
        unfocused = layout(State([Box("hi")]), 11, 11)[0]
        self.assertEqual(focused.width, unfocused.width + 1)

    def test_a_second_box_sits_below_the_first(self):
        first, second = layout(State([Box(), Box()]), cols=11, rows=11)
        self.assertEqual(second.y - first.y, BOX_HEIGHT + GAP)

    def test_stacked_boxes_are_separated_by_blank_rows(self):
        first, second = layout(State([Box(), Box()]), cols=11, rows=11)
        occupied = set(range(first.y, first.y + first.height))
        occupied |= set(range(second.y, second.y + second.height))
        between = set(range(first.y + first.height, second.y))
        self.assertEqual(len(between - occupied), GAP)

    def test_the_stack_stays_vertically_centered_as_boxes_are_added(self):
        def middle(count):
            placements = layout(State([Box()] * count), cols=11, rows=11)
            top = placements[0].y
            bottom = placements[-1].y + placements[-1].height
            return (top + bottom) / 2

        self.assertEqual([middle(1), middle(2), middle(3)], [11 / 2] * 3)

    def test_each_box_is_centered_on_its_own_width(self):
        cols = 11
        unfocused, focused = layout(
            State([Box("a"), Box("a")], mode="insert"), cols=cols, rows=11
        )[:2]
        self.assertNotEqual(unfocused.x, focused.x)
        for box in (unfocused, focused):
            left = box.x
            right = cols - (box.x + box.width)
            self.assertLessEqual(abs(left - right), 1)

    def test_only_the_focused_box_gets_a_cursor(self):
        first, second, cursor = layout(
            State([Box("hi"), Box("hi")], mode="insert"), cols=11, rows=11
        )
        self.assertIsInstance(cursor.node, Cursor)
        self.assertTrue(second.y < cursor.y < second.y + second.height)
        self.assertTrue(second.x < cursor.x < second.x + second.width)


class PlacementTest(unittest.TestCase):
    def test_positional_and_keyword_construction_agree(self):
        self.assertEqual(
            Placement(Box(), 4, 4, 3, 3),
            Placement(Box(), x=4, y=4, width=3, height=3),
        )


if __name__ == "__main__":
    unittest.main()
