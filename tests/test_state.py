import unittest

from sketch.state import (
    PAD,
    PALETTE_SIZE,
    PLAIN,
    Box,
    State,
    at,
    colour_row,
    grow,
    handle_key,
    next_colour,
    rewrite,
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


class BoxChildrenTest(unittest.TestCase):
    def test_boxes_default_to_no_children(self):
        self.assertEqual(Box().children, ())

    def test_boxes_with_different_children_are_not_equal(self):
        self.assertNotEqual(Box(children=(Box("a"),)), Box(children=(Box("b"),)))


class StateTest(unittest.TestCase):
    def test_state_starts_running(self):
        self.assertIs(State().running, True)

    def test_state_starts_in_command_mode(self):
        self.assertEqual(State().mode, "command")

    def test_an_empty_state_has_no_boxes(self):
        self.assertEqual(State().boxes, ())

    def test_an_empty_state_has_no_selection(self):
        self.assertEqual(State().selected, ())


class AtTest(unittest.TestCase):
    def test_at_a_single_index_returns_the_top_level_box(self):
        boxes = (Box("a"), Box("b"))
        self.assertEqual(at(boxes, (1,)), Box("b"))

    def test_at_a_longer_path_walks_into_children(self):
        boxes = (Box("a", children=(Box("c"), Box("d"))),)
        self.assertEqual(at(boxes, (0, 1)), Box("d"))

    def test_at_a_deep_path_walks_multiple_levels(self):
        boxes = (Box("a", children=(Box("b", children=(Box("c"),)),)),)
        self.assertEqual(at(boxes, (0, 0, 0)), Box("c"))


class RewriteTest(unittest.TestCase):
    def test_rewrite_replaces_the_top_level_box(self):
        boxes = (Box("a"), Box("b"))
        result = rewrite(boxes, (1,), lambda box: Box("z"))
        self.assertEqual(result, (Box("a"), Box("z")))

    def test_rewrite_replaces_a_nested_box_and_rebuilds_the_spine(self):
        boxes = (Box("a", children=(Box("c"), Box("d"))),)
        result = rewrite(boxes, (0, 1), lambda box: Box("z"))
        self.assertEqual(result, (Box("a", children=(Box("c"), Box("z"))),))

    def test_rewrite_does_not_mutate_the_given_boxes(self):
        boxes = (Box("a"),)
        rewrite(boxes, (0,), lambda box: Box("z"))
        self.assertEqual(boxes, (Box("a"),))


class GrowTest(unittest.TestCase):
    def test_grow_on_the_canvas_appends_a_top_level_box(self):
        boxes, path = grow((), ())
        self.assertEqual(boxes, (Box(PAD),))
        self.assertEqual(path, (0,))

    def test_grow_on_the_canvas_appends_after_existing_boxes(self):
        boxes, path = grow((Box("a"),), ())
        self.assertEqual(boxes, (Box("a"), Box(PAD)))
        self.assertEqual(path, (1,))

    def test_grow_on_a_box_appends_a_child(self):
        boxes, path = grow((Box("a"),), (0,))
        self.assertEqual(boxes, (Box("a", children=(Box(PAD),)),))
        self.assertEqual(path, (0, 0))

    def test_grow_on_a_box_with_a_child_appends_a_second_child(self):
        boxes = (Box("a", children=(Box("c"),)),)
        boxes, path = grow(boxes, (0,))
        self.assertEqual(boxes, (Box("a", children=(Box("c"), Box(PAD))),))
        self.assertEqual(path, (0, 1))

    def test_grow_does_not_mutate_the_given_boxes(self):
        boxes = (Box("a"),)
        grow(boxes, ())
        self.assertEqual(boxes, (Box("a"),))


class HandleKeyBTest(unittest.TestCase):
    def test_b_on_an_empty_canvas_appends_a_box(self):
        self.assertEqual(handle_key(State(), "b").boxes, (Box(PAD),))

    def test_b_on_an_empty_canvas_enters_insert_mode(self):
        self.assertEqual(handle_key(State(), "b").mode, "insert")

    def test_b_on_an_empty_canvas_selects_the_new_box(self):
        self.assertEqual(handle_key(State(), "b").selected, (0,))

    def test_b_on_an_empty_canvas_keeps_the_state_running(self):
        self.assertIs(handle_key(State(), "b").running, True)

    def test_b_on_an_empty_canvas_preserves_a_stopped_state(self):
        state = handle_key(State(), "q")
        self.assertIs(handle_key(state, "b").running, False)

    def test_b_on_an_empty_canvas_does_not_mutate_the_given_state(self):
        state = State()
        handle_key(state, "b")
        self.assertEqual(state.boxes, ())

    def test_b_on_a_selected_box_appends_a_child(self):
        state = State(boxes=(Box("a"),), selected=(0,))
        result = handle_key(state, "b")
        self.assertEqual(result.boxes, (Box("a", children=(Box(PAD),)),))

    def test_b_on_a_selected_box_selects_the_new_child(self):
        state = State(boxes=(Box("a"),), selected=(0,))
        self.assertEqual(handle_key(state, "b").selected, (0, 0))

    def test_b_on_a_selected_box_enters_insert_mode(self):
        state = State(boxes=(Box("a"),), selected=(0,))
        self.assertEqual(handle_key(state, "b").mode, "insert")

    def test_a_second_b_on_the_same_parent_places_a_second_child(self):
        state = State(boxes=(Box("a"),), selected=(0,))
        state = handle_key(state, "b")
        state = handle_key(state, "\x1b")
        state = handle_key(state, "h")
        state = handle_key(state, "b")
        self.assertEqual(
            state.boxes, (Box("a", children=(Box(""), Box(PAD))),)
        )
        self.assertEqual(state.selected, (0, 1))

    def test_unknown_key_returns_the_state_unchanged(self):
        state = State(boxes=(Box("a"),))
        self.assertEqual(handle_key(state, "x"), state)

    def test_q_stops_the_state(self):
        self.assertIs(handle_key(State(boxes=(Box("a"),)), "q").running, False)

    def test_q_preserves_the_boxes(self):
        self.assertEqual(
            handle_key(State(boxes=(Box("a"),)), "q").boxes, (Box("a"),)
        )

    def test_q_does_not_mutate_the_given_state(self):
        state = State(boxes=(Box("a"),))
        handle_key(state, "q")
        self.assertIs(state.running, True)

    def test_q_preserves_the_selection(self):
        state = State(boxes=(Box("a"),), selected=(0,))
        self.assertEqual(handle_key(state, "q").selected, (0,))

    def test_q_keeps_command_mode(self):
        self.assertEqual(handle_key(State(), "q").mode, "command")

    def test_insert_mode_is_dispatched_separately(self):
        state = State(boxes=(Box(PAD),), mode="insert", selected=(0,))
        self.assertIs(handle_key(state, "q").running, True)


class MoveSelectionParentTest(unittest.TestCase):
    def test_h_selects_the_parent(self):
        state = State(boxes=(Box("a", children=(Box("c"),)),), selected=(0, 0))
        self.assertEqual(handle_key(state, "h").selected, (0,))

    def test_h_on_a_top_level_box_keeps_the_selection(self):
        state = State(boxes=(Box("a"),), selected=(0,))
        self.assertEqual(handle_key(state, "h").selected, (0,))

    def test_h_on_an_empty_canvas_returns_the_state_unchanged(self):
        state = State()
        self.assertEqual(handle_key(state, "h"), state)

    def test_h_preserves_the_mode_the_running_flag_and_the_boxes(self):
        state = State(boxes=(Box("a", children=(Box("c"),)),), selected=(0, 0))
        moved = handle_key(state, "h")
        self.assertEqual(
            (moved.mode, moved.running, moved.boxes),
            (state.mode, state.running, state.boxes),
        )

    def test_h_does_not_mutate_the_given_state(self):
        state = State(boxes=(Box("a", children=(Box("c"),)),), selected=(0, 0))
        handle_key(state, "h")
        self.assertEqual(state.selected, (0, 0))

    def test_h_in_insert_mode_types_the_letter_h(self):
        state = State(boxes=(Box("a" + PAD),), mode="insert", selected=(0,))
        self.assertEqual(handle_key(state, "h").boxes, (Box("ah" + PAD),))


class MoveSelectionFirstChildTest(unittest.TestCase):
    def test_l_selects_the_first_child(self):
        boxes = (Box("a", children=(Box("c"), Box("d"))),)
        state = State(boxes=boxes, selected=(0,))
        self.assertEqual(handle_key(state, "l").selected, (0, 0))

    def test_l_with_no_children_keeps_the_selection(self):
        state = State(boxes=(Box("a"),), selected=(0,))
        self.assertEqual(handle_key(state, "l").selected, (0,))

    def test_l_on_an_empty_canvas_returns_the_state_unchanged(self):
        state = State()
        self.assertEqual(handle_key(state, "l"), state)

    def test_l_preserves_the_mode_the_running_flag_and_the_boxes(self):
        boxes = (Box("a", children=(Box("c"),)),)
        state = State(boxes=boxes, selected=(0,))
        moved = handle_key(state, "l")
        self.assertEqual(
            (moved.mode, moved.running, moved.boxes),
            (state.mode, state.running, state.boxes),
        )

    def test_l_does_not_mutate_the_given_state(self):
        boxes = (Box("a", children=(Box("c"),)),)
        state = State(boxes=boxes, selected=(0,))
        handle_key(state, "l")
        self.assertEqual(state.selected, (0,))

    def test_l_in_insert_mode_types_the_letter_l(self):
        state = State(boxes=(Box("a" + PAD),), mode="insert", selected=(0,))
        self.assertEqual(handle_key(state, "l").boxes, (Box("al" + PAD),))


class MoveSelectionSiblingTest(unittest.TestCase):
    def test_j_selects_the_next_top_level_sibling(self):
        state = State(boxes=(Box("a"), Box("b")), selected=(0,))
        self.assertEqual(handle_key(state, "j").selected, (1,))

    def test_j_with_no_next_sibling_keeps_the_selection(self):
        state = State(boxes=(Box("a"), Box("b")), selected=(1,))
        self.assertEqual(handle_key(state, "j").selected, (1,))

    def test_j_on_an_empty_canvas_returns_the_state_unchanged(self):
        state = State()
        self.assertEqual(handle_key(state, "j"), state)

    def test_j_selects_the_next_child_sibling(self):
        boxes = (Box("a", children=(Box("c"), Box("d"))),)
        state = State(boxes=boxes, selected=(0, 0))
        self.assertEqual(handle_key(state, "j").selected, (0, 1))

    def test_j_preserves_the_mode_the_running_flag_and_the_boxes(self):
        state = State(boxes=(Box("a"), Box("b")), selected=(0,))
        moved = handle_key(state, "j")
        self.assertEqual(
            (moved.mode, moved.running, moved.boxes),
            (state.mode, state.running, state.boxes),
        )

    def test_j_does_not_mutate_the_given_state(self):
        state = State(boxes=(Box("a"), Box("b")), selected=(0,))
        handle_key(state, "j")
        self.assertEqual(state.selected, (0,))

    def test_j_in_insert_mode_types_the_letter_j(self):
        state = State(boxes=(Box("a" + PAD),), mode="insert", selected=(0,))
        self.assertEqual(handle_key(state, "j").boxes, (Box("aj" + PAD),))

    def test_k_selects_the_previous_top_level_sibling(self):
        state = State(boxes=(Box("a"), Box("b")), selected=(1,))
        self.assertEqual(handle_key(state, "k").selected, (0,))

    def test_k_on_the_first_sibling_keeps_the_selection(self):
        state = State(boxes=(Box("a"), Box("b")), selected=(0,))
        self.assertEqual(handle_key(state, "k").selected, (0,))

    def test_k_on_an_empty_canvas_returns_the_state_unchanged(self):
        state = State()
        self.assertEqual(handle_key(state, "k"), state)

    def test_k_selects_the_previous_child_sibling(self):
        boxes = (Box("a", children=(Box("c"), Box("d"))),)
        state = State(boxes=boxes, selected=(0, 1))
        self.assertEqual(handle_key(state, "k").selected, (0, 0))

    def test_k_preserves_the_mode_the_running_flag_and_the_boxes(self):
        state = State(boxes=(Box("a"), Box("b")), selected=(1,))
        moved = handle_key(state, "k")
        self.assertEqual(
            (moved.mode, moved.running, moved.boxes),
            (state.mode, state.running, state.boxes),
        )

    def test_k_does_not_mutate_the_given_state(self):
        state = State(boxes=(Box("a"), Box("b")), selected=(1,))
        handle_key(state, "k")
        self.assertEqual(state.selected, (1,))

    def test_k_in_insert_mode_types_the_letter_k(self):
        state = State(boxes=(Box("a" + PAD),), mode="insert", selected=(0,))
        self.assertEqual(handle_key(state, "k").boxes, (Box("ak" + PAD),))


class EnterInsertModeTest(unittest.TestCase):
    def test_i_enters_insert_mode(self):
        state = State(boxes=(Box("a"),), selected=(0,))
        self.assertEqual(handle_key(state, "i").mode, "insert")

    def test_i_leaves_the_selection_alone(self):
        state = State(boxes=(Box("a"), Box("b")), selected=(0,))
        self.assertEqual(handle_key(state, "i").selected, (0,))

    def test_i_on_an_empty_canvas_returns_the_state_unchanged(self):
        state = State()
        self.assertEqual(handle_key(state, "i"), state)

    def test_i_appends_pad_to_the_selected_boxs_label(self):
        state = State(boxes=(Box("a"), Box("b")), selected=(1,))
        self.assertEqual(handle_key(state, "i").boxes, (Box("a"), Box("b" + PAD)))

    def test_i_types_into_the_selected_box(self):
        state = State(boxes=(Box("a"), Box("b")), selected=(1,))
        typed = handle_key(handle_key(state, "i"), "z")
        self.assertEqual(typed.boxes, (Box("a"), Box("bz" + PAD)))

    def test_i_keeps_the_state_running(self):
        state = State(boxes=(Box("hi"),), selected=(0,))
        self.assertIs(handle_key(state, "i").running, True)

    def test_i_does_not_mutate_the_given_state(self):
        state = State(boxes=(Box("hi"),), selected=(0,))
        handle_key(state, "i")
        self.assertEqual(state.mode, "command")

    def test_i_on_an_empty_canvas_leaves_the_mode_as_command(self):
        self.assertEqual(handle_key(State(), "i").mode, "command")

    def test_i_on_a_nested_box_edits_that_box(self):
        boxes = (Box("a", children=(Box("c"),)),)
        state = State(boxes=boxes, selected=(0, 0))
        self.assertEqual(
            handle_key(state, "i").boxes,
            (Box("a", children=(Box("c" + PAD),)),),
        )


class HandleInsertTest(unittest.TestCase):
    def test_a_printable_character_appends_to_the_selected_box_label(self):
        state = handle_key(State(), "b")
        self.assertEqual(handle_key(state, "h").boxes, (Box("h" + PAD),))

    def test_letters_bound_in_command_mode_are_ordinary_here(self):
        state = State(boxes=(Box(PAD),), mode="insert", selected=(0,))
        self.assertEqual(handle_key(state, "b").boxes, (Box("b" + PAD),))

    def test_q_does_not_stop_the_state(self):
        state = State(boxes=(Box(PAD),), mode="insert", selected=(0,))
        self.assertIs(handle_key(state, "q").running, True)

    def test_typing_stays_in_insert_mode(self):
        state = State(boxes=(Box(PAD),), mode="insert", selected=(0,))
        self.assertEqual(handle_key(state, "h").mode, "insert")

    def test_typing_appends_to_an_existing_label(self):
        state = State(boxes=(Box("h" + PAD),), mode="insert", selected=(0,))
        self.assertEqual(handle_key(state, "i").boxes, (Box("hi" + PAD),))

    def test_typing_edits_only_the_selected_box(self):
        state = State(
            boxes=(Box("a"), Box("b" + PAD)), mode="insert", selected=(1,)
        )
        self.assertEqual(
            handle_key(state, "c").boxes, (Box("a"), Box("bc" + PAD))
        )

    def test_esc_then_i_resumes_the_existing_label(self):
        state = handle_key(State(), "b")
        state = handle_key(state, "h")
        state = handle_key(state, "i")
        state = handle_key(state, "\x1b")
        state = handle_key(state, "i")
        self.assertEqual(handle_key(state, "!").boxes, (Box("hi!" + PAD),))

    def test_typing_does_not_mutate_the_given_state(self):
        state = State(boxes=(Box("h" + PAD),), mode="insert", selected=(0,))
        handle_key(state, "i")
        self.assertEqual(state.boxes, (Box("h" + PAD),))

    def test_space_is_printable(self):
        state = State(boxes=(Box("a" + PAD),), mode="insert", selected=(0,))
        self.assertEqual(handle_key(state, " ").boxes, (Box("a " + PAD),))

    def test_tilde_is_printable(self):
        state = State(boxes=(Box(PAD),), mode="insert", selected=(0,))
        self.assertEqual(handle_key(state, "~").boxes, (Box("~" + PAD),))

    def test_backspace_drops_the_last_character(self):
        state = State(boxes=(Box("hi" + PAD),), mode="insert", selected=(0,))
        self.assertEqual(handle_key(state, "\x7f").boxes, (Box("h" + PAD),))

    def test_backspace_on_an_empty_label_is_a_no_op(self):
        state = State(boxes=(Box(PAD),), mode="insert", selected=(0,))
        self.assertEqual(handle_key(state, "\x7f"), state)

    def test_backspace_does_not_mutate_the_given_state(self):
        state = State(boxes=(Box("hi" + PAD),), mode="insert", selected=(0,))
        handle_key(state, "\x7f")
        self.assertEqual(state.boxes, (Box("hi" + PAD),))

    def test_backspace_edits_the_selected_box(self):
        state = State(
            boxes=(Box("ab" + PAD), Box("cd")), mode="insert", selected=(0,)
        )
        self.assertEqual(
            handle_key(state, "\x7f").boxes, (Box("a" + PAD), Box("cd"))
        )

    def test_esc_returns_to_command_mode(self):
        state = State(boxes=(Box(PAD),), mode="insert", selected=(0,))
        self.assertEqual(handle_key(state, "\x1b").mode, "command")

    def test_esc_preserves_the_boxes(self):
        state = State(boxes=(Box("hi" + PAD),), mode="insert", selected=(0,))
        self.assertEqual(handle_key(state, "\x1b").boxes, (Box("hi"),))

    def test_esc_preserves_the_selection(self):
        state = State(
            boxes=(Box("a" + PAD), Box("b")), mode="insert", selected=(0,)
        )
        self.assertEqual(handle_key(state, "\x1b").selected, (0,))

    def test_esc_does_not_mutate_the_given_state(self):
        state = State(boxes=(Box(PAD),), mode="insert", selected=(0,))
        handle_key(state, "\x1b")
        self.assertEqual(state.mode, "insert")

    def test_a_control_character_returns_the_state_unchanged(self):
        state = State(boxes=(Box("hi" + PAD),), mode="insert", selected=(0,))
        self.assertEqual(handle_key(state, "\x01"), state)

    def test_a_non_ascii_character_returns_the_state_unchanged(self):
        state = State(boxes=(Box("hi" + PAD),), mode="insert", selected=(0,))
        self.assertEqual(handle_key(state, "é"), state)

    def test_typing_leaves_the_boxs_colour_unchanged(self):
        state = State(
            boxes=(Box("a" + PAD, colour=0),), mode="insert", selected=(0,)
        )
        self.assertEqual(
            handle_key(state, "z").boxes, (Box("az" + PAD, colour=0),)
        )

    def test_c_in_insert_mode_types_the_letter_c(self):
        state = State(boxes=(Box("a" + PAD),), mode="insert", selected=(0,))
        self.assertEqual(handle_key(state, "c").boxes, (Box("ac" + PAD),))

    def test_f_in_insert_mode_types_the_letter_f(self):
        state = State(boxes=(Box("a" + PAD),), mode="insert", selected=(0,))
        self.assertEqual(handle_key(state, "f").boxes, (Box("af" + PAD),))

    def test_t_in_insert_mode_types_the_letter_t(self):
        state = State(boxes=(Box("a" + PAD),), mode="insert", selected=(0,))
        self.assertEqual(handle_key(state, "t").boxes, (Box("at" + PAD),))


class CycleColourTest(unittest.TestCase):
    def test_c_advances_the_selected_box_from_plain(self):
        state = State(boxes=(Box("a"),), selected=(0,))
        self.assertEqual(
            handle_key(state, "c").boxes, (Box("a", colour=next_colour(PLAIN)),)
        )

    def test_c_advances_the_selected_box_through_the_cycle(self):
        state = State(boxes=(Box("a"),), selected=(0,))
        for _ in range(PALETTE_SIZE):
            state = handle_key(state, "c")
        self.assertNotEqual(state.boxes[0].colour, PLAIN)
        state = handle_key(state, "c")
        self.assertEqual(state.boxes[0].colour, PLAIN)

    def test_c_changes_only_the_selected_box(self):
        state = State(boxes=(Box("a"), Box("b")), selected=(1,))
        self.assertEqual(
            handle_key(state, "c").boxes,
            (Box("a"), Box("b", colour=next_colour(PLAIN))),
        )

    def test_c_preserves_the_label(self):
        state = State(boxes=(Box("hi"),), selected=(0,))
        self.assertEqual(handle_key(state, "c").boxes[0].label, "hi")

    def test_c_on_an_empty_canvas_returns_the_state_unchanged(self):
        state = State()
        self.assertEqual(handle_key(state, "c"), state)

    def test_c_does_not_mutate_the_given_state(self):
        state = State(boxes=(Box("a"),), selected=(0,))
        handle_key(state, "c")
        self.assertEqual(state.boxes, (Box("a"),))

    def test_c_on_a_nested_box_changes_only_that_box(self):
        boxes = (Box("a", children=(Box("c"), Box("d"))),)
        state = State(boxes=boxes, selected=(0, 1))
        self.assertEqual(
            handle_key(state, "c").boxes,
            (Box("a", children=(Box("c"), Box("d", colour=next_colour(PLAIN)))),),
        )

    def test_c_does_not_change_fill(self):
        state = State(boxes=(Box("a", colour=next_colour(PLAIN)),), selected=(0,))
        self.assertEqual(handle_key(state, "c").boxes[0].fill, PLAIN)


class CycleFillTest(unittest.TestCase):
    def test_f_advances_the_selected_box_from_plain(self):
        state = State(boxes=(Box("a"),), selected=(0,))
        self.assertEqual(
            handle_key(state, "f").boxes, (Box("a", fill=next_colour(PLAIN)),)
        )

    def test_f_advances_the_selected_box_through_the_cycle(self):
        state = State(boxes=(Box("a"),), selected=(0,))
        for _ in range(PALETTE_SIZE):
            state = handle_key(state, "f")
        self.assertNotEqual(state.boxes[0].fill, PLAIN)
        state = handle_key(state, "f")
        self.assertEqual(state.boxes[0].fill, PLAIN)

    def test_f_changes_only_the_selected_box(self):
        state = State(boxes=(Box("a"), Box("b")), selected=(1,))
        self.assertEqual(
            handle_key(state, "f").boxes,
            (Box("a"), Box("b", fill=next_colour(PLAIN))),
        )

    def test_f_preserves_the_label(self):
        state = State(boxes=(Box("hi"),), selected=(0,))
        self.assertEqual(handle_key(state, "f").boxes[0].label, "hi")

    def test_f_on_an_empty_canvas_returns_the_state_unchanged(self):
        state = State()
        self.assertEqual(handle_key(state, "f"), state)

    def test_f_does_not_mutate_the_given_state(self):
        state = State(boxes=(Box("a"),), selected=(0,))
        handle_key(state, "f")
        self.assertEqual(state.boxes, (Box("a"),))

    def test_f_does_not_change_colour(self):
        state = State(boxes=(Box("a", fill=next_colour(PLAIN)),), selected=(0,))
        self.assertEqual(handle_key(state, "f").boxes[0].colour, PLAIN)

    def test_colour_and_fill_are_independent(self):
        state = State(boxes=(Box("a"),), selected=(0,))
        state = handle_key(state, "c")
        state = handle_key(state, "f")
        self.assertEqual(
            state.boxes,
            (Box("a", colour=next_colour(PLAIN), fill=next_colour(PLAIN)),),
        )


class CycleBorderTest(unittest.TestCase):
    def test_new_box_starts_with_a_thin_border(self):
        self.assertEqual(Box("a").border, 1)

    def test_t_cycles_the_selected_box_through_all_four_levels(self):
        state = State(boxes=(Box("a"),), selected=(0,))
        state = handle_key(state, "t")
        self.assertEqual(state.boxes, (Box("a", border=2),))
        state = handle_key(state, "t")
        self.assertEqual(state.boxes, (Box("a", border=3),))
        state = handle_key(state, "t")
        self.assertEqual(state.boxes, (Box("a", border=4),))
        state = handle_key(state, "t")
        self.assertEqual(state.boxes, (Box("a", border=1),))

    def test_t_changes_only_the_selected_box(self):
        state = State(boxes=(Box("a"), Box("b")), selected=(1,))
        self.assertEqual(
            handle_key(state, "t").boxes,
            (Box("a"), Box("b", border=2)),
        )

    def test_t_on_an_empty_canvas_returns_the_state_unchanged(self):
        state = State()
        self.assertEqual(handle_key(state, "t"), state)

    def test_t_does_not_mutate_the_given_state(self):
        state = State(boxes=(Box("a"),), selected=(0,))
        handle_key(state, "t")
        self.assertEqual(state.boxes, (Box("a"),))

    def test_t_on_a_nested_box_changes_only_that_box(self):
        boxes = (Box("a", children=(Box("c"), Box("d"))),)
        state = State(boxes=boxes, selected=(0, 1))
        self.assertEqual(
            handle_key(state, "t").boxes,
            (Box("a", children=(Box("c"), Box("d", border=2))),),
        )

    def test_t_does_not_change_colour_or_fill(self):
        state = State(
            boxes=(Box("a", colour=next_colour(PLAIN), fill=next_colour(PLAIN)),),
            selected=(0,),
        )
        result = handle_key(state, "t").boxes[0]
        self.assertEqual(result.colour, next_colour(PLAIN))
        self.assertEqual(result.fill, next_colour(PLAIN))


class ColourRowTest(unittest.TestCase):
    def test_colour_row_advances_uniformly_coloured_siblings(self):
        boxes = (Box("a"), Box("b"))
        self.assertEqual(
            colour_row(boxes, (0,)),
            (Box("a", colour=next_colour(PLAIN)), Box("b", colour=next_colour(PLAIN))),
        )

    def test_colour_row_sets_mixed_siblings_to_the_first_palette_colour(self):
        boxes = (Box("a", colour=0), Box("b", colour=1))
        self.assertEqual(
            colour_row(boxes, (0,)),
            (Box("a", colour=0), Box("b", colour=0)),
        )

    def test_colour_row_leaves_other_top_level_boxes_alone(self):
        boxes = (Box("a", children=(Box("c"), Box("d"))), Box("e"))
        result = colour_row(boxes, (0, 0))
        self.assertEqual(result[1], Box("e"))

    def test_colour_row_leaves_the_parent_unchanged(self):
        boxes = (Box("a", children=(Box("c"), Box("d"))),)
        result = colour_row(boxes, (0, 0))
        self.assertEqual(result[0].label, "a")

    def test_colour_row_only_affects_the_given_siblings_group(self):
        boxes = (
            Box("a", children=(Box("c"), Box("d"))),
            Box("e", children=(Box("f"),)),
        )
        result = colour_row(boxes, (0, 0))
        self.assertEqual(result[1], Box("e", children=(Box("f"),)))

    def test_colour_row_affects_nested_siblings(self):
        boxes = (Box("a", children=(Box("c"), Box("d"))),)
        self.assertEqual(
            colour_row(boxes, (0, 0)),
            (
                Box(
                    "a",
                    children=(
                        Box("c", colour=next_colour(PLAIN)),
                        Box("d", colour=next_colour(PLAIN)),
                    ),
                ),
            ),
        )

    def test_colour_row_does_not_mutate_the_given_boxes(self):
        boxes = (Box("a"), Box("b"))
        colour_row(boxes, (0,))
        self.assertEqual(boxes, (Box("a"), Box("b")))

    def test_colour_row_cycles_back_to_plain(self):
        boxes = (Box("a"), Box("b"))
        for _ in range(PALETTE_SIZE):
            boxes = colour_row(boxes, (0,))
        self.assertNotEqual(boxes[0].colour, PLAIN)
        boxes = colour_row(boxes, (0,))
        self.assertEqual(boxes[0].colour, PLAIN)
        self.assertEqual(boxes[1].colour, PLAIN)


class ColourRowKeyTest(unittest.TestCase):
    def test_capital_c_on_a_top_level_box_does_nothing(self):
        state = State(boxes=(Box("a"), Box("b")), selected=(0,))
        self.assertEqual(handle_key(state, "C"), state)

    def test_capital_c_with_nothing_selected_does_nothing(self):
        state = State(boxes=(Box("a"),), selected=())
        self.assertEqual(handle_key(state, "C"), state)

    def test_capital_c_advances_uniformly_coloured_siblings(self):
        boxes = (Box("a", children=(Box("c"), Box("d"))),)
        state = State(boxes=boxes, selected=(0, 1))
        self.assertEqual(
            handle_key(state, "C").boxes,
            (
                Box(
                    "a",
                    children=(
                        Box("c", colour=next_colour(PLAIN)),
                        Box("d", colour=next_colour(PLAIN)),
                    ),
                ),
            ),
        )

    def test_capital_c_sets_mixed_siblings_to_the_first_palette_colour(self):
        boxes = (Box("a", children=(Box("c", colour=0), Box("d", colour=1))),)
        state = State(boxes=boxes, selected=(0, 1))
        self.assertEqual(
            handle_key(state, "C").boxes,
            (Box("a", children=(Box("c", colour=0), Box("d", colour=0))),),
        )

    def test_capital_c_cycles_back_to_plain(self):
        boxes = (Box("a", children=(Box("c"), Box("d"))),)
        state = State(boxes=boxes, selected=(0, 1))
        for _ in range(PALETTE_SIZE):
            state = handle_key(state, "C")
        self.assertNotEqual(state.boxes[0].children[0].colour, PLAIN)
        state = handle_key(state, "C")
        self.assertEqual(state.boxes[0].children[0].colour, PLAIN)
        self.assertEqual(state.boxes[0].children[1].colour, PLAIN)

    def test_capital_c_leaves_unrelated_boxes_alone(self):
        boxes = (
            Box("a", children=(Box("c"), Box("d"))),
            Box("e", children=(Box("f"),)),
        )
        state = State(boxes=boxes, selected=(0, 0))
        result = handle_key(state, "C").boxes
        self.assertEqual(result[1], Box("e", children=(Box("f"),)))

    def test_capital_c_does_not_mutate_the_given_state(self):
        boxes = (Box("a", children=(Box("c"), Box("d"))),)
        state = State(boxes=boxes, selected=(0, 0))
        handle_key(state, "C")
        self.assertEqual(
            state.boxes, (Box("a", children=(Box("c"), Box("d"))),)
        )


if __name__ == "__main__":
    unittest.main()
