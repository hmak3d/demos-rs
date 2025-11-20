//! [IntoIterator::into_iter()] for array `[T]` changed in Rust 2021 over previous editions.
//! However, slices `&[T]` have _not_ changed.
//!
//! See Rust 2021 release notes [here](https://doc.rust-lang.org/edition-guide/rust-2021/IntoIterator-for-arrays.html)
//! for details

fn assert_owned(_s: String) {}

fn assert_borrowed(_s: &String) {}

pub fn main() {
    // for an owned array

    let arr = [String::default()];

    for s in arr.iter() {
        // s is &String in all Rust editions
        assert_borrowed(s);
    }

    // NB: Same as: for s in arr { ... }
    for s in arr.into_iter() {
        // s is &String in Rust 2018
        // Following will *not* compile in newer Rust 2021
        // assert_borrowed(s);

        // s is String in Rust 2021
        // Following will *not* compile in older Rust 2018
        assert_owned(s);
    }

    // for slice

    let slice = &[String::default()];

    for s in slice.iter() {
        // s is &String in all Rust editions
        assert_borrowed(s);
    }

    for s in slice.into_iter() {
        // s is &String in all Rust editions
        assert_borrowed(s);
    }

    for s in slice {
        // s is &String in all Rust editions
        assert_borrowed(s);
    }
}
