//! re: <https://exercism.org/tracks/rust/exercises/luhn-from>

use std::marker::PhantomData;
use std::ops::{BitAnd, Div, Rem};

pub struct Luhn<'a, T>(T, PhantomData<&'a T>);

/// We use [u32] because this simplifies `impl IntoDigitsIterator for str`
/// (i.e., [char::to_digit] is [u32]). Otherwise, [u8] would have been a better
/// choice.
type DigitValue = u32;

/// Main type that abstracts the digits that [Luhn] can consume.
///
/// For numeric types collection types ([u8], [u16], [u32], [u64], [usize]),
/// this is used to by `impl_into_luhn` macro to create [NumericIter].
///
/// String types ([str], [String]) have their own implementations.
pub trait IntoDigitsIterator {
    type Error;

    fn iter_digits(&self) -> impl Iterator<Item = Result<DigitValue, Self::Error>>;
}

impl<'a, T> Luhn<'a, T>
where
    T: IntoDigitsIterator,
{
    #[expect(clippy::manual_is_multiple_of)]
    pub fn is_valid(&self) -> bool {
        let mut len = 0;
        let mut sum: DigitValue = 0;
        for (i, val_from) in self.0.iter_digits().enumerate() {
            let Ok(val) = val_from else {
                // IntoDigitsIterator::Error may be:
                // For str: not a digit char
                // For u8, u16, ...: impossible/infallible
                // For u64: <u64 as TryInto>::Error which should *NOT* happen unless there is bug bit arithmetic in NumericIter impl
                // In all these cases, we'll indicate validation failed
                return false;
            };
            let delta = if i % 2 == 0 {
                val
            } else {
                match val {
                    0..=4 => 2 * val,
                    5..=9 => 2 * val - 9,
                    _ => unreachable!(),
                }
            };
            sum += delta as DigitValue;
            len += 1;
        }
        // len > 1 && sum.is_multiple_of(10)
        len > 1 && sum % 10 == 0
    }
}

/// Here is the example of how the From trait could be implemented
/// for the &str type. Naturally, you can implement this trait
/// by hand for every other type presented in the test suite,
/// but your solution will fail if a new type is presented.
/// Perhaps there exists a better solution for this problem?
impl<'a> From<&'a str> for Luhn<'a, &'a str> {
    fn from(input: &'a str) -> Self {
        Luhn(input, PhantomData)
    }
}

/// NB: We implement for both [str] and [&str] so that impl `impl IntoDigitsIterator for String`
/// can call [str::iter_digits] ... having it call `&lt;&amp;str&gt;::iter_digits` directly was not possible.
impl IntoDigitsIterator for str {
    type Error = ();

    fn iter_digits(&self) -> impl Iterator<Item = Result<DigitValue, ()>> {
        self.chars()
            .rev()
            .filter(|ch| !ch.is_whitespace())
            .map(|ch| ch.to_digit(10).ok_or(()))
    }
}

impl IntoDigitsIterator for &str {
    type Error = ();

    fn iter_digits(&self) -> impl Iterator<Item = Result<DigitValue, ()>> {
        str::iter_digits(self)
    }
}

impl<'a> From<String> for Luhn<'a, String> {
    fn from(input: String) -> Self {
        Luhn(input, PhantomData)
    }
}

impl IntoDigitsIterator for String {
    type Error = ();

    fn iter_digits(&self) -> impl Iterator<Item = Result<DigitValue, ()>> {
        str::iter_digits(self)
    }
}

/// Prepare numeric "collection" types into something that [Luhn] can iterator over
/// i.e.,
/// ```ignore
/// impl From<u8> for Luhn { ... }
/// impl IntoDigitsIterator for u8 { ... }
/// ```
/// and repeat for u16, etc.
macro_rules! impl_into_luhn {
    ($($name:tt),+) => {
        $(
            impl<'a> From<$name> for Luhn<'a, $name> {
                fn from(input: $name) -> Self {
                    Luhn(input, PhantomData)
                }
            }

            impl IntoDigitsIterator for $name {
                type Error = <$name as TryInto<DigitValue>>::Error;

                fn iter_digits(&self) -> impl Iterator<Item = Result<DigitValue, Self::Error>> {
                    NumericIter(*self)
                }
            }
        )+
    };
}

impl_into_luhn! {u8, u16, u32, u64, usize}

/// [Iterator] that returns the least significant digits of a non-negative number
pub struct NumericIter<T>(T);

impl<T> Iterator for NumericIter<T>
where
    T: Rem<Output = T> + Div<Output = T> + BitAnd<Output = T> + PartialEq + Copy,
    u8: Into<T>,
    T: TryInto<DigitValue>,
{
    type Item = Result<DigitValue, <T as TryInto<DigitValue>>::Error>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.0 == 0u8.into() {
            None
        } else {
            let rem = self.0 % 10u8.into();
            self.0 = self.0 / 10u8.into();

            // NB: Shouldn't fail because &0xFF makes it < T::MAX
            Some((rem & 0xFFu8.into()).try_into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn you_can_validate_from_a_str() {
        let valid = Luhn::from("046 454 286");
        let invalid = Luhn::from("046 454 287");
        assert!(valid.is_valid());
        assert!(!invalid.is_valid());
    }
    #[test]
    fn you_can_validate_from_a_string() {
        let valid = Luhn::from(String::from("046 454 286"));
        let invalid = Luhn::from(String::from("046 454 287"));
        assert!(valid.is_valid());
        assert!(!invalid.is_valid());
    }
    #[test]
    fn you_can_validate_from_a_u8() {
        let valid = Luhn::from(240u8);
        let invalid = Luhn::from(241u8);
        assert!(valid.is_valid());
        assert!(!invalid.is_valid());
    }
    #[test]
    fn you_can_validate_from_a_u16() {
        let valid = Luhn::from(64_436u16);
        let invalid = Luhn::from(64_437u16);
        assert!(valid.is_valid());
        assert!(!invalid.is_valid());
    }
    #[test]
    fn you_can_validate_from_a_u32() {
        let valid = Luhn::from(46_454_286u32);
        let invalid = Luhn::from(46_454_287u32);
        assert!(valid.is_valid());
        assert!(!invalid.is_valid());
    }
    #[test]
    fn you_can_validate_from_a_u64() {
        let valid = Luhn::from(8273_1232_7352_0562u64);
        let invalid = Luhn::from(8273_1232_7352_0569u64);
        assert!(valid.is_valid());
        assert!(!invalid.is_valid());
    }
    #[test]
    fn you_can_validate_from_a_usize() {
        let valid = Luhn::from(8273_1232_7352_0562usize);
        let invalid = Luhn::from(8273_1232_7352_0569usize);
        assert!(valid.is_valid());
        assert!(!invalid.is_valid());
    }
    #[test]
    fn single_digit_string_is_invalid() {
        assert!(!Luhn::from("1").is_valid());
    }
    #[test]
    fn single_zero_string_is_invalid() {
        assert!(!Luhn::from("0").is_valid());
    }
    #[test]
    fn valid_canadian_sin_is_valid() {
        assert!(Luhn::from("046 454 286").is_valid());
    }
    #[test]
    fn invalid_canadian_sin_is_invalid() {
        assert!(!Luhn::from("046 454 287").is_valid());
    }
    #[test]
    fn invalid_credit_card_is_invalid() {
        assert!(!Luhn::from("8273 1232 7352 0569").is_valid());
    }
    #[test]
    fn strings_that_contain_non_digits_are_invalid() {
        assert!(!Luhn::from("046a 454 286").is_valid());
    }
    #[test]
    fn input_digit_9_is_still_correctly_converted_to_output_digit_9() {
        assert!(Luhn::from("091").is_valid());
    }
}
