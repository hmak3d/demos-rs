//! re: https://exercism.org/tracks/rust/exercises/doubly-linked-list/edit
// this module adds some functionality based on the required implementations
// here like: `LinkedList::pop_back` or `Clone for LinkedList<T>`
// You are free to use anything in it, but it's mainly for the test framework.

use std::ptr::NonNull;

type NonNullNodePtr<T> = NonNull<Node<T>>;
type NodePtr<T> = Option<NonNullNodePtr<T>>;

/// For converting non-null Node pointer to a Node (owned or ref).
/// Node ref is useful to access the contents behind the pointer.
/// Owned Node is useful for releasing/dropping resources.
trait ToNodeExt<T> {
    /// Convert [NonNullNodePtr] (aka NodePtr.unwrap()) -> [Box<Node<T>>] (so Box can be dropped)
    ///
    /// Is reverse of [Node::into_node_ptr]
    ///
    /// # Safety
    ///
    /// * The pointer is not concurrently being used like as an argument to [Self::as_node_mut].
    /// * The original pointer must have come from [Node::into_node_ptr].
    unsafe fn into_boxed_node(self) -> Box<Node<T>>;
}

/// Add a conversion method to all non-null Node pointers
impl<T> ToNodeExt<T> for NonNull<Node<T>> {
    unsafe fn into_boxed_node(self) -> Box<Node<T>> {
        // SAFETY: See doc comment for trait
        unsafe { Box::from_raw(self.as_ptr()) }
    }
}

#[derive(Default)]
struct Node<T> {
    data: T,
    next: NodePtr<T>,
    prev: NodePtr<T>,
}

impl<T> Node<T> {
    fn new(data: T) -> Box<Node<T>> {
        Box::new(Self {
            data,
            next: None,
            prev: None,
        })
    }

    /// Convert [Box<Node<T>>] -> [NodePtr]
    ///
    /// Is useful for:
    /// - preventing Box from being automatically dropped (we want to do it manually)
    /// - hiding lifetime checks from borrow check (as we will do this manually) w/ unsafe code
    ///
    /// Is reverse of [ToNode::into_boxed_node]
    ///
    /// The resulting pointer will be properly aligned and non-null,
    /// so `into_node_ptr().unwrap()` will always return [NonNullNodePtr].
    fn into_node_ptr(self: Box<Node<T>>) -> NodePtr<T> {
        // SAFETY: *mut T can't be null because Box::into_raw() isn't
        Some(unsafe { NonNull::new_unchecked(Box::into_raw(self)) })
    }
}

#[derive(Default)]
pub struct LinkedList<T> {
    head: NodePtr<T>,
    tail: NodePtr<T>,
    len: usize,
}

// SAFETY: NonNull<T> is usually !Send but in our case, override this.
// It's okay because:
// 1. there is no aliasing ... we never expose internal pointers; &mut are careful exposed from unsafe blocks
// 2. LinkedList does haven't any funky stuff like Rc and/or interior mutability
unsafe impl<T> Send for LinkedList<T> {}

// SAFETY: NonNull<T> is usually !Sync but in our case, override this.
// It's okay because:
// 1. there is no aliasing ... we never expose internal pointers; &mut are careful exposed from unsafe blocks
// 2. LinkedList does haven't any funky stuff like Rc and/or interior mutability
unsafe impl<T> Sync for LinkedList<T> {}

pub struct Cursor<'list, T> {
    cur: NodePtr<T>,
    list: &'list mut LinkedList<T>,
}

pub struct Iter<'list, T> {
    cur: NodePtr<T>,
    // prevent &mut ref on list while were are iterating
    #[expect(unused)]
    list: &'list LinkedList<T>,
}

impl<T> LinkedList<T> {
    pub fn new() -> Self {
        Self {
            head: None,
            tail: None,
            len: 0,
        }
    }

    // You may be wondering why it's necessary to have is_empty()
    // when it can easily be determined from len().
    // It's good custom to have both because len() can be expensive for some types,
    // whereas is_empty() is almost always cheap.
    // (Also ask yourself whether len() is expensive for LinkedList)
    pub fn is_empty(&self) -> bool {
        self.head.is_none()
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn push_back(&mut self, data: T) {
        self.cursor_back().insert_after(data);
    }

    pub fn pop_back(&mut self) -> Option<T> {
        self.cursor_back().take()
    }

    pub fn push_front(&mut self, data: T) {
        self.cursor_front().insert_before(data);
    }

    pub fn pop_front(&mut self) -> Option<T> {
        self.cursor_front().take()
    }

    /// Return a cursor positioned on the front element
    pub fn cursor_front(&mut self) -> Cursor<'_, T> {
        Cursor {
            cur: self.head,
            list: self,
        }
    }

    /// Return a cursor positioned on the back element
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
        while self.pop_front().is_some() {}
    }
}

impl<T> FromIterator<T> for LinkedList<T> {
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
        let mut non_null_cur = self.cur?;
        // SAFETY: No other &mut Node exists for current node, as to get one
        // one needs &mut Cursor, which no-one else has since we current
        // method has it. No other &Node exists because Iter + Cursor
        // cannot both co-exist.
        Some(&mut unsafe { non_null_cur.as_mut() }.data)
    }

    /// Move one position forward (towards the back) and
    /// return a reference to the new position
    #[expect(
        clippy::should_implement_trait,
        reason = "suppress name of next() warning"
    )]
    pub fn next(&mut self) -> Option<&mut T> {
        let non_null_cur = self.cur.take()?;

        // Advance cursor
        // SAFETY: Notes for peek_mut() also apply
        self.cur = unsafe { non_null_cur.as_ref() }.next;

        let mut non_null_cur = self.cur?;
        // SAFETY: Notes for peek_mut() also apply
        Some(unsafe { &mut non_null_cur.as_mut().data })
    }

    /// Move one position backward (towards the front) and
    /// return a reference to the new position
    pub fn prev(&mut self) -> Option<&mut T> {
        let non_null_cur = self.cur.take()?;

        // Move back cursor
        // SAFETY: Notes for peek_mut() also apply
        self.cur = unsafe { non_null_cur.as_ref() }.prev;

        let mut non_null_cur = self.cur?;
        // SAFETY: Notes for peek_mut() also apply
        Some(unsafe { &mut non_null_cur.as_mut().data })
    }

    /// Remove and return the element at the current position and move the cursor
    /// to the neighboring element that's closest to the back. This can be
    /// either the next or previous position.
    pub fn take(&mut self) -> Option<T> {
        let non_null_cur = self.cur.take()?;

        // SAFETY: There are no other [mut/immut] references to Node
        // because there is no other concurrently running methods from Cursor/Iter/etc
        // since we have &mut Cursor.
        // All NodePtr were produce by Node::into_node_ptr().
        let node = unsafe { non_null_cur.into_boxed_node() };

        // We will drop the boxed node, so make sure everything that used to
        // point it now point to something else

        if let Some(mut prev) = node.prev {
            // SAFETY: There are no other [mut/immut] references to previous Node
            // because
            // * at start of this method take(), we start fresh so no such ref can exist
            // * there are no cycles/loops in the linked list (including loopback pointers)
            unsafe { prev.as_mut().next = node.next };
        }
        if let Some(mut next) = node.next {
            // SAFETY: Notes from node.prev above apply
            unsafe { next.as_mut().prev = node.prev };
        }

        if Some(non_null_cur) == self.list.head {
            // head was pointing to boxed node => make it point to next node
            self.list.head = node.next;
        }
        if Some(non_null_cur) == self.list.tail {
            // tail was pointing to boxed node => make it point to previous node
            self.list.tail = node.prev;
        }

        self.cur = match node.next {
            // Advance cursor forward
            Some(next) => Some(next),
            // Advance cursor backwards when we're at the end
            None => node.prev,
        };

        self.list.len -= 1;

        Some(node.data)
        // drop(boxed_node);
    }

    //
    //
    //   0   1   ....  (n-1)
    // +-------------------------+
    // |   |   |   |   |   |ghost|
    // +-------------------------+
    //                        ^
    //                        cur
    //
    // Cursor is always _on_ an element, with the "ghost" element:
    // - necessary for empty lists
    // - insert_after while on ghost element => insert at front of list
    // - insert_before while on ghost element => insert at back of list

    pub fn insert_after(&mut self, data: T) {
        let mut new_node = Node::new(data);
        let node_ptr = {
            if let Some(mut cur) = self.cur {
                // SAFETY: We are only method that can have &mut Node because we occupy &mut Cursor.
                // All NodePtr were produce by Node::into_node_ptr() so they are properly aligned/etc.
                let cur_mut = unsafe { cur.as_mut() };
                new_node.next = cur_mut.next;
                new_node.prev = self.cur;

                let node_ptr = new_node.into_node_ptr();
                if let Some(mut next) = cur_mut.next {
                    // SAFETY: Above SAFETY notes apply.
                    // Also there are no cycles in the linked list so &mut next-Node
                    // does not overlap with &mut new_node
                    unsafe { next.as_mut().prev = node_ptr };
                }
                cur_mut.next = node_ptr;
                node_ptr
            } else {
                // At "ghost" entry => put new node before head
                new_node.next = self.list.head;
                let node_ptr = new_node.into_node_ptr();
                if let Some(mut head) = self.list.head {
                    // SAFETY: Above SAFETY notes apply.
                    // Also, head is not same as new node so their &ref cannot overlap.
                    unsafe { head.as_mut().prev = node_ptr };
                } else {
                    self.list.head = node_ptr;
                }
                node_ptr
            }
        };
        if self.list.tail.is_none() || self.list.tail == self.cur {
            // insert into empty list or after old tail always sets a new tail
            self.list.tail = node_ptr;
        }
        self.list.len += 1;
    }

    pub fn insert_before(&mut self, data: T) {
        let mut new_node = Node::new(data);
        let node_ptr = {
            if let Some(mut cur) = self.cur {
                // SAFETY: We are only method that can have &mut Node because we occupy &mut Cursor.
                // All NodePtr were produce by Node::into_node_ptr() so they are properly aligned/etc.
                let cur_mut = unsafe { cur.as_mut() };
                new_node.prev = cur_mut.prev;
                new_node.next = self.cur;

                let node_ptr = new_node.into_node_ptr();
                if let Some(mut prev) = cur_mut.prev {
                    // SAFETY: Above SAFETY notes apply.
                    // Also there are no cycles in the linked list so &mut prev-Node
                    // does not overlap with &mut new_node
                    unsafe { prev.as_mut().next = node_ptr };
                }
                cur_mut.prev = node_ptr;
                node_ptr
            } else {
                // At "ghost" entry => put new node after tail
                new_node.prev = self.list.tail;
                let node_ptr = new_node.into_node_ptr();
                if let Some(mut tail) = self.list.tail {
                    // SAFETY: Above SAFETY notes apply.
                    // Also, tail is not same as new node so their &ref cannot overlap.
                    unsafe { tail.as_mut().next = node_ptr };
                } else {
                    self.list.tail = node_ptr;
                }
                node_ptr
            }
        };
        if self.list.head.is_none() || self.list.head == self.cur {
            // insert into empty list or after old head always sets a new head
            self.list.head = node_ptr;
        }
        self.list.len += 1;
    }

    pub fn seek_forward(&mut self, skip: usize) -> bool {
        let mut cur = self.cur;
        for _ in 0..skip {
            match cur {
                Some(non_null_cur) => {
                    // SAFETY: No other [mut/immut] references to Node exist
                    // because we have &mut Cursor
                    cur = unsafe { non_null_cur.as_ref() }.next;
                }
                None => return false,
            }
        }
        self.cur = cur;
        true
    }

    pub fn seek_backward(&mut self, skip: usize) -> bool {
        let mut cur = self.cur;
        for _ in 0..skip {
            match cur {
                Some(non_null_cur) => {
                    // SAFETY: No other [mut/immut] references to Node exist
                    // because we have &mut Cursor
                    cur = unsafe { non_null_cur.as_ref() }.prev
                }
                None => return false,
            }
        }
        self.cur = cur;
        true
    }
}

impl<'list, T> Iterator for Iter<'list, T> {
    type Item = &'list T;

    fn next(&mut self) -> Option<&'list T> {
        let non_null_cur = self.cur.take()?;
        // SAFETY: No mut references to Node exist because we have &mut Iter which
        // occupies a &LinkedList via Iter.list, which prevents Cursor from existing
        let node_ref = unsafe { non_null_cur.as_ref() };
        self.cur = node_ref.next;
        Some(&node_ref.data)
    }
}

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
