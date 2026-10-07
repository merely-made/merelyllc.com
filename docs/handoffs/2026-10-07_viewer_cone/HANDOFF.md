---
artifact_contract: "ce-handoff/v1"
created_at: "2026-10-07T03:59:13Z"
title: "mer3ly P0 viewer cone: the calm-machine receipts and Ruling 107's timings"
summary: "The ThinkPad release speed rows and approved 21-node frame benchmark pass. Both viewer builds have about 60 Hz median cadence; GPU physics submits no work. Plain viewer selected under Ruling 121; candidate rebased and license gate repaired. Current speed-lane acceptance, conatus review, and the numerical payload cap remain open. See RESUME.md."
keywords: ["mer3ly", "viewer-cone", "graphshell-web", "P0", "Ruling 26", "Ruling 107", "payload budget", "headed receipts", "canvas-gpu"]
cwd: "C:/Users/mark_/Code (machine-local)"
resume_focus: "Forward REVIEW_REQUEST.md to the existing Claude conatus chat; resolve the combined speed-lane conflict and acceptance, and settle the numerical payload cap before integration."
repository: "merely-made/mere (the cone) and merely-made/merelyllc.com (this plan, mer3ly)"
branch: "mere: viewer-cone"
head: "mere local viewer-cone e9678fdd; measured origin/viewer-cone 6963ebe5; mer3ly main carries this file"
---

# P0 viewer cone: handoff

**Resumed 2026-10-07:** [RESUME.md](RESUME.md) records the ThinkPad's headed
reruns, unchanged bundle hashes and load samples. All five outstanding release
rows pass; the four fast development rows still fail. The slow viewer row
passes both profiles. A planted-stall receipt detects its intended miss.
Under Ruling 120, the 21-node frame benchmark also passes in both release
viewer builds: approximately 60 Hz median cadence, zero physics-device
submissions. Mark selected the plain viewer (Ruling 121). The local candidate is rebased
and its license gate passes; current speed-lane acceptance, numerical cap
and conatus review remain open. The remainder
preserves the original handoff's evidence and context.

## What this is

mer3ly's site canvas plan, P0: build a viewer feature cone of mere's
`graphshell-web`, measure it, and let Mark set the payload budget against it
(Ruling 26). The plan is `docs/2026-09-30_graphshell_site_canvas_plan.md` in
this repo. Rulings 105-119 and the Progress entries from 2026-10-04 onward are
the record of this work; read them before anything else. The Progress entry
"2026-10-07: the cone's receipts and release measurements" is the latest state.

Mark sent the rest to another machine because this one's CPU stayed at 93-100%
from other sessions' builds (his words: "Stop the watcher. Give me a handoff and
i'll have it happen on another machine").

## Where the code is

- mere, branch `viewer-cone`, pushed: `origin/viewer-cone` at `6963ebe5`. Its
  base is mere `28985298`; origin/main has moved on since (at `fde06dc0` when
  this was written). Four commits:
  - `80d869e5`: the feature table in `ports/graphshell/web/Cargo.toml`.
    `main-page`, `product`, `remote`, `canvas-gpu` and `image-decode` are all on
    by default. The viewer is `--no-default-features`.
  - `390e0f50`: the cone itself.
    - `ports/graphshell/src/web.rs` keeps the module tree, `start`, `window`,
      `document` and (under `product`) `resolve_storage_persistence`. It pulls in
      `web_main.rs` (the H5 reference host plus `mount`, moved verbatim) with
      `#[cfg(feature = "main-page")] include!`.
    - Modules gated on `main-page` (Ruling 118): `web_events`, `web_practice`,
      `web_product`, `web_projection`, `web_remote`, `web_view`.
    - Inert stand-ins (Ruling 119): `web_tree/product_off.rs` and
      `web_tree/remote_off.rs`. Without `canvas-gpu`, `web_tree.rs` makes the
      physics device an uninhabited type, and `NoRepulsionLane` makes the
      repulsion lane report zero counts.
    - `web_scenario.rs` keeps `mark`, `page_element` and `publish_capture` for
      every build; everything else in it is main-page only.
  - `4d013313` and `6963ebe5`: without `main-page`, `web.rs` exports
    `connect_remote` and `run_scenario` for the tree page. The loader calls both,
    and before these the viewer could not run a scenario.
- mer3ly main: the plan, and this folder.
- Not merged anywhere. Ruling 109's agreement with the conatus session ("Conatus,
  physics, seiche status") is that the branch goes to that session for a check
  before main. It has not been sent yet.

## Status

Done and verified on this machine:
- Every feature combination compiles for wasm32 with no warnings from the
  crate: none, `canvas-gpu`, `product`, `remote`, `product,remote`,
  `main-page`, `image-decode`, default.
- `cargo tree` on the viewer: 470 packages against the default's 722. Gone:
  muniment `indexeddb`, `mere-webrtc-carrier`, the image decoders, and
  Burn/CubeCL (with `canvas-gpu` off).
- Headed receipts, default build (`logs/default-r1.log`, `logs/rerun-saved-r3.log`):
  61 of 63 as expected. That covers every planted control and the live WebRTC
  receipts.
- Headed receipts, viewer build (`logs/viewer-r2.log`, `logs/rerun-viewer-cdp-r3.log`):
  every tree receipt and control as expected, and all five product and remote
  receipts refused cleanly (RESULT fail, no page error).
- Release payload (`logs/release-r2.summary.txt`,
  `logs/release-r3-viewers.summary.txt`), with P0's settings (opt-level "s",
  thin LTO, strip symbols, codegen-units 1), at `6963ebe5`:

  | Build | raw | gzip -9 | brotli -q 11 |
  | --- | --- | --- | --- |
  | Full page (default) | 18,945,432 | 5,335,292 | 3,464,730 |
  | Viewer | 8,821,049 | 2,915,887 | 2,024,097 |
  | Viewer + `canvas-gpu` | 14,609,347 | 4,211,738 | 2,771,775 |

  `wasm-opt -Os` enlarges every compressed figure, by 3-7%.

Not done:
1. **Development timing acceptance after calm reruns.** Reruns are now recorded
   in RESUME.md: both fast rows still fail in development on the ThinkPad;
   all five release rows pass. The original claim that they missed only under
   load is no longer supported. Mark's ruling requires calm reruns; whether
   release passes satisfy the gate remains for conatus review and Mark. The
   physics catalog now records newer own-1x/budget-relative bars in the speed
   lane; the combined candidate must be checked against those ruled bars.
   - Default build: `p6_tree_speed_fast`
     (`tree.html?nodes=300&seed=7&links=none&gpu=off&physics_speed=max`) and
     `p6_tree_speed_fast_control`
     (`tree.html?nodes=24&seed=7&links=none&gpu=off&physics_speed=50`).
   - Viewer build: the same two, plus `p6_tree_speed_slow` (`?physics_speed=0.2`).
     Its original first run never started because the sink did not come up;
     it now passes in development and release on the ThinkPad.
2. **Ruling 107 settled for the initial site (Ruling 121).** Mark's
   "Proceed" accepts the plain viewer. Both 21-node release benchmarks pass
   with similar median cadence and no physics-device submissions. This does
   not measure the exact site's edges/content, forced dispatch or larger graphs.
3. **The conatus session's check of the branch**, and then its merge to main.
   The local branch is now rebased onto `a59e4c47`, with license repairs at
   `e9678fdd`. `REVIEW_REQUEST.md` identifies the speed-lane merge conflict.
   Remote branch and headed measurements remain at `6963ebe5`.
4. **Mark's budget ruling (Ruling 26)**, with the sizes and timings together.

## Findings to carry

- `personae` stays in the viewer through `pandect`: the tree page runs
  `GraphshellApp` on muniment's `MemoryBackend` through graphshell's own `web`
  feature. Cutting it means splitting graphshell's feature, so it is a question
  for Mark after the budget.
- The full page grew from P0's 2.56 MB brotli (2026-10-01) to 3.46 MB as
  `canvas-gpu`, stable Burn and more landed.

## The harness, and its traps

`harness/` holds this lane's scripts. They are Windows and PowerShell, with
paths hard-coded to this machine (`C:/Users/mark_/...`, Chrome under
`C:\Program Files`, wasm-bindgen 0.2.129 at `C:/t/wasm-bindgen-0.2.129`, and the
fixture's bind address `192.168.4.36`). On another machine, adapt them or use
them as the record of what was run.
- `run-scenario.ps1` is a copy of the grammar-g9 lane's runner. It gives each
  run its own Chrome profile and port, and cleans up only its own processes.
  The shared `Code/testing/mere/scripts/run-graphshell-web-scenario.ps1` kills
  every sink and every Chrome on the shared profile, so it disturbs other
  sessions' runs.
- `run-batch.ps1` takes `-Set default|viewer|rerun`, `-Only <names>` and
  `-Web <dir>`. Each receipt's expected outcome is pass, a planted fail, a gate
  fail, or refused.
- `build-bundle.sh` is the dev bundle (`CARGO_PROFILE_DEV_DEBUG=0`, wasm-bindgen
  `--no-demangle`). `measure-release.sh` is the release payload.

Traps already hit, fixed in these scripts:
- IndexedDB is per origin, port included, so `p4_tree_saved_edit` and
  `p4_tree_saved_reopen` must share both a port and a profile.
- Main's `p6_tree_speed_fast` asserts `physics-speed == max`; run it with
  `physics_speed=max`, not 50.
- PowerShell's `-like` is case-insensitive, and the runner lists a file named
  `result.json`, so match the result line with `-clike "RESULT *"`.
- `wasm-opt` 116 needs rustc's default wasm features:
  `--enable-bulk-memory --enable-multivalue --enable-mutable-globals
  --enable-nontrapping-float-to-int --enable-reference-types --enable-sign-ext`.
- graphshell-web's `Cargo.lock` is gitignored. A lock seeded from an old
  checkout pinned `uuid 1.25.0`, which stable Burn rejects; `cargo update -p uuid`
  fixed it.
- The remote receipts need the native `c4_webrtc_host` fixture,
  `cargo build -p graphshell --bin c4_webrtc_host --features webrtc-session`.
- Serve the viewer from its own copy of `ports/graphshell/web` with the viewer
  bundle in `pkg/`, so the default bundle stays available for its reruns.
- os error 1455 (paging file exhausted) appears when several sessions compile at
  once. Retry with fewer cargo jobs.

## Constraints that apply

- Mark's rule (2026-10-06): no more than three tasks at a time per agent,
  counting lanes, subagents and background jobs.
- Run headed receipts one at a time. Use ports nobody else holds (this lane
  used 8960-8999), and count a port as held only when a live process owns it.
- Pushes, merges to main and rulings are Mark's calls. The conatus session
  checks the branch before main.
