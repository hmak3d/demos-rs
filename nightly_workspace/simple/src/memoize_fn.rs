//! [Memoize](https://en.wikipedia.org/wiki/Memoization) a function.
use std::collections::HashMap;
use std::hash::Hash;

pub struct MemoizedFn<F, A, R> {
    /// Underlying function to actually run
    core: F,
    /// Cache of results from previous invocations
    cache: HashMap<A, R>,
}

impl<F, A, R> MemoizedFn<F, A, R>
where
    F: FnOnce(A) -> R,
    A: Clone + Eq + Hash, // so we can do cache lookup, save arg to cache, and give arg to underlying function
    R: Clone,             // so cache can store previous result
{
    pub fn new(core: F) -> Self {
        Self {
            core,
            cache: HashMap::new(),
        }
    }
}

// Override the default Send autotrait impl because it also requires that A + R be Send.
// SAFETY The input and return type of function does not affect
// whether or not the function can go across threads.
unsafe impl<F, A, R> Send for MemoizedFn<F, A, R> where F: Send {}

// Override the default Sync autotrait impl because it also requires that A + R be Sync.
// SAFETY Same reasoning for Send applies to also to Sync
unsafe impl<F, A, R> Sync for MemoizedFn<F, A, R> where F: Sync {}

// Override the default Unpin autotrait impl because it also requires that A + R be Unpin.
// SAFETY Same reasoning for Send applies to also to Unpin
impl<F, A, R> Unpin for MemoizedFn<F, A, R> where F: Unpin {}

impl<F, A, R> FnOnce<(A,)> for MemoizedFn<F, A, R>
where
    F: FnOnce(A) -> R,
    A: Clone + Eq + Hash, // so we can do cache lookup, save arg to cache, and give arg to underlying function
    R: Clone,             // so cache can store previous result
{
    type Output = R;

    /// Run once
    ///
    /// FIXME Make the following doctest pass.
    /// Seems to be some issue where the same struct
    /// cannot implement *both* FnOnce and FnMut w/o
    /// requiring the underlying function \[delegated to\]
    /// be FnMut :(
    /// ```ignore
    /// use simple::memoize_fn::MemoizedFn;
    /// let s = String::from("hi");
    /// let called = false;
    /// let f = move |_a1: i32| {
    ///     drop(s);    // Make closure FnOnce only
    ///     called = true;
    /// };
    /// let mut f2 = MemoizedFn::new(f);
    /// let res = f2(42);
    /// dbg!(res);
    /// assert!(called);
    /// ```
    ///
    /// Cannot decorate functions that take more 1 parameter
    /// ```compile_fail
    /// use simple::memoize_fn::MemoizedFn
    /// let s = String::from("hi");
    /// let f = move |_a1: i32, _a2: i32| {
    ///     drop(s);    // Make closure FnOnce only
    /// };
    /// let mut f2 = MemoizedFn::new(f);
    /// dbg!(f2(42, 43));
    /// ```
    extern "rust-call" fn call_once(self, args: (A,)) -> Self::Output {
        let arg1 = args.0;
        self.cache.get(&arg1).cloned().unwrap_or_else(|| {
            (self.core)(arg1)
            // NB: No point updating cache as self can be re-used
        })
    }
}

impl<F, A, R> FnMut<(A,)> for MemoizedFn<F, A, R>
where
    F: FnMut(A) -> R,
    A: Clone + Eq + Hash, // so we can do cache lookup, save arg to cache, and give arg to underlying function
    R: Clone,             // so cache can store previous result
{
    /// Delegate [multiple times] to underlying fucntion.
    ///
    /// Decorate function passes through FnMut.
    /// If underlying function can be called multiple times, then so can the decorated function.
    /// ```
    /// use simple::memoize_fn::MemoizedFn;
    /// let mut miss_count = 0;
    /// let f = |a1: i32| {
    ///     miss_count += 1;
    ///     a1
    /// };
    /// let mut f2 = MemoizedFn::new(f);
    /// dbg!(f2(42));
    /// dbg!(f2(42));   // 2nd call should work
    /// dbg!(f2(43));   // this should result in cache miss
    /// assert_eq!(miss_count, 2);
    /// ```
    ///
    /// If underlying function can _not_ be called multiple times, nor can the decorated function.
    /// ```compile_fail
    /// use simple::memoize_fn::MemoizedFn;
    /// let s = String::from("hi");
    /// let f = move |_a1: i32| {
    ///     drop(s);    // Make closure FnOnce only
    /// };
    /// let mut f2 = MemoizedFn::new(f);
    /// dbg!(f2(42));
    /// dbg!(f2(42)); // 2nd call won't compile
    /// ```
    extern "rust-call" fn call_mut(&mut self, args: (A,)) -> Self::Output {
        let arg1 = args.0;
        self.cache.get(&arg1).cloned().unwrap_or_else(|| {
            let res = (self.core)(arg1.clone());
            self.cache.insert(arg1, res.clone());
            res
        })
    }
}
