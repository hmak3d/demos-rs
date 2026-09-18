This is a separate workspace the uses nightly rust toolchain \[instead of stable\].
This is also the only way to [use miri](../README.md) to find UB in unsafe code.

Unfortunately, projects/code that use different toolchains cannot be mixed in the same workspace.
