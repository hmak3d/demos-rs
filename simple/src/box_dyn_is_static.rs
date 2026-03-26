//! Vanilla `Box<dyn MyTrait>` is same as `Box<dyn MyTrait + 'static>`
//!
//! For shorter lived value, use explicit lifetime `Box<dyn MyTrait + 'a>`

#![allow(unused)]

use std::hint::black_box;

trait MyTrait {}

struct MyStructSimple;

impl MyTrait for MyStructSimple {}

struct MyStructWrapper<T> {
    val: T,
}

impl<T> MyTrait for MyStructWrapper<T> {}

/// Show compiler error. See TODO in function body.
fn wrap_it(s: &str) -> Box<dyn MyTrait> {
    // TODO swap MyStructWrapper for MyStructSimple to get compiler error:
    //          error: lifetime may not live long enough
    //        --> simple/src/bin/box_dyn_is_static.rs:37:5
    //         |
    //      22 | fn wrap_it(s: &str) -> Box<dyn MyTrait> {
    //         |               - let's call the lifetime of this reference `'1`
    //      ...
    //      37 |     Box::new(MyStructWrapper { val: s })
    //         |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ returning this value requires that `'1` must outlive `'static`
    //         |
    //      help: to declare that the trait object captures data from argument `s`, you can add an explicit `'_` lifetime bound
    //         |
    //      22 | fn wrap_it(s: &str) -> Box<dyn MyTrait + '_> {
    //         |                                        ++++
    // Box::new(MyStructWrapper { val: s })
    Box::new(MyStructSimple)
}

/// Fix above compiler issue by increasing lifetime of captured input
///
/// `Box<dyn MyTrait>` is same as `Box<dyn MyTrait + 'static>`
/// which creates backpressure for captured input `s` to also be static
fn wrap_it_fix1(s: &'static str) -> Box<dyn MyTrait> {
    Box::new(MyStructWrapper { val: s })
}

/// Fix above compiler issue by shortening lifetime of output
fn wrap_it_fix2<'a>(s: &'a str) -> Box<dyn MyTrait + 'a> {
    Box::new(MyStructWrapper { val: s })
}

/// Same as [wrap_it_fix2]  elide some stuff with anonymous lifetime
fn wrap_it_fix3(s: &str) -> Box<dyn MyTrait + '_> {
    Box::new(MyStructWrapper { val: s })
}

fn do_it() {
    // Demonstrate that output cannot outlive captured input

    {
        let s = String::from("hi");
        // TODO uncomment next below to get error
        // let x = wrap_it(&s);
        let x = wrap_it("hi");
        black_box(x);
    }

    {
        let s = String::from("hi");
        let x = wrap_it_fix2(&s);
        // TODO Uncomment next line for compiler error
        // drop(s);
        black_box(x);
    }

    {
        let s = String::from("hi");
        let x = wrap_it_fix3(&s);
        // TODO Uncomment next line for compiler error
        // drop(s);
        black_box(x);
    }
}
