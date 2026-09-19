<!-- START doctoc generated TOC please keep comment here to allow auto update -->
<!-- DON'T EDIT THIS SECTION, INSTEAD RE-RUN doctoc TO UPDATE -->

- [Description](#description)
- [Some Commands](#some-commands)

<!-- END doctoc generated TOC please keep comment here to allow auto update -->

# Description

Explore Rust language with code snippets

# Some Commands

```sh
cargo test --lib test_par

# Run tests with nextest
#
# NB: --no-capture != --nocapture
# re: https://nexte.st/docs/reporting/#displaying-live-test-output
#
# NB: --no-capture will run tests serially [instead of in parallel]
cargo install cargo-nextest --version 0.9.128 --locked
cargo nextest run --lib test_par

# Run tests w/ miri to find UB
rustup component add --toolchain nightly miri
MIRIFLAGS="-Zmiri-tree-borrows" cargo +nightly miri test
```
