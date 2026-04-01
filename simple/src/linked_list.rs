pub struct List<T> {
    head: Link<T>,
}

struct Node<T> {
    data: T,
    next: Link<T>,
}

impl<T> Node<T> {
    fn new(data: T, next: Link<T>) -> Self {
        Self { data, next }
    }
}

type Link<T> = Option<Box<Node<T>>>;

impl<T> List<T> {
    pub fn new() -> Self {
        Self { head: None }
    }

    pub fn push(&mut self, data: T) {
        self.head = Some(Box::new(Node::new(data, self.head.take())));
    }

    pub fn pop(&mut self) -> Option<T> {
        self.head.take().map(|old_head| {
            self.head = old_head.next;
            old_head.data
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn push_and_pop() {
        let mut list = List::new();
        list.push(42);
        list.push(43);
        assert_eq!(list.pop(), Some(43));
        assert_eq!(list.pop(), Some(42));
        assert_eq!(list.pop(), None);
    }
}
