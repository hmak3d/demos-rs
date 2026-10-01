//! Lifetimes in Return Position Impl Trait (RPIT) can be tricky to understand
//! if the RPIT is a function.
//! Its meaning depends on `Fn`/`FnMut`-vs-`fn` and `for<'a>-vs-no-for<>`

#![expect(unused, non_snake_case, clippy::disallowed_names)]

////////////////////////////////////////////////////////////////////////////////
// RPIT is impl Fn

use std::cell::Cell;

/// Returned function can take argument of any lifetime
fn higher_ranked_Fn() -> impl for<'a> Fn(&'a str) {
    let captured = Cell::new("hi");
    move |s| {
        // error[E0521]: borrowed data escapes outside of closure
        //   --> simple/src/bin/rpit_higher_ranked_lifetime.rs:29:9
        //    |
        // 14 |     let captured = Cell::new("hi");
        //    |         -------- `captured` declared here, outside of the closure body
        // 15 |     move |s| {
        //    |           - `s` is a reference that is only valid in the closure body
        // ...
        // 30 |         captured.set(s); // <-- BAD won't compile (error[E0521]: borrowed data escapes outside of closure)
        //    |         ^^^^^^^^^^^^^^^ `s` escapes the closure body here
        //    |
        //    = note: requirement occurs because of the type `std::cell::Cell<&str>`, which makes the generic argument `&str` invariant
        //    = note: the struct `std::cell::Cell<T>` is invariant over the parameter `T`
        //    = help: see <https://doc.rust-lang.org/nomicon/subtyping.html> for more information about variance
        captured.set(s); // <-- BAD won't compile (error[E0521]: borrowed data escapes outside of closure)
    }
}

/// Returned function can take argument of any lifetime. The `for<'a>` is elided.
fn higher_ranked_Fn_elided() -> impl Fn(&str) {
    let captured = Cell::new("hi");
    move |s| {
        captured.set(s); // <-- BAD won't compile (error[E0521]: borrowed data escapes outside of closure)
    }
}

/// Returned function can only take argument of lifetime of function itself
fn lower_ranked_Fn<'a>() -> impl Fn(&'a str) {
    let captured = Cell::new("hi");
    // This is ok because ('a: 'closure) is implied
    move |s| {
        captured.set(s); // <-- OK compiles
    }
}

////////////////////////////////////////////////////////////////////////////////
// FnMut is treated the same as Fn

/// Returned function can take argument of any lifetime
fn higher_ranked_FnMut() -> impl for<'a> FnMut(&'a str) {
    let mut captured = "hi";
    move |s| {
        // error[E0521]: borrowed data escapes outside of closure
        //   --> simple/src/bin/rpit_higher_ranked_lifetime.rs:67:9
        //    |
        // 56 |     let mut captured = "hi";
        //    |         ------------ `captured` declared here, outside of the closure body
        // 57 |     move |s| {
        //    |           - `s` is a reference that is only valid in the closure body
        // ...
        // 68 |         captured = s; // <-- BAD won't compile (error[E0521]: borrowed data escapes outside of closure)
        //    |         ^^^^^^^^^^^^ `s` escapes the closure body here
        captured = s; // <-- BAD won't compile (error[E0521]: borrowed data escapes outside of closure)
    }
}

/// Returned function can take argument of any lifetime. The `for<'a>` is elided.
fn higher_ranked_FnMut_elided() -> impl FnMut(&str) {
    let mut captured = "hi";
    move |s| {
        captured = s; // <-- BAD won't compile (error[E0521]: borrowed data escapes outside of closure)
    }
}

/// Returned function can only take argument of lifetime of function itself
fn lower_ranked_FnMut<'a>() -> impl FnMut(&'a str) {
    let mut captured = "hi";
    move |s| {
        captured = s; // <-- OK compiles
    }
}

////////////////////////////////////////////////////////////////////////////////
// RPIT is fn means that all of below is equivalent????

/// Returned function can take argument of any lifetime
fn lower_ranked_fn<'a>() -> fn(&'a str) {
    |_x| {}
}

/// Returned function can take argument of any lifetime
fn higher_ranked_fn() -> for<'a> fn(&'a str) {
    |_x| {}
}

/// Returned function can take argument of any lifetime. The `for<'a>` is elided.
fn higher_ranked_fn_elided() -> fn(&str) {
    |_x| {}
}

////////////////////////////////////////////////////////////////////////////////

fn main() {
    // Fn scenario shows the interplay between the lifetime of the closure vs its argument
    {
        let a1 = String::from("a1");
        let foo = higher_ranked_Fn();
        let a2 = String::from("a2");
        foo(&a2); // OK because higher ranked Fn promises not to capture argument until it Fn drop()
        foo(&a1);
        println!("{} {}", a1, a2);
    }
    {
        let a1 = String::from("a1");
        let foo = higher_ranked_Fn_elided();
        let a2 = String::from("a2");
        foo(&a2); // OK because higher ranked Fn promises not to capture argument until it Fn drop()
        foo(&a1);
        println!("{} {}", a1, a2);
    }
    {
        let a1 = String::from("a1");
        let foo = lower_ranked_Fn();
        let a2 = String::from("a2");

        // error[E0597]: `a2` does not live long enough
        //    --> simple/src/bin/rpit_higher_ranked_lifetime.rs:147:13
        //     |
        // 129 |         let a2 = String::from("a2");
        //     |             -- binding `a2` declared here
        // ...
        // 147 |         foo(&a2); // <-- BAD won't compile (error[E0597]: `a2` does not live long enough)
        //     |             ^^^ borrowed value does not live long enough
        // ...
        // 151 |     }
        //     |     -
        //     |     |
        //     |     `a2` dropped here while still borrowed
        //     |     borrow might be used here, when `foo` is dropped and runs the destructor for type `impl Fn(&str)`
        //     |
        //     = note: values in a scope are dropped in the opposite order they are defined
        foo(&a2); // <-- BAD won't compile (error[E0597]: `a2` does not live long enough)

        foo(&a1);
        println!("{} {}", a1, a2);
    }

    // FnMut is treated the same as Fn
    {
        let a1 = String::from("a1");
        let mut foo = higher_ranked_FnMut();
        let a2 = String::from("a2");
        foo(&a2);
        foo(&a1);
        println!("{} {}", a1, a2);
    }
    {
        let a1 = String::from("a1");
        let mut foo = higher_ranked_FnMut_elided();
        let a2 = String::from("a2");
        foo(&a2);
        foo(&a1);
        println!("{} {}", a1, a2);
    }
    {
        let a1 = String::from("a1");
        let mut foo = lower_ranked_FnMut();
        let a2 = String::from("a2");
        foo(&a2); // <-- BAD won't compile (error[E0597]: `a2` does not live long enough)
        foo(&a1);
        println!("{} {}", a1, a2);
    }

    // fn (not Fn) does not have drop() and so can deal with shortening of its argument lifetimes
    {
        let a1 = String::from("a1");
        let foo = higher_ranked_fn();
        let a2 = String::from("a2");
        foo(&a2);
        foo(&a1);
        println!("{} {}", a1, a2);
    }
    {
        let a1 = String::from("a1");
        let foo = higher_ranked_fn_elided();
        let a2 = String::from("a2");
        foo(&a2);
        foo(&a1);
        println!("{} {}", a1, a2);
    }
    {
        let a1 = String::from("a1");
        let foo = lower_ranked_fn();
        let a2 = String::from("a2");
        foo(&a1);
        foo(&a2);
        println!("{} {}", a1, a2);
    }
}
