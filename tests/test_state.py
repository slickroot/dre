import unittest

from sketch.state import (
    PAD,
    PALETTE_SIZE,
    PLAIN,
    Arrow,
    Box,
    Pop,
    Push,
    Space,
    State,
    below,
    beside,
    edit,
    handle_key,
    next_colour,
    outgoing,
)


class BoxTest(unittest.TestCase):
    def test_boxes_are_equal(self):
        self.assertEqual(Box(), Box())


class BoxColourTest(unittest.TestCase):
    def test_boxes_default_to_the_plain_colour(self):
        self.assertEqual(Box().colour, PLAIN)


class BoxFillTest(unittest.TestCase):
    def test_boxes_default_to_the_plain_fill(self):
        self.assertEqual(Box().fill, PLAIN)


class BoxLabelTest(unittest.TestCase):
    def test_boxes_default_to_an_empty_label(self):
        self.assertEqual(Box(), Box(""))

    def test_boxes_with_different_labels_are_not_equal(self):
        self.assertNotEqual(Box("a"), Box("b"))


class PushTest(unittest.TestCase):
    def test_pushes_are_equal(self):
        self.assertEqual(Push(), Push())


class PopTest(unittest.TestCase):
    def test_pops_are_equal(self):
        self.assertEqual(Pop(), Pop())


class StateTest(unittest.TestCase):
    def test_state_starts_running(self):
        self.assertIs(State([]).running, True)

    def test_state_starts_in_command_mode(self):
        self.assertEqual(State([]).mode, "command")

    def test_an_empty_state_has_no_selection(self):
        self.assertEqual(State([]).selected, -1)

    def test_an_empty_state_has_no_source(self):
        self.assertEqual(State([]).source, -1)


class EditTest(unittest.TestCase):
    def test_edit_replaces_the_box_at_the_given_index(self):
        nodes = [Box("a"), Space(), Box("b"), Space(), Box("c")]
        self.assertEqual(
            edit(nodes, 2, "z"), [Box("a"), Space(), Box("z"), Space(), Box("c")]
        )

    def test_edit_does_not_mutate_the_given_nodes(self):
        nodes = [Box("a"), Space(), Box("b")]
        edit(nodes, 0, "z")
        self.assertEqual(nodes, [Box("a"), Space(), Box("b")])


def bj(state, key1="b", key2="j"):
    return handle_key(handle_key(state, key1), key2)


def bl(state, key1="b", key2="l"):
    return handle_key(handle_key(state, key1), key2)


class HandleKeyTest(unittest.TestCase):
    def test_bj_appends_a_box(self):
        self.assertEqual(bj(State([])).nodes, [Box(PAD)])

    def test_bj_appends_a_box_with_an_empty_label(self):
        self.assertEqual(bj(State([])).nodes, [Box(PAD)])

    def test_bj_enters_insert_mode(self):
        self.assertEqual(bj(State([])).mode, "insert")

    def test_bj_does_not_mutate_the_mode_of_the_given_state(self):
        state = State([])
        bj(state)
        self.assertEqual(state.mode, "command")

    def test_q_keeps_command_mode(self):
        self.assertEqual(handle_key(State([]), "q").mode, "command")

    def test_insert_mode_is_dispatched_separately(self):
        state = State([Box(PAD)], mode="insert", selected=0)
        self.assertIs(handle_key(state, "q").running, True)

    def test_bj_appends_to_existing_nodes(self):
        self.assertEqual(
            bj(State([Box()])).nodes, [Box(), Space(), Box(PAD)]
        )

    def test_bj_on_an_empty_canvas_appends_only_a_box(self):
        self.assertEqual(bj(State([])).nodes, [Box(PAD)])

    def test_bj_separates_the_new_box_from_the_last_one_with_a_space(self):
        state = State([Box("a")], selected=0)
        appended = bj(state)
        self.assertEqual(appended.nodes, [Box("a"), Space(), Box(PAD)])
        self.assertEqual(appended.selected, len(appended.nodes) - 1)

    def test_bj_does_not_mutate_the_given_state(self):
        state = State([])
        bj(state)
        self.assertEqual(state.nodes, [])

    def test_bj_keeps_the_state_running(self):
        self.assertIs(bj(State([])).running, True)

    def test_bj_preserves_a_stopped_state(self):
        state = handle_key(State([]), "q")
        self.assertIs(bj(state).running, False)

    def test_unknown_key_returns_the_state_unchanged(self):
        state = State([Box()])
        self.assertEqual(handle_key(state, "x"), state)

    def test_q_stops_the_state(self):
        self.assertIs(handle_key(State([Box()]), "q").running, False)

    def test_q_preserves_the_nodes(self):
        self.assertEqual(handle_key(State([Box()]), "q").nodes, [Box()])

    def test_q_does_not_mutate_the_given_state(self):
        state = State([Box()])
        handle_key(state, "q")
        self.assertIs(state.running, True)

    def test_bj_selects_the_new_box_on_an_empty_canvas(self):
        self.assertEqual(bj(State([])).selected, 0)

    def test_bj_selects_the_new_last_box(self):
        state = State([Box("a")], selected=0)
        self.assertEqual(bj(state).selected, 2)

    def test_bj_selects_the_new_box_when_the_selection_was_not_last(self):
        state = State([Box("a"), Space(), Box("b")], selected=0)
        self.assertEqual(bj(state).selected, 2)

    def test_q_preserves_the_selection(self):
        state = State([Box("a"), Space(), Box("b")], selected=0)
        self.assertEqual(handle_key(state, "q").selected, 0)

    def test_bj_clears_source(self):
        state = State([Box("a")], selected=0, source=0)
        self.assertEqual(bj(state).source, -1)

    def test_movement_still_works_after_branching_a_selected_box(self):
        state = State([])
        state = handle_key(handle_key(state, "b"), "j")  # A
        state = handle_key(state, "\x1b")
        state = handle_key(handle_key(state, "b"), "l")  # B, right of A
        state = handle_key(state, "\x1b")
        state = handle_key(state, "h")  # back to A
        self.assertEqual(state.selected, 0)
        state = handle_key(handle_key(state, "b"), "j")  # bracket New below A
        state = handle_key(state, "\x1b")
        state = handle_key(state, "k")  # back to A
        self.assertEqual(state.selected, 0)
        self.assertEqual(handle_key(state, "j").selected, 3)
        self.assertEqual(handle_key(state, "l").selected, 6)


class PendingCommandTest(unittest.TestCase):
    def test_b_sets_pending_to_b(self):
        self.assertEqual(handle_key(State([]), "b").pending, "b")

    def test_b_does_not_add_a_node(self):
        state = State([Box("a")], selected=0)
        self.assertEqual(handle_key(state, "b").nodes, [Box("a")])

    def test_b_does_not_change_mode(self):
        state = State([Box("a")], selected=0)
        self.assertEqual(handle_key(state, "b").mode, "command")

    def test_b_does_not_change_selected(self):
        state = State([Box("a")], selected=0)
        self.assertEqual(handle_key(state, "b").selected, 0)

    def test_b_does_not_change_source(self):
        state = State([Box("a")], selected=0, source=0)
        self.assertEqual(handle_key(state, "b").source, 0)

    def test_bl_appends_a_space_directed_right_and_a_box(self):
        state = State([Box("a")], selected=0)
        result = handle_key(handle_key(state, "b"), "l")
        self.assertEqual(result.nodes, [Box("a"), Space(direction="right"), Box(PAD)])

    def test_bl_selects_the_new_box(self):
        state = State([Box("a")], selected=0)
        result = handle_key(handle_key(state, "b"), "l")
        self.assertEqual(result.selected, len(result.nodes) - 1)

    def test_bl_enters_insert_mode(self):
        state = State([Box("a")], selected=0)
        result = handle_key(handle_key(state, "b"), "l")
        self.assertEqual(result.mode, "insert")

    def test_bl_clears_pending(self):
        state = State([Box("a")], selected=0)
        result = handle_key(handle_key(state, "b"), "l")
        self.assertEqual(result.pending, "")

    def test_bj_appends_a_space_directed_down_and_a_box(self):
        state = State([Box("a")], selected=0)
        result = handle_key(handle_key(state, "b"), "j")
        self.assertEqual(result.nodes, [Box("a"), Space(direction="down"), Box(PAD)])

    def test_bj_clears_pending(self):
        state = State([Box("a")], selected=0)
        result = handle_key(handle_key(state, "b"), "j")
        self.assertEqual(result.pending, "")

    def test_bj_appends_when_selected_is_last_via_negative_index(self):
        state = State([Box("a")], selected=-1)
        result = handle_key(handle_key(state, "b"), "j")
        self.assertEqual(result.nodes, [Box("a"), Space(direction="down"), Box(PAD)])
        self.assertEqual(result.selected, 2)

    def test_b_then_unrecognized_key_clears_pending(self):
        state = State([Box("a")], selected=0)
        result = handle_key(handle_key(state, "b"), "i")
        self.assertEqual(result.pending, "")

    def test_b_then_unrecognized_key_adds_no_node(self):
        state = State([Box("a")], selected=0)
        result = handle_key(handle_key(state, "b"), "i")
        self.assertEqual(result.nodes, [Box("a")])

    def test_b_then_i_does_not_enter_insert_mode(self):
        state = State([], mode="command", selected=-1)
        result = handle_key(handle_key(state, "b"), "i")
        self.assertEqual(result.mode, "command")

    def test_b_then_unrecognized_key_leaves_selected_and_source_unchanged(self):
        state = State([Box("a")], selected=0, source=0)
        result = handle_key(handle_key(state, "b"), "i")
        self.assertEqual((result.selected, result.source), (0, 0))


class InsertAtSelectionTest(unittest.TestCase):
    def test_bj_splices_below_a_selected_box_that_is_not_last(self):
        state = State([Box("a"), Space("right"), Box("b")], selected=0)
        result = bj(state)
        self.assertEqual(
            result.nodes,
            [
                Box("a"),
                Push(),
                Space("down"),
                Box(PAD),
                Pop(),
                Space("right"),
                Box("b"),
            ],
        )

    def test_bj_selects_the_newly_spliced_box(self):
        state = State([Box("a"), Space("right"), Box("b")], selected=0)
        result = bj(state)
        self.assertEqual(result.selected, 3)

    def test_bj_enters_insert_mode_when_splicing(self):
        state = State([Box("a"), Space("right"), Box("b")], selected=0)
        self.assertEqual(bj(state).mode, "insert")

    def test_bj_clears_pending_when_splicing(self):
        state = State([Box("a"), Space("right"), Box("b")], selected=0)
        self.assertEqual(bj(state).pending, "")

    def test_bj_leaves_the_tail_branch_after_the_selected_box_in_place(self):
        state = State([Box("a"), Space("right"), Box("b")], selected=0)
        result = bj(state)
        self.assertEqual(result.nodes[5:], [Space("right"), Box("b")])

    def test_bl_splices_right_of_a_selected_box_that_is_not_last(self):
        state = State([Box("a"), Space("down"), Box("b")], selected=0)
        result = bl(state)
        self.assertEqual(
            result.nodes,
            [
                Box("a"),
                Push(),
                Space("right"),
                Box(PAD),
                Pop(),
                Space("down"),
                Box("b"),
            ],
        )

    def test_bj_extends_an_existing_down_branch_at_its_separator(self):
        nodes = [
            Box("a"),
            Push(),
            Space("down"),
            Box("c"),
            Pop(),
            Space("right"),
            Box("b"),
        ]
        state = State(nodes, selected=0)
        result = bj(state)
        self.assertEqual(
            result.nodes,
            [
                Box("a"),
                Push(),
                Space("down"),
                Box(PAD),
                Space("down"),
                Box("c"),
                Pop(),
                Space("right"),
                Box("b"),
            ],
        )

    def test_bj_extending_a_branch_selects_the_new_box(self):
        nodes = [
            Box("a"),
            Push(),
            Space("down"),
            Box("c"),
            Pop(),
            Space("right"),
            Box("b"),
        ]
        state = State(nodes, selected=0)
        self.assertEqual(bj(state).selected, 3)

    def test_bj_extending_a_branch_leaves_the_existing_chain_intact(self):
        nodes = [
            Box("a"),
            Push(),
            Space("down"),
            Box("c"),
            Pop(),
            Space("right"),
            Box("b"),
        ]
        state = State(nodes, selected=0)
        result = bj(state)
        self.assertEqual(result.nodes[5:], [Box("c"), Pop(), Space("right"), Box("b")])

    def test_bl_extends_an_existing_right_branch_at_its_separator(self):
        state = State([Box("a"), Space("right"), Box("b")], selected=0)
        result = bl(state)
        self.assertEqual(
            result.nodes,
            [Box("a"), Space("right"), Box(PAD), Space("right"), Box("b")],
        )

    def test_bj_is_a_no_op_when_the_down_branch_is_an_arrow(self):
        state = State([Box("a"), Arrow("down"), Box("b")], selected=0)
        result = bj(state)
        self.assertEqual(result.nodes, [Box("a"), Arrow("down"), Box("b")])
        self.assertEqual(result.selected, 0)
        self.assertEqual(result.mode, "command")

    def test_bj_arrow_guard_still_clears_pending(self):
        state = State([Box("a"), Arrow("down"), Box("b")], selected=0)
        self.assertEqual(bj(state).pending, "")

    def test_bl_is_a_no_op_when_the_right_branch_is_an_arrow(self):
        state = State([Box("a"), Arrow("right"), Box("b")], selected=0)
        result = bl(state)
        self.assertEqual(result.nodes, [Box("a"), Arrow("right"), Box("b")])
        self.assertEqual(result.selected, 0)
        self.assertEqual(result.mode, "command")

    def test_bj_still_appends_when_selected_box_is_last(self):
        state = State([Box("a"), Space("right"), Box("b")], selected=2)
        result = bj(state)
        self.assertEqual(
            result.nodes,
            [Box("a"), Space("right"), Box("b"), Space("down"), Box(PAD)],
        )
        self.assertEqual(result.selected, 4)


class MoveSelectionTest(unittest.TestCase):
    def test_k_skips_a_space_and_lands_on_the_previous_box(self):
        state = State([Box("a"), Space(), Box("b"), Space(), Box("c")], selected=4)
        self.assertEqual(handle_key(state, "k").selected, 2)

    def test_k_on_the_top_box_keeps_the_selection(self):
        state = State([Box("a"), Space(), Box("b")], selected=0)
        self.assertEqual(handle_key(state, "k").selected, 0)

    def test_k_on_an_empty_canvas_returns_the_state_unchanged(self):
        state = State([])
        self.assertEqual(handle_key(state, "k"), state)

    def test_k_preserves_the_mode_the_running_flag_and_the_nodes(self):
        state = State([Box("a"), Space(), Box("b")], selected=2)
        moved = handle_key(state, "k")
        self.assertEqual(
            (moved.mode, moved.running, moved.nodes),
            (state.mode, state.running, state.nodes),
        )

    def test_k_does_not_mutate_the_given_state(self):
        state = State([Box("a"), Space(), Box("b")], selected=2)
        handle_key(state, "k")
        self.assertEqual(state.selected, 2)

    def test_k_preserves_source(self):
        state = State([Box("a"), Space(), Box("b")], selected=2, source=2)
        self.assertEqual(handle_key(state, "k").source, 2)


class BesideTest(unittest.TestCase):
    def test_crosses_a_space_right_to_the_box_beside_it(self):
        nodes = [Box("a"), Space(direction="right"), Box("b")]
        self.assertEqual(beside(nodes, 0, 1), 2)

    def test_crosses_an_arrow_right_to_the_box_beside_it(self):
        nodes = [Box("a"), Arrow(direction="right"), Box("b")]
        self.assertEqual(beside(nodes, 0, 1), 2)

    def test_crosses_an_arrow_left_to_the_box_beside_it(self):
        nodes = [Box("a"), Arrow(direction="left"), Box("b")]
        self.assertEqual(beside(nodes, 0, 1), 2)

    def test_refuses_across_a_space_down(self):
        nodes = [Box("a"), Space(direction="down"), Box("b")]
        self.assertEqual(beside(nodes, 0, 1), 0)

    def test_refuses_on_the_rightmost_box(self):
        nodes = [Box("a"), Space(direction="right"), Box("b")]
        self.assertEqual(beside(nodes, 2, 1), 2)

    def test_refuses_when_the_slot_holds_a_box(self):
        nodes = [Box("a"), Box("b"), Box("c")]
        self.assertEqual(beside(nodes, 0, 1), 0)

    def test_crosses_a_space_left_to_the_box_beside_it(self):
        nodes = [Box("a"), Space(direction="right"), Box("b")]
        self.assertEqual(beside(nodes, 2, -1), 0)

    def test_crosses_an_arrow_left_to_the_box_beside_it_going_left(self):
        nodes = [Box("a"), Arrow(direction="right"), Box("b")]
        self.assertEqual(beside(nodes, 2, -1), 0)

    def test_refuses_on_the_leftmost_box(self):
        nodes = [Box("a"), Space(direction="right"), Box("b")]
        self.assertEqual(beside(nodes, 0, -1), 0)

    def test_crosses_a_bracketed_branch_to_reach_the_tail_box(self):
        # [A, Push, Space(down), New, Pop, Space(right), B]: A's tail branch
        # (right, to B) survives even though A also branched down to New.
        nodes = [
            Box("a"),
            Push(),
            Space(direction="down"),
            Box("new"),
            Pop(),
            Space(direction="right"),
            Box("b"),
        ]
        self.assertEqual(beside(nodes, 0, 1), 6)

    def test_crosses_back_over_a_bracketed_branch_going_left(self):
        nodes = [
            Box("a"),
            Push(),
            Space(direction="down"),
            Box("new"),
            Pop(),
            Space(direction="right"),
            Box("b"),
        ]
        self.assertEqual(beside(nodes, 6, -1), 0)

    def test_refuses_left_across_a_bracketed_down_branch(self):
        # New sits below A, not beside anything: no left neighbour for it.
        nodes = [
            Box("a"),
            Push(),
            Space(direction="down"),
            Box("new"),
            Pop(),
            Space(direction="right"),
            Box("b"),
        ]
        self.assertEqual(beside(nodes, 3, -1), 3)


class BelowTest(unittest.TestCase):
    def test_crosses_a_space_down_to_the_box_below_it(self):
        nodes = [Box("a"), Space(direction="down"), Box("b")]
        self.assertEqual(below(nodes, 0), 2)

    def test_crosses_an_arrow_down_to_the_box_below_it(self):
        nodes = [Box("a"), Arrow(direction="down"), Box("b")]
        self.assertEqual(below(nodes, 0), 2)

    def test_crosses_an_arrow_up_to_the_box_below_it(self):
        nodes = [Box("a"), Arrow(direction="up"), Box("b")]
        self.assertEqual(below(nodes, 0), 2)

    def test_refuses_across_a_space_right(self):
        nodes = [Box("a"), Space(direction="right"), Box("b")]
        self.assertEqual(below(nodes, 0), 0)

    def test_refuses_on_the_bottom_box(self):
        nodes = [Box("a"), Space(direction="down"), Box("b")]
        self.assertEqual(below(nodes, 2), 2)

    def test_crosses_a_bracketed_branch_to_reach_its_box(self):
        # A's down branch is bracketed away from its tail (right) branch.
        nodes = [
            Box("a"),
            Push(),
            Space(direction="down"),
            Box("new"),
            Pop(),
            Space(direction="right"),
            Box("b"),
        ]
        self.assertEqual(below(nodes, 0), 3)

    def test_refuses_below_the_tail_box_of_a_bracketed_selection(self):
        nodes = [
            Box("a"),
            Push(),
            Space(direction="down"),
            Box("new"),
            Pop(),
            Space(direction="right"),
            Box("b"),
        ]
        self.assertEqual(below(nodes, 6), 6)

    def test_refuses_when_the_slot_holds_a_box(self):
        nodes = [Box("a"), Box("b"), Box("c")]
        self.assertEqual(below(nodes, 0), 0)


class MoveSelectionRightTest(unittest.TestCase):
    def test_l_moves_the_selection_to_the_box_on_the_right(self):
        state = State([Box("a"), Space(direction="right"), Box("b")], selected=0)
        self.assertEqual(handle_key(state, "l").selected, 2)

    def test_l_with_nothing_on_the_right_keeps_the_selection(self):
        state = State([Box("a"), Space(direction="right"), Box("b")], selected=2)
        self.assertEqual(handle_key(state, "l").selected, 2)

    def test_l_on_an_empty_canvas_returns_the_state_unchanged(self):
        state = State([])
        self.assertEqual(handle_key(state, "l"), state)

    def test_l_preserves_the_mode_the_running_flag_and_the_nodes(self):
        state = State([Box("a"), Space(direction="right"), Box("b")], selected=0)
        moved = handle_key(state, "l")
        self.assertEqual(
            (moved.mode, moved.running, moved.nodes),
            (state.mode, state.running, state.nodes),
        )

    def test_l_preserves_source(self):
        state = State(
            [Box("a"), Space(direction="right"), Box("b")], selected=0, source=0
        )
        self.assertEqual(handle_key(state, "l").source, 0)

    def test_l_does_not_mutate_the_given_state(self):
        state = State([Box("a"), Space(direction="right"), Box("b")], selected=0)
        handle_key(state, "l")
        self.assertEqual(state.selected, 0)

    def test_l_leaves_colours_unchanged(self):
        nodes = [
            Box("a", colour=0),
            Space(direction="right"),
            Box("b", colour=1),
        ]
        state = State(nodes, selected=0)
        self.assertEqual(handle_key(state, "l").nodes, nodes)

    def test_l_leaves_fills_unchanged(self):
        nodes = [Box("a", fill=0), Space(direction="right"), Box("b", fill=1)]
        state = State(nodes, selected=0)
        self.assertEqual(handle_key(state, "l").nodes, nodes)

    def test_l_in_insert_mode_types_the_letter_l(self):
        state = State([Box("a" + PAD)], mode="insert", selected=0)
        self.assertEqual(handle_key(state, "l").nodes, [Box("al" + PAD)])


class MoveSelectionLeftTest(unittest.TestCase):
    def test_h_moves_the_selection_to_the_box_on_the_left(self):
        state = State([Box("a"), Space(direction="right"), Box("b")], selected=2)
        self.assertEqual(handle_key(state, "h").selected, 0)

    def test_h_with_nothing_on_the_left_keeps_the_selection(self):
        state = State([Box("a"), Space(direction="right"), Box("b")], selected=0)
        self.assertEqual(handle_key(state, "h").selected, 0)

    def test_h_on_an_empty_canvas_returns_the_state_unchanged(self):
        state = State([])
        self.assertEqual(handle_key(state, "h"), state)

    def test_h_in_insert_mode_types_the_letter_h(self):
        state = State([Box("a" + PAD)], mode="insert", selected=0)
        self.assertEqual(handle_key(state, "h").nodes, [Box("ah" + PAD)])


class MoveSelectionDownTest(unittest.TestCase):
    def test_j_moves_the_selection_to_the_box_below(self):
        state = State([Box("a"), Space(direction="down"), Box("b")], selected=0)
        self.assertEqual(handle_key(state, "j").selected, 2)

    def test_j_with_nothing_below_it_keeps_the_selection(self):
        state = State([Box("a"), Space(direction="down"), Box("b")], selected=2)
        self.assertEqual(handle_key(state, "j").selected, 2)

    def test_j_on_an_empty_canvas_returns_the_state_unchanged(self):
        state = State([])
        self.assertEqual(handle_key(state, "j"), state)

    def test_j_preserves_the_mode_the_running_flag_and_the_nodes(self):
        state = State([Box("a"), Space(direction="down"), Box("b")], selected=0)
        moved = handle_key(state, "j")
        self.assertEqual(
            (moved.mode, moved.running, moved.nodes),
            (state.mode, state.running, state.nodes),
        )

    def test_j_preserves_source(self):
        state = State(
            [Box("a"), Space(direction="down"), Box("b")], selected=0, source=0
        )
        self.assertEqual(handle_key(state, "j").source, 0)

    def test_j_does_not_mutate_the_given_state(self):
        state = State([Box("a"), Space(direction="down"), Box("b")], selected=0)
        handle_key(state, "j")
        self.assertEqual(state.selected, 0)

    def test_j_leaves_colours_unchanged(self):
        nodes = [
            Box("a", colour=0),
            Space(direction="down"),
            Box("b", colour=1),
        ]
        state = State(nodes, selected=0)
        self.assertEqual(handle_key(state, "j").nodes, nodes)

    def test_j_leaves_fills_unchanged(self):
        nodes = [Box("a", fill=0), Space(direction="down"), Box("b", fill=1)]
        state = State(nodes, selected=0)
        self.assertEqual(handle_key(state, "j").nodes, nodes)

    def test_j_in_insert_mode_types_the_letter_j(self):
        state = State([Box("a" + PAD)], mode="insert", selected=0)
        self.assertEqual(handle_key(state, "j").nodes, [Box("aj" + PAD)])

    def test_j_does_not_cross_a_space_right(self):
        state = State([Box("a"), Space(direction="right"), Box("b")], selected=0)
        self.assertEqual(handle_key(state, "j").selected, 0)


class OutgoingTest(unittest.TestCase):
    def test_a_lone_box_has_no_outgoing_branches(self):
        self.assertEqual(outgoing([Box()], 0), [])

    def test_a_bare_separator_is_the_tail_branch(self):
        nodes = [Box(), Space("right"), Box()]
        self.assertEqual(outgoing(nodes, 0), [("right", 1)])

    def test_a_bracketed_branch_and_a_tail_branch_are_both_reported(self):
        nodes = [Box(), Push(), Space("down"), Box(), Pop(), Space("right"), Box()]
        self.assertEqual(outgoing(nodes, 0), [("down", 2), ("right", 5)])

    def test_a_lone_bracketed_branch_with_no_tail(self):
        nodes = [Box(), Push(), Space("down"), Box(), Pop()]
        self.assertEqual(outgoing(nodes, 0), [("down", 2)])

    def test_an_arrow_is_reported_as_a_tail_branch(self):
        nodes = [Box(), Arrow("down"), Box()]
        self.assertEqual(outgoing(nodes, 0), [("down", 1)])

    def test_a_bracket_can_contain_nested_brackets_before_its_matching_pop(self):
        nodes = [
            Box(),
            Push(),
            Space("down"),
            Box(),
            Push(),
            Space("right"),
            Box(),
            Pop(),
            Pop(),
            Space("right"),
            Box(),
        ]
        self.assertEqual(outgoing(nodes, 0), [("down", 2), ("right", 9)])

    def test_only_branches_of_the_given_box_are_reported(self):
        nodes = [Box(), Space("right"), Box(), Space("right"), Box()]
        self.assertEqual(outgoing(nodes, 2), [("right", 3)])

    def test_a_box_followed_immediately_by_a_pop_has_no_branches(self):
        nodes = [Box(), Push(), Space("down"), Box(), Pop()]
        self.assertEqual(outgoing(nodes, 3), [])


class SpaceTest(unittest.TestCase):
    def test_spaces_are_equal(self):
        self.assertEqual(Space(), Space())

    def test_a_space_is_not_a_box(self):
        self.assertNotEqual(Space(), Box())


class HandleInsertTest(unittest.TestCase):
    def test_a_printable_character_appends_to_the_last_box_label(self):
        state = bj(State([]))
        self.assertEqual(handle_key(state, "h").nodes, [Box("h" + PAD)])

    def test_letters_bound_in_command_mode_are_ordinary_here(self):
        state = State([Box(PAD)], mode="insert", selected=0)
        self.assertEqual(handle_key(state, "b").nodes, [Box("b" + PAD)])

    def test_q_does_not_stop_the_state(self):
        state = State([Box(PAD)], mode="insert", selected=0)
        self.assertIs(handle_key(state, "q").running, True)

    def test_typing_stays_in_insert_mode(self):
        state = State([Box(PAD)], mode="insert", selected=0)
        self.assertEqual(handle_key(state, "h").mode, "insert")

    def test_typing_appends_to_an_existing_label(self):
        state = State([Box("h" + PAD)], mode="insert", selected=0)
        self.assertEqual(handle_key(state, "i").nodes, [Box("hi" + PAD)])

    def test_typing_edits_only_the_last_box(self):
        state = State([Box("a"), Space(), Box("b" + PAD)], mode="insert", selected=2)
        self.assertEqual(
            handle_key(state, "c").nodes, [Box("a"), Space(), Box("bc" + PAD)]
        )

    def test_esc_then_i_resumes_the_existing_label(self):
        state = bj(State([]))
        state = handle_key(state, "h")
        state = handle_key(state, "i")
        state = handle_key(state, "\x1b")
        state = handle_key(state, "i")
        self.assertEqual(handle_key(state, "!").nodes, [Box("hi!" + PAD)])

    def test_esc_then_i_resumes_the_last_of_several_bj_boxes(self):
        state = bj(State([]))
        state = handle_key(state, "a")
        state = handle_key(state, "\x1b")
        state = bj(state)
        state = handle_key(state, "b")
        state = handle_key(state, "\x1b")
        state = handle_key(state, "i")
        self.assertEqual(
            handle_key(state, "c").nodes, [Box("a"), Space(), Box("bc" + PAD)]
        )

    def test_typing_does_not_mutate_the_given_state(self):
        state = State([Box("h" + PAD)], mode="insert", selected=0)
        handle_key(state, "i")
        self.assertEqual(state.nodes, [Box("h" + PAD)])

    def test_space_is_printable(self):
        state = State([Box("a" + PAD)], mode="insert", selected=0)
        self.assertEqual(handle_key(state, " ").nodes, [Box("a " + PAD)])

    def test_tilde_is_printable(self):
        state = State([Box(PAD)], mode="insert", selected=0)
        self.assertEqual(handle_key(state, "~").nodes, [Box("~" + PAD)])

    def test_backspace_drops_the_last_character(self):
        state = State([Box("hi" + PAD)], mode="insert", selected=0)
        self.assertEqual(handle_key(state, "\x7f").nodes, [Box("h" + PAD)])

    def test_backspace_on_an_empty_label_is_a_no_op(self):
        state = State([Box(PAD)], mode="insert", selected=0)
        self.assertEqual(handle_key(state, "\x7f"), state)

    def test_backspace_does_not_mutate_the_given_state(self):
        state = State([Box("hi" + PAD)], mode="insert", selected=0)
        handle_key(state, "\x7f")
        self.assertEqual(state.nodes, [Box("hi" + PAD)])

    def test_esc_returns_to_command_mode(self):
        state = State([Box(PAD)], mode="insert", selected=0)
        self.assertEqual(handle_key(state, "\x1b").mode, "command")

    def test_esc_preserves_the_nodes(self):
        state = State([Box("hi" + PAD)], mode="insert", selected=0)
        self.assertEqual(handle_key(state, "\x1b").nodes, [Box("hi")])

    def test_esc_preserves_the_selection(self):
        state = State([Box("a" + PAD), Space(), Box("b")], mode="insert", selected=0)
        self.assertEqual(handle_key(state, "\x1b").selected, 0)

    def test_esc_does_not_mutate_the_given_state(self):
        state = State([Box(PAD)], mode="insert", selected=0)
        handle_key(state, "\x1b")
        self.assertEqual(state.mode, "insert")

    def test_a_control_character_returns_the_state_unchanged(self):
        state = State([Box("hi" + PAD)], mode="insert", selected=0)
        self.assertEqual(handle_key(state, "\x01"), state)

    def test_a_non_ascii_character_returns_the_state_unchanged(self):
        state = State([Box("hi" + PAD)], mode="insert", selected=0)
        self.assertEqual(handle_key(state, "\u00e9"), state)

    def test_typing_leaves_the_boxs_colour_unchanged(self):
        state = State([Box("a" + PAD, colour=0)], mode="insert", selected=0)
        self.assertEqual(handle_key(state, "z").nodes, [Box("az" + PAD, colour=0)])


    def test_typing_edits_the_selected_box(self):
        state = State([Box("a" + PAD), Space(), Box("b")], mode="insert", selected=0)
        self.assertEqual(
            handle_key(state, "c").nodes, [Box("ac" + PAD), Space(), Box("b")]
        )

    def test_backspace_edits_the_selected_box(self):
        state = State([Box("ab" + PAD), Space(), Box("cd")], mode="insert", selected=0)
        self.assertEqual(
            handle_key(state, "\x7f").nodes, [Box("a" + PAD), Space(), Box("cd")]
        )


class CycleColourTest(unittest.TestCase):
    def test_c_advances_the_selected_box_from_plain(self):
        state = State([Box("a")], selected=0)
        self.assertEqual(handle_key(state, "c").nodes, [Box("a", colour=next_colour(PLAIN))])

    def test_c_advances_the_selected_box_through_the_cycle(self):
        state = State([Box("a")], selected=0)
        for _ in range(PALETTE_SIZE):
            state = handle_key(state, "c")
        self.assertNotEqual(state.nodes[0].colour, PLAIN)
        state = handle_key(state, "c")
        self.assertEqual(state.nodes[0].colour, PLAIN)

    def test_c_changes_only_the_selected_box(self):
        state = State([Box("a"), Space(), Box("b")], selected=2)
        self.assertEqual(
            handle_key(state, "c").nodes,
            [Box("a"), Space(), Box("b", colour=next_colour(PLAIN))],
        )

    def test_c_preserves_the_label(self):
        state = State([Box("hi")], selected=0)
        self.assertEqual(handle_key(state, "c").nodes[0].label, "hi")

    def test_c_on_an_empty_canvas_returns_the_state_unchanged(self):
        state = State([])
        self.assertEqual(handle_key(state, "c"), state)

    def test_c_does_not_mutate_the_given_state(self):
        state = State([Box("a")], selected=0)
        handle_key(state, "c")
        self.assertEqual(state.nodes, [Box("a")])

    def test_j_leaves_colours_unchanged(self):
        nodes = [Box("a", colour=0), Space(), Box("b", colour=1)]
        state = State(nodes, selected=0)
        self.assertEqual(handle_key(state, "j").nodes, nodes)

    def test_k_leaves_colours_unchanged(self):
        nodes = [Box("a", colour=0), Space(), Box("b", colour=1)]
        state = State(nodes, selected=2)
        self.assertEqual(handle_key(state, "k").nodes, nodes)

    def test_bj_leaves_existing_colours_unchanged(self):
        state = State([Box("a", colour=0)], selected=0)
        self.assertEqual(
            bj(state).nodes,
            [Box("a", colour=0), Space(), Box(PAD)],
        )

    def test_bj_adds_a_box_with_the_plain_colour(self):
        state = State([])
        self.assertEqual(bj(state).nodes, [Box(PAD, colour=PLAIN)])

    def test_c_in_insert_mode_types_the_letter_c(self):
        state = State([Box("a" + PAD)], mode="insert", selected=0)
        self.assertEqual(handle_key(state, "c").nodes, [Box("ac" + PAD)])


class CycleFillTest(unittest.TestCase):
    def test_f_advances_the_selected_box_from_plain(self):
        state = State([Box("a")], selected=0)
        self.assertEqual(handle_key(state, "f").nodes, [Box("a", fill=next_colour(PLAIN))])

    def test_f_advances_the_selected_box_through_the_cycle(self):
        state = State([Box("a")], selected=0)
        for _ in range(PALETTE_SIZE):
            state = handle_key(state, "f")
        self.assertNotEqual(state.nodes[0].fill, PLAIN)
        state = handle_key(state, "f")
        self.assertEqual(state.nodes[0].fill, PLAIN)

    def test_f_changes_only_the_selected_box(self):
        state = State([Box("a"), Box("b")], selected=1)
        self.assertEqual(
            handle_key(state, "f").nodes,
            [Box("a"), Box("b", fill=next_colour(PLAIN))],
        )

    def test_f_preserves_the_label(self):
        state = State([Box("hi")], selected=0)
        self.assertEqual(handle_key(state, "f").nodes[0].label, "hi")

    def test_f_on_an_empty_canvas_returns_the_state_unchanged(self):
        state = State([])
        self.assertEqual(handle_key(state, "f"), state)

    def test_f_does_not_mutate_the_given_state(self):
        state = State([Box("a")], selected=0)
        handle_key(state, "f")
        self.assertEqual(state.nodes, [Box("a")])

    def test_j_leaves_fills_unchanged(self):
        state = State([Box("a", fill=0), Box("b", fill=1)], selected=0)
        self.assertEqual(
            handle_key(state, "j").nodes,
            [Box("a", fill=0), Box("b", fill=1)],
        )

    def test_k_leaves_fills_unchanged(self):
        state = State([Box("a", fill=0), Box("b", fill=1)], selected=1)
        self.assertEqual(
            handle_key(state, "k").nodes,
            [Box("a", fill=0), Box("b", fill=1)],
        )

    def test_bj_leaves_existing_fills_unchanged(self):
        state = State([Box("a", fill=0)], selected=0)
        self.assertEqual(
            bj(state).nodes,
            [Box("a", fill=0), Space(), Box(PAD)],
        )

    def test_f_in_insert_mode_types_the_letter_f(self):
        state = State([Box("a" + PAD)], mode="insert", selected=0)
        self.assertEqual(handle_key(state, "f").nodes, [Box("af" + PAD)])

    def test_c_does_not_change_fill(self):
        state = State([Box("a", fill=next_colour(PLAIN))], selected=0)
        self.assertEqual(handle_key(state, "c").nodes[0].fill, next_colour(PLAIN))

    def test_f_does_not_change_colour(self):
        state = State([Box("a", colour=next_colour(PLAIN))], selected=0)
        self.assertEqual(handle_key(state, "f").nodes[0].colour, next_colour(PLAIN))

    def test_colour_and_fill_are_independent(self):
        state = State([Box("a")], selected=0)
        state = handle_key(state, "c")
        state = handle_key(state, "f")
        self.assertEqual(
            state.nodes,
            [Box("a", colour=next_colour(PLAIN), fill=next_colour(PLAIN))],
        )


class EnterInsertModeTest(unittest.TestCase):
    def test_i_enters_insert_mode(self):
        state = State([Box("a")], selected=0)
        self.assertEqual(handle_key(state, "i").mode, "insert")

    def test_i_leaves_the_selection_alone(self):
        state = State([Box("a"), Space(), Box("b")], selected=0)
        self.assertEqual(handle_key(state, "i").selected, 0)

    def test_i_on_an_empty_canvas_returns_the_state_unchanged(self):
        state = State([])
        self.assertEqual(handle_key(state, "i"), state)

    def test_i_types_into_the_selected_box(self):
        state = State([Box("a"), Space(), Box("b"), Space(), Box("c")], selected=2)
        typed = handle_key(handle_key(state, "i"), "z")
        self.assertEqual(
            typed.nodes, [Box("a"), Space(), Box("bz" + PAD), Space(), Box("c")]
        )

    def test_i_preserves_the_nodes(self):
        state = State([Box("a"), Space(), Box("b")], selected=0)
        self.assertEqual(
            handle_key(state, "i").nodes, [Box("a" + PAD), Space(), Box("b")]
        )

    def test_i_keeps_the_state_running(self):
        state = State([Box("hi")], selected=0)
        self.assertIs(handle_key(state, "i").running, True)

    def test_i_does_not_mutate_the_given_state(self):
        state = State([Box("hi")], selected=0)
        handle_key(state, "i")
        self.assertEqual(state.mode, "command")

    def test_i_on_an_empty_canvas_leaves_the_mode_as_command(self):
        self.assertEqual(handle_key(State([]), "i").mode, "command")

    def test_i_clears_source(self):
        state = State([Box("a")], selected=0, source=0)
        self.assertEqual(handle_key(state, "i").source, -1)


class PressATest(unittest.TestCase):
    def test_a_on_the_selected_box_sets_source(self):
        state = State([Box("a")], selected=0)
        self.assertEqual(handle_key(state, "a").source, 0)

    def test_a_changes_nothing_else(self):
        state = State([Box("a"), Space(), Box("b")], selected=2)
        pressed = handle_key(state, "a")
        self.assertEqual(
            (pressed.nodes, pressed.selected, pressed.mode, pressed.running),
            (state.nodes, state.selected, state.mode, state.running),
        )

    def test_a_on_an_empty_canvas_returns_the_state_unchanged(self):
        state = State([])
        self.assertEqual(handle_key(state, "a"), state)

    def test_a_does_not_mutate_the_given_state(self):
        state = State([Box("a")], selected=0)
        handle_key(state, "a")
        self.assertEqual(state.source, -1)

    def test_a_on_the_box_below_the_source_draws_a_forward_arrow(self):
        state = State([Box("a"), Space(), Box("b")], selected=2, source=0)
        connected = handle_key(state, "a")
        self.assertEqual(
            connected.nodes, [Box("a"), Arrow("down"), Box("b")]
        )

    def test_a_on_the_box_above_the_source_draws_a_backward_arrow(self):
        state = State([Box("a"), Space(), Box("b")], selected=0, source=2)
        connected = handle_key(state, "a")
        self.assertEqual(
            connected.nodes, [Box("a"), Arrow("up"), Box("b")]
        )

    def test_drawing_an_arrow_leaves_the_length_and_other_indices_unchanged(self):
        nodes = [Box("a"), Space(), Box("b"), Space(), Box("c")]
        state = State(nodes, selected=2, source=0)
        connected = handle_key(state, "a")
        self.assertEqual(len(connected.nodes), len(nodes))
        self.assertEqual(connected.nodes[0], nodes[0])
        self.assertEqual(connected.nodes[2], nodes[2])
        self.assertEqual(connected.nodes[3], nodes[3])
        self.assertEqual(connected.nodes[4], nodes[4])

    def test_drawing_an_arrow_clears_source(self):
        state = State([Box("a"), Space(), Box("b")], selected=2, source=0)
        self.assertEqual(handle_key(state, "a").source, -1)

    def test_a_two_boxes_from_the_source_does_nothing(self):
        nodes = [
            Box("a"),
            Space(),
            Box("b"),
            Space(),
            Box("c"),
            Space(),
            Box("d"),
        ]
        state = State(nodes, selected=4, source=0)
        connected = handle_key(state, "a")
        self.assertEqual(connected.nodes, nodes)
        self.assertEqual(connected.source, 0)

    def test_a_on_the_source_box_itself_does_nothing(self):
        state = State([Box("a")], selected=0, source=0)
        connected = handle_key(state, "a")
        self.assertEqual(connected.nodes, [Box("a")])
        self.assertEqual(connected.source, 0)

    def test_a_on_the_box_to_the_right_of_the_source_draws_a_right_arrow(self):
        state = State(
            [Box("a"), Space("right"), Box("b")], selected=2, source=0
        )
        connected = handle_key(state, "a")
        self.assertEqual(
            connected.nodes, [Box("a"), Arrow("right"), Box("b")]
        )

    def test_a_on_the_box_to_the_left_of_the_source_draws_a_left_arrow(self):
        state = State(
            [Box("a"), Space("right"), Box("b")], selected=0, source=2
        )
        connected = handle_key(state, "a")
        self.assertEqual(
            connected.nodes, [Box("a"), Arrow("left"), Box("b")]
        )

    def test_a_over_an_existing_horizontal_arrow_flips_direction_keeping_axis(
        self,
    ):
        state = State(
            [Box("a"), Arrow("right"), Box("b")], selected=0, source=2
        )
        connected = handle_key(state, "a")
        self.assertEqual(
            connected.nodes, [Box("a"), Arrow("left"), Box("b")]
        )

    def test_a_over_an_existing_arrow_replaces_it_flipping_direction(self):
        state = State([Box("a"), Space(), Box("b")], selected=0, source=-1)
        state = handle_key(state, "a")
        state = handle_key(state, "j")
        state = handle_key(state, "a")
        self.assertEqual(state.nodes, [Box("a"), Arrow("down"), Box("b")])
        state = handle_key(state, "a")
        state = handle_key(state, "k")
        state = handle_key(state, "a")
        self.assertEqual(state.nodes, [Box("a"), Arrow("up"), Box("b")])


if __name__ == "__main__":
    unittest.main()
