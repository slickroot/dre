pub struct Tree<T> {
    value: T,
    children: Vec<Tree<T>>,
}

impl<T> Tree<T> {
    pub fn leaf(value: T) -> Self {
        Self::new(value, Vec::new())
    }

    pub fn new(value: T, children: Vec<Tree<T>>) -> Self {
        Self { value, children }
    }

    pub fn root(children: Vec<Tree<T>>) -> Self
    where
        T: Default,
    {
        Self::new(T::default(), children)
    }

    pub fn child(&self, path: &[usize]) -> Vec<usize> {
        assert!(!path.is_empty(), "the root has no child");
        if self.get(path).children.is_empty() {
            path.to_vec()
        } else {
            [path, &[0]].concat()
        }
    }

    pub fn parent(&self, path: &[usize]) -> Vec<usize> {
        self.position(path);
        match path {
            [_] => path.to_vec(),
            _ => path[..path.len() - 1].to_vec(),
        }
    }

    pub fn next(&self, path: &[usize]) -> Vec<usize> {
        let (last, count) = self.position(path);
        if last + 1 < count {
            [&path[..path.len() - 1], &[last + 1]].concat()
        } else {
            path.to_vec()
        }
    }

    pub fn previous(&self, path: &[usize]) -> Vec<usize> {
        let (last, _) = self.position(path);
        if last > 0 {
            [&path[..path.len() - 1], &[last - 1]].concat()
        } else {
            path.to_vec()
        }
    }

    fn position(&self, path: &[usize]) -> (usize, usize) {
        let (&last, ancestors) = path.split_last().expect("the root has no siblings");
        let count = self.get(ancestors).children.len();
        assert!(last < count, "no box at the path");
        (last, count)
    }

    fn get(&self, path: &[usize]) -> &Tree<T> {
        path.iter().fold(self, |tree, &index| &tree.children[index])
    }

    fn get_mut(&mut self, path: &[usize]) -> &mut Tree<T> {
        path.iter()
            .fold(self, |tree, &index| &mut tree.children[index])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Tree<&'static str> {
        Tree::root(vec![
            Tree::new("a", vec![Tree::leaf("a0"), Tree::leaf("a1")]),
            Tree::leaf("b"),
        ])
    }

    #[test]
    fn parent_of_a_nested_path_drops_the_last_step() {
        let tree = sample();
        let parent = tree.parent(&[0, 1]);
        assert_eq!(parent, vec![0]);
        assert_eq!(tree.get(&parent).value, "a");
    }

    #[test]
    fn parent_of_a_top_level_path_is_the_same_path() {
        assert_eq!(sample().parent(&[1]), vec![1]);
    }

    #[test]
    #[should_panic]
    fn parent_panics_on_the_root_path() {
        sample().parent(&[]);
    }

    #[test]
    #[should_panic]
    fn parent_panics_on_a_path_that_addresses_no_box() {
        sample().parent(&[0, 2]);
    }

    #[test]
    fn get_returns_the_box_at_a_path() {
        let tree = sample();
        assert_eq!(tree.get(&[0]).value, "a");
        assert_eq!(tree.get(&[0, 1]).value, "a1");
    }

    #[test]
    fn get_returns_the_root_at_the_empty_path() {
        let tree = sample();
        assert_eq!(tree.get(&[]).value, "");
        assert_eq!(tree.get(&[]).children.len(), 2);
    }

    #[test]
    #[should_panic]
    fn get_panics_on_a_path_that_addresses_no_box() {
        sample().get(&[0, 2]);
    }

    #[test]
    fn get_mut_changes_only_the_box_at_the_path() {
        let mut tree = sample();
        tree.get_mut(&[0, 1]).value = "changed";
        assert_eq!(tree.get(&[0, 1]).value, "changed");
        assert_eq!(tree.get(&[0]).value, "a");
        assert_eq!(tree.get(&[0, 0]).value, "a0");
        assert_eq!(tree.get(&[1]).value, "b");
        assert_eq!(tree.get(&[]).value, "");
    }

    #[test]
    fn child_of_a_box_with_children_is_its_first_child() {
        assert_eq!(sample().child(&[0]), vec![0, 0]);
    }

    #[test]
    fn child_of_a_childless_box_is_the_same_path() {
        assert_eq!(sample().child(&[1]), vec![1]);
        assert_eq!(sample().child(&[0, 1]), vec![0, 1]);
    }

    #[test]
    #[should_panic]
    fn child_panics_on_the_root_path() {
        sample().child(&[]);
    }

    #[test]
    #[should_panic]
    fn child_panics_on_an_index_past_the_last_sibling() {
        sample().child(&[2]);
    }

    #[test]
    #[should_panic]
    fn child_panics_on_a_path_through_a_box_with_too_few_children() {
        sample().child(&[0, 2, 0]);
    }

    #[test]
    fn next_of_a_box_with_a_next_sibling_is_that_sibling() {
        assert_eq!(sample().next(&[0]), vec![1]);
        assert_eq!(sample().next(&[0, 0]), vec![0, 1]);
    }

    #[test]
    fn next_of_the_last_sibling_is_the_same_path() {
        assert_eq!(sample().next(&[1]), vec![1]);
        assert_eq!(sample().next(&[0, 1]), vec![0, 1]);
    }

    #[test]
    #[should_panic]
    fn next_panics_on_the_root_path() {
        sample().next(&[]);
    }

    #[test]
    #[should_panic]
    fn next_panics_on_an_index_past_the_last_sibling() {
        sample().next(&[2]);
    }

    #[test]
    #[should_panic]
    fn next_panics_on_a_path_through_a_box_with_too_few_children() {
        sample().next(&[0, 2, 0]);
    }

    #[test]
    fn previous_of_a_box_with_a_previous_sibling_is_that_sibling() {
        assert_eq!(sample().previous(&[1]), vec![0]);
        assert_eq!(sample().previous(&[0, 1]), vec![0, 0]);
    }

    #[test]
    fn previous_of_the_first_sibling_is_the_same_path() {
        assert_eq!(sample().previous(&[0]), vec![0]);
        assert_eq!(sample().previous(&[0, 0]), vec![0, 0]);
    }

    #[test]
    #[should_panic]
    fn previous_panics_on_the_root_path() {
        sample().previous(&[]);
    }

    #[test]
    #[should_panic]
    fn previous_panics_on_an_index_past_the_last_sibling() {
        sample().previous(&[2]);
    }

    #[test]
    #[should_panic]
    fn previous_panics_on_a_path_through_a_box_with_too_few_children() {
        sample().previous(&[0, 2, 0]);
    }
}
