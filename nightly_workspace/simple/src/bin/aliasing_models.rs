//! Explore aliasing models for unsafe Rust
//! (i.e., the rules for avoiding undefined behavior during pointer manipulations in `unsafe` code)
//!
//! * re: newer [Tree Borrows paper](https://perso.crans.org/vanille/treebor/aux/preprint.pdf) (2025)
//! * re: older [Stacked Borrows paper](https://doi.org/10.1145/3371109) (2020)
//! * re: [UB accross FFI boundaries paper](https://arxiv.org/pdf/2404.11671>) (2025)

#![allow(unused)]

mod tree_borrows_ok_but_not_stacked_borrows {
    fn write(x: &mut i32) {
        *x = 42;
    }

    /// This is okay under both Tree Borrows and Stacked Borrows
    ///
    /// See Example 4 in "Tree Borrows paper"
    pub fn test1_ok_all() {
        let x = &mut 0;
        let y = x as *mut _;
        write(x);
        unsafe { *y = 24 }
    }

    /// This is okay under Tree Borrows but not under Stacked Borrows.
    /// This is despite the fact that [test2_tree_borrow_ok_stacked_borrow_bad()] is logically equivalent
    /// to [test1_ok_all()]!
    ///
    /// ```sh
    /// cargo miri run --bin aliasing_models
    ///     Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.00s
    ///      Running `/Users/ho32745/.rustup/toolchains/nightly-2026-03-20-aarch64-apple-darwin/bin/cargo-miri runner target/miri/aarch64-apple-darwin/debug/aliasing_models`
    /// error: Undefined Behavior: attempting a write access using <551> at alloc198[0x0], but that tag does not exist in the borrow stack for this location
    ///   --> simple/src/bin/aliasing_models.rs:72:18
    ///    |
    /// 72 |         unsafe { *y = 24 }
    ///    |                  ^^^^^^^ this error occurs as part of an access at alloc198[0x0..0x4]
    ///    |
    ///    = help: this indicates a potential bug in the program: it performed an invalid operation, but the Stacked Borrows rules it violated are still experimental
    ///    = help: see https://github.com/rust-lang/unsafe-code-guidelines/blob/master/wip/stacked-borrows.md for further information
    /// help: <551> was created by a SharedReadWrite retag at offsets [0x0..0x4]
    ///   --> simple/src/bin/aliasing_models.rs:70:17
    ///    |
    /// 70 |         let y = x as *mut _;
    ///    |                 ^
    /// help: <551> was later invalidated at offsets [0x0..0x4] by a write access
    ///   --> simple/src/bin/aliasing_models.rs:71:9
    ///    |
    /// 71 |         *x = 42;
    ///    |         ^^^^^^^
    ///    = note: stack backtrace:
    ///            0: tree_borrows_ok_but_not_stacked_borrows::test2_tree_borrow_ok_stacked_borrow_bad
    ///                at simple/src/bin/aliasing_models.rs:72:18: 72:25
    ///            1: main
    ///                at simple/src/bin/aliasing_models.rs:78:5: 78:83
    ///
    /// note: some details are omitted, run with `MIRIFLAGS=-Zmiri-backtrace=full` for a verbose backtrace
    ///
    /// error: aborting due to 1 previous error
    /// ```
    /// vs
    /// ```sh
    /// MIRIFLAGS=-Zmiri-tree-borrows cargo miri run --bin aliasing_models
    ///
    /// => outputs no error
    /// ```
    ///
    /// See Example 4 in [Tree Borrows paper](https://perso.crans.org/vanille/treebor/aux/preprint.pdf)
    pub fn test2_tree_borrow_ok_stacked_borrow_bad() {
        let x = &mut 0;
        let y = x as *mut _;
        *x = 42;
        unsafe { *y = 24 }
    }
}

pub fn main() {
    // tree_borrows_ok_but_not_stacked_borrows::test1_ok_all();
    tree_borrows_ok_but_not_stacked_borrows::test2_tree_borrow_ok_stacked_borrow_bad();
}
