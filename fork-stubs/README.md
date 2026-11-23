# fork-stubs

Word fork only. Stand-ins for crates the personal fork does not ship, swapped in through
`[patch.crates-io]` in the root `Cargo.toml`. Each keeps only the API its one caller in
craft-libs uses, and never captures anything, so the feature behind it reports a failure.

| Stub | Replaces | Caller | Effect |
|---|---|---|---|
| `xcap` | `xcap` 0.9.8 (and `libwayshot-xcap`) | `craft-platform::screen` (Windows) | screen colour picker unavailable on Windows |
| `core-graphics2` | `core-graphics2` 0.6.1 | `craft-platform::screen` (macOS) | screen colour picker unavailable on macOS |

Why: the fork does not take dependencies of Chinese origin, transitive ones included.
When upstream moves craft-libs and `craft-platform` uses more of either crate, the build fails
here first: add the missing item to the stub, still without capturing anything.
