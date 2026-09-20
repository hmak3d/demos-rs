//! re: <https://exercism.org/tracks/rust/exercises/doubly-linked-list/edit>
// this module adds some functionality based on the required implementations
// here like: `LinkedList::pop_back` or `Clone for LinkedList<T>`
// You are free to use anything in it, but it's mainly for the test framework.

use std::fmt::Debug;
use std::iter::FusedIterator;
use std::mem::MaybeUninit;
use std::ptr::NonNull;

/// A non-null pointer [to a node].
///
/// The usage of "ghost node" enables this to be non-null
/// (see [LinkedList] docs).
///
/// Dropping this will _not_ release/drop node resources.
/// For that, you'd need to use [ToNodeExt::into_boxed_node].
type Link<T> = NonNull<Node<T>>;

/// For converting [Link] to an owned [Node].
/// Is used to release/drop resources.
trait ToNodeExt<T> {
    /// Convert [Link] -> [`Box<Node<T>>`] (so Box can be dropped)
    ///
    /// Is reverse of [Node::into_link]
    ///
    /// # Safety
    ///
    /// * The link is not concurrently being borrowed
    ///   (e.g., copies are passed to [NonNull::as_mut]/etc and
    ///   the returned lifetimes are still active)
    /// * The original link must have come from [Node::into_link].
    unsafe fn into_boxed_node(self) -> Box<Node<T>>;
}

/// Add a conversion method to all [Link]
impl<T> ToNodeExt<T> for NonNull<Node<T>> {
    unsafe fn into_boxed_node(self) -> Box<Node<T>> {
        // SAFETY: See doc comment for trait
        unsafe { Box::from_raw(self.as_ptr()) }
    }
}

/// A node in the linked list.
/// Holds the actual element value/data.
///
/// The main factory methods are [Node::new_link] and [Node::new_ghost_link]
struct Node<T> {
    /// Element value/data
    ///
    /// This is [`MaybeUninit<T>`] instead of `T` to accomodate the ghost node
    ///
    /// If `is_ghost` == true => the value is initialized.
    /// `data.assume_init*()` family of methods can be safetly called.
    ///
    /// If `is_ghost` == false => the value is uninitialized.
    /// It is UB/illegal to call `data.assume_init*()`
    data: MaybeUninit<T>,
    prev: Link<T>,
    next: Link<T>,
    is_ghost: bool,
}

impl<T> Debug for Node<T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut res = f.debug_struct("Node");
        if !self.is_ghost {
            // SAFETY: Data exists because this is not a ghost node
            res.field("data", unsafe { self.data.assume_init_ref() });
        }
        res.field("prev", &self.prev)
            .field("next", &self.next)
            .finish()
    }
}

impl<T> Node<T> {
    /// Create [Node] holding actual data, then return new [Link] pointing to it.
    ///
    /// This calls [Node::into_link] internally so the return value
    /// satisfies some of the safety requirements of the argument passed to
    /// [ToNodeExt::into_boxed_node].
    ///
    /// ATTN: Callers must eventually call [ToNodeExt::into_boxed_node]
    /// and then [Node::into_data] to avoid resource leaks. Simply dropping
    /// [Link] is insufficient.
    fn new_link(data: T, prev: Link<T>, next: Link<T>) -> Link<T> {
        Box::new(Self {
            data: MaybeUninit::new(data),
            prev,
            next,
            is_ghost: false,
        })
        .into_link()
    }

    /// Create ghost [Node], then return new [Link] pointing to it.
    ///
    /// This calls [Node::into_link] internally so the return value
    /// satisfies some of the safety requirements of the argument passed to
    /// [ToNodeExt::into_boxed_node].
    ///
    /// ATTN: Callers must eventually call [ToNodeExt::into_boxed_node]
    /// and then [Node::into_data] to avoid resource leaks. Simply dropping
    /// [Link] is insufficient.
    ///
    /// # Safety
    ///
    /// This method is safe, but you need to be careful.
    /// Since the return value is self pointing, you must not overlap
    /// access to `my_ref = returned_link.as_mut()` and `my_ref.prev.as_mut()`.
    /// Overlapping `my_ref` and `my_ref.prev`-read is okay since latter is raw
    /// pointer.
    fn new_ghost_link() -> Link<T> {
        let node = Box::new(Self {
            data: MaybeUninit::uninit(),
            prev: NonNull::dangling(), // temporary dangling points will be overwritten below
            next: NonNull::dangling(),
            is_ghost: true,
        });

        Node::make_self_pointing(node)
    }

    #[cfg(not(feature = "doubly_linked_list_ub"))]
    /// Make the node behind a link point to itself (e.g., for ghost nodes).
    /// Return link/pointer that should be used for node going forward.
    fn make_self_pointing(node: Box<Node<T>>) -> Link<T> {
        let mut link = node.into_link();

        // SAFETY: link is aligned because it came from Node::into_link().
        // No other &mut exists in this method.
        let node_mut = unsafe { link.as_mut() };
        node_mut.prev = link;
        node_mut.next = link;

        // ATTN: All modifications [whether through Node.prev/prev or LinkedList.head/tail]
        // must use the _same_ pointer [which means same alias]. This will help us avoid UB.
        // For an example of what problems can occurs if we don't do this, see the
        // "doubly_linked_list_ub" intentional INCORRECT alternative method below.
        link
    }

    #[cfg(feature = "doubly_linked_list_ub")]
    /// *INCORRECTLY* make the node behind a link point to itself (e.g., for ghost nodes).
    /// Return link/pointer that should be used for node going forward.
    fn make_self_pointing(node: Box<Node<T>>) -> Link<T> {
        let link = node.into_link();

        // SAFETY: link is aligned because it came from Node::into_link().
        // No other &mut exists in this method.
        let mut node = unsafe { link.into_boxed_node() };
        node.prev = link;
        node.next = link;

        // Returning "new_node_link" here instead of "link" will cause UB to
        // occur if both are modified. This is not allowed because:
        // - For stacked borrows, they are not at the same level in the borrow stack
        // - For tree borrows, they have different alias (aka in diff parts of the
        //   borrow tree), and so mods on one will count as foreign access and invalidate
        //   the other
        let new_node_link = node.into_link();
        assert_eq!(link.as_ptr(), new_node_link.as_ptr());
        new_node_link
    }

    /// Convert [`Box<Node<T>>`] -> [Link]
    ///
    /// Is useful for:
    /// - preventing Box from being automatically dropped (we want to do it manually)
    /// - hiding lifetime checks from borrow check (as we will do this manually w/ unsafe code)
    ///
    /// Is reverse of [ToNodeExt::into_boxed_node]
    ///
    /// The resulting link/pointer will be properly aligned and non-null
    ///
    /// Also see [Node::new_link] and [Node::new_ghost_link]
    fn into_link(self: Box<Node<T>>) -> Link<T> {
        // SAFETY: *mut T can't be null because Box::into_raw() isn't
        unsafe { NonNull::new_unchecked(Box::into_raw(self)) }
    }

    /// Move data out of Node
    ///
    /// This must always be called to avoid resource leaks
    fn into_data(self) -> Option<T> {
        if self.is_ghost {
            None
        } else {
            // SAFETY: Node was created by Node::new_link(), which means there is
            // actual data to move out
            Some(unsafe { self.data.assume_init() })
        }
    }
}

// NB: Cannot implement Drop because of Node::into_data() tries to move out [non-Copy] self.data
// which is illegal because drop() takes &mut self
// impl<T> Drop for Node<T> {
//     fn drop(&mut self) {
//         if !self.is_ghost {
//             // SAFETY: Node was created by Node::new_link(), which means that there
//             // actual data to drop
//             // NB: Cannot call drop(self.data.assume_init()) because we cannot
//             // move the data out of the struct _and_ T does not impl Copy
//             unsafe { self.data.assume_init_drop() };
//             self.is_ghost = true;
//         }
//     }
// }

/// A linked list of a sequence of nodes with links/pointers to each other going
/// in both directions. We hide [from Rust] the ownership of the nodes during inserts
/// and unhide them [in unsafe code] during take/removal.
///
/// ## Ghost node
///
/// A internal dummy "ghost" node is used to connect the tail node to the head
/// node forming a cycle.
/// All empty [LinkedList] start with 1 ghost node.
///
/// ### Advantages:
/// - Reduces edges case in the logic to insert/remove nodes
/// - Links between nodes can be non-null. No need to deal with [Option] types.
///
/// ### Disadvantages:
/// - Easier to introduce UB in unsafe code. In empty lists, the ghost node can
///   point to itself. So mutating both `node` and `node.next` at the same time
///   is tricky. They need to be ordered to respect the stacked borrows or tree
///   borrows model of miri.
///
/// ## Cursor
///
/// A Cursor can be used to mutate the list at arbitrary positions.
/// In fact, the [LinkedList::push_back] family or methods just uses [Cursor]
/// under the hood.
///
/// Cursor is always _on_ a node.
///
/// So an `n` element linked list with the cursor on the ghost node looks like:
///
/// ```text
/// +------------------------------+
/// | 1st | 2nd | ... | n-th |ghost|
/// +------------------------------+
///   ^                   ^     ^
///  head               tail   cur
/// ```
///
/// where
/// - [Cursor::insert_after]  => inserts at front of list
/// - [Cursor::insert_before] => inserts at back of list
///
/// e.g.,
/// [LinkedList::cursor_back()].[insert_after()](Cursor::insert_after()) has the same effect as
/// [LinkedList::cursor_front()].move_to_prev_into_ghost().[insert_before()](Cursor::insert_before())
/// (they insert a new tail).
///
/// e.g.,
/// [LinkedList::cursor_front()].[insert_before()](Cursor::insert_before()) has the same effect as
/// [LinkedList::cursor_back()].move_to_next_into_ghost().[insert_after()](Cursor::insert_after())
/// (they insert a new head).
pub struct LinkedList<T> {
    head: Link<T>,
    tail: Link<T>,
    len: usize,
}

// SAFETY: NonNull<T> is usually !Send but in our case, override this.
// It's okay because:
// 1. there is no aliasing ... we never expose internal pointers; &mut are
//   carefully managed in unsafe blocks
// 2. LinkedList does haven't any funky stuff like Rc and/or interior mutability
unsafe impl<T> Send for LinkedList<T> {}

// SAFETY: NonNull<T> is usually !Sync but in our case, override this.
// It's okay because:
// 1. there is no aliasing ... we never expose internal pointers; &mut are
//   carefully managed in unsafe blocks
// 2. LinkedList does haven't any funky stuff like Rc and/or interior mutability
unsafe impl<T> Sync for LinkedList<T> {}

/// Like an iterator but supercharged to be able to change the list
pub struct Cursor<'list, T> {
    cur: Link<T>,

    /// Reference to list ensures:
    /// - [Cursor] cannot outlive [LinkedList]
    /// - prevent any other [Cursor] or [Iter] from existing
    list: &'list mut LinkedList<T>,
}

pub struct Iter<'list, T> {
    cur: Link<T>,

    #[expect(unused)]
    /// Reference to list ensures:
    /// - [Iter] cannot outlive [LinkedList]
    /// - prevent any other [Cursor] from existing. Multiple [Iter] are ok.
    list: &'list LinkedList<T>,
}

impl<T> LinkedList<T> {
    #[expect(
        clippy::new_without_default,
        reason = "Cannot implement Default for Node"
    )]
    pub fn new() -> Self {
        let ghost_link = Node::new_ghost_link();
        Self {
            head: ghost_link,
            tail: ghost_link,
            len: 0,
        }
    }

    // You may be wondering why it's necessary to have is_empty()
    // when it can easily be determined from len().
    // It's good custom to have both because len() can be expensive for some types,
    // whereas is_empty() is almost always cheap.
    // (Also ask yourself whether len() is expensive for LinkedList)
    pub fn is_empty(&self) -> bool {
        // SAFETY: All links are properly aligned because they come from
        // Node::into_link()
        unsafe { self.head.as_ref().is_ghost }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn push_back(&mut self, data: T)
    // where
    //     T: Debug,
    {
        self.cursor_back().insert_after(data);
    }

    pub fn pop_back(&mut self) -> Option<T> {
        self.cursor_back().take()
    }

    pub fn push_front(&mut self, data: T)
    // where
    //     T: Debug,
    {
        self.cursor_front().insert_before(data);
    }

    pub fn pop_front(&mut self) -> Option<T> {
        self.cursor_front().take()
    }

    /// Return a cursor positioned on the front element.
    /// Empty lists will have cursor pointing to ghost node.
    pub fn cursor_front(&mut self) -> Cursor<'_, T> {
        Cursor {
            cur: self.head,
            list: self,
        }
    }

    /// Return a cursor positioned on the back element
    /// Empty lists will have cursor pointing to ghost node.
    pub fn cursor_back(&mut self) -> Cursor<'_, T> {
        Cursor {
            cur: self.tail,
            list: self,
        }
    }

    /// Return an iterator that moves from front to back
    pub fn iter(&self) -> Iter<'_, T> {
        Iter {
            cur: self.head,
            list: self,
        }
    }
}

impl<T> Drop for LinkedList<T> {
    fn drop(&mut self) {
        // Drop all data nodes
        let mut cursor = self.cursor_front();
        while cursor.take().is_some() {}

        // Drop the ghost node (which cursor never releases)

        // SAFETY: All links are properly aligned because they come from
        // Node::into_link()
        assert!(unsafe { self.head.as_ref().is_ghost });

        // SAFETY: We never expose the ghost node and current method has &mut to
        // the list => no &mut or & to node exists.
        // The original link came from Node::new_ghost_link().
        let data = unsafe { self.head.into_boxed_node() }.into_data();
        assert!(data.is_none());
    }
}

impl<T> FromIterator<T> for LinkedList<T>
// where
//     T: Debug,
{
    fn from_iter<S: IntoIterator<Item = T>>(iter: S) -> Self {
        let mut list = LinkedList::new();
        for item in iter {
            list.push_back(item);
        }
        list
    }
}

// the cursor is expected to act as if it is at the position of an element
// and it also has to work with and be able to insert into an empty list.
impl<T> Cursor<'_, T> {
    /// Take a mutable reference to the current element
    pub fn peek_mut(&mut self) -> Option<&mut T> {
        // SAFETY: All links are properly aligned because they come from
        // Node::into_link()
        if unsafe { self.cur.as_ref() }.is_ghost {
            None
        } else {
            // SAFETY: No other &mut Node exists for current node, as to get one
            // one needs &mut Cursor, which no-one else has since current
            // method has it. No other &Node exists because Iter + Cursor
            // cannot both co-exist.
            // Only ghost nodes are missing data.
            Some(unsafe { self.cur.as_mut().data.assume_init_mut() })
        }
    }

    /// Move one position forward (towards the front) and
    /// return a reference to the new position.
    ///
    /// The cursor is not "fused" due to the fact the ghost node makes the
    /// linked list a loop.  After returning None (i.e., you have moved into the
    /// ghost node), the subsequent invocation will return the _head_ value [if
    /// it exists].
    #[expect(
        clippy::should_implement_trait,
        reason = "suppress name of next() warning"
    )]
    pub fn next(&mut self) -> Option<&mut T> {
        // SAFETY: All links are properly aligned because they come from
        // Node::into_link()
        self.cur = unsafe { self.cur.as_ref().next };
        self.peek_mut()
    }

    /// Move one position backward (towards the front) and
    /// return a reference to the new position
    ///
    /// The cursor is not "fused" due to the fact the ghost node makes the
    /// linked list a loop.  After returning None (i.e., you have moved into the
    /// ghost node), the subsequent invocation will return the _tail_ value [if
    /// it exists].
    pub fn prev(&mut self) -> Option<&mut T> {
        // SAFETY: All links are properly aligned because they come from
        // Node::into_link()
        self.cur = unsafe { self.cur.as_ref().prev };
        self.peek_mut()
    }

    /// Remove and return the element at the current position and move the cursor
    /// to the neighboring element that's closest to the back. This can be
    /// either the next or previous position.
    ///
    /// If the removal results in an empty list, the cursor will be moved to the
    /// ghost node.
    pub fn take(&mut self) -> Option<T> {
        let cur = self.cur;
        if unsafe { cur.as_ref().is_ghost } {
            None
        } else {
            // SAFETY: We never release the link/pointer to the data node
            // and current method has &mut to the list => no &mut or & to node
            // exists.
            // The original link came from Node::new_link.
            let node = unsafe { cur.into_boxed_node() };

            // We will drop the boxed node, so make sure everything that used to
            // point it now point to something else

            let mut prev = node.prev;
            let mut next = node.next;

            if cur == self.list.head {
                // head was pointing to boxed node => make it point to next node
                self.list.head = next;
            }
            if cur == self.list.tail {
                // tail was pointing to boxed node => make it point to previous node
                self.list.tail = prev;
            }

            self.cur = if unsafe { next.as_ref().is_ghost } {
                // avoid advancing off list; stay on new tail [unless list has
                // become empty ... in which case either prev or next
                // works as both point to ghost node]
                prev
            } else {
                // advance forward [into non-ghost node]
                next
            };
            self.list.len -= 1;

            let val = Some(node.into_data().expect("node should not be ghost"));

            // ATTN: Must change prev + next at end _after_ we are done accessing "node: Box".
            // This is necessary to avoid UB flagged by miri.

            // SAFETY: There are no other [mut/immut] references to previous Node
            // because
            // * at start of this method take(), we start fresh so no such ref can exist
            unsafe {
                prev.as_mut().next = next;
                next.as_mut().prev = prev;
            }

            val
            // drop(boxed_node);
        }
    }

    /// Insert after current node. Cursor is not moved.
    /// Calling this on the ghost node will insert a new head.
    pub fn insert_after(&mut self, data: T)
    // where
    //     T: Debug,
    {
        // SAFETY: At the start of this method, there no references (&mut/&)
        // to _any_ nodes because:
        // - We have &mut Cursor => no other cursor methods are running
        // - Cursor has &mut LinkedList => no other list methods are running
        //
        // Below, we only need to worry about circular pointers inside this
        // method.  An even then it's only possible when inserting into empty
        // list ... when current node is a self pointing ghost node.

        // SAFETY: Do the next/prev.as_mut() last to avoid overlapping &mut
        // See Node.new_ghost_link() for more details
        let cur_mut: &mut Node<T> = unsafe { self.cur.as_mut() };

        // Point new node to what it is spliced between
        let new_node_link = Node::new_link(data, self.cur, cur_mut.next);

        if cur_mut.next == self.list.head {
            self.list.head = new_node_link;
        }
        if self.cur == self.list.tail {
            self.list.tail = new_node_link;
        }

        // Point formerly adjacent nodes to new node

        // Copy the raw pointer because we need to create &mut from it later
        let mut old_next = cur_mut.next;

        // This is the last access to cur_mut, ending its lifetime
        cur_mut.next = new_node_link;

        // SAFETY: cur_mut is on only other _possible_ &mut to the node, and its
        // lifetime has ended.  We can get a new &mut
        unsafe { old_next.as_mut().prev = new_node_link };

        self.list.len += 1;
    }

    /// Insert before current node. Cursor is not moved.
    /// Calling this on the ghost node will insert a new tail.
    pub fn insert_before(&mut self, data: T)
    // where
    //     T: Debug,
    {
        // SAFETY: At the start of this method, there no references (&mut/&)
        // to _any_ nodes because:
        // - We have &mut Cursor => no other cursor methods are running
        // - Cursor has &mut LinkedList => no other list methods are running
        //
        // Below, we only need to worry about circular pointers inside this
        // method.  An even then it's only possible when inserting into empty
        // list ... when current node is a self pointing ghost node.

        // SAFETY: Do the prev/next.as_mut() last to avoid overlapping &mut
        // See Node.new_ghost_link() for more details
        let cur_mut: &mut Node<T> = unsafe { self.cur.as_mut() };

        // Point new node to what it is spliced between
        let new_node_link = Node::new_link(data, cur_mut.prev, self.cur);

        if cur_mut.prev == self.list.tail {
            self.list.tail = new_node_link;
        }
        if self.cur == self.list.head {
            self.list.head = new_node_link;
        }

        // Point formerly adjacent nodes to new node

        // Copy the raw pointer because we need to create &mut from it later
        let mut old_prev = cur_mut.prev;

        // This is the last access to cur_mut, ending its lifetime
        cur_mut.prev = new_node_link;

        // SAFETY: cur_mut is on only other _possible_ &mut to the node, and its
        // lifetime has ended.  We can get a new &mut
        unsafe { old_prev.as_mut().next = new_node_link };

        self.list.len += 1;
    }

    pub fn seek_forward(&mut self, skip: usize) -> bool {
        let mut cur = self.cur;
        for _ in 0..skip {
            // SAFETY: All links are properly aligned because they come from
            // Node::into_link()
            // Also no other reference exist because we have &mut Cursor and
            // &mut LinkedList
            let cur_ref = unsafe { cur.as_ref() };
            if cur_ref.is_ghost {
                return false;
            }
            cur = cur_ref.next;
        }
        self.cur = cur;
        true
    }

    pub fn seek_backward(&mut self, skip: usize) -> bool {
        let mut cur = self.cur;
        for _ in 0..skip {
            // SAFETY: All links are properly aligned because they come from
            // Node::into_link()
            // Also no other reference exist because we have &mut Cursor and
            // &mut LinkedList
            let cur_ref = unsafe { cur.as_ref() };
            if cur_ref.is_ghost {
                return false;
            }
            cur = cur_ref.prev;
        }
        self.cur = cur;
        true
    }
}

impl<'list, T> Iterator for Iter<'list, T> {
    type Item = &'list T;

    fn next(&mut self) -> Option<&'list T> {
        // SAFETY: All links are properly aligned because they come from
        // Node::into_link()
        // Also no other reference exist because we have &mut Iter and
        // &LinkedList, which also prevents any existing &mut Cursor
        let cur_ref = unsafe { self.cur.as_ref() };
        if cur_ref.is_ghost {
            // NB: Don't advance past ghost node so iterator is fused
            None
        } else {
            // Only advance cursor if not on ghost node
            self.cur = cur_ref.next;

            // SAFETY: Only ghost nodes are missing values
            Some(unsafe { cur_ref.data.assume_init_ref() })
        }
    }
}

// None state is sticky
impl<'list, T> FusedIterator for Iter<'list, T> {}

#[cfg(test)]
mod tests {
    // mod pre_implemented;

    use super::*;

    #[test]
    fn is_generic() {
        struct Foo;
        LinkedList::<Foo>::new();
    }
    // ———————————————————————————————————————————————————————————
    // Tests for Step 1: push / pop at front and back
    // ———————————————————————————————————————————————————————————
    #[test]
    fn basics_empty_list() {
        let list: LinkedList<i32> = LinkedList::new();
        assert_eq!(list.len(), 0);
        assert!(list.is_empty());
    }
    // push / pop at back ————————————————————————————————————————
    #[test]
    fn basics_single_element_back() {
        let mut list: LinkedList<i32> = LinkedList::new();
        list.push_back(5);
        assert_eq!(list.len(), 1);
        assert!(!list.is_empty());
        assert_eq!(list.pop_back(), Some(5));
        assert_eq!(list.len(), 0);
        assert!(list.is_empty());
    }
    #[test]
    fn basics_push_pop_at_back() {
        let mut list: LinkedList<i32> = LinkedList::new();
        for i in 0..10 {
            list.push_back(i);
            assert_eq!(list.len(), i as usize + 1);
            assert!(!list.is_empty());
        }
        for i in (0..10).rev() {
            assert_eq!(list.len(), i as usize + 1);
            assert!(!list.is_empty());
            assert_eq!(i, list.pop_back().unwrap());
        }
        assert_eq!(list.len(), 0);
        assert!(list.is_empty());
    }
    // push / pop at front ———————————————————————————————————————
    #[test]
    fn basics_single_element_front() {
        let mut list: LinkedList<i32> = LinkedList::new();
        list.push_front(5);
        assert_eq!(list.len(), 1);
        assert!(!list.is_empty());
        assert_eq!(list.pop_front(), Some(5));
        assert_eq!(list.len(), 0);
        assert!(list.is_empty());
    }
    #[test]
    fn basics_push_pop_at_front() {
        let mut list: LinkedList<i32> = LinkedList::new();
        for i in 0..10 {
            list.push_front(i);
            assert_eq!(list.len(), i as usize + 1);
            assert!(!list.is_empty());
        }
        for i in (0..10).rev() {
            assert_eq!(list.len(), i as usize + 1);
            assert!(!list.is_empty());
            assert_eq!(i, list.pop_front().unwrap());
        }
        assert_eq!(list.len(), 0);
        assert!(list.is_empty());
    }
    // push / pop at mixed sides —————————————————————————————————
    #[test]
    fn basics_push_front_pop_back() {
        let mut list: LinkedList<i32> = LinkedList::new();
        for i in 0..10 {
            list.push_front(i);
            assert_eq!(list.len(), i as usize + 1);
            assert!(!list.is_empty());
        }
        for i in 0..10 {
            assert_eq!(list.len(), 10 - i as usize);
            assert!(!list.is_empty());
            assert_eq!(i, list.pop_back().unwrap());
        }
        assert_eq!(list.len(), 0);
        assert!(list.is_empty());
    }
    #[test]
    fn basics_push_back_pop_front() {
        let mut list: LinkedList<i32> = LinkedList::new();
        for i in 0..10 {
            list.push_back(i);
            assert_eq!(list.len(), i as usize + 1);
            assert!(!list.is_empty());
        }
        for i in 0..10 {
            assert_eq!(list.len(), 10 - i as usize);
            assert!(!list.is_empty());
            assert_eq!(i, list.pop_front().unwrap());
        }
        assert_eq!(list.len(), 0);
        assert!(list.is_empty());
    }
    // ———————————————————————————————————————————————————————————
    // Tests for Step 2: iteration
    // ———————————————————————————————————————————————————————————
    #[test]
    fn iter() {
        let mut list: LinkedList<i32> = LinkedList::new();
        for num in 0..10 {
            list.push_back(num);
        }
        for (num, &entered_num) in (0..10).zip(list.iter()) {
            assert_eq!(num, entered_num);
        }
    }
    // ———————————————————————————————————————————————————————————
    // Tests for Step 3: full cursor functionality
    // ———————————————————————————————————————————————————————————
    #[test]
    fn cursor_insert_before_on_empty_list() {
        // insert_after on empty list is already tested via push_back()
        let mut list = LinkedList::new();
        list.cursor_front().insert_before(0);
        assert_eq!(Some(0), list.pop_front());
    }
    #[test]
    fn cursor_insert_after_in_middle() {
        let mut list = (0..10).collect::<LinkedList<_>>();
        {
            let mut cursor = list.cursor_front();
            let didnt_run_into_end = cursor.seek_forward(4);
            assert!(didnt_run_into_end);
            for n in (0..10).rev() {
                cursor.insert_after(n);
            }
        }
        assert_eq!(list.len(), 20);
        let expected = (0..5).chain(0..10).chain(5..10);
        assert_eq!(
            expected.collect::<Vec<_>>(),
            list.iter().cloned().collect::<Vec<_>>()
        );
    }
    #[test]
    fn cursor_insert_before_in_middle() {
        let mut list = (0..10).collect::<LinkedList<_>>();
        {
            let mut cursor = list.cursor_back();
            let didnt_run_into_end = cursor.seek_backward(4);
            assert!(didnt_run_into_end);
            for n in 0..10 {
                cursor.insert_before(n);
            }
        }
        assert_eq!(list.len(), 20);
        let expected = (0..5).chain(0..10).chain(5..10);
        assert_eq!(
            expected.collect::<Vec<_>>(),
            list.iter().cloned().collect::<Vec<_>>()
        );
    }
    // "iterates" via next() and checks that it visits the right elements
    #[test]
    fn cursor_next_and_peek() {
        let mut list = (0..10).collect::<LinkedList<_>>();
        let mut cursor = list.cursor_front();
        assert_eq!(cursor.peek_mut(), Some(&mut 0));
        for n in 1..10 {
            let next = cursor.next().cloned();
            assert_eq!(next, Some(n));
            assert_eq!(next, cursor.peek_mut().cloned());
        }
    }
    // "iterates" via prev() and checks that it visits the right elements
    #[test]
    fn cursor_prev_and_peek() {
        let mut list = (0..10).collect::<LinkedList<_>>();
        let mut cursor = list.cursor_back();
        assert_eq!(cursor.peek_mut(), Some(&mut 9));
        for n in (0..9).rev() {
            let prev = cursor.prev().cloned();
            assert_eq!(prev, Some(n));
            assert_eq!(prev, cursor.peek_mut().cloned());
        }
    }
    // removes all elements starting from the middle
    #[test]
    fn cursor_take() {
        let mut list = (0..10).collect::<LinkedList<_>>();
        let mut cursor = list.cursor_front();
        cursor.seek_forward(5);
        for expected in (5..10).chain((0..5).rev()) {
            assert_eq!(cursor.take(), Some(expected));
        }
    }
    // ———————————————————————————————————————————————————————————
    // Tests for Step 4: clean-up via `Drop`
    // ———————————————————————————————————————————————————————————
    // The leak tests that are also for this step are separated into
    // their own files so that nothing else interferes with the allocator
    // whilst they run
    // checks number of drops
    // may pass for incorrect programs if double frees happen
    // exactly as often as destructor leaks
    #[test]
    fn drop_no_double_frees() {
        use std::cell::Cell;
        // #[derive(Debug)]
        struct DropCounter<'a>(&'a Cell<usize>);
        impl Drop for DropCounter<'_> {
            fn drop(&mut self) {
                let num = self.0.get();
                self.0.set(num + 1);
            }
        }
        const N: usize = 15;
        let counter = Cell::new(0);
        let list = std::iter::repeat_with(|| DropCounter(&counter))
            .take(N)
            .collect::<LinkedList<_>>();
        assert_eq!(list.len(), N);
        drop(list);
        assert_eq!(counter.get(), N);
    }
    #[test]
    #[cfg(feature = "slow_tests")]
    fn drop_large_list() {
        drop((0..2_000_000).collect::<LinkedList<i32>>());
    }
    // ———————————————————————————————————————————————————————————
    // Tests for Step 5 (advanced): covariance and Send/Sync
    // ———————————————————————————————————————————————————————————
    // These are compile time tests. They won't compile unless your
    // code passes.
    // Additional tests for code that must *not* compile are in
    // pre_implemented.rs for technical reasons.
    #[test]
    fn advanced_linked_list_is_send_sync() {
        #[allow(unused)]
        trait AssertSend: Send {}
        #[allow(unused)]
        trait AssertSync: Sync {}
        impl<T: Send> AssertSend for LinkedList<T> {}
        impl<T: Sync> AssertSync for LinkedList<T> {}
    }
    #[allow(dead_code)]
    #[test]
    fn advanced_is_covariant() {
        fn a<'a>(x: LinkedList<&'static str>) -> LinkedList<&'a str> {
            x
        }
        fn a_iter<'a>(i: Iter<'static, &'static str>) -> Iter<'a, &'a str> {
            i
        }
    }
}
