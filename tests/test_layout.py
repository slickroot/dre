import unittest

from sketch.layout import (
    BORDERS,
    BOX_HEIGHT,
    GAP_HEIGHT,
    GAP_WIDTH,
    LEAF_STRIDE,
    Arrow,
    Cursor,
    Label,
    Placement,
    Track,
    assign,
    centre,
    forest,
    height,
    interior,
    layout,
    span,
    tracks,
    walk,
    width,
    with_cursor,
)
from sketch.state import PAD, Box, State


def boxes(placements):
    return [p for p in placements if isinstance(p.node, Box)]


def arrows(placements):
    return [p for p in placements if isinstance(p.node, Arrow)]


def cursors(placements):
    return [p for p in placements if isinstance(p.node, Cursor)]


def labels(placements):
    return [p for p in placements if isinstance(p.node, Label)]


def find(placements, label):
    for placement in boxes(placements):
        if placement.node.label == label:
            return placement
    raise AssertionError(f"no box placed with label {label!r}")


class LayoutTest(unittest.TestCase):
    def test_empty_state_has_no_placements(self):
        self.assertEqual(layout(State(()).boxes, cols=11, rows=11), [])

    def test_box_is_centered(self):
        placements = layout(State((Box(),)).boxes, cols=11, rows=11)
        self.assertEqual(
            boxes(placements),
            [Placement(Box(), x=4, y=4, width=BORDERS + 1, height=BOX_HEIGHT)],
        )

    def test_box_is_sized_by_the_layout(self):
        placement = layout(State((Box(),)).boxes, cols=11, rows=11)[0]
        self.assertEqual((placement.width, placement.height), (BORDERS + 1, BOX_HEIGHT))

    def test_box_stays_centered_in_a_larger_terminal(self):
        placement = layout(State((Box(),)).boxes, cols=80, rows=25)[0]
        self.assertLessEqual(abs(80 - 2 * placement.x - placement.width), 1)
        self.assertLessEqual(abs(25 - 2 * placement.y - placement.height), 1)

    def test_box_widens_to_fit_the_label(self):
        state = State((Box("hi" + PAD),), mode="insert", selected=(0,))
        placements = layout(state.boxes, cols=11, rows=11)
        self.assertEqual(
            boxes(placements)[0],
            Placement(Box("hi" + PAD), x=3, y=4, width=5, height=3),
        )

    def test_box_stays_centered_as_it_grows(self):
        def x(label):
            state = State((Box(label + PAD),), mode="insert", selected=(0,))
            return boxes(layout(state.boxes, 21, 11))[0].x

        self.assertEqual([x(""), x("ab"), x("abcd")], [9, 8, 7])

    def test_two_top_level_boxes_stack_as_siblings(self):
        first, second = boxes(layout(State((Box(), Box())).boxes, cols=11, rows=11))
        self.assertEqual(second.y - first.y, BOX_HEIGHT + GAP_HEIGHT)
        self.assertEqual(first.x, second.x)

    def test_the_stack_of_top_level_boxes_is_vertically_centered(self):
        placements = boxes(layout(State((Box(), Box(), Box())).boxes, cols=11, rows=11))
        top = placements[0].y
        bottom = placements[-1].y + placements[-1].height
        self.assertLessEqual(abs((top + bottom) / 2 - 11 / 2), 0.5)


class ColumnWidthTest(unittest.TestCase):
    def test_top_level_boxes_share_the_width_of_the_widest(self):
        placements = layout(State((Box("x"), Box("wide"))).boxes, cols=31, rows=21)
        short, wide = find(placements, "x"), find(placements, "wide")
        self.assertEqual(short.width, width(Box("wide")))
        self.assertEqual(short.width, wide.width)

    def test_boxes_in_a_column_are_left_aligned_with_each_other(self):
        placements = layout(State((Box("x"), Box("wide"))).boxes, cols=31, rows=21)
        self.assertEqual(find(placements, "x").x, find(placements, "wide").x)

    def test_siblings_share_the_width_of_the_widest_sibling(self):
        state = State((Box("a", children=(Box("c"), Box("dddd"))),))
        placements = layout(state.boxes, cols=31, rows=21)
        short, wide = find(placements, "c"), find(placements, "dddd")
        self.assertEqual(short.width, width(Box("dddd")))
        self.assertEqual(short.x, wide.x)

    def test_a_narrower_column_does_not_widen_to_match_another(self):
        state = State((Box("a", children=(Box("dddd"),)),))
        placements = layout(state.boxes, cols=31, rows=21)
        self.assertEqual(find(placements, "a").width, width(Box("a")))


class CursorTest(unittest.TestCase):
    def test_an_empty_canvas_emits_no_cursor(self):
        placements = with_cursor(layout(State(()).boxes, cols=11, rows=11), ())
        self.assertEqual(cursors(placements), [])

    def test_nothing_selected_emits_no_cursor(self):
        placements = with_cursor(
            layout(State((Box("hi"),)).boxes, cols=11, rows=11), ()
        )
        self.assertEqual(cursors(placements), [])

    def test_command_mode_cursor_sits_on_the_last_character(self):
        state = State((Box("hi"),), selected=(0,))
        placements = with_cursor(layout(state.boxes, 11, 11), state.selected)
        box = boxes(placements)[0]
        cursor = cursors(placements)[0]
        interior = box.x + BORDERS // 2
        self.assertEqual(cursor.x, interior + len("hi") - 1)
        self.assertEqual(cursor.y, box.y + BOX_HEIGHT // 2)

    def test_insert_mode_cursor_sits_one_past_the_last_character(self):
        state = State((Box("hi" + PAD),), mode="insert", selected=(0,))
        placements = with_cursor(layout(state.boxes, 11, 11), state.selected)
        box = boxes(placements)[0]
        cursor = cursors(placements)[0]
        interior = box.x + BORDERS // 2
        self.assertEqual(cursor.x, interior + len("hi"))
        self.assertEqual(cursor.y, box.y + BOX_HEIGHT // 2)

    def test_cursor_follows_a_selected_child(self):
        state = State((Box("a", children=(Box("bb"),)),), selected=(0, 0))
        placements = with_cursor(
            layout(state.boxes, cols=21, rows=11), state.selected
        )
        child = find(placements, "bb")
        cursor = cursors(placements)[0]
        self.assertTrue(child.x <= cursor.x < child.x + child.width)
        self.assertTrue(child.y <= cursor.y < child.y + child.height)

    def test_layout_alone_never_emits_a_cursor(self):
        state = State((Box("hi"),), selected=(0,))
        placements = layout(state.boxes, cols=11, rows=11)
        self.assertEqual(cursors(placements), [])


class WithCursorTest(unittest.TestCase):
    def test_an_empty_placement_list_comes_back_empty(self):
        self.assertEqual(with_cursor([], selected=(0,)), [])

    def test_no_matching_label_comes_back_unchanged(self):
        placements = [
            Placement(Box("hi"), x=0, y=0, width=4, height=3),
            Placement(Label("hi", (0,)), x=1, y=1, width=2, height=1),
        ]
        self.assertEqual(with_cursor(placements, selected=(1,)), placements)

    def test_cursor_is_appended_at_the_right_hand_end_of_the_label(self):
        placements = [
            Placement(Label("hi", (0,)), x=5, y=2, width=2, height=1),
        ]
        result = with_cursor(placements, selected=(0,))
        cursor = cursors(result)[0]
        self.assertEqual(cursor.x, 6)
        self.assertEqual(cursor.y, 2)
        self.assertEqual(cursor.width, 1)
        self.assertEqual(cursor.height, 1)

    def test_a_one_cell_label_puts_the_cursor_on_its_single_cell(self):
        placements = [
            Placement(Label("", (0,)), x=3, y=4, width=1, height=1),
        ]
        result = with_cursor(placements, selected=(0,))
        cursor = cursors(result)[0]
        self.assertEqual(cursor.x, 3)
        self.assertEqual(cursor.y, 4)

    def test_only_the_selected_label_attracts_the_cursor(self):
        placements = [
            Placement(Label("aa", (0,)), x=0, y=0, width=2, height=1),
            Placement(Label("bb", (1,)), x=10, y=10, width=2, height=1),
        ]
        result = with_cursor(placements, selected=(1,))
        cursor = cursors(result)[0]
        self.assertEqual(cursor.x, 11)
        self.assertEqual(cursor.y, 10)

    def test_exactly_one_cursor_is_appended_and_it_is_last(self):
        placements = [
            Placement(Label("aa", (0,)), x=0, y=0, width=2, height=1),
            Placement(Label("bb", (1,)), x=10, y=10, width=2, height=1),
        ]
        result = with_cursor(placements, selected=(0,))
        self.assertEqual(len(cursors(result)), 1)
        self.assertEqual(len(result), len(placements) + 1)
        self.assertIsInstance(result[-1].node, Cursor)


class LabelLayoutTest(unittest.TestCase):
    def test_a_widened_box_recentres_its_label(self):
        state = State((Box("x"), Box("wide")))
        placements = layout(state.boxes, cols=31, rows=21)
        widened_box = [p for p in boxes(placements) if p.node.label == "x"][0]
        widened_label = [p for p in labels(placements) if p.node.text == "x"][0]

        alone = layout(State((Box("x"),)).boxes, cols=31, rows=21)
        solo_box = boxes(alone)[0]
        solo_label = labels(alone)[0]

        self.assertGreater(
            widened_label.x - widened_box.x, solo_label.x - solo_box.x
        )

    def test_the_cursor_sits_on_the_last_cell_of_the_label(self):
        state = State((Box("hi" + PAD),), mode="insert", selected=(0,))
        placements = with_cursor(
            layout(state.boxes, cols=11, rows=11), state.selected
        )
        label = labels(placements)[0]
        cursor = cursors(placements)[0]
        self.assertEqual(cursor.x, label.x + label.width - 1)
        self.assertEqual(cursor.y, label.y)

    def test_the_cursor_sits_in_the_middle_of_a_widened_empty_box(self):
        state = State((Box(""), Box("wide")), selected=(0,))
        placements = with_cursor(
            layout(state.boxes, cols=31, rows=21), state.selected
        )
        empty = [p for p in boxes(placements) if p.node.label == ""][0]
        cursor = cursors(placements)[0]
        self.assertGreater(empty.width, width(Box("")))
        self.assertEqual(cursor.x, empty.x + centre(empty.width, ""))


class ArrowLayoutTest(unittest.TestCase):
    def test_a_childless_box_emits_no_arrow(self):
        placements = layout(State((Box(),)).boxes, cols=11, rows=11)
        self.assertEqual(arrows(placements), [])

    def test_a_single_child_gets_a_straight_arrow_with_no_trunk(self):
        state = State((Box("a", children=(Box("bb"),)),))
        placements = layout(state.boxes, cols=21, rows=11)
        arrow = arrows(placements)[0]
        self.assertEqual(arrow.node.stops, (0,))

    def test_the_arrow_sits_in_the_gap_column_between_parent_and_child(self):
        state = State((Box("a", children=(Box("bb"),)),))
        placements = layout(state.boxes, cols=21, rows=11)
        parent, child = find(placements, "a"), find(placements, "bb")
        arrow = arrows(placements)[0]
        self.assertEqual(arrow.x, parent.x + parent.width)
        self.assertEqual(arrow.width, GAP_WIDTH)
        self.assertEqual(child.x, arrow.x + arrow.width)

    def test_the_straight_arrow_is_centred_on_the_boxes_it_joins(self):
        state = State((Box("a", children=(Box("bb"),)),))
        placements = layout(state.boxes, cols=21, rows=11)
        parent, child = find(placements, "a"), find(placements, "bb")
        arrow = arrows(placements)[0]
        self.assertEqual(arrow.y, parent.y + parent.height // 2)
        self.assertEqual(arrow.y, child.y + child.height // 2)

    def test_two_children_branch_from_a_shared_trunk(self):
        state = State((Box("a", children=(Box("c"), Box("d"))),))
        placements = layout(state.boxes, cols=21, rows=21)
        first, second = find(placements, "c"), find(placements, "d")
        arrow = arrows(placements)[0]
        self.assertEqual(
            arrow.node.stops,
            (
                first.y + first.height // 2 - arrow.y,
                second.y + second.height // 2 - arrow.y,
            ),
        )
        self.assertEqual(arrow.node.stops[0], 0)
        self.assertGreater(arrow.node.stops[1], 0)

    def test_the_arrow_spans_from_the_first_child_row_to_the_last_child_row(self):
        state = State((Box("a", children=(Box("c"), Box("d"), Box("e"))),))
        placements = layout(state.boxes, cols=21, rows=21)
        c, e = find(placements, "c"), find(placements, "e")
        arrow = arrows(placements)[0]
        self.assertEqual(arrow.y, c.y + c.height // 2)
        self.assertEqual(
            arrow.y + arrow.height - 1, e.y + e.height // 2
        )

    def test_a_second_child_sits_below_the_first(self):
        state = State((Box("a", children=(Box("c"), Box("d"))),))
        placements = layout(state.boxes, cols=21, rows=21)
        first, second = find(placements, "c"), find(placements, "d")
        self.assertEqual(second.y - first.y, BOX_HEIGHT + GAP_HEIGHT)

    def test_a_grandchild_sits_two_columns_over(self):
        state = State((Box("a", children=(Box("b", children=(Box("c"),)),)),))
        placements = layout(state.boxes, cols=31, rows=11)
        a, b, c = find(placements, "a"), find(placements, "b"), find(placements, "c")
        self.assertEqual(b.x - a.x, a.width + GAP_WIDTH)
        self.assertEqual(c.x - b.x, b.width + GAP_WIDTH)
        self.assertEqual(len(arrows(placements)), 2)

    def test_a_parent_centres_between_its_first_and_last_child(self):
        state = State((Box("a", children=(Box("c"), Box("d"))),))
        placements = layout(state.boxes, cols=21, rows=21)
        parent = find(placements, "a")
        first, last = find(placements, "c"), find(placements, "d")
        self.assertEqual(2 * parent.y, first.y + last.y)

    def test_a_parent_lines_up_with_the_middle_child_when_odd(self):
        state = State((Box("a", children=(Box("c"), Box("d"), Box("e"))),))
        placements = layout(state.boxes, cols=21, rows=21)
        parent, middle = find(placements, "a"), find(placements, "d")
        self.assertEqual(parent.y, middle.y)

    def test_a_parent_fills_the_gap_between_the_two_middle_children_when_even(self):
        state = State(
            (Box("a", children=(Box("c"), Box("d"), Box("e"), Box("f"))),)
        )
        placements = layout(state.boxes, cols=21, rows=21)
        parent = find(placements, "a")
        middle_low, middle_high = find(placements, "d"), find(placements, "e")
        self.assertEqual(parent.y, middle_low.y + middle_low.height)
        self.assertEqual(parent.y + parent.height, middle_high.y)


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


class WidthHeightTest(unittest.TestCase):
    def test_a_box_is_as_wide_as_its_interior_and_borders(self):
        self.assertEqual(width(Box("hi")), len("hi") + BORDERS)

    def test_an_empty_box_has_a_one_column_interior(self):
        self.assertEqual(width(Box()), BORDERS + 1)

    def test_a_box_is_as_tall_as_a_box(self):
        self.assertEqual(height(Box("hi")), BOX_HEIGHT)


class CentreTest(unittest.TestCase):
    def test_no_leftover_starts_one_cell_in(self):
        label = "hi"
        self.assertEqual(centre(width(Box(label)), label), 1)

    def test_an_odd_leftover_puts_the_extra_cell_on_the_left(self):
        label = "hi"
        box_width = width(Box(label)) + 1
        leftover = box_width - BORDERS - interior(label)
        self.assertEqual(leftover, 1)
        self.assertEqual(centre(box_width, label), 1 + leftover - leftover // 2)
        self.assertEqual(centre(box_width, label), 2)

    def test_an_empty_label_is_measured_as_one_cell(self):
        self.assertEqual(centre(width(Box()), ""), centre(width(Box("x")), "x"))


class AssignTest(unittest.TestCase):
    def test_a_leaf_takes_the_free_row_and_hands_back_the_next(self):
        node, free = assign(Box("a"), 0, (0,), 4)
        self.assertEqual(node.row, 4)
        self.assertEqual(free, 4 + LEAF_STRIDE)

    def test_consecutive_leaves_are_one_stride_apart(self):
        tree, _ = assign(Box("a", children=(Box("b"), Box("c"))), 0, (0,), 0)
        first, second = tree.children
        self.assertEqual(second.row - first.row, LEAF_STRIDE)

    def test_a_child_sits_one_column_right_of_its_parent(self):
        tree, _ = assign(Box("a", children=(Box("b"),)), 0, (0,), 0)
        self.assertEqual(tree.children[0].column, tree.column + 1)

    def test_a_child_extends_the_parents_path_by_its_index(self):
        tree, _ = assign(Box("a", children=(Box("b"), Box("c"))), 0, (0,), 0)
        self.assertEqual([child.path for child in tree.children], [(0, 0), (0, 1)])

    def test_three_leaf_children_straddle_the_parent(self):
        tree, _ = assign(
            Box("a", children=(Box("b"), Box("c"), Box("d"))), 0, (0,), 0
        )
        self.assertEqual(
            [child.row for child in tree.children],
            [0, LEAF_STRIDE, 2 * LEAF_STRIDE],
        )
        self.assertEqual(tree.row, LEAF_STRIDE)

    def test_each_gap_is_sized_from_its_own_two_neighbours(self):
        middle = Box("c", children=(Box(), Box()))
        tree, _ = assign(Box("a", children=(Box("b"), middle, Box("d"))), 0, (0,), 0)
        self.assertEqual([child.row for child in tree.children], [0, 3, 6])
        self.assertEqual(tree.row, 3)

    def test_an_even_parent_sits_a_half_slot_below_the_first_middle_child(self):
        tree, _ = assign(Box("a", children=(Box("b"), Box("c"))), 0, (0,), 0)
        self.assertEqual(tree.row, tree.children[0].row + 1)

    def test_no_two_boxes_in_a_column_are_closer_than_a_stride(self):
        tree, _ = assign(
            Box(
                "a",
                children=(
                    Box("b", children=(Box(), Box())),
                    Box("c"),
                    Box("d", children=(Box("e", children=(Box(), Box(), Box())),)),
                ),
            ),
            0,
            (0,),
            0,
        )
        columns = {}
        for node in walk((tree,)):
            columns.setdefault(node.column, []).append(node.row)
        for rows in columns.values():
            rows.sort()
            for earlier, later in zip(rows, rows[1:]):
                self.assertGreaterEqual(later - earlier, LEAF_STRIDE)


class ForestTest(unittest.TestCase):
    def test_a_tall_middle_child_only_spreads_its_own_neighbours(self):
        box = Box(
            "a", children=(Box("b"), Box("c", children=(Box(), Box())), Box("d"))
        )
        tree, = forest((box,))
        self.assertEqual([child.row for child in tree.children], [0, 3, 6])
        self.assertEqual(tree.row, 3)

    def test_two_leaf_roots_are_one_stride_apart(self):
        first, second = forest((Box("a"), Box("b")))
        self.assertEqual([first.row, second.row], [0, LEAF_STRIDE])

    def test_three_leaf_children_straddle_the_parent(self):
        tree, = forest((Box("a", children=(Box("b"), Box("c"), Box("d"))),))
        self.assertEqual(
            [child.row for child in tree.children],
            [0, LEAF_STRIDE, 2 * LEAF_STRIDE],
        )
        self.assertEqual(tree.row, LEAF_STRIDE)

    def test_a_later_root_starts_below_the_previous_tree(self):
        trees = forest((Box("a", children=(Box(), Box())), Box("b")))
        self.assertEqual(trees[1].row, trees[0].children[-1].row + LEAF_STRIDE)

    def test_every_row_is_non_negative(self):
        trees = forest(
            (
                Box("a", children=(Box("b", children=(Box(), Box())), Box("c"))),
                Box("d", children=(Box(), Box(), Box())),
            )
        )
        self.assertTrue(all(node.row >= 0 for node in walk(trees)))


if __name__ == "__main__":
    unittest.main()
