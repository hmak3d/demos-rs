//! re: <https://exercism.org/tracks/rust/exercises/pov>
//!
//! Implement tree point of view change 2 ways:
//! - using node ID into HashMap lookup (see [lookup_impl])
//! - using unsafe raw pointers (see [unsafe_impl])
//!
//! ```sh
//! $ cargo test pov
//! $ cargo test pov -F pov_unsafe
//! $ cargo miri test pov -F pov_unsafe # stacked borrows aliasing FAILS
//! $ MIRIFLAGS="-Zmiri-tree-borrows" cargo miri test pov -F pov_unsafe # tree borrows aliasing passes
//!
//! $ cargo bench pov::tests::from_pov::bench_
//! test exercism::pov::tests::from_pov::bench_can_reroot_a_complex_tree_with_cousins       ... bench:       4,164.82 ns/iter (+/- 93.23)
//!
//! $ cargo bench pov::tests::from_pov::bench_ -F pov_unsafe
//! test exercism::pov::tests::from_pov::bench_can_reroot_a_complex_tree_with_cousins       ... bench:       2,087.86 ns/iter (+/- 52.45)
//! ```

/// Implement using node ID into HashMap lookup.
///
/// This is slower but avoids added compexity/danger from unsafe code.
pub mod lookup_impl {
    use std::collections::{BTreeSet, HashMap, HashSet};
    use std::fmt::Debug;
    use std::sync::atomic::{AtomicUsize, Ordering};

    //////////////////////////////////////////////////////////////////////////////// Main data structures

    #[derive(Debug)]
    pub struct Tree<T: Debug + Ord> {
        root_id: NodeId,

        // HashMap not BTreeMap because we want get_disjoint_mut()
        all_nodes: HashMap<NodeId, Node<T>>,
    }

    impl<T: Debug + Ord> PartialEq for Tree<T> {
        fn eq(&self, other: &Self) -> bool {
            // Compare root nodes
            OrdNode::wrap(self.get_node(&self.root_id).unwrap(), &self.all_nodes)
                == OrdNode::wrap(other.get_node(&other.root_id).unwrap(), &other.all_nodes)
        }
    }

    type NodeId = usize;

    /// A tree node.
    /// We keep this distinct from [Tree] because graph edges are node IDs instead of pointers
    #[derive(Debug)]
    struct Node<T> {
        id: NodeId,
        label: T,

        // NB: Link nodes by node ID not ref/pointers because we often need to
        // modify multiple nodes at once. If use ref (&), then may need interior
        // mutability. If use ref mut (&mut), then getting simultaneous disjoint
        // &mut will be impossible???
        parent: Option<NodeId>,
        children_ids: BTreeSet<NodeId>,
    }

    static NEXT_NODE_ID: AtomicUsize = AtomicUsize::new(0);

    impl<T> Node<T>
    where
        T: PartialEq,
    {
        fn new(label: T) -> Self {
            Self {
                id: NEXT_NODE_ID.fetch_add(1, Ordering::Relaxed),
                label,
                parent: None,
                children_ids: BTreeSet::new(),
            }
        }

        /// DFS to find path from current Node to another Node with a specific label
        fn path_to(&self, to: &T, nodes: &HashMap<NodeId, Node<T>>) -> Option<Vec<NodeId>> {
            self.dfs(to, nodes, &mut HashSet::new())
        }

        /// DFS search to find a label.
        ///
        /// Note that search can go up the tree (i.e., we treat the tree more like a graph)
        ///
        /// Returns path from current node to the found node, or [None] if label is not found
        fn dfs(
            &self,
            to: &T,
            nodes: &HashMap<NodeId, Node<T>>,
            visited: &mut HashSet<NodeId>,
        ) -> Option<Vec<NodeId>> {
            // 1. check against current node. if match, then return 1 element list containing current node
            // 2. for each child, try to find path. if found, then return that path prefixed with current node

            if !visited.insert(self.id) {
                // Already visited
                return None;
            }
            if &self.label == to {
                return Some(vec![self.id]);
            }
            for child in self
                .children_ids
                .iter()
                .chain(&self.parent)
                .map(|node_id| nodes.get(node_id).unwrap())
            {
                if let Some(mut path) = child.dfs(to, nodes, visited) {
                    path.insert(0, self.id);
                    return Some(path);
                }
            }

            // Search failed. Label does not exist
            None
        }
    }

    //////////////////////////////////////////////////////////////////////////////// OrdNode

    /// Wrapper to compare 2 nodes.
    /// Recursively resolve children node IDs to children nodes.
    struct OrdNode<'a, T> {
        wrapped: &'a Node<T>,
        nodes: &'a HashMap<NodeId, Node<T>>,
    }

    impl<'a, T: Ord> Eq for OrdNode<'a, T> {}

    impl<'a, T: Ord> Ord for OrdNode<'a, T> {
        fn cmp(&self, other: &Self) -> std::cmp::Ordering {
            // Compare labels
            match self.wrapped.label.cmp(&other.wrapped.label) {
                core::cmp::Ordering::Equal => {}
                ord => return ord,
            }

            // Compare children count
            match self
                .wrapped
                .children_ids
                .len()
                .cmp(&other.wrapped.children_ids.len())
            {
                core::cmp::Ordering::Equal => {}
                ord => return ord,
            }

            // Look up children + sort them.
            // Sorting will eliminate the need to O(n^2) compares between children.
            let mut my_sorted_children = self
                .wrapped
                .children_ids
                .iter()
                .map(|node_id| OrdNode::wrap(self.nodes.get(node_id).unwrap(), self.nodes))
                .collect::<Vec<_>>();
            my_sorted_children.sort();

            let mut other_sorted_children = self
                .wrapped
                .children_ids
                .iter()
                .map(|node_id| OrdNode::wrap(self.nodes.get(node_id).unwrap(), self.nodes))
                .collect::<Vec<_>>();
            other_sorted_children.sort();

            // Recursively compare matching children
            for (my_child, other_child) in my_sorted_children.into_iter().zip(other_sorted_children)
            {
                match my_child.cmp(&other_child) {
                    std::cmp::Ordering::Equal => {}
                    ord => return ord,
                }
            }
            std::cmp::Ordering::Equal
        }
    }

    impl<'a, T: Ord> PartialEq for OrdNode<'a, T> {
        fn eq(&self, other: &Self) -> bool {
            OrdNode::cmp(self, other) == std::cmp::Ordering::Equal
        }
    }

    impl<'a, T: Ord> PartialOrd for OrdNode<'a, T> {
        fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
            Some(OrdNode::cmp(self, other))
        }
    }

    impl<'a, T> OrdNode<'a, T> {
        fn wrap(wrapped: &'a Node<T>, nodes: &'a HashMap<NodeId, Node<T>>) -> Self {
            Self { wrapped, nodes }
        }
    }

    //////////////////////////////////////////////////////////////////////////////// Public API

    impl<T: Debug + Ord> Tree<T> {
        pub fn new(label: T) -> Self {
            let node = Node::new(label);
            let node_id = node.id;
            Self {
                root_id: node_id,
                all_nodes: HashMap::from([(node_id, node)]),
            }
        }

        pub fn with_child(mut self, child: Self) -> Self {
            // Adopt child's descendents
            self.all_nodes.extend(child.all_nodes);

            // Point us to child
            let root_id = self.root_id;
            let child_id = child.root_id;
            let root_node = self.get_node_mut(&root_id).unwrap();
            root_node.children_ids.insert(child_id);

            // Point child to us
            let child_node = self.get_node_mut(&child_id).unwrap();
            child_node.parent = Some(root_id);

            self
        }

        pub fn pov_from(&mut self, from: &T) -> bool {
            let root_node = self.get_node(&self.root_id).unwrap();

            // Get path from old to new root
            let Some(path_to_new_root) = root_node.path_to(from, &self.all_nodes) else {
                return false;
            };
            let new_root_id = *path_to_new_root.last().unwrap();

            // Along the path, "pivot" adjacent nodes
            for pair_ids in path_to_new_root.windows(2) {
                let [old_parent_mut, old_child_mut] = self
                    .get_disjoint_node_mut([&pair_ids[0], &pair_ids[1]])
                    .map(Option::unwrap);

                // Make the old parent become a child of old child (the child is "hosted" up)
                old_parent_mut.children_ids.remove(&old_child_mut.id);
                old_child_mut.children_ids.insert(old_parent_mut.id);
            }

            self.root_id = new_root_id;

            true
        }

        pub fn path_between<'a>(&'a mut self, from: &'a T, to: &'a T) -> Option<Vec<&'a T>> {
            let from_node_id = self
                .get_node(&self.root_id)?
                .path_to(from, &self.all_nodes)?
                .into_iter()
                .last()
                .unwrap();
            //
            let path: Vec<NodeId> = self.get_node(&from_node_id)?.path_to(to, &self.all_nodes)?;
            Some(
                path.iter()
                    .map(|node_id| &self.get_node(node_id).unwrap().label)
                    .collect::<Vec<&T>>(),
            )
        }

        fn get_node(&self, node_id: &NodeId) -> Option<&Node<T>> {
            self.all_nodes.get(node_id)
        }

        fn get_node_mut(&mut self, node_id: &NodeId) -> Option<&mut Node<T>> {
            self.all_nodes.get_mut(node_id)
        }

        fn get_disjoint_node_mut<const N: usize>(
            &mut self,
            node_ids: [&NodeId; N],
        ) -> [Option<&mut Node<T>>; N] {
            self.all_nodes.get_disjoint_mut(node_ids)
        }
    }
}

/// Implement using unsafe raw pointers.
///
/// This speeds things up by eliminating the HashMap<NodeId, Node<T>> lookup
/// table at the cost of added compexity/danger from unsafe code.
pub mod unsafe_impl {
    use std::collections::{BTreeSet, HashSet};
    use std::fmt::{self, Debug, Formatter};
    use std::mem;
    use std::ptr::NonNull;

    //////////////////////////////////////////////////////////////////////////////// Main data structures

    pub struct Tree<T: Debug + Ord> {
        root: Option<Link<T>>,
    }

    impl<T: Debug + Ord> PartialEq for Tree<T> {
        fn eq(&self, other: &Self) -> bool {
            // Compare root nodes
            // FIXME drop unwrap
            unsafe {
                OrdNode::from(self.root.unwrap().as_ref())
                    == OrdNode::from(other.root.unwrap().as_ref())
            }
        }
    }

    impl<T: Debug + Ord> Debug for Tree<T> {
        fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
            let mut dbg_fmt = f.debug_struct("Tree");
            if let Some(root) = self.root {
                dbg_fmt
                    // forward to Node
                    .field("root", unsafe { &*root.as_ptr() });
            }

            dbg_fmt.finish()
        }
    }

    impl<T: Debug + Ord> Drop for Tree<T> {
        fn drop(&mut self) {
            if let Some(mut root) = self.root.take() {
                // SAFETY: &mut self on Tree means no other &mut/& Node exists
                let root_mut = unsafe { root.as_mut() };

                // Deallocate all but root node
                root_mut.deallocate_descendants();

                // Deallocate root node.
                // This must be done separate from above because &mut Node is in used above
                #[expect(unused_must_use)]
                unsafe {
                    Box::from_raw(root.as_ptr())
                };
            }
        }
    }

    type Link<T> = NonNull<Node<T>>;

    /// A tree node.
    /// We keep this distinct from [Tree] because this helps with memory management especially
    /// for [Tree::pov_from] which takes a `&mut Tree` not `Tree` as a receiver.
    struct Node<T> {
        label: T,
        parent: Option<Link<T>>,
        children: BTreeSet<Link<T>>,
    }

    impl<T: Debug> Debug for Node<T> {
        fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
            let mut dbg_fmt = f.debug_struct("Node");
            dbg_fmt.field("label", &self.label);
            if self.parent.is_none() {
                // SAFETY: Only Tree::fmt() call this method and it holds &Tree so no other &mut/&Node can exist
                dbg_fmt.field("parent", &"n/a");
            }
            if !self.children.is_empty() {
                dbg_fmt.field(
                    "children",
                    &self
                        .children
                        .iter()
                        // SAFETY: Only Tree::fmt() call this method and it holds &Tree so no other &mut/&Node can exist
                        .map(|child_ptr| unsafe { child_ptr.as_ref() })
                        .collect::<Vec<_>>(),
                );
            }
            dbg_fmt.finish()
        }
    }

    impl<T> Node<T>
    where
        T: PartialEq,
    {
        fn new(label: T) -> Self {
            Self {
                label,
                parent: None,
                children: BTreeSet::new(),
            }
        }

        /// DFS to find path from current Node to another Node with a specific
        /// label.
        ///
        /// Receiver is &mut not & because we want NonNull::as_mut() to work on
        /// returned value.
        fn path_to(&mut self, self_ptr: Link<T>, to: &T) -> Option<Vec<Link<T>>> {
            self.dfs(
                self_ptr,
                // Stop DFS if found label
                Some(&|node_mut: &mut Node<T>| &node_mut.label == to),
                None::<&fn(Link<T>)>,
            )
        }

        /// See [Node::dfs_with]
        fn dfs(
            &mut self,
            self_ptr: Link<T>,
            should_stop: Option<&impl Fn(&mut Node<T>) -> bool>,
            on_post_visit: Option<&impl Fn(Link<T>)>,
        ) -> Option<Vec<Link<T>>> {
            self.dfs_with(self_ptr, should_stop, on_post_visit, &mut HashSet::new())
        }

        /// DFS search to until callback tells us to stop.
        ///
        /// Note that search can go up the tree (i.e., we treat the tree more like a graph)
        ///
        /// # Arguments
        ///
        /// * `self_ptr` - Same address as `&mut self` except it has "earlier"
        ///   provenance (i.e., is the pointer immediately created when Node was
        ///   allocated).  This is needed to avoid UB.
        /// * `should_stop` - Visitor callback invoked before children are visited.
        ///   Returns whether or not DFS search should stop and return path.
        /// * `on_post_visit` - Visitor callback invoked after children are visited
        /// * `visited` - Already visited nodes
        ///
        /// # Returns
        ///
        /// Returns path from current node to the node where `should_stop()`
        /// returned `true`.
        ///
        /// Otherwise, [None] if either
        /// - every `should_stop()` reachable from us returned `false`
        /// - or adjacents nodes were already visited
        fn dfs_with(
            &mut self,
            self_ptr: Link<T>,
            should_stop: Option<&impl Fn(&mut Node<T>) -> bool>,
            on_post_visit: Option<&impl Fn(Link<T>)>,
            visited: &mut HashSet<Link<T>>,
        ) -> Option<Vec<Link<T>>> {
            // 1. check against current node. if match, then return 1 element list containing current node
            // 2. for each child, try to find path. if found, then return that path prefixed with current node

            // NB: We never dereference anything put into visited, so its usage is
            // always safe
            if !visited.insert(self_ptr) {
                // Already visited
                return None;
            }
            if let Some(should_stop) = should_stop
                && should_stop(self)
            {
                // NB: Do _not_ return NonNull::new_unchecked(self as *mut Node<T>)
                // as this will result in later UB. "&mut self" lifetime eventually
                // goes away so any NonNull/(*mut Node) created from it will be
                // invalidated.
                //
                // For stacked borrows:
                // NonNull/(*mut Node) derived from &mut self will be higher up stack.
                // So when &mut self lifetime ends, derived NonNull access is UB.
                //
                // For tree borrows:
                // Creating another &mut self on Node will foreign invalidate NonNull ?
                return Some(vec![self_ptr]);
            }
            // else DfsSearchMode::Continue
            for mut child_ptr in self.children.iter().chain(&self.parent).copied() {
                if let Some(mut path) =
                    // SAFETY: Use of visited means we only visit each Node once. So
                    // it's okay to &mut on every one of them.
                    unsafe { child_ptr.as_mut() }.dfs_with(
                        child_ptr,
                        should_stop,
                        on_post_visit,
                        visited,
                    )
                {
                    // Found path from child to descendant that has "to" label => prefix with ourselves
                    path.insert(0, self_ptr);
                    return Some(path);
                }

                // Parents should not be visited twice
                if Some(child_ptr) != self.parent
                    && let Some(on_post_visit) = on_post_visit
                {
                    on_post_visit(child_ptr);
                }
            }

            // Search failed. Label does not exist
            None
        }

        fn deallocate_descendants(&mut self) {
            // Use fake self ptr to make compiler happy.
            // The root node this method is called on is actually cleaned up by the caller.
            // SAFETY: &mut self is non-null and aligned
            let fake_self_ptr_do_not_access_or_drop =
                unsafe { NonNull::new_unchecked(self as *mut Node<T>) };

            Node::dfs(
                self,
                // This will returned back by dfs() but we won't deref it so it's
                // okay to pass this in as an input argument
                fake_self_ptr_do_not_access_or_drop,
                // Never stop DFS
                None::<&fn(&mut Node<T>) -> bool>,
                // On post visit (leaving a node during DFS), we won't ever visit it
                // again + the current method is called from Tree::drop(), so we can
                // free the node memory
                Some(&|node_ptr: Link<T>| {
                    if node_ptr != fake_self_ptr_do_not_access_or_drop {
                        #[expect(unused_must_use)]
                        unsafe {
                            Box::from_raw(node_ptr.as_ptr())
                        };
                    }
                    // else visiting root node => don't deallocate it. Leave it to caller.
                }),
            );
        }
    }

    //////////////////////////////////////////////////////////////////////////////// OrdNode

    /// Wrapper to compare 2 nodes.
    /// Recursively resolve children node IDs to children nodes.
    struct OrdNode<'a, T>(&'a Node<T>);

    impl<'a, T: Ord> Eq for OrdNode<'a, T> {}

    /// Implementing [Ord] makes [Node::eq] and therefore [Tree::eq] more efficient
    impl<'a, T: Ord> Ord for OrdNode<'a, T> {
        fn cmp(&self, other: &Self) -> std::cmp::Ordering {
            // Compare labels
            match self.0.label.cmp(&other.0.label) {
                core::cmp::Ordering::Equal => {}
                ord => return ord,
            }

            // Compare children count
            match self.0.children.len().cmp(&other.0.children.len()) {
                core::cmp::Ordering::Equal => {}
                ord => return ord,
            }

            // Look up children + sort them.
            // Sorting will eliminate the need to O(n^2) compares between children.
            let mut my_sorted_children = self
                .0
                .children
                .iter()
                // SAFETY; OrdNode only created from Tree:eq, which has &Tree so
                // no other &mut/& Node exists for this particular node outside of OrdNode::cmp()
                .map(|node_ptr| unsafe { node_ptr.clone().as_ref().into() })
                .collect::<Vec<OrdNode<T>>>();
            my_sorted_children.sort();

            let mut other_sorted_children = other
                .0
                .children
                .iter()
                // Convert NonNull -> &Node -> OrdNode
                // SAFETY; OrdNode only created from Tree:eq, which has &Tree so
                // no other &mut/& Node exists for this particular node outside of OrdNode::cmp()
                .map(|node_ptr| unsafe { node_ptr.clone().as_ref().into() })
                .collect::<Vec<OrdNode<T>>>();
            other_sorted_children.sort();

            // Recursively compare matching children.
            // Because the children are sorted, we have O(n) instead of O(n^2) compares.
            // So overall cmp() is O(d * n * log(n)) [from sort] instead of O(d * n^2) where
            // "d" is depth and "n" is average children/node.
            for (my_child, other_child) in my_sorted_children.into_iter().zip(other_sorted_children)
            {
                match my_child.cmp(&other_child) {
                    std::cmp::Ordering::Equal => {}
                    ord => return ord,
                }
            }
            std::cmp::Ordering::Equal
        }
    }

    impl<'a, T: Ord> PartialEq for OrdNode<'a, T> {
        fn eq(&self, other: &Self) -> bool {
            OrdNode::cmp(self, other) == std::cmp::Ordering::Equal
        }
    }

    impl<'a, T: Ord> PartialOrd for OrdNode<'a, T> {
        fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
            Some(OrdNode::cmp(self, other))
        }
    }

    impl<'a, T> From<&'a Node<T>> for OrdNode<'a, T> {
        fn from(wrapped: &'a Node<T>) -> Self {
            Self(wrapped)
        }
    }

    //////////////////////////////////////////////////////////////////////////////// Public API

    impl<T: Debug + Ord> Tree<T> {
        pub fn new(label: T) -> Self {
            Self {
                // SAFETY: Box::into_row() is always non-null and aligned
                root: Some(unsafe {
                    NonNull::new_unchecked(Box::into_raw(Box::new(Node::new(label))))
                }),
            }
        }

        pub fn with_child(self, mut child: Self) -> Self {
            let mut root_ptr = self
                .root
                .expect("not possible as tree was consumed by with_child() RHS");

            if let Some(mut child_ptr) = child.root.take() {
                // Point us to child
                unsafe { root_ptr.as_mut() }.children.insert(child_ptr);

                // Point child to us
                unsafe { child_ptr.as_mut() }.parent = Some(root_ptr);
            }

            self
        }

        pub fn pov_from(&mut self, from: &T) -> bool {
            let mut root_ptr = self
                .root
                .expect("not possible as tree was consumed by with_child() RHS");

            // SAFETY: &mut self on Tree prevents code outside method to have &mut/& Node.
            let root_mut = unsafe { root_ptr.as_mut() };

            // Get path from old to new root
            let Some(path_to_new_root) = root_mut.path_to(root_ptr, from) else {
                return false;
            };

            // For SAFETY below
            #[expect(dropping_references)]
            drop(root_mut);

            // Along the path, "pivot" parent-child nodes.
            // This inverts the tree hierarchy and makes the most nested child the new root.
            for adj_ptrs in path_to_new_root.windows(2) {
                let (mut old_parent_ptr, mut old_child_ptr) = (adj_ptrs[0], adj_ptrs[1]);

                // SAFETY: &mut self on Tree prevents code outside method to have &mut/& Node.
                // Within this method in above, we have &mut Node on root_not, but this is dropped
                // and no longer needs to be considered.
                let old_parent_mut = unsafe { old_parent_ptr.as_mut() };
                let old_child_mut = unsafe { old_child_ptr.as_mut() };

                // Make the old parent become a child of old child
                old_parent_mut.children.remove(&old_child_ptr);
                old_child_mut.children.insert(old_parent_ptr);

                // Following effectively "bubbles" the None parent into the last/most-nested child
                assert!(old_parent_mut.parent.is_none());
                mem::swap(&mut old_parent_mut.parent, &mut old_child_mut.parent);

                self.root = Some(old_child_ptr);
            }

            true
        }

        pub fn path_between<'a>(&'a mut self, from: &'a T, to: &'a T) -> Option<Vec<&'a T>> {
            let mut root_ptr = self
                .root
                .expect("not possible as tree was consumed by with_child() RHS");
            // SAFETY: &mut self prevents code outside to have &mut/& Node
            let mut from_node_ptr = unsafe { root_ptr.as_mut() }
                .path_to(root_ptr, from)?
                .into_iter()
                .last()?;
            // SAFETY: &mut self prevents code outside to have &mut/& Node
            let from_node_ref = unsafe { from_node_ptr.as_mut() };

            let path: Vec<Link<T>> = from_node_ref.path_to(from_node_ptr, to)?;
            Some(
                path.into_iter()
                    // SAFETY: &mut self prevents code outside to have &mut/& Node
                    .map(|mut node_ptr| &unsafe { node_ptr.as_mut() }.label)
                    .collect::<Vec<&T>>(),
            )
        }
    }
}

#[cfg(all(test, not(feature = "pov_unsafe")))]
use lookup_impl as pov;

#[cfg(all(test, feature = "pov_unsafe"))]
use unsafe_impl as pov;

#[cfg(test)]
mod tests {
    extern crate test;
    use super::pov;
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
