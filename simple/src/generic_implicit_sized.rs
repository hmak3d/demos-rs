//! `: Sized` is a implicit constraint for generic parameters
//!
//! To visualize this on VSCode, add to `settings.json` a setting for
//! [implicitSizedBoundHints](https://rust-analyzer.github.io/book/configuration.html#inlayHints.implicitSizedBoundHints.enable)
//! ```json
//! "rust-analyzer.inlayHints.implicitSizedBoundHints.enable": true
//! ```

#![allow(unused)]

pub fn main() {}

struct MyStruct<T>
// <T: Sized> is implicit
{
    f: T,
}

fn meth1<T>(x: T)
// <T: Sized> is implicit
{
}
