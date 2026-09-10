import unittest

from sketch.state import (
    PAD,
    PALETTE_SIZE,
    PLAIN,
    Arrow,
    Box,
    Space,
    State,
    below,
    beside,
    edit,
    handle_key,
    next_colour,
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


class HandleKeyTest(unittest.TestCase):
    def test_b_on_an_empty_canvas_appends_only_a_box(self):
        self.assertEqual(handle_key(State([]), "b").nodes, [Box(PAD)])

    def test_b_on_an_empty_canvas_enters_insert_mode(self):
        self.assertEqual(handle_key(State([]), "b").mode, "insert")

    def test_b_on_an_empty_canvas_selects_the_new_box(self):
        self.assertEqual(handle_key(State([]), "b").selected, 0)

    def test_b_on_an_empty_canvas_keeps_the_state_running(self):
        self.assertIs(handle_key(State([]), "b").running, True)

    def test_b_on_an_empty_canvas_preserves_a_stopped_state(self):
        state = handle_key(State([]), "q")
        self.assertIs(handle_key(state, "b").running, False)

    def test_b_on_an_empty_canvas_does_not_mutate_the_given_state(self):
        state = State([])
        handle_key(state, "b")
        self.assertEqual(state.nodes, [])

    def test_q_keeps_command_mode(self):
        self.assertEqual(handle_key(State([]), "q").mode, "command")

    def test_insert_mode_is_dispatched_separately(self):
        state = State([Box(PAD)], mode="insert", selected=0)
        self.assertIs(handle_key(state, "q").running, True)

    def test_bj_appends_to_existing_nodes(self):
        self.assertEqual(
            bj(State([Box()])).nodes, [Box(), Space(), Box(PAD)]
        )

    def test_bj_separates_the_new_box_from_the_last_one_with_a_space(self):
        state = State([Box("a")], selected=0)
        appended = bj(state)
        self.assertEqual(appended.nodes, [Box("a"), Space(), Box(PAD)])
        self.assertEqual(appended.selected, len(appended.nodes) - 1)

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

    def test_bj_selects_the_new_last_box(self):
        state = State([Box("a")], selected=0)
        self.assertEqual(bj(state).selected, 2)

    def test_bj_selects_the_new_box_when_the_selection_was_not_last(self):
        state = State([Box("a"), Space(), Box("b")], selected=0)
        self.assertEqual(bj(state).selected, 4)

    def test_q_preserves_the_selection(self):
        state = State([Box("a"), Space(), Box("b")], selected=0)
        self.assertEqual(handle_key(state, "q").selected, 0)

    def test_bj_clears_source(self):
        state = State([Box("a")], selected=0, source=0)
        self.assertEqual(bj(state).source, -1)


class PendingCommandTest(unittest.TestCase):
    def test_b_sets_pending_to_b(self):
        state = State([Box("a")], selected=0)
        self.assertEqual(handle_key(state, "b").pending, "b")

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

    def test_b_then_unrecognized_key_clears_pending(self):
        state = State([Box("a")], selected=0)
        result = handle_key(handle_key(state, "b"), "i")
        self.assertEqual(result.pending, "")

    def test_b_then_unrecognized_key_adds_no_node(self):
        state = State([Box("a")], selected=0)
        result = handle_key(handle_key(state, "b"), "i")
        self.assertEqual(result.nodes, [Box("a")])

    def test_b_then_i_does_not_enter_insert_mode(self):
        state = State([Box("a")], selected=0)
        result = handle_key(handle_key(state, "b"), "i")
        self.assertEqual(result.mode, "command")

    def test_b_then_unrecognized_key_leaves_selected_and_source_unchanged(self):
        state = State([Box("a")], selected=0, source=0)
        result = handle_key(handle_key(state, "b"), "i")
        self.assertEqual((result.selected, result.source), (0, 0))


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


class SpaceTest(unittest.TestCase):
    def test_spaces_are_equal(self):
        self.assertEqual(Space(), Space())

    def test_a_space_is_not_a_box(self):
        self.assertNotEqual(Space(), Box())


class HandleInsertTest(unittest.TestCase):
    def test_a_printable_character_appends_to_the_last_box_label(self):
        state = handle_key(State([]), "b")
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
        state = handle_key(State([]), "b")
        state = handle_key(state, "h")
        state = handle_key(state, "i")
        state = handle_key(state, "\x1b")
        state = handle_key(state, "i")
        self.assertEqual(handle_key(state, "!").nodes, [Box("hi!" + PAD)])

    def test_esc_then_i_resumes_the_last_of_several_bj_boxes(self):
        state = handle_key(State([]), "b")
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

    def test_b_adds_a_box_with_the_plain_colour_on_an_empty_canvas(self):
        state = State([])
        self.assertEqual(handle_key(state, "b").nodes, [Box(PAD, colour=PLAIN)])

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
