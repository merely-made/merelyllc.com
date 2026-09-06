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
accepted relation follow-up is `d79382b6e6243dca0f15351bc4debbdc6bad7fab`.
Pages workflow run `33948812138` passed exact-artifact validation, headed
desktop/mobile/fallback smoke, artifact upload, and deployment. The deployed
site returned HTTP 200 after that run.

## DNS verification and HTTPS follow-up

Completed later on 2026-09-05 through the authenticated Cloudflare and GitHub
Pages settings:

- added DNS-only TXT `_github-pages-challenge-merely-made.merelyllc.com` with
  GitHub's issued verification value;
- changed the existing apex and `www` CNAME records for
  `merely-made.github.io` from Cloudflare-proxied to DNS-only;
- preserved the existing MX, DKIM, DMARC, SPF, and Apple verification records;
- GitHub organization Pages now reports `merelyllc.com` verified;
- repository Pages reports `protected_domain_state=verified`, certificate
  state `approved` for `merelyllc.com` and `www.merelyllc.com`, and
  `https_enforced=true`.

Authoritative Cloudflare DNS returned all four GitHub Pages IPv4 addresses and
all four IPv6 addresses for the flattened apex, the GitHub-issued TXT value,
and `www.merelyllc.com CNAME merely-made.github.io`, each at TTL 300. Live HTTP
acceptance returned `301 Location: https://merelyllc.com/` for the apex HTTP
URL, HTTP 200 for the apex HTTPS URL, and a 301 from the HTTPS `www` URL to the
HTTPS apex. This closes the DNS/HTTPS follow-up without changing mail DNS.
