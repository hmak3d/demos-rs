// Use nightly-only feature to silence error in memoize_fn.rs:
//      error[E0183]: manual implementations of `std::ops::FnOnce` are experimental
//        --> simple/src/memoize_fn.rs:21:15
//         |
//      21 | impl<F, A, R> FnOnce<(A,)> for MemoizedFn<F, A, R>
//         |               ^^^^^^^^^^^^ manual implementations of `std::ops::FnOnce` are experimental
//         |
//         = help: add `#![feature(unboxed_closures)]` to the crate attributes to enable
#![feature(unboxed_closures)]

// Use nightly-only feature to silence error in memoize_fn.rs:
//      error[E0658]: use of unstable library feature `fn_traits`
//        --> simple/src/memoize_fn.rs:52:5
//         |
//      52 |     extern "rust-call" fn call_once(mut self, args: (A,)) -> Self::Output {
//         |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
//         |
//         = note: see issue #29625 <https://github.com/rust-lang/rust/issues/29625> for more information
//         = help: add `#![feature(fn_traits)]` to the crate attributes to enable
//         = note: this compiler was built on 2026-03-19; consider upgrading it if it is out of date
#![feature(fn_traits)]

pub mod memoize_fn;
