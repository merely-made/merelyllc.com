# P6 platform-topology receipt

**Date:** 2026-09-05
**Scope:** the public repository metadata and protection lane of Mere's
`2026-09-02_platform_boundary_and_repository_topology_plan.md`. Woodshed is
outside this receipt.

## Personal upstream disposition

`mark-ik/p2panda` remains personal. Its `main` was compared with
`p2panda/p2panda:main` immediately before this receipt: it is diverged,
**8 commits ahead and 199 behind**. Mere-family consumers continue to use the
immutable annotated tag `mere-p2panda-net-0.7.2` (tag object
`2d893e1e1b23796353dc97595eca5df74f1e5de0`, target commit
`dec8a45697519db1b05d099fd4689069cc713174`), rather than following either
branch. No transfer, archive, or dependency-source change was made.

## Public inventory and metadata

- `mark-ik/graphshell` was briefly unarchived because GitHub rejects edits to
  archived repositories, then re-archived. Its final public metadata is
  `Archived 2026-07-23: moved into merely-made/mere. History preserved here
  and carried across.` with homepage `https://github.com/merely-made/mere`.
- `merely-made/ringdown` now describes the HyVibe desktop client and clean-room
  Rust protocol implementation. `merely-made/cleromancy` now describes the
  local-first journal for replayable Tarot, dice, and astrology readings.
- `merely-made/sonance` and `merely-made/anise` returned GitHub 404. They are
  intentionally deleted, not archived repositories. The refreshed manifest
  removes both stale archive records.
- Curie's separately owned transfer moved `mark-ik/emblem` to
  `merely-made/emblem`; old-slug redirect and both Git URLs resolve `main` to
  `8c9aebb8ed2a512392d96fbcfc25b6e9d94be6b2`. The metadata refresh includes
  that public repository.
- `scripts/refresh-public-metadata.ps1` completed atomically at
  `2026-09-05T05:16:59Z`: 25 public repositories and 12 public events.

## Shared-wgpu main protections

Each branch is strict, does not enforce on administrators, and disallows force
pushes and deletions. Required checks include only shared software gates; the
headed hardware lanes remain non-required.

| Repository | Required software contexts |
| --- | --- |
| `wgpu-scry` | `gate / Resolve pinned toolchain`, `gate / rustfmt`, Ubuntu/macOS/Windows wgpu-28, wgpu-29, and wgpu-30, plus `gate / windows-latest / extra packages` |
| `wgpu-weld` | `gate / Resolve pinned toolchain`, `gate / rustfmt`, Ubuntu/macOS/Windows wgpu-28, wgpu-29, and wgpu-30 |
| `wgpu-graft` | `Check and test (Linux)`, `Check (macOS / Metal)`, `Check core (Windows / Vulkan + D3D)`, `Check Servo demos (Windows)`, `Check Iced Servo demo (Windows / DX12)`, and `Check (Windows / Vulkan + D3D)` |

## Mer3ly delivery

The public-site smoke cardinality was corrected from the obsolete 8-item,
9-relation projection to the current 9-item, 11-relation projection. The
delivery commit, Pages workflow run, and post-deploy HTTP acceptance are added
below once they are available.
