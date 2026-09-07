import unittest

from sketch.state import Box, State, handle_key


class BoxTest(unittest.TestCase):
    def test_boxes_are_equal(self):
        self.assertEqual(Box(), Box())


class StateTest(unittest.TestCase):
    def test_state_starts_running(self):
        self.assertIs(State([]).running, True)


class HandleKeyTest(unittest.TestCase):
    def test_b_appends_a_box(self):
        self.assertEqual(handle_key(State([]), "b").nodes, [Box()])

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


if __name__ == "__main__":
    unittest.main()
