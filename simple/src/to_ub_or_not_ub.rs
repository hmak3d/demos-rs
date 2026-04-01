//! Intentionally introduce UB in unsafe code.
//! Regular tests won't catch this problem but miri tool will.

#[cfg(test)]
mod tests {
    #[test]
    fn test_ub1() {
        unsafe {
            let mut x = 42;
            let mut1 = &mut x;
            let ptr2 = mut1 as *mut _;
            let mut3 = &mut *ptr2;

            // miri will flag UB:
            // error: Undefined Behavior: attempting a read access using <138599> at alloc43912[0x0], but that tag does not exist in the borrow stack for this location
            //   --> simple/src/to_ub_or_not_ub.rs:41:13
            //    |
            // 41 |             *mut3 += 1;
            //    |             ^^^^^^^^^^ this error occurs as part of an access at alloc43912[0x0..0x4]
            //    |
            //    = help: this indicates a potential bug in the program: it performed an invalid operation, but the Stacked Borrows rules it violated are still experimental
            //    = help: see https://github.com/rust-lang/unsafe-code-guidelines/blob/master/wip/stacked-borrows.md for further information
            // help: <138599> was created by a Unique retag at offsets [0x0..0x4]
            //   --> simple/src/to_ub_or_not_ub.rs:12:24
            //    |
            // 12 |             let mut3 = &mut *ptr2;
            //    |                        ^^^^^^^^^^
            // help: <138599> was later invalidated at offsets [0x0..0x4] by a write access
            //   --> simple/src/to_ub_or_not_ub.rs:39:13
            //    |
            // 39 |             *ptr2 += 1; // This will cause UB because accessing ptr2 will invalidate mut3
            //    |             ^^^^^^^^^^
            //    = note: this is on thread `to_ub_or_not_ub::tests::test_ub1`
            //    = note: stack backtrace:
            //            0: to_ub_or_not_ub::tests::test_ub1
            //                at simple/src/to_ub_or_not_ub.rs:41:13: 41:23
            //            1: to_ub_or_not_ub::tests::test_ub1::{closure#0}
            //                at simple/src/to_ub_or_not_ub.rs:7:18: 7:18
            *ptr2 += 1; // This will cause UB because accessing ptr2 will invalidate mut3

            *mut3 += 1;
            *ptr2 += 1;
            *mut1 += 1;

            assert_eq!(x, 46);
        }
    }

    #[test]
    fn test_ub2() {
        unsafe {
            let mut x = [0; 8];
            let mut1_at_0 = &mut x[0];
            let ptr2_at_0 = mut1_at_0 as *mut i32;
            let ptr3_at_0 = ptr2_at_0.add(1).sub(1);

            // miri will flag UB:
            // error: Undefined Behavior: attempting a read access using <169485> at alloc53056[0x4], but that tag does not exist in the borrow stack for this location
            //   --> simple/src/to_ub_or_not_ub.rs:78:13
            //    |
            // 78 |             *ptr4_at_1 += 1; // ... so we cannot sneak in access to another element
            //    |             ^^^^^^^^^^^^^^^ this error occurs as part of an access at alloc53056[0x4..0x8]
            //    |
            //    = help: this indicates a potential bug in the program: it performed an invalid operation, but the Stacked Borrows rules it violated are still experimental
            //    = help: see https://github.com/rust-lang/unsafe-code-guidelines/blob/master/wip/stacked-borrows.md for further information
            // help: <169485> was created by a SharedReadWrite retag at offsets [0x0..0x4]
            //   --> simple/src/to_ub_or_not_ub.rs:54:29
            //    |
            // 54 |             let ptr2_at_0 = mut1_at_0 as *mut i32;
            //    |                             ^^^^^^^^^
            //    = note: this is on thread `to_ub_or_not_ub::tests::test_ub2`
            //    = note: stack backtrace:
            //            0: to_ub_or_not_ub::tests::test_ub2
            //                at simple/src/to_ub_or_not_ub.rs:78:13: 78:28
            //            1: to_ub_or_not_ub::tests::test_ub2::{closure#0}
            //                at simple/src/to_ub_or_not_ub.rs:50:18: 50:18
            let ptr4_at_1 = ptr3_at_0.add(1); //  This is UB as the orig ptr is pointing to element not slice ...
            *ptr4_at_1 += 1; // ... so we cannot sneak in access to another element

            *ptr3_at_0 += 1;
            *ptr2_at_0 += 1;
            *mut1_at_0 += 1;

            assert_eq!(x, [3, 1, 0, 0, 0, 0, 0, 0]);
        }
    }
}
