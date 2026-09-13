//! re: https://exercism.org/tracks/rust/exercises/doubly-linked-list/edit
// this module adds some functionality based on the required implementations
// here like: `LinkedList::pop_back` or `Clone for LinkedList<T>`
// You are free to use anything in it, but it's mainly for the test framework.

use std::ptr::NonNull;

type NodePtr<T> = Option<NonNull<Node<T>>>;

#[derive(Default)]
struct Node<T> {
    data: T,
    next: NodePtr<T>,
    prev: NodePtr<T>,
}

impl<T> Node<T> {
    fn new(data: T) -> Self {
        Self {
            data,
            next: None,
            prev: None,
        }
    }

    fn into_node_ptr(self) -> NodePtr<T> {
        // Some(NonNull::from_mut(Box::leak(Box::new(self))))

        // let ptr = NonNull::new(Box::into_raw(Box::new(self)));
        // assert!(ptr.is_some());
        // ptr

        // SAFETY: Box::into_raw() will never return null
        Some(unsafe { NonNull::new_unchecked(Box::into_raw(Box::new(self))) })
    }

    /// # Safety
    ///
    /// Caller must be the only thing pointing to Node
    unsafe fn to_boxed_node(non_null: NonNull<Node<T>>) -> Box<Node<T>> {
        unsafe { Box::from_raw(non_null.as_ptr()) }
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
        while let Some(_) = self.pop_front() {}
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
        let mut non_null = self.cur?;
        Some(&mut unsafe { non_null.as_mut() }.data)
    }

    /// Move one position forward (towards the back) and
    /// return a reference to the new position
    #[expect(
        clippy::should_implement_trait,
        reason = "suppress name of next() warning"
    )]
    pub fn next(&mut self) -> Option<&mut T> {
        let non_null = self.cur.take()?;
        self.cur = unsafe { non_null.as_ref() }.next;
        self.cur.map(|mut x| &mut unsafe { x.as_mut() }.data)
    }

    /// Move one position backward (towards the front) and
    /// return a reference to the new position
    pub fn prev(&mut self) -> Option<&mut T> {
        let non_null = self.cur.take()?;
        self.cur = unsafe { non_null.as_ref() }.prev;
        self.cur.map(|mut x| &mut unsafe { x.as_mut() }.data)
    }

    /// Remove and return the element at the current position and move the cursor
    /// to the neighboring element that's closest to the back. This can be
    /// either the next or previous position.
    pub fn take(&mut self) -> Option<T> {
        let non_null_cur = self.cur.take()?; // BAD
        // SAFETY: ...
        let boxed_node = unsafe { Node::to_boxed_node(non_null_cur) };

        // We will drop the boxed node, so make sure everything that used to
        // point it now point to something else

        if let Some(mut prev) = boxed_node.prev {
            // Make previous node point to next node [to bypass boxed node]
            unsafe { prev.as_mut() }.next = boxed_node.next;
        }
        if let Some(mut next) = boxed_node.next {
            // Make next node point to previous node [to bypass boxed node]
            unsafe { next.as_mut() }.prev = boxed_node.prev;
        }

        if Some(non_null_cur) == self.list.head {
            // head was pointing to boxed node => make it point to next node
            self.list.head = boxed_node.next;
        }
        if Some(non_null_cur) == self.list.tail {
            // tail was pointing to boxed node => make it point to previous node
            self.list.tail = boxed_node.prev;
        }

        self.cur = match boxed_node.next {
            // Advance cursor forward
            Some(next) => Some(next),
            // Advance cursor backwards when we're at the end
            None => boxed_node.prev,
        };

        self.list.len -= 1;

        Some(boxed_node.data)
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
        // want: cur <-> new <-> next
        //
        // cur <-  new
        //
        // if cur exists:
        //              new  -> next
        //      cur  -> new
        //              new <-  next (if cur.next     exists)
        //              new <-  tail (if cur.next not exists)
        //
        // if cur not exists:
        //      want: head -> new -> old_head
        //
        //      new -> old_head
        //
        //      if old_head exists:
        //              new <- old_head
        //      else:
        //              tail -> new
        //
        //      head -> new

        // new stuff always come after current element
        new_node.prev = self.cur;
        let new_node_ptr = match self.cur {
            Some(mut cur) => {
                let old_next = unsafe { cur.as_mut() }.next;
                new_node.next = old_next;
                let new_node_ptr = new_node.into_node_ptr();
                unsafe { cur.as_mut() }.next = new_node_ptr;
                if let Some(mut old_next) = old_next {
                    unsafe { old_next.as_mut() }.prev = new_node_ptr;
                } else {
                    self.list.tail = new_node_ptr;
                }
                new_node_ptr
            }
            None => {
                // on "ghost" element => insert to front of list
                let old_head = self.list.head.take();

                new_node.next = old_head;
                let new_node_ptr = new_node.into_node_ptr();
                self.list.head = new_node_ptr;

                if let Some(mut old_head) = old_head {
                    unsafe { old_head.as_mut() }.prev = new_node_ptr;
                }
                new_node_ptr
            }
        };
        if self.list.tail.is_none() || self.list.tail == self.cur {
            // insert into empty list or after old tail always sets a new tail
            self.list.tail = new_node_ptr;
        }
        self.list.len += 1;
    }

    pub fn insert_before(&mut self, data: T) {
        let mut new_node = Node::new(data);
        // new want: prev <-> new <-> cur
        // prev <-> next
        // head <-> tail
        // cur -> prev
        // next -> cur

        // want: cur <-> new <-> next
        //
        // cur <-  new
        //
        // if cur exists:
        //              new  -> next
        //      cur  -> new
        //              new <-  next (if cur.next     exists)
        //              new <-  tail (if cur.next not exists)
        //
        // if cur not exists:
        //      want: head -> new -> old_head
        //
        //      new -> old_head
        //
        //      if old_head exists:
        //              new <- old_head
        //      else:
        //              tail -> new
        //
        //      head -> new

        // new stuff always come after current element
        new_node.next = self.cur;
        let new_node_ptr = match self.cur {
            Some(mut cur) => {
                let old_prev = unsafe { cur.as_mut() }.prev;
                new_node.prev = old_prev;
                let new_node_ptr = new_node.into_node_ptr();
                unsafe { cur.as_mut() }.prev = new_node_ptr;
                if let Some(mut old_prev) = old_prev {
                    unsafe { old_prev.as_mut() }.next = new_node_ptr;
                } else {
                    self.list.head = new_node_ptr;
                }
                new_node_ptr
            }
            None => {
                // on "ghost" element => insert to front of list
                let old_tail = self.list.tail.take();

                new_node.prev = old_tail;
                let new_node_ptr = new_node.into_node_ptr();
                self.list.tail = new_node_ptr;

                if let Some(mut old_tail) = old_tail {
                    unsafe { old_tail.as_mut() }.next = new_node_ptr;
                }
                new_node_ptr
            }
        };
        if self.list.head.is_none() || self.list.head == self.cur {
            // insert into empty list or before old head always sets a new head
            self.list.head = new_node_ptr;
        }
        self.list.len += 1;
    }

    pub fn seek_forward(&mut self, skip: usize) -> bool {
        let mut cur = self.cur;
        for _ in 0..skip {
            match cur {
                Some(non_null_cur) => cur = unsafe { non_null_cur.as_ref() }.next,
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
                Some(non_null_cur) => cur = unsafe { non_null_cur.as_ref() }.prev,
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
        let non_null = self.cur.take()?;
        let node = unsafe { non_null.as_ref() };
        self.cur = node.next;
        Some(&node.data)
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
        trait AssertSend: Send {}
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
