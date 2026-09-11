import unittest
from dataclasses import replace

from sketch.layout import (
    BORDERS,
    BOX_HEIGHT,
    GAP_HEIGHT,
    GAP_WIDTH,
    Arrow,
    Label,
    Placement,
    Track,
    cells,
    centre,
    fold_up,
    height,
    interior,
    layout,
    measure,
    push_down,
    span,
    tracks,
    width,
)
from sketch.state import PAD, Box, Cursor, State


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
        self.assertEqual(layout(State(()), cols=11, rows=11), [])

    def test_box_is_centered(self):
        placements = layout(State((Box(),)), cols=11, rows=11)
        self.assertEqual(
            boxes(placements),
            [Placement(Box(), x=4, y=4, width=BORDERS + 1, height=BOX_HEIGHT)],
        )

    def test_box_is_sized_by_the_layout(self):
        placement = layout(State((Box(),)), cols=11, rows=11)[0]
        self.assertEqual((placement.width, placement.height), (BORDERS + 1, BOX_HEIGHT))

    def test_box_stays_centered_in_a_larger_terminal(self):
        placement = layout(State((Box(),)), cols=80, rows=25)[0]
        self.assertLessEqual(abs(80 - 2 * placement.x - placement.width), 1)
        self.assertLessEqual(abs(25 - 2 * placement.y - placement.height), 1)

    def test_box_widens_to_fit_the_label(self):
        state = State((Box("hi" + PAD),), mode="insert", selected=(0,))
        placements = layout(state, cols=11, rows=11)
        self.assertEqual(
            boxes(placements)[0],
            Placement(Box("hi" + PAD), x=3, y=4, width=5, height=3),
        )

    def test_box_stays_centered_as_it_grows(self):
        def x(label):
            state = State((Box(label + PAD),), mode="insert", selected=(0,))
            return boxes(layout(state, 21, 11))[0].x

        self.assertEqual([x(""), x("ab"), x("abcd")], [9, 8, 7])

    def test_two_top_level_boxes_stack_as_siblings(self):
        first, second = boxes(layout(State((Box(), Box())), cols=11, rows=11))
        self.assertEqual(second.y - first.y, BOX_HEIGHT + GAP_HEIGHT)
        self.assertEqual(first.x, second.x)

    def test_the_stack_of_top_level_boxes_is_vertically_centered(self):
        placements = boxes(layout(State((Box(), Box(), Box())), cols=11, rows=11))
        top = placements[0].y
        bottom = placements[-1].y + placements[-1].height
        self.assertLessEqual(abs((top + bottom) / 2 - 11 / 2), 0.5)


class ColumnWidthTest(unittest.TestCase):
    def test_top_level_boxes_share_the_width_of_the_widest(self):
        placements = layout(State((Box("x"), Box("wide"))), cols=31, rows=21)
        short, wide = find(placements, "x"), find(placements, "wide")
        self.assertEqual(short.width, width(Box("wide")))
        self.assertEqual(short.width, wide.width)

    def test_boxes_in_a_column_are_left_aligned_with_each_other(self):
        placements = layout(State((Box("x"), Box("wide"))), cols=31, rows=21)
        self.assertEqual(find(placements, "x").x, find(placements, "wide").x)

    def test_siblings_share_the_width_of_the_widest_sibling(self):
        state = State((Box("a", children=(Box("c"), Box("dddd"))),))
        placements = layout(state, cols=31, rows=21)
        short, wide = find(placements, "c"), find(placements, "dddd")
        self.assertEqual(short.width, width(Box("dddd")))
        self.assertEqual(short.x, wide.x)

    def test_a_narrower_column_does_not_widen_to_match_another(self):
        state = State((Box("a", children=(Box("dddd"),)),))
        placements = layout(state, cols=31, rows=21)
        self.assertEqual(find(placements, "a").width, width(Box("a")))


class CursorTest(unittest.TestCase):
    def test_an_empty_canvas_emits_no_cursor(self):
        self.assertEqual(cursors(layout(State(()), cols=11, rows=11)), [])

    def test_nothing_selected_emits_no_cursor(self):
        self.assertEqual(
            cursors(layout(State((Box("hi"),)), cols=11, rows=11)), []
        )

    def test_command_mode_cursor_sits_on_the_last_character(self):
        state = State((Box("hi"),), selected=(0,))
        box = boxes(layout(state, 11, 11))[0]
        cursor = cursors(layout(state, 11, 11))[0]
        interior = box.x + BORDERS // 2
        self.assertEqual(cursor.x, interior + len("hi") - 1)
        self.assertEqual(cursor.y, box.y + BOX_HEIGHT // 2)

    def test_insert_mode_cursor_sits_one_past_the_last_character(self):
        state = State((Box("hi" + PAD),), mode="insert", selected=(0,))
        box = boxes(layout(state, 11, 11))[0]
        cursor = cursors(layout(state, 11, 11))[0]
        interior = box.x + BORDERS // 2
        self.assertEqual(cursor.x, interior + len("hi"))
        self.assertEqual(cursor.y, box.y + BOX_HEIGHT // 2)

    def test_cursor_follows_a_selected_child(self):
        state = State((Box("a", children=(Box("bb"),)),), selected=(0, 0))
        placements = layout(state, cols=21, rows=11)
        child = find(placements, "bb")
        cursor = cursors(placements)[0]
        self.assertTrue(child.x <= cursor.x < child.x + child.width)
        self.assertTrue(child.y <= cursor.y < child.y + child.height)


class LabelLayoutTest(unittest.TestCase):
    def test_a_widened_box_recentres_its_label(self):
        state = State((Box("x"), Box("wide")))
        placements = layout(state, cols=31, rows=21)
        widened_box = [p for p in boxes(placements) if p.node.label == "x"][0]
        widened_label = [p for p in labels(placements) if p.node.text == "x"][0]

        alone = layout(State((Box("x"),)), cols=31, rows=21)
        solo_box = boxes(alone)[0]
        solo_label = labels(alone)[0]

        self.assertGreater(
            widened_label.x - widened_box.x, solo_label.x - solo_box.x
        )

    def test_the_cursor_sits_on_the_last_cell_of_the_label(self):
        state = State((Box("hi" + PAD),), mode="insert", selected=(0,))
        placements = layout(state, cols=11, rows=11)
        label = labels(placements)[0]
        cursor = cursors(placements)[0]
        self.assertEqual(cursor.x, label.x + label.width - 1)
        self.assertEqual(cursor.y, label.y)

    def test_the_cursor_sits_in_the_middle_of_a_widened_empty_box(self):
        state = State((Box(""), Box("wide")), selected=(0,))
        placements = layout(state, cols=31, rows=21)
        empty = [p for p in boxes(placements) if p.node.label == ""][0]
        cursor = cursors(placements)[0]
        self.assertGreater(empty.width, width(Box("")))
        self.assertEqual(cursor.x, empty.x + centre(empty.width, ""))


class ArrowLayoutTest(unittest.TestCase):
    def test_a_childless_box_emits_no_arrow(self):
        placements = layout(State((Box(),)), cols=11, rows=11)
        self.assertEqual(arrows(placements), [])

    def test_a_single_child_gets_a_straight_arrow_with_no_trunk(self):
        state = State((Box("a", children=(Box("bb"),)),))
        placements = layout(state, cols=21, rows=11)
        arrow = arrows(placements)[0]
        self.assertEqual(arrow.node.stops, (0,))

    def test_the_arrow_sits_in_the_gap_column_between_parent_and_child(self):
        state = State((Box("a", children=(Box("bb"),)),))
        placements = layout(state, cols=21, rows=11)
        parent, child = find(placements, "a"), find(placements, "bb")
        arrow = arrows(placements)[0]
        self.assertEqual(arrow.x, parent.x + parent.width)
        self.assertEqual(arrow.width, GAP_WIDTH)
        self.assertEqual(child.x, arrow.x + arrow.width)

    def test_the_straight_arrow_is_centred_on_the_boxes_it_joins(self):
        state = State((Box("a", children=(Box("bb"),)),))
        placements = layout(state, cols=21, rows=11)
        parent, child = find(placements, "a"), find(placements, "bb")
        arrow = arrows(placements)[0]
        self.assertEqual(arrow.y, parent.y + parent.height // 2)
        self.assertEqual(arrow.y, child.y + child.height // 2)

    def test_two_children_branch_from_a_shared_trunk(self):
        state = State((Box("a", children=(Box("c"), Box("d"))),))
        placements = layout(state, cols=21, rows=21)
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

    def test_the_arrow_spans_from_the_parent_row_to_the_last_child_row(self):
        state = State((Box("a", children=(Box("c"), Box("d"), Box("e"))),))
        placements = layout(state, cols=21, rows=21)
        parent, e = find(placements, "a"), find(placements, "e")
        arrow = arrows(placements)[0]
        self.assertEqual(arrow.y, parent.y + parent.height // 2)
        self.assertEqual(
            arrow.y + arrow.height - 1, e.y + e.height // 2
        )

    def test_a_second_child_sits_below_the_first(self):
        state = State((Box("a", children=(Box("c"), Box("d"))),))
        placements = layout(state, cols=21, rows=21)
        first, second = find(placements, "c"), find(placements, "d")
        self.assertEqual(second.y - first.y, BOX_HEIGHT + GAP_HEIGHT)

    def test_a_grandchild_sits_two_columns_over(self):
        state = State((Box("a", children=(Box("b", children=(Box("c"),)),)),))
        placements = layout(state, cols=31, rows=11)
        a, b, c = find(placements, "a"), find(placements, "b"), find(placements, "c")
        self.assertEqual(b.x - a.x, a.width + GAP_WIDTH)
        self.assertEqual(c.x - b.x, b.width + GAP_WIDTH)
        self.assertEqual(len(arrows(placements)), 2)

    def test_a_parent_is_top_aligned_with_its_first_child(self):
        state = State((Box("a", children=(Box("c"), Box("d"))),))
        placements = layout(state, cols=21, rows=21)
        parent, first = find(placements, "a"), find(placements, "c")
        self.assertEqual(parent.y, first.y)


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


def celled(box, index=0):
    return push_down(cells, fold_up(measure, box), (0, 0, (index,)))


class CombinatorTest(unittest.TestCase):
    def test_fold_up_gives_a_parent_its_transformed_children(self):
        tree = Box("a", children=(Box("b"), Box("c")))
        folded = fold_up(
            lambda box, kids: replace(
                box, label=box.label + "".join(k.label for k in kids)
            ),
            tree,
        )
        self.assertEqual(folded.label, "abc")

    def test_fold_up_preserves_the_shape_of_the_tree(self):
        tree = Box("a", children=(Box("b", children=(Box("c"),)),))
        self.assertEqual(
            fold_up(lambda box, _: replace(box, label=box.label * 2), tree),
            Box("aa", children=(Box("bb", children=(Box("cc"),)),)),
        )

    def test_push_down_hands_each_child_its_own_context(self):
        tree = Box("a", children=(Box("b"), Box("c")))

        def step(box, context):
            return replace(box, label=f"{box.label}{context}"), [context + 1] * len(
                box.children
            )

        pushed = push_down(step, tree, 0)
        self.assertEqual(pushed.label, "a0")
        self.assertEqual([c.label for c in pushed.children], ["b1", "c1"])

    def test_push_down_preserves_the_shape_of_the_tree(self):
        tree = Box("a", children=(Box("b", children=(Box("c"),)),))
        pushed = push_down(
            lambda box, ctx: (replace(box, label=str(ctx)), [ctx + 1]), tree, 0
        )
        self.assertEqual(pushed, Box("0", children=(Box("1", children=(Box("2"),)),)))


class MeasureTest(unittest.TestCase):
    def test_a_leaf_spans_a_single_row(self):
        self.assertEqual(fold_up(measure, Box()).span, 1)

    def test_a_parent_spans_the_sum_of_its_children(self):
        tree = fold_up(measure, Box("a", children=(Box(), Box(), Box())))
        self.assertEqual(tree.span, 3)

    def test_spans_accumulate_through_generations(self):
        inner = Box("b", children=(Box(), Box()))
        tree = fold_up(measure, Box("a", children=(inner, Box())))
        self.assertEqual(tree.span, 3)

    def test_a_box_is_measured_at_its_own_width(self):
        self.assertEqual(fold_up(measure, Box("hi")).width, width(Box("hi")))


class CellsTest(unittest.TestCase):
    def test_the_root_takes_the_context_it_is_given(self):
        root = celled(Box("a"))
        self.assertEqual((root.column, root.row, root.path), (0, 0, (0,)))

    def test_a_child_sits_one_column_right_of_its_parent(self):
        tree = celled(Box("a", children=(Box("b"),)))
        self.assertEqual(tree.children[0].column, 1)

    def test_a_parent_shares_the_row_of_its_first_child(self):
        tree = celled(Box("a", children=(Box("b"), Box("c"))))
        self.assertEqual(tree.children[0].row, tree.row)

    def test_a_sibling_starts_below_the_previous_subtree(self):
        parent = Box("a", children=(Box("b", children=(Box(), Box())), Box("c")))
        first, second = celled(parent).children
        self.assertEqual(
            second.row,
            first.row + fold_up(measure, parent).children[0].span,
        )

    def test_a_child_extends_the_parents_path_by_its_index(self):
        tree = celled(Box("a", children=(Box("b"), Box("c"))))
        self.assertEqual([c.path for c in tree.children], [(0, 0), (0, 1)])


if __name__ == "__main__":
    unittest.main()
