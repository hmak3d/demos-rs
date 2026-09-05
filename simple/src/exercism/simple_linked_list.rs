//! https://exercism.org/tracks/rust/exercises/simple-linked-list/edit

type NodePointer<T> = Option<Box<Node<T>>>;

struct Node<T> {
    data: T,
    next: NodePointer<T>,
}

impl<T> Node<T> {
    fn new(data: T, next: NodePointer<T>) -> Self {
        Self { data, next }
    }
}

pub struct SimpleLinkedList<T> {
    head: NodePointer<T>,
}

impl<T> SimpleLinkedList<T> {
    pub fn new() -> Self {
        Self { head: None }
    }

    // You may be wondering why it's necessary to have is_empty()
    // when it can easily be determined from len().
    // It's good custom to have both because len() can be expensive for some types,
    // whereas is_empty() is almost always cheap.
    // (Also ask yourself whether len() is expensive for SimpleLinkedList)
    pub fn is_empty(&self) -> bool {
        self.head.is_none()
    }

    pub fn len(&self) -> usize {
        let mut len = 0;
        let mut cur = &self.head;
        while let Some(next) = cur {
            cur = &next.next;
            len += 1;
        }
        len
    }

    pub fn push(&mut self, element: T) {
        self.head = Some(Box::new(Node::new(element, self.head.take())));
    }

    pub fn pop(&mut self) -> Option<T> {
        let old_head = self.head.take()?;
        self.head = old_head.next;
        Some(old_head.data)
    }

    pub fn peek(&self) -> Option<&T> {
        self.head.as_deref().map(|val: &Node<T>| &val.data)
    }

    #[must_use]
    pub fn rev(mut self) -> SimpleLinkedList<T> {
        let mut res = SimpleLinkedList::new();
        let mut cur = self.head.take();
        while let Some(cur2) = cur {
            res.push(cur2.data);
            cur = cur2.next;
        }
        res
    }
}

impl<T> FromIterator<T> for SimpleLinkedList<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut list = SimpleLinkedList::new();
        for item in iter {
            list.push(item);
        }
        list
    }
}

// In general, it would be preferable to implement IntoIterator for SimpleLinkedList<T>
// instead of implementing an explicit conversion to a vector. This is because, together,
// FromIterator and IntoIterator enable conversion between arbitrary collections.
//
// The reason this exercise's API includes an explicit conversion to Vec<T> instead
// of IntoIterator is that implementing that interface is fairly complicated, and
// demands more of the student than we expect at this point in the track.
//
// Please note that the "front" of the linked list should correspond to the "back"
// of the vector as far as the tests are concerned.

impl<T> From<SimpleLinkedList<T>> for Vec<T> {
    fn from(list: SimpleLinkedList<T>) -> Vec<T> {
        list.into_iter().collect()
    }
}

impl<T> IntoIterator for SimpleLinkedList<T> {
    type Item = T;

    type IntoIter = SimpleLinkedListIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        SimpleLinkedListIter(self.rev())
    }
}

pub struct SimpleLinkedListIter<T>(SimpleLinkedList<T>);

impl<T> Iterator for SimpleLinkedListIter<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.pop()
    }
}

mod tests {
    use super::SimpleLinkedList;

    #[test]
    fn new_list_is_empty() {
        let list: SimpleLinkedList<u32> = SimpleLinkedList::new();
        assert_eq!(list.len(), 0, "list's length must be 0");
    }
    #[test]
    fn push_increments_length() {
        let mut list: SimpleLinkedList<u32> = SimpleLinkedList::new();
        list.push(1);
        assert_eq!(list.len(), 1, "list's length must be 1");
        list.push(2);
        assert_eq!(list.len(), 2, "list's length must be 2");
    }
    #[test]
    fn pop_decrements_length() {
        let mut list: SimpleLinkedList<u32> = SimpleLinkedList::new();
        list.push(1);
        list.push(2);
        list.pop();
        assert_eq!(list.len(), 1, "list's length must be 1");
        list.pop();
        assert_eq!(list.len(), 0, "list's length must be 0");
    }
    #[test]
    fn is_empty() {
        let mut list: SimpleLinkedList<u32> = SimpleLinkedList::new();
        assert!(list.is_empty(), "List wasn't empty on creation");
        for inserts in 0..100 {
            for i in 0..inserts {
                list.push(i);
                assert!(
                    !list.is_empty(),
                    "List was empty after having inserted {i}/{inserts} elements"
                );
            }
            for i in 0..inserts {
                assert!(
                    !list.is_empty(),
                    "List was empty before removing {i}/{inserts} elements"
                );
                list.pop();
            }
            assert!(
                list.is_empty(),
                "List wasn't empty after having removed {inserts} elements"
            );
        }
    }
    #[test]
    fn pop_returns_head_element_and_removes_it() {
        let mut list: SimpleLinkedList<u32> = SimpleLinkedList::new();
        list.push(1);
        list.push(2);
        assert_eq!(list.pop(), Some(2), "Element must be 2");
        assert_eq!(list.pop(), Some(1), "Element must be 1");
        assert_eq!(list.pop(), None, "No element should be contained in list");
    }
    #[test]
    fn peek_returns_reference_to_head_element_but_does_not_remove_it() {
        let mut list: SimpleLinkedList<u32> = SimpleLinkedList::new();
        assert_eq!(list.peek(), None, "No element should be contained in list");
        list.push(2);
        assert_eq!(list.peek(), Some(&2), "Element must be 2");
        assert_eq!(list.peek(), Some(&2), "Element must be still 2");
        list.push(3);
        assert_eq!(list.peek(), Some(&3), "Head element is now 3");
        assert_eq!(list.pop(), Some(3), "Element must be 3");
        assert_eq!(list.peek(), Some(&2), "Head element is now 2");
        assert_eq!(list.pop(), Some(2), "Element must be 2");
        assert_eq!(list.peek(), None, "No element should be contained in list");
    }
    #[test]
    fn from_slice() {
        let mut array = vec!["1", "2", "3", "4"];
        let mut list: SimpleLinkedList<_> = array.drain(..).collect();
        assert_eq!(list.pop(), Some("4"));
        assert_eq!(list.pop(), Some("3"));
        assert_eq!(list.pop(), Some("2"));
        assert_eq!(list.pop(), Some("1"));
    }
    #[test]
    fn reverse() {
        let mut list: SimpleLinkedList<u32> = SimpleLinkedList::new();
        list.push(1);
        list.push(2);
        list.push(3);
        let mut rev_list = list.rev();
        assert_eq!(rev_list.pop(), Some(1));
        assert_eq!(rev_list.pop(), Some(2));
        assert_eq!(rev_list.pop(), Some(3));
        assert_eq!(rev_list.pop(), None);
    }
    #[test]
    fn into_vector() {
        let mut v = Vec::new();
        let mut s = SimpleLinkedList::new();
        for i in 1..4 {
            v.push(i);
            s.push(i);
        }
        let s_as_vec: Vec<i32> = s.into();
        // let s_as_vec: Vec<i32> = s.into_iter().collect();
        assert_eq!(v, s_as_vec);
    }
}
