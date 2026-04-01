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
cargo install cargo-nextest --version 0.9.128 --locked
cargo nextest run --lib test_par

# Run tests w/ miri to find UB
rustup component add --toolchain nightly-aarch64-apple-darwin miri
cargo +nightly miri test --lib test_ub

# Nightly
# rustup component add --toolchain nightly-2026-03-20-aarch64-apple-darwin miri
# MIRIFLAGS="-Zmiri-tag-raw-pointers" cargo +nightly miri test --lib test_ub
```
