# Verify

Run the local CI checklist for this crate. Report pass/fail for each step. Fix failures only if the user asks.

## Steps

From the repository root:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo build --examples
cargo build --example plot_stops --features plot
```

Optional (security workflow parity):

```bash
# if Cargo.lock is missing:
cargo generate-lockfile
cargo audit
```

## Output

Summarize each command as pass or fail with the first failing error line if any. Do not commit.
