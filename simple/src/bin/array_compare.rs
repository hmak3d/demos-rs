//! Explore the differences between arrays and slices.
//!
//! Show some examples that demonstrate [PartialEq] impl for slices and arrays.
//!
//! | id    | type          | impl                      | link |
//! | ---   | ---           | ---                       | --- |
//! | (a)   | `[T; N]`      | `impl PartialEq<[U]>`     | array cmp slice [slice docs](https://doc.rust-lang.org/stable/std/primitive.slice.html#impl-PartialEq%3C%5BU%5D%3E-for-%5BT;+N%5D) [array docs](https://doc.rust-lang.org/stable/std/primitive.array.html#impl-PartialEq%3C%5BU%5D%3E-for-%5BT;+N%5D) |
//! | (b)   | `[T; N]`      | `impl PartialEq<&[U]>`    | array cmp slice [slice docs](https://doc.rust-lang.org/stable/std/primitive.slice.html#impl-PartialEq%3C%26%5BU%5D%3E-for-%5BT;+N%5D) [array docs](https://doc.rust-lang.org/stable/std/primitive.array.html#impl-PartialEq%3C%26%5BU%5D%3E-for-%5BT;+N%5D) |
//! | (c)   | `[T; N]`      | `impl PartialEq<[U; N]>`  | array cmp array [array docs](https://doc.rust-lang.org/stable/std/primitive.array.html#impl-PartialEq%3C%5BU;+N%5D%3E-for-%5BT;+N%5D) |
//! | (d)   | `[T]`         | `impl PartialEq<[U; N]>`  | slice cmp array [slice docs](https://doc.rust-lang.org/stable/std/primitive.slice.html#impl-PartialEq%3C%5BU;+N%5D%3E-for-%5BT%5D) [array docs](https://doc.rust-lang.org/stable/std/primitive.array.html#impl-PartialEq%3C%5BU;+N%5D%3E-for-%5BT%5D) |
//! | (e)   | `[T]`         | `impl PartialEq<[U]>`     | slice cmp slice [slice docs](https://doc.rust-lang.org/stable/std/primitive.slice.html#impl-PartialEq%3C%5BU%5D%3E-for-%5BT%5D) |
//! | (f)   | `&[T]`        | `impl PartialEq<[U; N]>`  | slice cmp array [slice docs](https://doc.rust-lang.org/stable/std/primitive.slice.html#impl-PartialEq%3C%5BU;+N%5D%3E-for-%26%5BT%5D) [array docs](https://doc.rust-lang.org/stable/std/primitive.array.html#impl-PartialEq%3C%5BU;+N%5D%3E-for-%26%5BT%5D) |
//! | (g)   | `&mut [T]`    | `impl PartialEq<[U; N]>`  | mut slice cmp array [slice docs](https://doc.rust-lang.org/stable/std/primitive.slice.html#impl-PartialEq%3C%5BU;+N%5D%3E-for-%26mut+%5BT%5D) [array docs](https://doc.rust-lang.org/stable/std/primitive.array.html#impl-PartialEq%3C%5BU;+N%5D%3E-for-%26mut+%5BT%5D) |
//!
//! Also demo the sizes of the various types
//!
//! ## Recall ...
//!
//! ### arrays are not slices
//!
//! The TL;DR is that the compiler knows more about arrays so the runtime
//! structures are more lightweight. In contrast, slices requires more runtime
//! bookkeeping since the compiler cannot assume as much stuff about them.
//!
//! |                       | arrays `[T; N]`       | slices `[T]` |
//! | ---                   | ---                   | --- |
//! | size is known at ..   | compiled time         | runtime |
//! | references are ..     | _not_ wide pointers   | wide pointers |
//! | size-ness is ..       | sized                 | unsized |
//!
//! ### == operator desugar to borrows
//!
//! `A == B` desugars to `PartialEq::eq(&A, &B)`. The borrow is applied to LHS and RHS.
//!
//! ### == on refs forwards to pointees (i.e., pointee forward)
//!
//! `&A == &B` is treated as `A == B` due to blanket impl that forwards `eq` to pointees. i.e.,
//! [`Partial<&B>` for `&A`](https://doc.rust-lang.org/stable/std/primitive.reference.html#impl-PartialEq%3C%26B%3E-for-%26A)
//!
//! ### Deref coercion
//!
//! `&&T`` can often be treated as `&T`` due to deref coercions `impl Deref for &T { Target = T }`
//! <https://doc.rust-lang.org/stable/std/ops/trait.Deref.html#impl-Deref-for-%26T>
//!
//! This is not relevant to the code below but it's good to remember nonetheless

#![allow(dead_code, clippy::eq_op)]

fn main() {
    let arr: [u8; 2] = *b"12";
    let arr2: [u8; 4] = *b"1234";
    let arr_ref: &[u8; 2] = b"12";
    let slice_ref: &[u8] = b"12";

    ////////////////////////////////////////////////////////////////////////////////
    // The runtime memory layout of the arrays, slices, and references to them are
    // different.
    // In particular, the wide pointers from slices are more "costly" than the
    // the points to arrays

    // dbg!(std::mem::size_of::<[u8]>()); // doesn't compile; slices can only be manipulated via refs
    dbg!(std::mem::size_of::<[u8; 2]>()); // => 2 (array)
    dbg!(std::mem::size_of::<&[u8; 2]>()); // => 8 (pointer to array)
    dbg!(std::mem::size_of::<&[u8]>()); // => 16 (wide pointer to slice)

    ////////////////////////////////////////////////////////////////////////////////
    // Compare [T; N]-ish as LHS ...

    // ... and [T; N]-ish on RHS
    // let _ = arr == arr_ref; // bad (of course); [u8; 2] == &[u8; 2]; the & nesting doesn't "line up"
    let _ = arr == arr; // ok (reflexive; of course)
    let _ = arr.eq(&arr); // equiv desugar (c) impl PartialEq<[U; N]> for [T; N]
    let _ = arr == *arr_ref; // ok; [u8; 2] == [u2; 2]
    let _ = arr.eq(arr_ref); // equiv desugar (c) impl PartialEq<[U; N]> for [T; N]
    let _ = &arr == arr_ref; // ok; pointee forward makes this same as (arr == *arr_ref)

    // ... and [T]/&[T]-ish on RHS
    let _ = arr == *slice_ref; // ok; [u8; 2] == [u8]
    let _ = arr.eq(slice_ref); // equiv desugar (a) impl PartialEq<[U]> for [T; N]
    let _ = arr == slice_ref; // ok (quirky but ok becuase [T;N] has multiple PartialEq impl); [u8; 2] == &[u8]
    let _ = arr.eq(&slice_ref); // equiv desugar (b) impl PartialEq<&[U]> for [T; N]

    // ... and [T; M] (different const generic type) on RHS
    // let _ = arr == arr2; // bad (of course); [u8; 2] == [u8; 1]; array can only be cmp to array of same len N

    ////////////////////////////////////////////////////////////////////////////////
    // Compare &[T; N]-ish as LHS ...

    // ... and &[T; N]-ish on RHS
    let _ = arr_ref == arr_ref; // ok (reflexive; of course)
    let _ = &arr_ref == &arr_ref; // ok because of pointee forward
    let _ = &&arr_ref == &&arr_ref; // ok because of pointee forward
    let _ = *arr_ref == *arr_ref; // equiv because of pointee forward
    let _ = arr_ref.eq(arr_ref); // equiv desugar (c) impl PartialEq<[U; N]> for [T; N]
    // let _ = arr_ref == arr; // bad (of course); &[u8; 2] == [u8; 2]
    let _ = arr_ref == &arr; // ok because pointee forward

    // ... and [T]/&[T]-ish on RHS
    let _ = arr_ref == slice_ref; // ok because of pointee forward (to become below)
    let _ = *arr_ref == *slice_ref; // ok; [u8; 2] == [u8]
    let _ = arr_ref.eq(slice_ref); // equiv desugar (a) impl PartialEq<[U]> for [T; N]
    let _ = *arr_ref == slice_ref; // ok; [u8; 2] == &[u8]
    let _ = arr_ref.eq(&slice_ref); // equiv desugar (b) impl PartialEq<&[U]> for [T; N]

    ////////////////////////////////////////////////////////////////////////////////
    // Compare [T]/&[T]-ish as LHS ...

    // ... and [T]/&[T]-ish on RHS
    let _ = slice_ref == slice_ref; // ok (reflexive; of course)
    let _ = *slice_ref == *slice_ref; // ok; this is one of the unusual times where slices can be used _without_ ref
    let _ = slice_ref.eq(slice_ref); // equiv desugar (e) impl PartialEq<[U]> for [T]

    // ... and [T; N]-ish on RHS
    let _ = *slice_ref == arr; // ok; [u8] == [u8; 2]
    let _ = slice_ref.eq(&arr); // equiv desugar (d) impl PartialEq<[U; N]> for [T]
    let _ = slice_ref == &arr; // ok becomes (*slice_ref == arr) above due to pointee forward
    let _ = slice_ref == arr; // ok; &[u8] == [u8; 2]
    let _ = (&slice_ref).eq(&arr); // equiv desugar (f) impl PartialEq<[U; N]> for &[T]

    // ... and &[T; N]-ish on RHS
    let _ = slice_ref == arr_ref; // ok becomes (*slice_ref == *arr_ref) below due to pointee forward
    let _ = *slice_ref == *arr_ref; // ok; [u8] == [u8; 2]
    let _ = slice_ref.eq(arr_ref); // equiv desugar (d) impl PartialEq<[U; N]> for [T];  [u8] == [u8; 2]
    let _ = &slice_ref == arr_ref; // ok becomes (slice_ref == *arr_ref) below due to pointee forward
    let _ = slice_ref == *arr_ref; // ok; &[u8] == [u8; 2]
    let _ = (&slice_ref).eq(arr_ref); // equiv desugar (f) impl PartialEq<[U; N]> for &[T]
}

fn is_arr(x: [u8; 2]) -> [u8; 2] {
    x
}

fn is_arr_ref(x: &[u8; 2]) -> &[u8; 2] {
    x
}

fn is_slice(x: &[u8]) -> &[u8] {
    x
}
