import unittest

from sketch.layout import BORDERS, BOX_HEIGHT, GAP, Placement, layout
from sketch.state import Box, Cursor, State


def boxes(placements):
    return [p for p in placements if isinstance(p.node, Box)]


def cursors(placements):
    return [p for p in placements if isinstance(p.node, Cursor)]


class LayoutTest(unittest.TestCase):
    def test_empty_state_has_no_placements(self):
        self.assertEqual(layout(State([]), cols=11, rows=11), [])

    def test_box_is_centered(self):
        placements = layout(State([Box()]), cols=11, rows=11)
        self.assertEqual(
            placements,
            [Placement(Box(), x=4, y=4, width=BORDERS + 1, height=BOX_HEIGHT)],
        )

    def test_box_is_sized_by_the_layout(self):
        placement = layout(State([Box()]), cols=11, rows=11)[0]
        self.assertEqual(
            (placement.width, placement.height),
            (BORDERS + 1, BOX_HEIGHT),
        )

    def test_box_stays_centered_in_a_larger_terminal(self):
        placement = layout(State([Box()]), cols=80, rows=25)[0]
        self.assertLessEqual(abs(80 - 2 * placement.x - placement.width), 1)
        self.assertLessEqual(abs(25 - 2 * placement.y - placement.height), 1)

    def test_every_node_gets_a_placement(self):
        self.assertEqual(len(boxes(layout(State([Box(), Box()]), cols=11, rows=11))), 2)

    def test_box_widens_to_fit_the_label(self):
        placements = layout(
            State([Box("hi")], mode="insert", selected=0), cols=11, rows=11
        )
        self.assertEqual(
            placements[0],
            Placement(Box("hi"), x=3, y=4, width=5, height=3),
        )

    def test_box_stays_centered_as_it_grows(self):
        def x(label):
            state = State([Box(label)], mode="insert", selected=0)
            return layout(state, 21, 11)[0].x

        self.assertEqual([x(""), x("ab"), x("abcd")], [9, 8, 7])

    def test_insert_mode_puts_a_cursor_inside_the_focused_box(self):
        box, cursor = layout(State([Box("hi")], mode="insert", selected=0), 11, 11)
        self.assertIsInstance(cursor.node, Cursor)
        self.assertTrue(box.x < cursor.x < box.x + box.width)
        self.assertTrue(box.y < cursor.y < box.y + box.height)

    def test_command_mode_emits_a_cursor(self):
        placements = layout(State([Box("hi")], selected=0), 11, 11)
        box, cursor = placements
        self.assertIsInstance(cursor.node, Cursor)
        self.assertTrue(box.x < cursor.x < box.x + box.width)
        self.assertTrue(box.y < cursor.y < box.y + box.height)

    def test_the_command_mode_cursor_sits_on_the_last_character(self):
        box, cursor = layout(State([Box("hi")], selected=0), 11, 11)
        interior = box.x + BORDERS // 2
        self.assertEqual(cursor.x, interior + len(box.node.label) - 1)
        self.assertEqual(cursor.y, box.y + BOX_HEIGHT // 2)

    def test_the_insert_mode_cursor_sits_one_past_the_last_character(self):
        box, cursor = layout(State([Box("hi")], mode="insert", selected=0), 11, 11)
        interior = box.x + BORDERS // 2
        self.assertEqual(cursor.x, interior + len(box.node.label))
        self.assertEqual(cursor.y, box.y + BOX_HEIGHT // 2)

    def test_the_cursor_follows_the_selection_not_the_last_box(self):
        placements = layout(
            State([Box("a"), Box("bb"), Box("c")], selected=0), cols=11, rows=11
        )
        first = boxes(placements)[0]
        cursor = cursors(placements)[0]
        self.assertTrue(first.x <= cursor.x < first.x + first.width)
        self.assertTrue(first.y <= cursor.y < first.y + first.height)

    def test_an_empty_canvas_emits_no_cursor(self):
        self.assertEqual(cursors(layout(State([]), cols=11, rows=11)), [])

    def test_an_unselected_canvas_emits_no_cursor(self):
        self.assertEqual(cursors(layout(State([Box("hi")]), cols=11, rows=11)), [])

    def test_an_empty_box_has_a_one_column_interior(self):
        box, cursor = layout(State([Box()], selected=0), cols=11, rows=11)
        self.assertEqual(box.width, BORDERS + 1)
        self.assertEqual(cursor.x, box.x + BORDERS // 2)

    def test_the_cursor_widens_the_box_it_sits_in(self):
        focused = layout(State([Box("hi")], mode="insert", selected=0), 11, 11)[0]
        unfocused = layout(State([Box("hi")], selected=0), 11, 11)[0]
        self.assertEqual(focused.width, unfocused.width + 1)

    def test_boxes_are_unaffected_by_which_one_is_selected(self):
        nodes = [Box("a"), Box("bb"), Box("c")]

        def placed(selected):
            return boxes(layout(State(nodes, selected=selected), cols=11, rows=11))

        self.assertEqual(placed(0), placed(2))

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
            placements = boxes(layout(State([Box()] * count), cols=11, rows=11))
            top = placements[0].y
            bottom = placements[-1].y + placements[-1].height
            return (top + bottom) / 2

        self.assertEqual([middle(1), middle(2), middle(3)], [11 / 2] * 3)

    def test_each_box_is_centered_on_its_own_width(self):
        cols = 11
        unfocused, focused = layout(
            State([Box("a"), Box("a")], mode="insert", selected=1), cols=cols, rows=11
        )[:2]
        self.assertNotEqual(unfocused.x, focused.x)
        for box in (unfocused, focused):
            left = box.x
            right = cols - (box.x + box.width)
            self.assertLessEqual(abs(left - right), 1)

    def test_only_the_focused_box_gets_a_cursor(self):
        first, second, cursor = layout(
            State([Box("hi"), Box("hi")], mode="insert", selected=1), cols=11, rows=11
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
