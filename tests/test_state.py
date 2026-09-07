import unittest

from sketch.state import Box, State, handle_key


class BoxTest(unittest.TestCase):
    def test_boxes_are_equal(self):
        self.assertEqual(Box(), Box())


class HandleKeyTest(unittest.TestCase):
    def test_b_appends_a_box(self):
        self.assertEqual(handle_key(State([]), "b").nodes, [Box()])

    def test_b_appends_to_existing_nodes(self):
        self.assertEqual(handle_key(State([Box()]), "b").nodes, [Box(), Box()])

    def test_b_does_not_mutate_the_given_state(self):
        state = State([])
        handle_key(state, "b")
        self.assertEqual(state.nodes, [])

    def test_unknown_key_returns_the_state_unchanged(self):
        state = State([Box()])
        self.assertEqual(handle_key(state, "x"), state)


if __name__ == "__main__":
    unittest.main()
