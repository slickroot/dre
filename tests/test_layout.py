import unittest

from sketch.layout import (
    BORDERS,
    BOX_HEIGHT,
    GAP_HEIGHT,
    Coordinate,
    Placement,
    Track,
    height,
    layout,
    span,
    tracks,
    walk,
    width,
)
from sketch.state import PAD, Arrow, Box, Cursor, Space, State


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
        placements = layout(State([Box(), Space(), Box()]), cols=11, rows=11)
        self.assertEqual(len(boxes(placements)), 2)

    def test_placement_indices_correspond_to_node_indices(self):
        nodes = [Box("a"), Space(), Box("bb")]
        placements = layout(State(nodes), cols=11, rows=11)
        self.assertEqual([p.node for p in placements], nodes)

    def test_a_space_is_placed_one_row_high(self):
        first, space, second = layout(State([Box(), Space(), Box()]), 11, 11)
        self.assertEqual(space.height, height(Space()))
        self.assertEqual(space.y, first.y + first.height)
        self.assertEqual(second.y, space.y + space.height)

    def test_the_stack_is_centered_on_the_total_height_of_its_nodes(self):
        nodes = [Box(), Space(), Box()]
        total = sum(height(node) for node in nodes)
        first = layout(State(nodes), cols=11, rows=11)[0]
        self.assertEqual(first.y, (11 - total) // 2)

    def test_box_widens_to_fit_the_label(self):
        placements = layout(
            State([Box("hi" + PAD)], mode="insert", selected=0), cols=11, rows=11
        )
        self.assertEqual(
            placements[0],
            Placement(Box("hi" + PAD), x=3, y=4, width=5, height=3),
        )

    def test_box_stays_centered_as_it_grows(self):
        def x(label):
            state = State([Box(label + PAD)], mode="insert", selected=0)
            return layout(state, 21, 11)[0].x

        self.assertEqual([x(""), x("ab"), x("abcd")], [9, 8, 7])

    def test_insert_mode_puts_a_cursor_inside_the_focused_box(self):
        box, cursor = layout(State([Box("hi" + PAD)], mode="insert", selected=0), 11, 11)
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
        box, cursor = layout(State([Box("hi" + PAD)], mode="insert", selected=0), 11, 11)
        interior = box.x + BORDERS // 2
        self.assertEqual(cursor.x, interior + len("hi"))
        self.assertEqual(cursor.y, box.y + BOX_HEIGHT // 2)

    def test_the_cursor_follows_the_selection_not_the_last_box(self):
        placements = layout(
            State(
                [Box("a"), Space(), Box("bb"), Space(), Box("c")], selected=0
            ),
            cols=11,
            rows=11,
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
        focused = layout(State([Box("hi" + PAD)], mode="insert", selected=0), 11, 11)[0]
        unfocused = layout(State([Box("hi")], selected=0), 11, 11)[0]
        self.assertEqual(focused.width, unfocused.width + 1)

    def test_boxes_are_unaffected_by_which_one_is_selected(self):
        nodes = [Box("a"), Space(), Box("bb"), Space(), Box("c")]

        def placed(selected):
            return boxes(layout(State(nodes, selected=selected), cols=11, rows=11))

        self.assertEqual(placed(0), placed(4))

    def test_a_second_box_sits_below_the_first(self):
        first, _, second = layout(State([Box(), Space(), Box()]), cols=11, rows=11)
        self.assertEqual(second.y - first.y, BOX_HEIGHT + height(Space()))

    def test_stacked_boxes_are_separated_by_blank_rows(self):
        first, _, second = layout(State([Box(), Space(), Box()]), cols=11, rows=11)
        occupied = set(range(first.y, first.y + first.height))
        occupied |= set(range(second.y, second.y + second.height))
        between = set(range(first.y + first.height, second.y))
        self.assertEqual(len(between - occupied), height(Space()))

    def test_the_stack_stays_vertically_centered_as_boxes_are_added(self):
        def middle(count):
            nodes = [Box()]
            for _ in range(count - 1):
                nodes += [Space(), Box()]
            placements = boxes(layout(State(nodes), cols=11, rows=11))
            top = placements[0].y
            bottom = placements[-1].y + placements[-1].height
            return (top + bottom) / 2

        for count in (1, 2, 3):
            self.assertLessEqual(abs(middle(count) - 11 / 2), 0.5)

    def test_each_box_is_centered_on_its_own_width(self):
        cols = 11
        unfocused, _, focused = layout(
            State([Box("a"), Space(), Box("a" + PAD)], mode="insert", selected=2),
            cols=cols,
            rows=11,
        )[:3]
        self.assertNotEqual(unfocused.x, focused.x)
        for box in (unfocused, focused):
            left = box.x
            right = cols - (box.x + box.width)
            self.assertLessEqual(abs(left - right), 1)

    def test_a_box_placed_via_space_right_sits_four_columns_to_the_right(self):
        first, _, second = layout(
            State([Box("a"), Space(direction="right"), Box("bb")]),
            cols=21,
            rows=11,
        )
        self.assertEqual(second.x - (first.x + first.width), 4)

    def test_two_boxes_sharing_a_row_get_the_same_y(self):
        first, _, second = layout(
            State([Box("a"), Space(direction="right"), Box("bb")]),
            cols=21,
            rows=11,
        )
        self.assertEqual(first.y, second.y)

    def test_a_pair_of_boxes_on_one_row_is_centered_as_a_unit(self):
        cols = 21
        first, _, second = layout(
            State([Box("a"), Space(direction="right"), Box("bb")]),
            cols=cols,
            rows=11,
        )
        left = first.x
        right = cols - (second.x + second.width)
        self.assertLessEqual(abs(left - right), 1)

    def test_a_box_below_the_last_box_in_column_one_stays_in_column_one(self):
        nodes = [
            Box("a"),
            Space(direction="right"),
            Box("b"),
            Space(direction="down"),
            Box("c"),
        ]
        placements = layout(State(nodes), cols=21, rows=21)
        b = boxes(placements)[1]
        c = boxes(placements)[2]
        a = boxes(placements)[0]
        self.assertEqual(c.x, b.x)
        self.assertNotEqual(c.x, a.x)

    def test_only_the_focused_box_gets_a_cursor(self):
        first, _, second, cursor = layout(
            State([Box("hi"), Space(), Box("hi" + PAD)], mode="insert", selected=2),
            cols=11,
            rows=11,
        )
        self.assertIsInstance(cursor.node, Cursor)
        self.assertTrue(second.y < cursor.y < second.y + second.height)
        self.assertTrue(second.x < cursor.x < second.x + second.width)


class SelectedNodeCursorTest(unittest.TestCase):
    def test_a_selected_box_below_a_space_still_gets_its_cursor(self):
        placements = layout(
            State([Box("a"), Space(), Box("hi")], selected=2), cols=11, rows=11
        )
        box = placements[2]
        cursor = cursors(placements)[0]
        self.assertEqual(cursor.x, box.x + BORDERS // 2 + len(box.node.label) - 1)
        self.assertEqual(cursor.y, box.y + BOX_HEIGHT // 2)


class ArrowLayoutTest(unittest.TestCase):
    def test_an_arrow_is_placed_in_the_slots_row_at_the_centre_column(self):
        nodes = [Box("a"), Arrow("down"), Box("bb")]
        placements = layout(State(nodes), cols=11, rows=11)
        arrow = placements[1]
        box = layout(State([Box()]), cols=11, rows=11)[0]
        self.assertEqual(arrow.width, 1)
        self.assertEqual(arrow.height, GAP_HEIGHT)
        self.assertEqual(arrow.y, placements[0].y + placements[0].height)
        self.assertEqual(arrow.x, (11 - 1) // 2)
        self.assertEqual(arrow.x + arrow.width // 2, box.x + box.width // 2)

    def test_an_arrow_does_not_move_the_box_below_it(self):
        with_space = layout(
            State([Box("a"), Space(), Box("bb")]), cols=11, rows=11
        )
        with_arrow = layout(
            State([Box("a"), Arrow("down"), Box("bb")]), cols=11, rows=11
        )
        self.assertEqual(boxes(with_space), boxes(with_arrow))

    def test_a_rightward_arrow_does_not_move_the_box_beside_it(self):
        with_space = layout(
            State([Box("a"), Space(direction="right"), Box("bb")]),
            cols=21,
            rows=11,
        )
        with_arrow = layout(
            State([Box("a"), Arrow("right"), Box("bb")]), cols=21, rows=11
        )
        self.assertEqual(boxes(with_space), boxes(with_arrow))

    def test_a_leftward_arrow_does_not_move_the_box_beside_it(self):
        with_space = layout(
            State([Box("a"), Space(direction="right"), Box("bb")]),
            cols=21,
            rows=11,
        )
        with_arrow = layout(
            State([Box("a"), Arrow("left"), Box("bb")]), cols=21, rows=11
        )
        self.assertEqual(boxes(with_space), boxes(with_arrow))


class HeightTest(unittest.TestCase):
    def test_a_box_is_as_tall_as_a_box(self):
        self.assertEqual(height(Box("hi")), BOX_HEIGHT)

    def test_a_space_is_a_gap_tall(self):
        self.assertEqual(height(Space()), GAP_HEIGHT)

    def test_an_arrow_is_a_gap_tall(self):
        self.assertEqual(height(Arrow()), GAP_HEIGHT)

    def test_a_cursor_is_a_gap_tall(self):
        self.assertEqual(height(Cursor()), GAP_HEIGHT)


class WidthTest(unittest.TestCase):
    def test_a_box_is_as_wide_as_its_interior_and_borders(self):
        self.assertEqual(width(Box("hi")), len("hi") + BORDERS)

    def test_a_space_has_no_width(self):
        self.assertEqual(width(Space()), 0)

    def test_a_rightward_space_is_four_columns_wide(self):
        self.assertEqual(width(Space(direction="right")), 4)

    def test_an_arrow_is_one_column_wide(self):
        self.assertEqual(width(Arrow()), 1)

    def test_a_vertical_arrow_is_one_column_wide(self):
        self.assertEqual(width(Arrow("up")), 1)
        self.assertEqual(width(Arrow("down")), 1)

    def test_a_horizontal_arrow_is_four_columns_wide(self):
        self.assertEqual(width(Arrow("right")), 4)
        self.assertEqual(width(Arrow("left")), 4)


class WalkTest(unittest.TestCase):
    def test_a_lone_node_sits_at_the_origin(self):
        self.assertEqual(walk([Box()]), [Coordinate(0, 0)])

    def test_coordinates_are_index_aligned_with_the_nodes_including_separators(self):
        nodes = [Box("a"), Space(direction="down"), Box("b")]
        coordinates = walk(nodes)
        self.assertEqual(len(coordinates), len(nodes))

    def test_a_downward_space_and_the_box_after_it_each_take_their_own_row(self):
        nodes = [Box("a"), Space(direction="down"), Box("b")]
        self.assertEqual(
            walk(nodes), [Coordinate(0, 0), Coordinate(1, 0), Coordinate(2, 0)]
        )

    def test_a_rightward_space_and_the_box_after_it_take_their_own_column_on_the_same_row(
        self,
    ):
        nodes = [Box("a"), Space(direction="right"), Box("b")]
        self.assertEqual(
            walk(nodes), [Coordinate(0, 0), Coordinate(0, 1), Coordinate(0, 2)]
        )

    def test_turning_down_after_turning_right_keeps_the_column(self):
        nodes = [
            Box(),
            Space(direction="right"),
            Box(),
            Space(direction="down"),
            Box(),
        ]
        self.assertEqual(
            walk(nodes),
            [
                Coordinate(0, 0),
                Coordinate(0, 1),
                Coordinate(0, 2),
                Coordinate(1, 2),
                Coordinate(2, 2),
            ],
        )

    def test_an_arrow_moves_the_turtle_like_the_space_it_replaced_on_both_axes(self):
        with_space_down = walk([Box(), Space(direction="down"), Box()])
        with_arrow_down = walk([Box(), Arrow("down"), Box()])
        self.assertEqual(with_space_down, with_arrow_down)

        with_space_right = walk([Box(), Space(direction="right"), Box()])
        with_arrow_right = walk([Box(), Arrow("right"), Box()])
        self.assertEqual(with_space_right, with_arrow_right)

    def test_a_leftward_arrow_advances_the_column_like_a_rightward_arrow(self):
        with_arrow_right = walk([Box(), Arrow("right"), Box()])
        with_arrow_left = walk([Box(), Arrow("left"), Box()])
        self.assertEqual(with_arrow_right, with_arrow_left)

    def test_the_minimum_row_and_column_are_both_zero(self):
        nodes = [
            Box(),
            Space(direction="right"),
            Box(),
            Space(direction="down"),
            Box(),
        ]
        coordinates = walk(nodes)
        self.assertEqual(min(c.row for c in coordinates), 0)
        self.assertEqual(min(c.col for c in coordinates), 0)


class TracksTest(unittest.TestCase):
    def test_a_lone_track_takes_the_extent_of_its_only_member(self):
        self.assertEqual(tracks([5], [0]), [Track(offset=0, extent=5)])

    def test_a_track_takes_the_extent_of_its_widest_member(self):
        self.assertEqual(tracks([3, 5], [0, 0]), [Track(offset=0, extent=5)])

    def test_offsets_are_the_running_sum_of_the_extents_before_them(self):
        self.assertEqual(
            tracks([3, 5, 2], [0, 1, 2]),
            [
                Track(offset=0, extent=3),
                Track(offset=3, extent=5),
                Track(offset=8, extent=2),
            ],
        )

    def test_a_gap_between_indices_still_gets_a_zero_extent_track(self):
        self.assertEqual(
            tracks([4, 6], [0, 2]),
            [
                Track(offset=0, extent=4),
                Track(offset=4, extent=0),
                Track(offset=4, extent=6),
            ],
        )

    def test_span_is_the_sum_of_every_tracks_extent(self):
        self.assertEqual(span(tracks([3, 5, 2], [0, 1, 2])), 10)

    def test_span_of_a_single_track_is_its_extent(self):
        self.assertEqual(span(tracks([7], [0])), 7)


class PlacementTest(unittest.TestCase):
    def test_positional_and_keyword_construction_agree(self):
        self.assertEqual(
            Placement(Box(), 4, 4, 3, 3),
            Placement(Box(), x=4, y=4, width=3, height=3),
        )


if __name__ == "__main__":
    unittest.main()
