//! [Memoize](https://en.wikipedia.org/wiki/Memoization) a function.

// Use nightly-only feature to silence error:
//      error[E0183]: manual implementations of `std::ops::FnOnce` are experimental
//        --> simple/src/bin/memoize_fn.rs:45:15
//         |
//      45 | impl<F, A, R> FnOnce<(A,)> for MemoizedFn<F, A, R>
//         |               ^^^^^^^^^^^^ manual implementations of `std::ops::FnOnce` are experimental
//         |
//         = help: add `#![feature(unboxed_closures)]` to the crate attributes to enable
#![feature(unboxed_closures)]

// Use nightly-only feature to silence error:
//      error[E0658]: use of unstable library feature `fn_traits`
//        --> simple/src/bin/memoize_fn.rs:54:5
//         |
//      54 |     extern "rust-call" fn call_once(mut self, args: (A,)) -> Self::Output {
//         |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
//         |
//         = note: see issue #29625 <https://github.com/rust-lang/rust/issues/29625> for more information
//         = help: add `#![feature(fn_traits)]` to the crate attributes to enable
//         = note: this compiler was built on 2026-03-19; consider upgrading it if it is out of date
#![feature(fn_traits)]

use std::collections::HashMap;
use std::hash::Hash;

pub struct MemoizedFn<F, A, R> {
    /// Underlying function to actually run
    core: F,
    /// Cache of results from previous invocations
    cache: HashMap<A, R>,
}

impl<F, A, R> MemoizedFn<F, A, R>
{
    fn new(core: F) -> Self {
        Self {
            core,
            cache: HashMap::new(),
        }
    }
}

impl<F, A, R> FnOnce<(A,)> for MemoizedFn<F, A, R>
where
    F: FnMut(A) -> R,
    A: Clone + Eq + Hash, // so we can do cache lookup
    A: Clone,             // so argument can be saved in cache *and* given to underlying function
    R: Clone,             // so cache can store previous result
{
    type Output = R;

    extern "rust-call" fn call_once(mut self, args: (A,)) -> Self::Output {
        MemoizedFn::invoke(&mut self, args.0)
    }
}

impl<F, A, R> FnMut<(A,)> for MemoizedFn<F, A, R>
where
    F: FnMut(A) -> R,
    A: Clone + Eq + Hash,
    A: Clone,
    R: Clone,
{
    extern "rust-call" fn call_mut(&mut self, args: (A,)) -> Self::Output {
        self.invoke(args.0)
    }
}

impl<F, A, R> MemoizedFn<F, A, R>
where
    F: FnMut(A) -> R,
    A: Clone + Eq + Hash,
    A: Clone,
    R: Clone,
{
    /// Call the "function"
    fn invoke(&mut self, arg1: A) -> R {
        self.cache.get(&arg1).cloned().unwrap_or_else(|| {
            let res = (self.core)(arg1.clone());
            self.cache.insert(arg1, res.clone());
            res
        })
    }
}

pub fn main() {
    let mut miss_count = 0;
    let f = |a1: i32| {
        miss_count += 1;
        a1
    };

    let mut f2 = MemoizedFn::new(f);
    dbg!(f2(42));
    dbg!(f2(42));
    dbg!(f2(42));

    assert_eq!(miss_count, 1);
}
