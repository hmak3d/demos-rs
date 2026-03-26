//! Anonymous lifetime has different meaning depending on position
//!
//! `&'_` in argument position means a one-off lifetime (unrelated to other arguments).
//!
//! `&'_` in return position means use the "obvious" choice from the input arguments.
//!
//! Note: The cargo doc will normalize function signatures and *not* render the `&'_` in them.
//! Read the source to understand the comments better.

#![allow(unused, clippy::needless_lifetimes)]

/// Scenario #1
fn f1(a1: &str, a2: &str) {}

/// Same but with explicit lifetimes
fn f2<'a, 'b>(a1: &'a str, a2: &'b str) {}

/// Same but with anonymous lifetime
///
/// `&'_` in argument position means a one-off lifetime (unrelated to other arguments).
fn f3(a1: &'_ str, a2: &'_ str) {}

/// Scenario #2
fn f4(a1: &str) -> &str {
    a1
}

/// Same but with explicit lifetimes
fn f5<'a>(a1: &'a str) -> &'a str {
    a1
}

/// Same but with anonymous lifetime
/// Here the obvious choice is the 1 input argument
fn f6(a1: &str) -> &'_ str {
    a1
}

struct MyStruct<'a>(&'a str);

impl<'a> MyStruct<'a> {
    /// Scenario #3
    fn f7(&self, a2: &str) -> &str {
        self.0
    }

    /// Same but with explicit lifetimes
    fn f8<'b, 'c>(&'b self, a2: &'c str) -> &'b str {
        self.0
    }

    /// Same but with anonymous lifetime
    ///
    /// Sometimes the "obvious" choice does not match what the notation may suggest.
    ///
    /// Here:
    /// * `&'_` in argument position means a one-off lifetime (unrelated to other arguments).
    /// * `&'_` in return position means use the "obvious" choice from the input arguments.
    ///   In this case, it is `self`.
    ///
    /// So `&'_` in argument position is **NOT** the same as `&'_` in return position!
    fn f9(&self, a2: &'_ str) -> &'_ str {
        self.0
    }
}
