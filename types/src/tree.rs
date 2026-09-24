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

    pub fn push(&mut self, parent: &[usize], child: Tree<T>) -> Vec<usize> {
        let children = &mut self.get_mut(parent).children;
        children.push(child);
        [parent, &[children.len() - 1]].concat()
    }

    pub fn remove(&mut self, path: &[usize]) -> Tree<T> {
        let (&last, ancestors) = path.split_last().expect("the root cannot be removed");
        self.get_mut(ancestors).children.remove(last)
    }

    pub fn value(&self, path: &[usize]) -> &T {
        assert!(!path.is_empty(), "the root has no value to read");
        &self.get(path).value
    }

    pub fn value_mut(&mut self, path: &[usize]) -> &mut T {
        assert!(!path.is_empty(), "the root has no value to edit");
        &mut self.get_mut(path).value
    }

    pub fn walk(&self) -> impl Iterator<Item = (Vec<usize>, &T)> {
        let mut visited = Vec::new();
        self.collect_descendants(&mut Vec::new(), &mut visited);
        visited.into_iter()
    }

    fn collect_descendants<'a>(
        &'a self,
        path: &mut Vec<usize>,
        visited: &mut Vec<(Vec<usize>, &'a T)>,
    ) {
        for (index, child) in self.children.iter().enumerate() {
            path.push(index);
            visited.push((path.clone(), &child.value));
            child.collect_descendants(path, visited);
            path.pop();
        }
    }

    pub fn map<U: Default>(&self, f: impl Fn(&T) -> U) -> Tree<U> {
        Tree::root(
            self.children
                .iter()
                .map(|child| child.map_box(&f))
                .collect(),
        )
    }

    fn map_box<U>(&self, f: &impl Fn(&T) -> U) -> Tree<U> {
        Tree::new(
            f(&self.value),
            self.children.iter().map(|child| child.map_box(f)).collect(),
        )
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

    #[test]
    fn push_adds_the_child_last_and_leaves_the_others_alone() {
        let mut tree = sample();
        tree.push(&[0], Tree::leaf("a2"));
        assert_eq!(tree.get(&[0]).children.len(), 3);
        assert_eq!(tree.get(&[0, 0]).value, "a0");
        assert_eq!(tree.get(&[0, 1]).value, "a1");
        assert_eq!(tree.get(&[0, 2]).value, "a2");
        assert_eq!(tree.get(&[1]).value, "b");
    }

    #[test]
    fn push_returns_the_path_of_the_new_child() {
        let mut tree = sample();
        let path = tree.push(&[0], Tree::leaf("a2"));
        assert_eq!(path, vec![0, 2]);
        assert_eq!(tree.get(&path).value, "a2");
    }

    #[test]
    fn push_under_the_root_path_adds_a_top_level_box() {
        let mut tree = sample();
        let path = tree.push(&[], Tree::leaf("c"));
        assert_eq!(path, vec![2]);
        assert_eq!(tree.get(&[2]).value, "c");
        assert_eq!(tree.get(&[]).children.len(), 3);
    }

    #[test]
    fn push_under_a_leaf_gives_it_its_first_child() {
        let mut tree = sample();
        let path = tree.push(&[1], Tree::leaf("b0"));
        assert_eq!(path, vec![1, 0]);
        assert_eq!(tree.get(&[1, 0]).value, "b0");
        assert_eq!(tree.child(&[1]), vec![1, 0]);
    }

    #[test]
    fn push_keeps_the_pushed_subtrees_own_children() {
        let mut tree = sample();
        let path = tree.push(&[1], Tree::new("c", vec![Tree::leaf("c0")]));
        assert_eq!(tree.get(&path).value, "c");
        assert_eq!(tree.get(&[1, 0, 0]).value, "c0");
    }

    #[test]
    #[should_panic]
    fn push_panics_on_a_parent_path_that_addresses_no_box() {
        sample().push(&[0, 2], Tree::leaf("x"));
    }

    #[test]
    #[should_panic]
    fn push_panics_on_a_parent_path_through_a_box_with_too_few_children() {
        sample().push(&[0, 2, 0], Tree::leaf("x"));
    }

    #[test]
    fn remove_returns_the_removed_box_with_its_subtree() {
        let removed = sample().remove(&[0]);
        assert_eq!(removed.value, "a");
        assert_eq!(removed.children.len(), 2);
        assert_eq!(removed.get(&[1]).value, "a1");
    }

    #[test]
    fn remove_takes_the_box_out_and_later_siblings_move_down() {
        let mut tree = Tree::root(vec![Tree::leaf("a"), Tree::leaf("b"), Tree::leaf("c")]);
        tree.remove(&[1]);
        assert_eq!(tree.get(&[]).children.len(), 2);
        assert_eq!(tree.get(&[0]).value, "a");
        assert_eq!(tree.get(&[1]).value, "c");
    }

    #[test]
    fn remove_at_any_depth_leaves_the_other_branches_alone() {
        let mut tree = sample();
        let removed = tree.remove(&[0, 0]);
        assert_eq!(removed.value, "a0");
        assert_eq!(tree.get(&[0]).children.len(), 1);
        assert_eq!(tree.get(&[0, 0]).value, "a1");
        assert_eq!(tree.get(&[1]).value, "b");
    }

    #[test]
    fn remove_of_the_only_child_leaves_its_parent_without_children() {
        let mut tree = Tree::root(vec![Tree::new("a", vec![Tree::leaf("a0")])]);
        tree.remove(&[0, 0]);
        assert!(tree.get(&[0]).children.is_empty());
        assert_eq!(tree.child(&[0]), vec![0]);
    }

    #[test]
    #[should_panic]
    fn remove_panics_on_the_root_path() {
        sample().remove(&[]);
    }

    #[test]
    #[should_panic]
    fn remove_panics_on_an_index_past_the_last_sibling() {
        sample().remove(&[2]);
    }

    #[test]
    #[should_panic]
    fn remove_panics_on_a_path_through_a_box_with_too_few_children() {
        sample().remove(&[0, 2, 0]);
    }

    #[test]
    fn value_of_a_top_level_box() {
        assert_eq!(*sample().value(&[1]), "b");
    }

    #[test]
    fn value_of_a_nested_box() {
        assert_eq!(*sample().value(&[0, 1]), "a1");
    }

    #[test]
    fn value_of_a_box_with_children_is_its_own() {
        assert_eq!(*sample().value(&[0]), "a");
    }

    #[test]
    fn value_mut_replaces_the_value_and_leaves_every_other_box_alone() {
        let mut tree = sample();
        *tree.value_mut(&[0, 1]) = "changed";
        assert_eq!(*tree.value(&[0, 1]), "changed");
        assert_eq!(*tree.value(&[0]), "a");
        assert_eq!(*tree.value(&[0, 0]), "a0");
        assert_eq!(*tree.value(&[1]), "b");
        assert_eq!(tree.get(&[]).value, "");
    }

    #[test]
    fn value_mut_edits_the_value_in_place() {
        let mut tree = Tree::root(vec![Tree::leaf(String::from("a"))]);
        tree.value_mut(&[0]).push_str("bc");
        assert_eq!(tree.value(&[0]), "abc");
    }

    #[test]
    #[should_panic]
    fn value_panics_on_the_root_path() {
        sample().value(&[]);
    }

    #[test]
    #[should_panic]
    fn value_panics_on_an_index_past_the_last_sibling() {
        sample().value(&[2]);
    }

    #[test]
    #[should_panic]
    fn value_panics_on_a_path_through_a_box_with_too_few_children() {
        sample().value(&[0, 2, 0]);
    }

    #[test]
    #[should_panic]
    fn value_mut_panics_on_the_root_path() {
        sample().value_mut(&[]);
    }

    #[test]
    #[should_panic]
    fn value_mut_panics_on_an_index_past_the_last_sibling() {
        sample().value_mut(&[2]);
    }

    #[test]
    #[should_panic]
    fn value_mut_panics_on_a_path_through_a_box_with_too_few_children() {
        sample().value_mut(&[0, 2, 0]);
    }

    #[test]
    fn walk_yields_every_box_in_pre_order() {
        let tree = sample();
        let visited: Vec<(Vec<usize>, &str)> =
            tree.walk().map(|(path, &value)| (path, value)).collect();
        assert_eq!(
            visited,
            vec![
                (vec![0], "a"),
                (vec![0, 0], "a0"),
                (vec![0, 1], "a1"),
                (vec![1], "b"),
            ]
        );
    }

    #[test]
    fn walk_never_yields_the_root_path() {
        assert!(sample().walk().all(|(path, _)| !path.is_empty()));
    }

    #[test]
    fn walk_yields_nothing_for_a_root_without_children() {
        assert_eq!(Tree::<&str>::root(Vec::new()).walk().count(), 0);
    }

    #[test]
    fn walk_reaches_a_deep_chain_with_the_full_path_for_each_box() {
        let tree = Tree::root(vec![Tree::new(
            "a",
            vec![Tree::new("a0", vec![Tree::leaf("a00")])],
        )]);
        let paths: Vec<Vec<usize>> = tree.walk().map(|(path, _)| path).collect();
        assert_eq!(paths, vec![vec![0], vec![0, 0], vec![0, 0, 0]]);
    }

    #[test]
    fn map_keeps_the_shape() {
        let tree = sample();
        let mapped = tree.map(|value| value.len());
        let paths: Vec<Vec<usize>> = tree.walk().map(|(path, _)| path).collect();
        let mapped_paths: Vec<Vec<usize>> = mapped.walk().map(|(path, _)| path).collect();
        assert_eq!(mapped_paths, paths);
        for path in paths.iter().chain(&[vec![]]) {
            assert_eq!(
                mapped.get(path).children.len(),
                tree.get(path).children.len()
            );
        }
    }

    #[test]
    fn map_applies_the_function_to_every_value() {
        let mapped = sample().map(|value| value.len());
        let visited: Vec<(Vec<usize>, usize)> =
            mapped.walk().map(|(path, &value)| (path, value)).collect();
        assert_eq!(
            visited,
            vec![(vec![0], 1), (vec![0, 0], 2), (vec![0, 1], 2), (vec![1], 1),]
        );
    }

    #[test]
    fn map_makes_the_root_default_without_showing_it_to_the_function() {
        let mapped = sample().map(|value| {
            assert!(!value.is_empty(), "the function saw the root");
            value.len() + 10
        });
        assert_eq!(mapped.get(&[]).value, 0);
    }

    #[test]
    fn map_of_an_empty_root_is_an_empty_root() {
        let mapped = Tree::<&str>::root(Vec::new()).map(|value| value.len());
        assert_eq!(mapped.walk().count(), 0);
        assert_eq!(mapped.get(&[]).value, 0);
    }
}
