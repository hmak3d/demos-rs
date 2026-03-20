//! Stacked borrow example to show how multiple &mut can be aliased
//!
//! [Stacked borrow](https://www.ralfj.de/blog/2018/11/16/stacked-borrows-implementation.html)
//! model in Rust allows two mutable references (aliases?) to exist on a value
//! as long as:
//! - they are arranged a "stack"
//! - creating new aliases pushes references onto the stack
//! - reference at top of stack is "active" (is only thing that can be used)
//! - reference below the top cannot be used [without discarding references above it]
//!
//! re: [Learning Rust With Entirely Too Many Linked Lists](https://rust-unofficial.github.io/too-many-lists/fifth-stacked-borrows.html)

pub fn main() {
    let mut x: i32 = 0;

    let ref1: &mut i32 = &mut x;
    let ref2: &mut i32 = &mut *ref1;

    // TODO reverse next two lines will break the permitted borrow stack order and cause compile error
    // error[E0503]: cannot use `*ref1` because it was mutably borrowed
    //   --> src/bin/borrow_stack.rs:14:5
    //    |
    // 11 |     let ref2: &mut i32 = &mut *ref1;
    //    |                          ---------- `*ref1` is borrowed here
    // ...
    // 24 |     *ref1 += 2;
    //    |     ^^^^^^^^^^ use of borrowed `*ref1`
    // 25 |     *ref2 += 1;
    //    |     ---------- borrow later used here
    *ref2 += 1;
    *ref1 += 2;

    println!("{ref1}"); // outputs: 3
}
