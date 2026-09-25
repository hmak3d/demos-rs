//! Code for creating and manipulating graphs

use crate::graph::fmt::FormattedNode;
use crate::graph::node::{Node, NodeId};
use std::collections::{HashMap, HashSet};
use std::fmt::{Debug, Formatter};
use std::rc::Rc;
use std::{cell::RefCell, hash::Hash};

//////////////////////////////////////////////////////////////////////////////// Node

/// Graph node
pub mod node {
    use std::collections::HashSet;
    use std::fmt::{Debug, Display, Formatter};
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[derive(Debug)]
    pub struct Node<T> {
        id: NodeId,
        adj: HashSet<NodeId>,
        value: T,
    }

    static NEXT_ID: AtomicUsize = AtomicUsize::new(0);

    impl<T> Node<T> {
        pub fn new(value: T) -> Self {
            let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
            Self {
                id: NodeId(id),
                adj: HashSet::new(),
                value,
            }
        }

        pub fn id(&self) -> NodeId {
            self.id
        }

        pub fn value(&self) -> &T {
            &self.value
        }

        pub fn add_adjacent(&mut self, ids: impl IntoIterator<Item = NodeId>) {
            self.adj.extend(ids);
        }

        pub fn get_all_adjacent(&self) -> impl Iterator<Item = NodeId> {
            self.adj.iter().copied()
        }
    }

    #[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
    pub struct NodeId(usize);

    impl Debug for NodeId {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            write!(f, "id({:?})", self.0)
        }
    }

    impl Display for NodeId {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            write!(f, "{:?}", self)
        }
    }
}

//////////////////////////////////////////////////////////////////////////////// Debug formatting

/// For {:?} formatting
pub mod fmt {
    use std::cell::RefCell;
    use std::collections::HashSet;
    use std::fmt::{Debug, Formatter};
    use std::rc::Rc;

    use crate::graph::{Graph, NodeId};

    pub struct FormattedNode<'g, T> {
        id: NodeId,
        graph: &'g Graph<T>,
        visited: Rc<RefCell<HashSet<NodeId>>>,
    }

    impl<'g, T> FormattedNode<'g, T> {
        pub fn new(id: NodeId, graph: &'g Graph<T>) -> Self {
            FormattedNode::new_internal(id, graph, Rc::new(RefCell::new(HashSet::new())))
        }

        pub fn new_internal(
            id: NodeId,
            graph: &'g Graph<T>,
            visited: Rc<RefCell<HashSet<NodeId>>>,
        ) -> Self {
            Self { id, graph, visited }
        }

        pub fn id(&self) -> NodeId {
            self.id
        }
    }

    impl<'g, T: Debug> Debug for FormattedNode<'g, T> {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            if RefCell::borrow_mut(&self.visited).insert(self.id) {
                let nd = self.graph.get_node(self.id).unwrap();
                let mut adj = nd
                    .get_all_adjacent()
                    .map(|id| FormattedNode::new_internal(id, self.graph, Rc::clone(&self.visited)))
                    .collect::<Vec<_>>();
                // Sort nodes to make output deterministic
                adj.sort_by_key(|fmt_nd| fmt_nd.id);
                if f.alternate() {
                    write!(f, "{{{:#?}: {:#?} -> {:#?}}}", nd.id(), nd.value(), adj)
                } else {
                    f.debug_struct("Node:")
                        .field("id", &nd.id())
                        .field("value", nd.value())
                        .field("adj", &adj)
                        .finish()
                }
            } else {
                // already visited => don't recurse
                write!(f, "{{{:?} ...}}", self.id)
            }
        }
    }
}

//////////////////////////////////////////////////////////////////////////////// Graph

/// Graph
pub struct Graph<T> {
    nodes: HashMap<NodeId, Node<T>>,
}

impl<T> FromIterator<(T, T)> for Graph<T>
where
    T: Eq + PartialEq + Hash + Clone,
{
    fn from_iter<I: IntoIterator<Item = (T, T)>>(iter: I) -> Self {
        let mut graph = Graph::new();
        graph.load_from_edges(iter);
        graph
    }
}

impl<T> Graph<T> {
    #[expect(clippy::new_without_default)]
    /// Create empty [Graph]
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
        }
    }

    /// Modify [Graph] to contain list of edges.
    /// * The edge is specify as 2-tuple of values
    /// * Each value \[in tuple\] is used create a [Node]
    /// * No two [Node]'s have the same value
    pub fn load_from_edges(&mut self, edges: impl IntoIterator<Item = (T, T)>)
    where
        T: Eq + PartialEq + Hash + Clone,
    {
        let mut val_to_node_id = HashMap::<T, NodeId>::new();

        for (val1, val2) in edges {
            let nid2 = val_to_node_id.get(&val2);
            let nid2 = match nid2 {
                Some(nid) => *nid,
                None => {
                    let nid = self.create_node(val2.clone());
                    val_to_node_id.insert(val2, nid);
                    nid
                }
            };

            let nid1 = val_to_node_id.get(&val1);
            let nid1 = match nid1 {
                Some(nid) => *nid,
                None => {
                    let nid = self.create_node(val1.clone());
                    val_to_node_id.insert(val1, nid);
                    nid
                }
            };

            let node1_mut = self.get_node_mut(nid1).unwrap();
            node1_mut.add_adjacent([nid2]);
        }
    }

    pub fn create_node(&mut self, value: T) -> NodeId {
        let nd = Node::new(value);
        let id = nd.id();
        self.nodes.insert(nd.id(), nd);
        id
    }

    pub fn get_node_mut(&mut self, id: NodeId) -> Option<&mut Node<T>> {
        self.nodes.get_mut(&id)
    }

    pub fn get_node(&self, id: NodeId) -> Option<&Node<T>> {
        self.nodes.get(&id)
    }

    /// Return IDs of nodes after topographical sort.
    /// Earlier nodes will always "points" to later nodes.
    /// * aka treat each edge as node must-come-before
    /// * aka reading results from left-to-right, all edges go from left to right
    ///
    /// ## Algorithm
    ///
    /// ```text
    /// For every node in graph
    ///     Do DFS, skipping nodes already visited
    ///     During DFS, post-insert (append current node into results _after_ visiting its adjacent edges)
    /// At end return results in reverse order
    /// (... you can skip this reverse if you "prepend" into results during DFS)
    /// ```
    ///
    /// re: <https://en.wikipedia.org/wiki/Topological_sorting#Depth-first_search>
    pub fn topo_sort(&self) -> impl Iterator<Item = NodeId> {
        let mut out = Vec::new();
        let mut visited = HashSet::new();

        // FIXME hmak Drop the sort while avoiding making the unit tests flakey
        // Sort the keys so that output is deterministic
        let mut ids: Vec<&NodeId> = self.nodes.keys().collect();

        #[cfg(feature = "graph_sorted_nodes")]
        ids.sort();

        for root_id in ids {
            if !visited.contains(root_id) {
                self.topo_sort_helper(*root_id, &mut out, &mut visited);
            }
            // else skip nodes already visited by topo_sort_helper
        }
        out.into_iter().rev()
    }

    pub fn topo_sort_helper(
        &self,
        id: NodeId,
        out: &mut Vec<NodeId>,
        visited: &mut HashSet<NodeId>,
    ) {
        if !visited.insert(id) {
            return;
        }
        // Only way for lookup to fail would be if Node::adj is corrupted
        let nd = self.nodes.get(&id).unwrap();
        for adj_id in nd.get_all_adjacent() {
            self.topo_sort_helper(adj_id, out, visited);
        }
        out.push(id);
    }
}

impl<T: Debug> Debug for Graph<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let visited = Rc::new(RefCell::new(HashSet::new()));

        // Sort nodes to make output deterministic
        let mut ids = self.nodes.keys().copied().collect::<Vec<_>>();

        #[cfg(feature = "graph_sorted_nodes")]
        ids.sort();

        let fmt_nodes = ids
            .into_iter()
            .map(|id| FormattedNode::new_internal(id, self, Rc::clone(&visited)))
            .filter(|fmt_node| !visited.borrow().contains(&fmt_node.id()));
        if f.alternate() {
            write!(f, "{:#?}", fmt_nodes.collect::<Vec<_>>())
        } else {
            f.debug_set().entries(fmt_nodes).finish()
        }
    }
}

//////////////////////////////////////////////////////////////////////////////// Tests

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_print_node() {
        // A <- B <- D
        // ^         |
        // \--- C <-/

        let mut graph = Graph::new();
        let nd_a = graph.create_node('A');
        let nd_b = graph.create_node('B');
        graph.get_node_mut(nd_b).unwrap().add_adjacent([nd_a]);
        let nd_c = graph.create_node('C');
        graph.get_node_mut(nd_c).unwrap().add_adjacent([nd_a]);
        let nd_d = graph.create_node('D');
        graph.get_node_mut(nd_d).unwrap().add_adjacent([nd_b, nd_c]);

        // {:?} works with FormattedNode
        let s = format!("{:?}", FormattedNode::new(nd_d, &graph));
        println!("nd_d = {s}");
        assert_eq!(
            s,
            format!(
                r#"Node: {{ id: {nd_d:?}, value: 'D', adj: [Node: {{ id: {nd_b:?}, value: 'B', adj: [Node: {{ id: {nd_a:?}, value: 'A', adj: [] }}] }}, Node: {{ id: {nd_c:?}, value: 'C', adj: [{{{nd_a:?} ...}}] }}] }}"#
            )
        );

        // {:#?} works with FormattedNode
        let s = format!("{:#?}", FormattedNode::new(nd_d, &graph));
        println!("nd_d alt = {s}");
        assert_eq!(
            s,
            format!(
                r#"
{{{nd_d:?}: 'D' -> [
    {{{nd_b:?}: 'B' -> [
        {{{nd_a:?}: 'A' -> []}},
    ]}},
    {{{nd_c:?}: 'C' -> [
        {{{nd_a:?} ...}},
    ]}},
]}}"#
            )
            .trim()
        );
    }

    #[test]
    fn debug_print_graph() {
        // A <- B <- D
        // ^         |
        // \--- C <-/

        let mut graph = Graph::new();
        let nd_a = graph.create_node('A');
        let nd_b = graph.create_node('B');
        graph.get_node_mut(nd_b).unwrap().add_adjacent([nd_a]);
        let nd_c = graph.create_node('C');
        graph.get_node_mut(nd_c).unwrap().add_adjacent([nd_a]);
        let nd_d = graph.create_node('D');
        graph.get_node_mut(nd_d).unwrap().add_adjacent([nd_b, nd_c]);

        // {:?} works with Graph
        let s = format!("{:?}", graph);
        println!("graph = {s}");
        assert_eq!(
            s,
            format!(
                r#"{{Node: {{ id: {nd_a:?}, value: 'A', adj: [] }}, Node: {{ id: {nd_b:?}, value: 'B', adj: [{{{nd_a:?} ...}}] }}, Node: {{ id: {nd_c:?}, value: 'C', adj: [{{{nd_a:?} ...}}] }}, Node: {{ id: {nd_d:?}, value: 'D', adj: [{{{nd_b:?} ...}}, {{{nd_c:?} ...}}] }}}}"#
            )
        );

        // {:#?} works with Graph
        let s = format!("{:#?}", graph);
        println!("graph alt = {s}");
        assert_eq!(
            s,
            format!(
                r#"
[
    {{{nd_a:?}: 'A' -> []}},
    {{{nd_b:?}: 'B' -> [
        {{{nd_a:?} ...}},
    ]}},
    {{{nd_c:?}: 'C' -> [
        {{{nd_a:?} ...}},
    ]}},
    {{{nd_d:?}: 'D' -> [
        {{{nd_b:?} ...}},
        {{{nd_c:?} ...}},
    ]}},
]"#
            )
            .trim()
        );
    }

    #[test]
    fn topo_sort() {
        // A <- B <- C - F <---- G
        // ^ <---------- | -----/
        // |             |
        // \--- D <- E <-/

        let graph: Graph<char> = [
            ('B', 'A'),
            ('D', 'A'),
            ('C', 'B'),
            ('E', 'D'),
            ('F', 'C'),
            ('F', 'E'),
            ('G', 'F'),
        ]
        .into_iter()
        .collect();

        println!("graph = {:#?}", graph);

        let topo_vals: Vec<char> = graph
            .topo_sort()
            .map(|id| graph.get_node(id).unwrap().value())
            .copied()
            .collect();
        println!("topo_sort = {:?}", topo_vals);
        assert_eq!(topo_vals, ['G', 'F', 'E', 'C', 'D', 'B', 'A']);
    }
}
