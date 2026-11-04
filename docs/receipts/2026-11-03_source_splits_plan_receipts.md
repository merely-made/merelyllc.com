# H7 source-split receipts

Baseline: `1cd5ccf5769bb543b1a80668ea0ba52329b10c92`. Compact summaries append
here under H7's shared 100 MB cap. Existing captures and source fixtures remain.


## 2026-11-03: H7 twelve originals

| Original source | Before | After |
|---|---:|---:|
| `src/artifact.rs` | 2070 | 314 |
| `src/devices.rs` | 624 | 529 |
| `src/pages/devices.rs` | 1173 | 525 |
| `src/pages/projects.rs` | 922 | 462 |
| `src/pages/radio.rs` | 1028 | 579 |
| `src/pages/repositories.rs` | 989 | 578 |
| `src/repositories.rs` | 1574 | 568 |
| `src/repository_history.rs` | 666 | 520 |
| `src/retinue_traces.rs` | 680 | 425 |
| `src/site.rs` | 716 | 477 |
| `crates/repo-graph/src/lib.rs` | 3664 | 427 |
| `crates/repo-graph/src/portable.rs` | 622 | 420 |

All children fit (maximum changed Rust file: 585 lines). Twelve exact
exceptions retire, leaving nine non-Rust exceptions. Public facade paths,
serialized field/variant names, page IDs and ordered validation remain.

- Baseline/post `cargo test --locked --no-fail-fast`: 110 passed each.
- Baseline/post `cargo test --manifest-path crates/repo-graph/Cargo.toml --locked --no-fail-fast`:
  43 passed each, including native/frozen matrix realization tests.
- `cargo check --locked --all-targets` and the standalone graph equivalent:
  pass. The first compile caught a test-only facade imported without cfg;
  matching its original gate fixes that scope error before qualification.
- `cargo check --manifest-path crates/repo-graph/Cargo.toml --locked --target wasm32-unknown-unknown --lib`:
  pass. Wasm exports and GraphPhysics remain rooted at their original facade.
  This is compiler evidence, not a fresh deployed/browser acceptance claim.
- `cargo doc --locked --no-deps` for both Cargo roots: pass.
- `prek run --all-files`: ceiling, root Clippy, graph Clippy and radio-mirror
  Clippy pass. The actual commit hook repeats the applicable gates.
- Move review matches 233 old/new unchanged function/data bodies and every
  compared signature/type layout; two bodies differ only in rustfmt blocks.
  Four moved public facade items retain their original exports. The ordered
  static-asset slice is separately reviewed: same nine copied-asset calls
  and CNAME check, same order/literals, same borrowed error vector.
- Matrix DTO private fields now have parent visibility in their private
  child module, preserving their former crate-root owner scope. They remain
  private to external consumers. No second data or projection authority.
- Source notices, assets, captured traces, tests and policy assertions remain.
  Manifest/lock/dependency and CI/deployment configuration are unchanged.
  Existing unused-patch warnings retain their source-cohort scope.
- Stable C:/t/cargo-targets/mer3ly, two jobs, locked cached inputs. Compact
  receipts only; no isolated target, Cargo home or worktree was created.
