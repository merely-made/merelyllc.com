# Source splits: H7 mer3ly batch

**Status (2026-11-03):** landed; 12/12 paths qualified.

Mark requested another fifty-file pass after H5 and H6. Mere's
`design_docs/mere_docs/implementation_strategy/2026-10-30_house_style_plan.md`
owns that aggregate. This owner contributes 12 distinct original paths;
previous qualified paths are excluded. The retained cap is 100 MB across
H7, with compact summaries instead of full retained build logs.

## Phases and done-conditions

- **M1**: repository graph and portable projection (2 originals), keeping the frozen/live matrix identities and root public exports.
- **M2**: artifact and manifest validation (5 originals), retaining existing public-data rejection, provenance and diagnostics.
- **M3**: page projections and shell (5 originals), retaining view ordering, IDs, links and authority-owned data.

An original counts after it and all new modules fit 600 physical lines,
its exact ceiling exception retires, relevant baseline/post test inventories
agree, feature and documentation checks cover the moved surfaces, and full
local/actual commit hooks pass with the implementation pushed. Public
facade paths, owner authority, cfg boundaries, errors, fixtures and numerical
values stay intact. Ordinary private moves and test grouping need no new
ruling. These software checks renew no headed, device, deployed-site or
other-platform acceptance. Stable approved Cargo targets and two jobs apply;
existing concurrent work and historical evidence remain intact.

## Findings

- 2026-11-03: cfg-gated moved functions need equally gated facade imports. The scan_public_text test helper demonstrated this at the first compile; its root alias now shares cfg(test). Pure internal aliases are removed or test-gated; every external facade export remains.
- 2026-11-03: static-asset checks now form one ordered helper. It borrows the same error collector and retains the original CNAME/asset sequence; public rejection behavior is covered by the 110-case site suite. Matrix DTO fields keep their former crate-owner visibility; Wasm entry points and GraphPhysics remain at the crate root.

- 2026-11-03: selected independent implementation and fixture groups from
  clean main. Literal bytes, method signatures, type fields and facade
  exports are reviewed alongside compiler/test results. Whole-file size
  includes rustfmt output; moving bodies alone does not protect API paths.
- The site and repo-graph have separate Cargo roots. Both need their
  own tests/gates; changes to src/artifact.rs preserve ordered rejection
  diagnostics. Existing CI/deployment configuration stays under its owner.
- Evidence appends to `docs/receipts/2026-11-03_source_splits_plan_receipts.md`.

## Selected originals

- `crates/repo-graph/src/lib.rs`
- `crates/repo-graph/src/portable.rs`
- `src/artifact.rs`
- `src/devices.rs`
- `src/pages/devices.rs`
- `src/pages/projects.rs`
- `src/pages/radio.rs`
- `src/pages/repositories.rs`
- `src/repositories.rs`
- `src/repository_history.rs`
- `src/retinue_traces.rs`
- `src/site.rs`

## Progress

- 2026-11-03: selected 12 originals; baseline checks precede source edits.

- 2026-11-03: M1-M3 landed: twelve originals and children fit; twelve exceptions retire. Native site/graph cases match baseline at 110/43, Wasm and docs checks pass, and full local/commit gates qualify the source.
