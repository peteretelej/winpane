# P4: Learnings

Corrections, gotchas, and tips discovered during implementation.

## Phase 1

- **Pre-existing `windows-future` build failure:** `cargo check --workspace` and `cargo check -p winpane-core` fail due to `windows-future v0.3.2` incompatibility with the installed `windows-core` version (`IMarshal` not found, `marshaler`/`submit` missing). This is a transitive dependency issue in the `windows 0.62` crate, not caused by P4 changes. Confirmed by testing on the clean tree (same failure). The types.rs file compiles cleanly when checked standalone. Full workspace builds require a Windows target machine or a windows-rs version bump to resolve.
