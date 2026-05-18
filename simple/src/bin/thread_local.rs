//! Use thread locals

use std::cell::{Cell, RefCell};
use std::thread;

thread_local! {
    static COUNTER: Cell<i32> = const { Cell::new(0) };
}

thread_local! {
    static COUNTER2: RefCell<i32> = const { RefCell::new(0) };
}

pub fn main() {
    thread::spawn(|| {
        loop {
            COUNTER.with(|counter| {
                counter.set(counter.get() + 1);
            });

            // COUNTER.with_borrow_mut(|counter| {
            //     *counter += 1;
            // });

            COUNTER2.with(|counter| {
                *counter.borrow_mut() = *counter.borrow() + 1;
            });

            COUNTER2.with_borrow_mut(|counter| {
                *counter += 1;
            });
        }
    });
}
