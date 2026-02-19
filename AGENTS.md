# Agents: Mandatory: use td usage --new-session to see open work.

# Agents: Before context ends, ALWAYS run:
```sh
td handoff <issue-id> --done "..." --remaining "..." --decision "..." --uncertain "..."
```

# Agents: Follow the below instructions when writing Rust code

## Default agent loop
- Format: `cargo fmt --all`
- Lint: `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- Build: `cargo build --workspace --all-features`
- Test: `cargo test --workspace --all-features`
- Docs (if public API changed): `cargo doc --workspace --all-features --no-deps`

## Operating rules
- Prefer minimal diffs; avoid drive-by refactors.
- Follow existing crate conventions (errors, features, module layout, naming).
- Do not add dependencies without justification; prefer `default-features = false` where sensible.
- Avoid `unwrap/expect` in library code (ok in tests).
- Keep modules hierarchical and aligned to domain boundaries
- Avoid huge modules; split into focused submodules when files grow large
- Each module should include unit tests for its public behavior
- Prefer named module files over `mod.rs` (use `foo.rs` + `foo/` submodules)
- If touching `pub` API: update rustdoc + add/adjust tests + consider semver impact.
- If requirements are ambiguous, stop and ask instead of guessing.

