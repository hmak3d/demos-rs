//! Reimplement Iterator::flatten()
//! Follow along Jon Gjenset ["Crust of Rust: Iterators"](https://www.youtube.com/watch?v=yozQ9C69pNs)

pub struct MyFlatten<I>
where
    I: Iterator,
    I::Item: IntoIterator,
{
    /// outer iterator; used to create inner iterator
    outer: I,

    /// inner iterator; used to produce actual iterator items
    inner: Option<<I::Item as IntoIterator>::IntoIter>,
}

impl<I> MyFlatten<I>
where
    I: Iterator,
    I::Item: IntoIterator,
{
    pub fn new(outer: I) -> Self {
        Self { outer, inner: None }
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
            if let Some(inner_mut) = &mut self.inner {
                if let Some(item) = inner_mut.next() {
                    return Some(item);
                }
                // exhausted inner => Make self.inner = None
                self.inner = None;
            }

            // inner is None => get another one from outer.next()
            let inner = self.outer.next()?;
            self.inner = Some(inner.into_iter());
        }
    }
}

/// [FlatExt] extension trait ... to make decorate all `Iterator<Item = IntoIterator>` to have `my_flatten()`
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
    use super::FlatExt; // Decorator iterator to have Iterator::my_flatten()
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
