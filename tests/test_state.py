import unittest

from sketch.state import Box, State, handle_key


class BoxTest(unittest.TestCase):
    def test_boxes_are_equal(self):
        self.assertEqual(Box(), Box())


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


class HandleKeyTest(unittest.TestCase):
    def test_b_appends_a_box(self):
        self.assertEqual(handle_key(State([]), "b").nodes, [Box()])

    def test_b_appends_a_box_with_an_empty_label(self):
        self.assertEqual(handle_key(State([]), "b").nodes, [Box("")])

    def test_b_enters_insert_mode(self):
        self.assertEqual(handle_key(State([]), "b").mode, "insert")

    def test_b_does_not_mutate_the_mode_of_the_given_state(self):
        state = State([])
        handle_key(state, "b")
        self.assertEqual(state.mode, "command")

    def test_q_keeps_command_mode(self):
        self.assertEqual(handle_key(State([]), "q").mode, "command")

    def test_insert_mode_is_dispatched_separately(self):
        state = State([Box("")], mode="insert")
        self.assertIs(handle_key(state, "q").running, True)

    def test_b_appends_to_existing_nodes(self):
        self.assertEqual(handle_key(State([Box()]), "b").nodes, [Box(), Box()])

    def test_b_does_not_mutate_the_given_state(self):
        state = State([])
        handle_key(state, "b")
        self.assertEqual(state.nodes, [])

    def test_b_keeps_the_state_running(self):
        self.assertIs(handle_key(State([]), "b").running, True)

    def test_b_preserves_a_stopped_state(self):
        state = handle_key(State([]), "q")
        self.assertIs(handle_key(state, "b").running, False)

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

    def test_i_enters_insert_mode(self):
        self.assertEqual(handle_key(State([Box("hi")]), "i").mode, "insert")

    def test_i_preserves_the_nodes(self):
        state = State([Box("a"), Box("b")])
        self.assertEqual(handle_key(state, "i").nodes, [Box("a"), Box("b")])

    def test_i_keeps_the_state_running(self):
        self.assertIs(handle_key(State([Box("hi")]), "i").running, True)

    def test_i_does_not_mutate_the_given_state(self):
        state = State([Box("hi")])
        handle_key(state, "i")
        self.assertEqual(state.mode, "command")

    def test_i_on_an_empty_canvas_leaves_the_mode_as_command(self):
        self.assertEqual(handle_key(State([]), "i").mode, "command")

    def test_i_on_an_empty_canvas_returns_the_state_unchanged(self):
        state = State([])
        self.assertEqual(handle_key(state, "i"), state)


class HandleInsertTest(unittest.TestCase):
    def test_a_printable_character_appends_to_the_last_box_label(self):
        state = handle_key(State([]), "b")
        self.assertEqual(handle_key(state, "h").nodes, [Box("h")])

    def test_letters_bound_in_command_mode_are_ordinary_here(self):
        state = State([Box("")], mode="insert")
        self.assertEqual(handle_key(state, "b").nodes, [Box("b")])

    def test_q_does_not_stop_the_state(self):
        state = State([Box("")], mode="insert")
        self.assertIs(handle_key(state, "q").running, True)

    def test_typing_stays_in_insert_mode(self):
        state = State([Box("")], mode="insert")
        self.assertEqual(handle_key(state, "h").mode, "insert")

    def test_typing_appends_to_an_existing_label(self):
        state = State([Box("h")], mode="insert")
        self.assertEqual(handle_key(state, "i").nodes, [Box("hi")])

    def test_typing_edits_only_the_last_box(self):
        state = State([Box("a"), Box("b")], mode="insert")
        self.assertEqual(handle_key(state, "c").nodes, [Box("a"), Box("bc")])

    def test_typing_does_not_mutate_the_given_state(self):
        state = State([Box("h")], mode="insert")
        handle_key(state, "i")
        self.assertEqual(state.nodes, [Box("h")])

    def test_space_is_printable(self):
        state = State([Box("a")], mode="insert")
        self.assertEqual(handle_key(state, " ").nodes, [Box("a ")])

    def test_tilde_is_printable(self):
        state = State([Box("")], mode="insert")
        self.assertEqual(handle_key(state, "~").nodes, [Box("~")])

    def test_backspace_drops_the_last_character(self):
        state = State([Box("hi")], mode="insert")
        self.assertEqual(handle_key(state, "\x7f").nodes, [Box("h")])

    def test_backspace_on_an_empty_label_is_a_no_op(self):
        state = State([Box("")], mode="insert")
        self.assertEqual(handle_key(state, "\x7f"), state)

    def test_backspace_does_not_mutate_the_given_state(self):
        state = State([Box("hi")], mode="insert")
        handle_key(state, "\x7f")
        self.assertEqual(state.nodes, [Box("hi")])

    def test_esc_returns_to_command_mode(self):
        state = State([Box("")], mode="insert")
        self.assertEqual(handle_key(state, "\x1b").mode, "command")

    def test_esc_preserves_the_nodes(self):
        state = State([Box("hi")], mode="insert")
        self.assertEqual(handle_key(state, "\x1b").nodes, [Box("hi")])

    def test_esc_does_not_mutate_the_given_state(self):
        state = State([Box("")], mode="insert")
        handle_key(state, "\x1b")
        self.assertEqual(state.mode, "insert")

    def test_a_control_character_returns_the_state_unchanged(self):
        state = State([Box("hi")], mode="insert")
        self.assertEqual(handle_key(state, "\x01"), state)

    def test_a_non_ascii_character_returns_the_state_unchanged(self):
        state = State([Box("hi")], mode="insert")
        self.assertEqual(handle_key(state, "\u00e9"), state)


if __name__ == "__main__":
    unittest.main()
