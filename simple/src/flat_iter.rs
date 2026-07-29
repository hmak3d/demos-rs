//! Reimplement Iterator::flatten()
//! Follow along Jon Gjengset ["Crust of Rust: Iterators"](https://www.youtube.com/watch?v=yozQ9C69pNs)

use std::iter::Fuse;

pub struct MyFlatten<I>
where
    I: Iterator,
    I::Item: IntoIterator,
{
    /// outer iterator; used to create inner iterator
    outer: Fuse<I>,

    /// inner front iterator; used to produce actual iterator items going forward
    front_inner: Option<<I::Item as IntoIterator>::IntoIter>,

    /// inner back iterator; used to produce actual iterator items from backwards
    back_inner: Option<<I::Item as IntoIterator>::IntoIter>,
}

impl<I> MyFlatten<I>
where
    I: Iterator,
    I::Item: IntoIterator,
{
    pub fn new(outer: I) -> Self {
        Self {
            outer: outer.fuse(),
            front_inner: None,
            back_inner: None,
        }
    }
}

impl<I> Iterator for MyFlatten<I>
where
    I: Iterator,
    I::Item: IntoIterator,
{
    type Item = <I::Item as IntoIterator>::Item;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            // consume inner iter
            if let Some(inner_mut) = &mut self.front_inner {
                if let Some(item) = inner_mut.next() {
                    return Some(item);
                }
                // exhausted front inner => Make self.front_inner = None
                self.front_inner = None;
            }

            // front inner is None => get another one from outer.next(), or when exhausted, then steal back_inner
            let inner = match self.outer.next() {
                // NB: Moving back_inner -> front_inner means that thrashing can occur when alternating between next()/next_back()
                None => Some(self.back_inner.take()?),
                Some(inner) => Some(inner.into_iter()),
            };
            self.front_inner = inner;
        }
    }
}

impl<I> DoubleEndedIterator for MyFlatten<I>
where
    I: DoubleEndedIterator,
    I::Item: IntoIterator,
    <I::Item as IntoIterator>::IntoIter: DoubleEndedIterator,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        loop {
            // consume inner iter
            if let Some(inner_mut) = &mut self.back_inner {
                if let Some(item) = inner_mut.next_back() {
                    return Some(item);
                }
                // exhausted back inner => Make self.back_inner = None
                self.back_inner = None;
            }

            // back inner is None => get another one from outer.next(), or when exhausted, then steal front_inner
            let inner = match self.outer.next_back() {
                // NB: Moving front_inner -> back_inner means that thrashing can occur when alternating between next()/next_back()
                None => Some(self.front_inner.take()?),
                Some(inner) => Some(inner.into_iter()),
            };
            self.back_inner = inner;
        }
    }
}

/// [FlatExt] extension trait ... to decorate iterator to have Iterator::my_flatten() method
pub trait FlatExt: Iterator {
    fn my_flatten(self) -> MyFlatten<Self>
    where
        Self::Item: IntoIterator,
        Self: Sized;
}

impl<T> FlatExt for T
where
    T: Iterator,
    T::Item: IntoIterator,
{
    fn my_flatten(self) -> MyFlatten<Self>
    where
        Self::Item: IntoIterator,
        Self: Sized,
    {
        MyFlatten::new(self)
    }
}

#[cfg(test)]
mod test {
    mod test_forward {
        use super::super::FlatExt; // Decorate iterator to have Iterator::my_flatten()
        use std::iter;

        #[test]
        fn test_empty_outer() {
            assert_eq!(iter::empty::<Vec<Vec<i32>>>().my_flatten().next(), None);
        }

        #[test]
        fn test_empty_inner() {
            assert_eq!(
                iter::once(iter::empty::<Vec<i32>>()).my_flatten().next(),
                None
            );
        }

        #[test]
        fn test_single() {
            assert_eq!(
                &vec![vec![1]].into_iter().my_flatten().collect::<Vec<_>>(),
                &[1]
            );
        }

        #[test]
        fn test_single_outer_multi_inner() {
            assert_eq!(
                &vec![vec![1, 2]]
                    .into_iter()
                    .my_flatten()
                    .collect::<Vec<_>>(),
                &[1, 2]
            );
        }

        #[test]
        fn test_multi_outer_single_inner() {
            assert_eq!(
                &vec![vec![1], vec![2]]
                    .into_iter()
                    .my_flatten()
                    .collect::<Vec<_>>(),
                &[1, 2]
            );
        }

        #[test]
        fn test_multi_outer_multi_inner() {
            assert_eq!(
                &vec![vec![1, 2], vec![3, 4]]
                    .into_iter()
                    .my_flatten()
                    .collect::<Vec<_>>(),
                &[1, 2, 3, 4]
            );
        }
    }

    mod test_double {
        use super::super::FlatExt; // Decorate iterator to have Iterator::my_flatten()
        use std::iter;

        #[test]
        fn test_empty_outer() {
            let mut iter = iter::empty::<Vec<Vec<i32>>>().my_flatten();
            assert_eq!(iter.next(), None);
            assert_eq!(iter.next_back(), None);

            // reversing access should not matter
            let mut iter = iter::empty::<Vec<Vec<i32>>>().my_flatten();
            assert_eq!(iter.next_back(), None);
            assert_eq!(iter.next(), None);
        }

        #[test]
        fn test_empty_inner() {
            let mut iter = iter::once(iter::empty::<Vec<i32>>()).my_flatten();
            assert_eq!(iter.next(), None);
            assert_eq!(iter.next_back(), None);

            // reversing access should not matter
            let mut iter = iter::once(iter::empty::<Vec<i32>>()).my_flatten();
            assert_eq!(iter.next_back(), None);
            assert_eq!(iter.next(), None);
        }

        #[test]
        fn test_single() {
            let mut iter = vec![vec![1]].into_iter().my_flatten();
            assert_eq!(iter.next(), Some(1));
            assert_eq!(iter.next(), None);
            assert_eq!(iter.next_back(), None);

            // reversing access should not matter
            let mut iter = vec![vec![1]].into_iter().my_flatten();
            assert_eq!(iter.next_back(), Some(1));
            assert_eq!(iter.next_back(), None);
            assert_eq!(iter.next(), None);
        }

        #[test]
        fn test_single_outer_multi_inner() {
            let mut iter = vec![vec![1, 2]].into_iter().my_flatten();
            assert_eq!(iter.next(), Some(1));
            assert_eq!(iter.next_back(), Some(2));
            assert_eq!(iter.next(), None);
            assert_eq!(iter.next_back(), None);

            let mut iter = vec![vec![1, 2]].into_iter().my_flatten();
            assert_eq!(iter.next(), Some(1));
            assert_eq!(iter.next(), Some(2));
            assert_eq!(iter.next_back(), None);
            assert_eq!(iter.next(), None);

            let mut iter = vec![vec![1, 2]].into_iter().my_flatten();
            assert_eq!(iter.next_back(), Some(2));
            assert_eq!(iter.next_back(), Some(1));
            assert_eq!(iter.next(), None);
            assert_eq!(iter.next_back(), None);
        }

        #[test]
        fn test_multi_outer_single_inner() {
            let mut iter = vec![vec![1], vec![2]].into_iter().my_flatten();
            assert_eq!(iter.next(), Some(1));
            assert_eq!(iter.next_back(), Some(2));
            assert_eq!(iter.next(), None);
            assert_eq!(iter.next_back(), None);

            let mut iter = vec![vec![1], vec![2]].into_iter().my_flatten();
            assert_eq!(iter.next_back(), Some(2));
            assert_eq!(iter.next(), Some(1));
            assert_eq!(iter.next_back(), None);
            assert_eq!(iter.next(), None);
        }

        #[test]
        fn test_multi_outer_multi_inner() {
            let mut iter = vec![vec![1, 2], vec![3, 4]].into_iter().my_flatten();
            assert_eq!(iter.next(), Some(1));
            assert_eq!(iter.next_back(), Some(4));
            assert_eq!(iter.next(), Some(2));
            assert_eq!(iter.next_back(), Some(3));
            assert_eq!(iter.next(), None);
            assert_eq!(iter.next_back(), None);

            let mut iter = vec![vec![1, 2], vec![3, 4]].into_iter().my_flatten();
            assert_eq!(iter.next_back(), Some(4));
            assert_eq!(iter.next_back(), Some(3));
            assert_eq!(iter.next_back(), Some(2));
            assert_eq!(iter.next_back(), Some(1));
            assert_eq!(iter.next_back(), None);
            assert_eq!(iter.next(), None);
        }
    }
}
