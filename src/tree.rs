pub struct Tree<T> {
    value: T,
    children: Vec<Tree<T>>,
}

pub fn parent(path: &[usize]) -> &[usize] {
    match path {
        [] => panic!("the root has no parent"),
        [_] => path,
        [ancestors @ .., _] => ancestors,
    }
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
        assert_eq!(parent(&[1, 2, 3]), &[1, 2]);
    }

    #[test]
    fn parent_of_a_top_level_path_is_the_same_path() {
        assert_eq!(parent(&[1]), &[1]);
    }

    #[test]
    #[should_panic]
    fn parent_panics_on_the_root_path() {
        parent(&[]);
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
}
