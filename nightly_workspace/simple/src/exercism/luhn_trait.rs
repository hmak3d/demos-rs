//! re: <https://exercism.org/tracks/rust/exercises/luhn-trait>

use std::ops::{Div, Rem};

/// We use [u32] because this simplifies `impl IntoDigitsIterator for str`
/// (i.e., [char::to_digit] is [u32]). Otherwise, [u8] would have been a better
/// choice.
type DigitValue = u32;

pub trait Luhn {
    type DigitError;

    #[expect(clippy::manual_is_multiple_of)]
    fn valid_luhn(&self) -> bool {
        let mut len: usize = 0;
        let mut sum: u32 = 0;
        for (i, digit_from) in self.iter_digits().enumerate() {
            let Ok(digit) = digit_from else {
                // Cannot produce more digits => fail the validation
                return false;
            };
            let delta = if i % 2 == 0 {
                digit
            } else {
                // "double" ever other digit
                match digit {
                    0..=4 => 2 * digit,
                    5..=9 => 2 * digit - 9,
                    _ => unreachable!(),
                }
            };
            // Prevent panic.
            // Wrapping add losing overflow is okay because that does not affect "% 10 == 0" check
            sum = sum.wrapping_add(delta);
            len += 1;
        }
        // len > 1 && sum.is_multiple_of(10)
        len > 1 && sum % 10 == 0
    }

    /// Main method that abstracts the digits that [Luhn] can consume
    ///
    /// For numeric types collection types ([u8], [u16], [u32], [u64], [usize]),
    /// this is used to by `impl_into_luhn` macro to create [NumericIterator].
    ///
    /// String types ([str], [String]) have their own implementations.
    fn iter_digits(&self) -> impl Iterator<Item = Result<DigitValue, Self::DigitError>>;
}

/// NB: We implement [Luhn] for both [str] and [&str].
/// - [str] so that `impl Luhn for String` can call [str::iter_digits]
/// - [&str] so that Luhn::from("123") compiles
impl Luhn for str {
    type DigitError = ();

    fn iter_digits(&self) -> impl Iterator<Item = Result<DigitValue, Self::DigitError>> {
        self.chars()
            .rev()
            .filter(|ch| !ch.is_whitespace())
            .map(|ch| ch.to_digit(10).ok_or(()))
    }
}

impl Luhn for &str {
    type DigitError = ();

    fn iter_digits(&self) -> impl Iterator<Item = Result<DigitValue, Self::DigitError>> {
        str::iter_digits(*self)
    }
}

impl Luhn for String {
    type DigitError = ();

    fn iter_digits(&self) -> impl Iterator<Item = Result<DigitValue, Self::DigitError>> {
        str::iter_digits(self)
    }
}

/// Implement [Luhn] for numeric "collection" types
/// i.e.,
/// ```ignore
/// impl Luhn for u8 {
///     type DigitError = <u8 as TryInto<DigitValue>>::Error;
///
///     // NB: `fn valid_luhn(&self) -> bool` is inherited from Luhn trait
///
///     fn iter_digits(&self) -> impl Iterator<Item = Result<DigitValue, Self::DigitError>> {
///         NumericIterator(*self)
///     }
/// }
/// ```
/// and repeat for u16, etc.
macro_rules! impl_numeric_luhn {
    ($($name:ty),+) => {
        $(
            impl Luhn for $name {
                type DigitError = <$name as TryInto<DigitValue>>::Error;

                fn iter_digits(&self) -> impl Iterator<Item = Result<DigitValue, Self::DigitError>> {
                    NumericIterator(*self)
                }
            }
        )*
    };
}

impl_numeric_luhn! {u8, u16, u32, u64, usize}

/// [Iterator] that returns the least significant digits of a non-negative number
pub struct NumericIterator<T>(T);

impl<T> Iterator for NumericIterator<T>
where
    T: PartialEq + Rem<Output = T> + Div<Output = T> + Copy + TryInto<DigitValue>,
    u8: Into<T>,
{
    type Item = Result<DigitValue, <T as TryInto<DigitValue>>::Error>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.0 == 0.into() {
            return None;
        }
        let rem = self.0 % 10u8.into();
        self.0 = self.0 / 10u8.into();

        // NB: Don't expect to result in Err because 10u8 < T::MAX
        Some(rem.try_into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn you_can_validate_from_a_str() {
        assert!("046 454 286".valid_luhn());
        assert!(!"046 454 287".valid_luhn());
    }
    #[test]
    fn you_can_validate_from_a_string() {
        assert!(String::from("046 454 286").valid_luhn());
        assert!(!String::from("046 454 287").valid_luhn());
    }
    #[test]
    fn you_can_validate_from_a_u8() {
        assert!(240u8.valid_luhn());
        assert!(!241u8.valid_luhn());
    }
    #[test]
    fn you_can_validate_from_a_u16() {
        let valid = 64_436u16;
        let invalid = 64_437u16;
        assert!(valid.valid_luhn());
        assert!(!invalid.valid_luhn());
    }
    #[test]
    fn you_can_validate_from_a_u32() {
        let valid = 46_454_286u32;
        let invalid = 46_454_287u32;
        assert!(valid.valid_luhn());
        assert!(!invalid.valid_luhn());
    }
    #[test]
    fn you_can_validate_from_a_u64() {
        let valid = 8273_1232_7352_0562u64;
        let invalid = 8273_1232_7352_0569u64;
        assert!(valid.valid_luhn());
        assert!(!invalid.valid_luhn());
    }
    #[test]
    fn you_can_validate_from_a_usize() {
        let valid = 8273_1232_7352_0562usize;
        let invalid = 8273_1232_7352_0569usize;
        assert!(valid.valid_luhn());
        assert!(!invalid.valid_luhn());
    }
    #[test]
    fn input_digit_9_is_still_correctly_converted_to_output_digit_9() {
        assert!("091".valid_luhn());
    }
}
