//! re: <https://exercism.org/tracks/rust/exercises/pov>

use std::fmt::Debug;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Tree<T: Debug + Ord> {
    remove_this: std::marker::PhantomData<T>,
}

impl<T: Debug + Ord> Tree<T> {
    pub fn new(label: T) -> Self {
        todo!("Create a new tree with the given {label:?}");
    }

    /// Builder-method for constructing a tree with children
    pub fn with_child(self, child: Self) -> Self {
        todo!("Add {child:?} to the tree and return the tree");
    }

    pub fn pov_from(&mut self, from: &T) -> bool {
        todo!("Reparent the tree with the label {from:?} as root");
    }

    pub fn path_between<'a>(&'a mut self, from: &'a T, to: &'a T) -> Option<Vec<&'a T>> {
        todo!("Return the shortest path between {from:?} and {to:?}");
    }
}

#[cfg(test)]
mod tests {
    extern crate test;
    use super as pov;
    use test::Bencher;

    /// Forbid implementations that rely on Clone.
    mod no_clone {
        use super::pov::*;
        #[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
        struct NotClone;

        #[test]
        fn doesnt_rely_on_clone() {
            let mut tree = Tree::new(NotClone);
            assert!(tree.pov_from(&NotClone));
        }
    }
    /// Equality on trees must be independent of the order of children.
    mod equality {
        use super::pov::*;
        // use pretty_assertions::assert_eq;

        #[test]
        fn equality_is_order_independent() {
            let a = Tree::new("root")
                .with_child(Tree::new("left"))
                .with_child(Tree::new("right"));
            let b = Tree::new("root")
                .with_child(Tree::new("right"))
                .with_child(Tree::new("left"));
            assert_eq!(a, b);
        }
    }
    /// Reroot a tree so that its root is the specified node.
    mod from_pov {
        use super::Bencher;
        use super::pov::*;
        // use pretty_assertions::assert_eq;

        #[test]
        fn results_in_the_same_tree_if_the_input_tree_is_a_singleton() {
            let mut tree = Tree::new("x");
            assert!(tree.pov_from(&"x"));
            let expected = Tree::new("x");
            assert_eq!(tree, expected);
        }

        #[test]
        fn can_reroot_a_tree_with_a_parent_and_one_sibling() {
            let mut tree = Tree::new("parent")
                .with_child(Tree::new("x"))
                .with_child(Tree::new("sibling"));
            assert!(tree.pov_from(&"x"));
            let expected =
                Tree::new("x").with_child(Tree::new("parent").with_child(Tree::new("sibling")));
            assert_eq!(tree, expected);
        }

        #[test]
        fn can_reroot_a_tree_with_a_parent_and_many_siblings() {
            let mut tree = Tree::new("parent")
                .with_child(Tree::new("a"))
                .with_child(Tree::new("x"))
                .with_child(Tree::new("b"))
                .with_child(Tree::new("c"));
            assert!(tree.pov_from(&"x"));
            let expected = Tree::new("x").with_child(
                Tree::new("parent")
                    .with_child(Tree::new("a"))
                    .with_child(Tree::new("b"))
                    .with_child(Tree::new("c")),
            );
            assert_eq!(tree, expected);
        }

        #[test]
        fn can_reroot_a_tree_with_new_root_deeply_nested_in_tree() {
            let mut tree = Tree::new("level-0").with_child(Tree::new("level-1").with_child(
                Tree::new("level-2").with_child(Tree::new("level-3").with_child(Tree::new("x"))),
            ));
            assert!(tree.pov_from(&"x"));
            let expected = Tree::new("x").with_child(
                Tree::new("level-3").with_child(
                    Tree::new("level-2")
                        .with_child(Tree::new("level-1").with_child(Tree::new("level-0"))),
                ),
            );
            assert_eq!(tree, expected);
        }

        #[test]
        fn moves_children_of_the_new_root_to_same_level_as_former_parent() {
            let mut tree = Tree::new("parent").with_child(
                Tree::new("x")
                    .with_child(Tree::new("kid-0"))
                    .with_child(Tree::new("kid-1")),
            );
            assert!(tree.pov_from(&"x"));
            let expected = Tree::new("x")
                .with_child(Tree::new("kid-0"))
                .with_child(Tree::new("kid-1"))
                .with_child(Tree::new("parent"));
            assert_eq!(tree, expected);
        }

        #[test]
        fn can_reroot_a_complex_tree_with_cousins() {
            let mut tree = Tree::new("grandparent")
                .with_child(
                    Tree::new("parent")
                        .with_child(
                            Tree::new("x")
                                .with_child(Tree::new("kid-0"))
                                .with_child(Tree::new("kid-1")),
                        )
                        .with_child(Tree::new("sibling-0"))
                        .with_child(Tree::new("sibling-1")),
                )
                .with_child(
                    Tree::new("uncle")
                        .with_child(Tree::new("cousin-0"))
                        .with_child(Tree::new("cousin-1")),
                );
            assert!(tree.pov_from(&"x"));
            let expected = Tree::new("x")
                .with_child(Tree::new("kid-1"))
                .with_child(Tree::new("kid-0"))
                .with_child(
                    Tree::new("parent")
                        .with_child(Tree::new("sibling-0"))
                        .with_child(Tree::new("sibling-1"))
                        .with_child(
                            Tree::new("grandparent").with_child(
                                Tree::new("uncle")
                                    .with_child(Tree::new("cousin-0"))
                                    .with_child(Tree::new("cousin-1")),
                            ),
                        ),
                );
            assert_eq!(tree, expected);
        }

        #[bench]
        fn bench_can_reroot_a_complex_tree_with_cousins(bench: &mut Bencher) {
            bench.iter(can_reroot_a_complex_tree_with_cousins)
        }

        #[test]
        fn errors_if_target_does_not_exist_in_a_singleton_tree() {
            let mut tree = Tree::new("x");
            assert!(!tree.pov_from(&"nonexistent"));
        }

        #[test]
        fn errors_if_target_does_not_exist_in_a_large_tree() {
            let mut tree = Tree::new("parent")
                .with_child(
                    Tree::new("x")
                        .with_child(Tree::new("kid-0"))
                        .with_child(Tree::new("kid-1")),
                )
                .with_child(Tree::new("sibling-0"))
                .with_child(Tree::new("sibling-1"));
            assert!(!tree.pov_from(&"nonexistent"));
        }
    }
    /// Given two nodes, find the path between them
    mod path_to {
        use super::pov::*;
        // use pretty_assertions::assert_eq;

        #[test]
        fn can_find_path_to_parent() {
            let mut tree = Tree::new("parent")
                .with_child(Tree::new("x"))
                .with_child(Tree::new("sibling"));
            let result = tree.path_between(&"x", &"parent");
            let expected = Some(vec![&"x", &"parent"]);
            assert_eq!(result, expected);
        }

        #[test]
        fn can_find_path_to_sibling() {
            let mut tree = Tree::new("parent")
                .with_child(Tree::new("a"))
                .with_child(Tree::new("x"))
                .with_child(Tree::new("b"))
                .with_child(Tree::new("c"));
            let result = tree.path_between(&"x", &"b");
            let expected = Some(vec![&"x", &"parent", &"b"]);
            assert_eq!(result, expected);
        }

        #[test]
        fn can_find_path_to_cousin() {
            let mut tree = Tree::new("grandparent")
                .with_child(
                    Tree::new("parent")
                        .with_child(
                            Tree::new("x")
                                .with_child(Tree::new("kid-0"))
                                .with_child(Tree::new("kid-1")),
                        )
                        .with_child(Tree::new("sibling-0"))
                        .with_child(Tree::new("sibling-1")),
                )
                .with_child(
                    Tree::new("uncle")
                        .with_child(Tree::new("cousin-0"))
                        .with_child(Tree::new("cousin-1")),
                );
            let result = tree.path_between(&"x", &"cousin-1");
            let expected = Some(vec![&"x", &"parent", &"grandparent", &"uncle", &"cousin-1"]);
            assert_eq!(result, expected);
        }

        #[test]
        fn can_find_path_not_involving_root() {
            let mut tree = Tree::new("grandparent").with_child(
                Tree::new("parent")
                    .with_child(Tree::new("x"))
                    .with_child(Tree::new("sibling-0"))
                    .with_child(Tree::new("sibling-1")),
            );
            let result = tree.path_between(&"x", &"sibling-1");
            let expected = Some(vec![&"x", &"parent", &"sibling-1"]);
            assert_eq!(result, expected);
        }

        #[test]
        fn can_find_path_from_nodes_other_than_x() {
            let mut tree = Tree::new("parent")
                .with_child(Tree::new("a"))
                .with_child(Tree::new("x"))
                .with_child(Tree::new("b"))
                .with_child(Tree::new("c"));
            let result = tree.path_between(&"a", &"c");
            let expected = Some(vec![&"a", &"parent", &"c"]);
            assert_eq!(result, expected);
        }

        #[test]
        fn errors_if_destination_does_not_exist() {
            let mut tree = Tree::new("parent")
                .with_child(
                    Tree::new("x")
                        .with_child(Tree::new("kid-0"))
                        .with_child(Tree::new("kid-1")),
                )
                .with_child(Tree::new("sibling-0"))
                .with_child(Tree::new("sibling-1"));
            let result = tree.path_between(&"x", &"nonexistent");
            let expected: Option<Vec<_>> = None;
            assert_eq!(result, expected);
        }

        #[test]
        fn errors_if_source_does_not_exist() {
            let mut tree = Tree::new("parent")
                .with_child(
                    Tree::new("x")
                        .with_child(Tree::new("kid-0"))
                        .with_child(Tree::new("kid-1")),
                )
                .with_child(Tree::new("sibling-0"))
                .with_child(Tree::new("sibling-1"));
            let result = tree.path_between(&"nonexistent", &"x");
            let expected: Option<Vec<_>> = None;
            assert_eq!(result, expected);
        }
    }
}
