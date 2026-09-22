This is a separate workspace the uses nightly rust toolchain \[instead of stable\].
This is also the only way to [use miri](../README.md) to find UB in unsafe code.

Unfortunately, projects/code that use different toolchains cannot be mixed in the same workspace.

# Some Commands

```sh
# Have miri verify there is no UB
MIRIFLAGS="-Zmiri-tree-borrows" cargo miri test doubly_linked_list --no-default-features -- --no-capture

# Have miri find intentional UB
RUSTFLAGS='--cfg doubly_linked_list_impl="ub"' MIRIFLAGS="-Zmiri-tree-borrows" cargo miri test doubly_linked_list --no-default-features -- --no-capture

# To generate docs for specific config
# Omit the --cfg if you want docs for default impl
RUSTDOCFLAGS='--cfg doubly_linked_list_impl="ub"' cargo doc --no-deps --document-private-items

# Run #[bench] benchmarks
cargo bench exercism::par_letters::tests::bench_large_tests -F par_letters_raw_chunks
```
