//! Code for https://exercism.org/tracks/rust/exercises/fizzy/edit
//!
//! A Matcher is a single rule of fizzbuzz: given a function on T, should
//! a word be substituted in? If yes, which word?

// pub struct Matcher<T>(Fn(T) -> Option<&'static str>);
pub struct Matcher<'a, T>(Box<dyn Fn(T) -> Option<&'a str> + 'a>);

impl<'a, T> Matcher<'a, T> {
    pub fn new<F>(is_match: F, replacement: &'a str) -> Matcher<'a, T>
    where
        F: Fn(T) -> bool,
        F: 'a,
    {
        Self(Box::new(move |val: T| is_match(val).then_some(replacement)))
    }
}

/// A Fizzy is a set of matchers, which may be applied to an iterator.
///
/// Strictly speaking, it's usually more idiomatic to use `iter.map()` than to
/// consume an iterator with an `apply` method. Given a Fizzy instance, it's
/// pretty straightforward to construct a closure which applies it to all
/// elements of the iterator. However, we're using the `apply` pattern
/// here because it's a simpler interface for students to implement.
///
/// Also, it's a good excuse to try out using impl trait.
pub struct Fizzy<'a, T>
where
    T: Copy,
{
    /// matcher rules to apply
    matchers: Vec<Matcher<'a, T>>,
}

impl<'a, T> Fizzy<'a, T>
where
    T: ToString + Copy,
{
    #[expect(clippy::new_without_default)] // Don't complain that Default should be impl for Fizzy
    pub fn new() -> Self {
        Self {
            matchers: Vec::new(),
        }
    }

    // feel free to change the signature to `mut self` if you like
    #[must_use]
    pub fn add_matcher(self, matcher: Matcher<'a, T>) -> Self {
        let mut matchers = self.matchers;
        matchers.push(matcher);
        Self { matchers }
    }

    /// map this fizzy onto every element of an iterator, returning a new iterator
    pub fn apply<I>(self, src_iter: I) -> impl Iterator<Item = String>
    where
        I: Iterator<Item = T>,
    {
        // "move" for self.matchers
        src_iter.map(move |item| {
            self.matchers
                .iter()
                .filter_map(|f| f.0(item))
                .map(String::from)
                .reduce(|mut accum: String, cur: String| -> String {
                    accum.push_str(&cur);
                    accum
                })
                .unwrap_or_else(|| item.to_string())
        })
    }
}

/// convenience function: return a Fizzy which applies the standard fizz-buzz rules
pub fn fizz_buzz<T>() -> Fizzy<'static, T>
where
    T: std::ops::Rem<T> + Copy,
    <T as std::ops::Rem<T>>::Output: PartialEq<T>,
    T: ToString + From<u8>,
{
    Fizzy::new()
        .add_matcher(Matcher::new(
            |val: T| (val % T::from(3u8)) == T::from(0u8),
            "fizz",
        ))
        .add_matcher(Matcher::new(
            |val: T| (val % T::from(5u8)) == T::from(0u8),
            "buzz",
        ))
}

#[cfg(test)]
mod tests {
    use super::{Fizzy, Matcher, fizz_buzz};
    #[test]
    fn simple() {
        let actual = fizz_buzz::<i32>().apply(1..=16).collect::<Vec<_>>();
        let expected = [
            "1", "2", "fizz", "4", "buzz", "fizz", "7", "8", "fizz", "buzz", "11", "fizz", "13",
            "14", "fizzbuzz", "16",
        ];
        assert_eq!(actual, expected);
    }
    #[test]
    fn u8() {
        let actual = fizz_buzz::<u8>().apply(1_u8..=16).collect::<Vec<_>>();
        let expected = [
            "1", "2", "fizz", "4", "buzz", "fizz", "7", "8", "fizz", "buzz", "11", "fizz", "13",
            "14", "fizzbuzz", "16",
        ];
        assert_eq!(actual, expected);
    }
    #[test]
    fn u64() {
        let actual = fizz_buzz::<u64>().apply(1_u64..=16).collect::<Vec<_>>();
        let expected = [
            "1", "2", "fizz", "4", "buzz", "fizz", "7", "8", "fizz", "buzz", "11", "fizz", "13",
            "14", "fizzbuzz", "16",
        ];
        assert_eq!(actual, expected);
    }
    #[test]
    fn nonsequential() {
        let collatz_12 = &[12, 6, 3, 10, 5, 16, 8, 4, 2, 1];
        let actual = fizz_buzz::<i32>()
            .apply(collatz_12.iter().cloned())
            .collect::<Vec<_>>();
        let expected = vec![
            "fizz", "fizz", "fizz", "buzz", "buzz", "16", "8", "4", "2", "1",
        ];
        assert_eq!(actual, expected);
    }
    #[test]
    fn custom() {
        let expected = vec![
            "1", "2", "Fizz", "4", "Buzz", "Fizz", "Bam", "8", "Fizz", "Buzz", "11", "Fizz", "13",
            "Bam", "BuzzFizz", "16",
        ];
        let fizzer: Fizzy<i32> = Fizzy::new()
            .add_matcher(Matcher::new(|n: i32| n % 5 == 0, "Buzz"))
            .add_matcher(Matcher::new(|n: i32| n % 3 == 0, "Fizz"))
            .add_matcher(Matcher::new(|n: i32| n % 7 == 0, "Bam"));
        let actual = fizzer.apply(1..=16).collect::<Vec<_>>();
        assert_eq!(actual, expected);
    }
    #[test]
    fn f64() {
        // a tiny bit more complicated becuase range isn't natively implemented on floats
        let actual = fizz_buzz::<f64>()
            .apply(std::iter::successors(Some(1.0), |prev| Some(prev + 1.0)))
            .take(16)
            .collect::<Vec<_>>();
        let expected = [
            "1", "2", "fizz", "4", "buzz", "fizz", "7", "8", "fizz", "buzz", "11", "fizz", "13",
            "14", "fizzbuzz", "16",
        ];
        assert_eq!(actual, expected);
    }
    #[test]
    fn minimal_generic_bounds() {
        use std::fmt;
        use std::ops::{Add, Rem};
        #[derive(Clone, Copy, Debug, Default, PartialEq)]
        struct Fizzable(u8);
        impl From<u8> for Fizzable {
            fn from(i: u8) -> Fizzable {
                Fizzable(i)
            }
        }
        impl fmt::Display for Fizzable {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                let Fizzable(n) = self;
                write!(f, "{n}")
            }
        }
        impl Add for Fizzable {
            type Output = Fizzable;
            fn add(self, rhs: Fizzable) -> Fizzable {
                let Fizzable(n1) = self;
                let Fizzable(n2) = rhs;
                Fizzable(n1 + n2)
            }
        }
        impl Rem for Fizzable {
            type Output = Fizzable;
            fn rem(self, rhs: Fizzable) -> Fizzable {
                let Fizzable(n1) = self;
                let Fizzable(n2) = rhs;
                Fizzable(n1 % n2)
            }
        }
        let actual = fizz_buzz::<Fizzable>()
            .apply(std::iter::successors(Some(Fizzable(1)), |prev| {
                Some(*prev + 1.into())
            }))
            .take(16)
            .collect::<Vec<_>>();
        let expected = [
            "1", "2", "fizz", "4", "buzz", "fizz", "7", "8", "fizz", "buzz", "11", "fizz", "13",
            "14", "fizzbuzz", "16",
        ];
        assert_eq!(actual, expected);
    }
}
