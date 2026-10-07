# Viewer cone: ThinkPad reruns, 2026-10-07

**Current checkpoint:** Mark authorized integration and cleanup. Mere main
is pushed at `ea9e6da7`, including latest-main Genet `965b64e2` and the reviewed
viewer/speed changes. See [Authorized integration](#authorized-integration)
for fresh gates and cleanup. The plain component now measures 2,147,689 B
brotli across six assets. The numerical cap remains open. Earlier `6963ebe5`
and `8175b69e` receipts remain historical, with their misses and controls
preserved.

## Evidence

Receipts, startup diagnostics and sampled machine load are in
[`logs/thinkpad/receipts/`](logs/thinkpad/receipts/), with a compact
[`summary.json`](logs/thinkpad/receipts/summary.json). Outputs were collected
without overwriting earlier receipts. There were no new builds or source
changes in Mere, and no rebase, commit, push, merge or message to another chat.

Machine: `thinkpad-l14-f`, AMD Ryzen 3 PRO 4450U, eight logical CPUs, Fedora
Linux, headed Flatpak Chrome 154.0.8037.97. One receipt at a time, separate
profiles, ports 8960-8964. CPU was sampled from `/proc/stat` before and during
each run; the latter includes the browser itself. Before release receipts,
CPU use was 1.1-4.6%; mean during-run use was 31.7-55.2%. There were no
concurrent Rust builds observed on that machine.

Stock Chrome exposed `navigator.gpu` but returned no adapter: the first
default run never became ready. `--enable-unsafe-webgpu` alone selected
SwiftShader and was rejected as a performance setup. With that flag plus
`--enable-features=Vulkan --use-angle=vulkan`, the separate adapter probe
reported AMD `gcn-5`, `isFallbackAdapter=false`, and timestamp-query support.
All speed rows below use those recorded flags, headed Wayland Chrome, and
the existing page's own scenario runner. No DevTools or external browser
input was used. These are Linux results, not replacement Windows receipts.

| Row | Development | Release |
| --- | --- | --- |
| Default, fast | fail | pass |
| Default, fast control | fail | pass |
| Viewer, fast | fail | pass |
| Viewer, fast control | fail | pass |
| Viewer, slow | pass | pass |

Every completed speed receipt has zero page errors and zero gate failures.
No assertion was relaxed. The failures were:

- Default fast: effective speed `0.998` against a floor of `1`, worst admitted
  budget overrun `867 us` against `100 us`, one frame past the grain.
- Viewer fast: worst admitted budget overrun `367 us`, one frame past the
  grain. Its measured timing window itself stayed inside the bound; the
  cumulative assertion retains the earlier miss.
- Default/viewer fast controls: effective speed `14.894`/`18.127` against
  `>=25`, with the budget unbound.

The release planted-stall scenario passed by detecting its intended miss:
`9467 us` worst admitted overrun and nine frames past the grain. This
scenario asserts that the bound is broken, so its expected result is `ok`.
It does not silently accept the stall as an ordinary passing fast receipt.

Reading, not ruled: development rendering and startup make these receipts
machine/profile-sensitive. Passing release rows establish the shipping
profile's behavior on this machine; they do not waive the development gate.

## Bundle identity

Transferred WASM hashes match the original build logs. All release payload
figures remain the earlier measurements; they were not recompressed here.

| Bundle | WASM SHA-256 |
| --- | --- |
| Default dev | `686fd0115f2a42588aecbda3f80c77e4ed3e50a8e30485807f9006e121fbd592` |
| Viewer dev | `a53a08ad6df7486895d59879117dca3de9f59ab2bb4dd17b43256eece33b035e` |
| Default release | `49b56bd3b6c8e971d215ec6aab43a15abe2ad9a9488e8ff0c3b899d53a45be7a` |
| Viewer release | `3723753272428b9df78e8321078aeae46230fc0b108adcc698319ccd465fef73` |
| Viewer + canvas-gpu release | `f0c94918df4e88961f61803093cc09ac83ab12d680d2383df8832c8471eb12ab` |

## Ruling 107: measured under Ruling 120

The tracked source projects **21 nodes and 24 edges**: reconcile
`content/repositories.toml` against `content/github-metadata.json`, then
filter `content/relations.toml` to the surviving public node IDs. The
metadata is dated 2026-09-24. The ignored generated `html/repos/index.html`
contains 26 nodes and 29 edges; that is a different, stale generated snapshot.
The initial 26-node clarification was corrected before any benchmark ran.

Mark approved the prepared 21-node benchmark at the normal GPU threshold
("Ok"), recorded as Ruling 120 in the site canvas plan. The count-matched
synthetic diagnostic is
[`harness/p0_viewer_site_frames_21.scn`](harness/p0_viewer_site_frames_21.scn):
seed 7, no links, speed 1, fit the graph, a 60-frame paused window, then
three 120-frame live windows after warmup. Record interval, CPU, GPU-render
span, stage timings, hidden state and device-dispatch counters on both
release viewer bundles. The plain viewer uses `gpu=off`; the `canvas-gpu`
viewer uses normal settings.

The normal web GPU threshold is **400 nodes**. At 21, having a physics device
does not imply GPU dispatch. The scenario asserts zero device steps
and logs the counters. GPU-render timestamps span the whole frame's
submissions and idle gaps; they are not GPU-physics kernel timings.

Both receipts passed with zero page errors and gate failures. The source
scenario hash was identical (`058f48dd97d84aae1e4a5ff35310781aa79e14f4f92a6c26ba5e6f5001d08f3a`),
as were viewport (1366 x 593) and logical/physical canvas size (1066 x 498).
The actual dimensions above are what was measured, despite the runner's
requested 1400 x 900 window setting. Each window retained
21 total and visible nodes and 89 paint items after culling. Neither page
was hidden during any timing window. CPU before the two runs was 2.12% and
2.37%; during-run mean, including Chrome, was 43.84% and 44.58%.

Receipts and structured timings:
[`logs/thinkpad/site21/summary.json`](logs/thinkpad/site21/summary.json),
[`plain viewer`](logs/thinkpad/site21/site21-viewer/result.json),
[`canvas-gpu viewer`](logs/thinkpad/site21/site21-viewer-gpu/result.json).
All timings below are milliseconds; percentiles belong to each window,
not to a pooled sample. The scenario records 62 paused and 122 live frames
per window, including the timing commands' surrounding frames.

| Build | Window | Frame interval p50 / p95 / max | CPU work p50 / p95 | GPU-render span p50 / p95 |
| --- | --- | --- | --- | --- |
| Viewer | Paused | 16.7 / 18.4 / 22.3 | 9.2 / 11.1 | 15.472 / 18.128 |
| Viewer + canvas-gpu | Paused | 16.7 / 17.4 / 20.2 | 8.8 / 10.7 | 15.678 / 18.916 |
| Viewer | Live 1 | 16.7 / 27.0 / 45.2 | 13.0 / 24.2 | 16.140 / 27.133 |
| Viewer + canvas-gpu | Live 1 | 16.7 / 26.1 / 32.3 | 14.0 / 23.5 | 15.937 / 24.828 |
| Viewer | Live 2 | 17.0 / 22.4 / 35.7 | 14.8 / 20.4 | 16.974 / 22.957 |
| Viewer + canvas-gpu | Live 2 | 16.7 / 19.4 / 24.3 | 12.7 / 17.4 | 15.959 / 18.934 |
| Viewer | Live 3 | 16.7 / 18.5 / 20.2 | 12.6 / 16.0 | 15.665 / 19.664 |
| Viewer + canvas-gpu | Live 3 | 16.6 / 18.8 / 25.3 | 13.3 / 16.1 | 15.966 / 18.110 |

The normal law was `spring.rapier`. The plain viewer reported device off;
the GPU build reported device on. Both reported zero device steps,
submissions, answers and failures at each live checkpoint. Physics-stage
CPU medians were 0.1 ms in all six live windows, with p95 0.2-0.3 ms.

Reading, not ruled: this small graph does not demonstrate a benefit from
shipping `canvas-gpu`. The median live cadence is about 60 Hz in both,
and the physics device does no work in this setup. The GPU bundle's lower
tail timings in some windows do not establish a causal GPU benefit:
this was one serial pair, with three windows in each, not randomized
independent trials. It measures the approved synthetic workload, not the
site's edges, content or larger graphs.

Recommendation for Ruling 107: use the plain viewer for the initial site
canvas. The added GPU machinery costs 747,678 B brotli WASM (about 36.9%
of the plain viewer) without demonstrated physics acceleration here.
For Ruling 26, the measured basis is 2,024,097 B brotli WASM, plus 13,904 B
for the generated module JS. A whole-component budget must also account for
the loader, stylesheet, font and other site assets. No numerical cap or
budget definition is imposed by this recommendation.

## Twiggy attribution at the measured checkpoint

Twiggy 0.8.0 `top -n 30` succeeds on all three existing release WASM files.
[`Commands and hashes`](logs/twiggy-summary.json) and the
[`default`](logs/twiggy-default.txt), [`viewer`](logs/twiggy-viewer.txt),
[`GPU viewer`](logs/twiggy-viewer-gpu.txt) item tables are retained.
The largest shallow item in the plain viewer is `data[0]`, 1,606,474 raw
bytes (18.21%); in the GPU viewer it is 2,510,483 bytes (17.18%). These
symbol-stripped files yield anonymous code/data indices, so the tables do
not identify crate owners or allocate compressed payload to individual
functions. No crate-level optimization claim follows from this output.

## Integration preparation after "Proceed"

Mark's "Proceed" accepts the plain viewer for the initial site (Ruling 121).
The numerical payload cap remains open. The measured six-asset component is
**2,120,331 bytes brotli**: WASM 2,024,097; generated JS 13,904; loader 5,881;
CSS 2,017; font 74,038; tree HTML 394. This excludes site data and HTTP overhead.
[`Payload hashes and commands`](logs/payload-assets.json) bind these figures
to the measured `6963ebe5` checkpoint.

Fetched Mere `origin/main` at `a59e4c47a2a5c2024427eeb54b689f2e4f2c345b`.
The existing clean cone worktree rebased onto it without conflicts. All four
patches compare equal in [`range-diff`](logs/cone-range-diff.log). The original
measured commit is preserved as local tag `viewer-cone-measured`; the remote
branch remains at `6963ebe5`. The local candidate is now `e9678fdd`, with two
additional commits supplying Exhibit A in the two cone stand-ins and in
main's inherited `framing/src/tests.rs`. The
[`candidate license check`](logs/license-candidate.log) passes, and its
[`planted-defect self-test`](logs/license-self-test.log) passes. The earlier
failed check remains recorded in [`license-check.log`](logs/license-check.log).
The candidate differs from main in eight paths, including that header repair.

Fresh wasm32 checks use the existing shared `C:/t/cargo-targets/mere`,
`--locked --offline` and four jobs. All eight combinations pass: none, `canvas-gpu`, `product`, `remote`,
`product,remote`, `main-page`, `image-decode` and default. The feature
matrix result and logs are recorded in
[`integration-checks.json`](logs/integration-checks.json). Warnings occur in
this current build, including 23 from graphshell-web for the plain viewer;
the original handoff's warning-free claim does not describe this invocation.
No release bundle was rebuilt, and no headed receipt is claimed for the
rebased candidate.

The named conatus chat is an existing Claude session, identified through its
local title metadata. Available Codex chat tools cannot deliver to it. The
concrete [`review request`](REVIEW_REQUEST.md) is prepared; no review response
has been received and Ruling 109's pre-main review is still required.

The current physics catalog records the own-1x fast bar and half-of-the-lesser
50x bar. Their implementation remains in `seiche-speed-estimator`, locally
`4b916a2e`. The old bundle's four development failures are preserved, not
waived or repaired by replacing its assertions. A read-only three-way
[`merge preview`](logs/speed-merge-preview.log) finds a content conflict in
`web.rs`; tree and Cargo changes auto-merge. The review must carry the worker
module into the shared root and move the host's worker field, feed and start
calls into `web_main.rs`, preserving the cone's cfgs and exports. Neither
lane was merged, pushed or rewritten to resolve the other lane's work.

## Retained resources

No Cargo home, isolated Cargo target or worktree was created during this resume.
The existing shared `C:/t/cargo-targets/mere` was reused for compile checks.
The existing `Code/worktrees/mere-viewer-cone` remains owned by this P0 lane
for review and integration. The transferred release binaries on the ThinkPad
are removed now that Ruling 107's measurements are recorded; source fixtures
and receipts remain as evidence. The original local release bundles remain
owned by the P0 lane. Receipt browsers and sinks are stopped. Transfer
archives and temporary Chrome profiles are removed after receipt collection
and ownership checks.

## Candidate review

Mark: "Eh, they're out of gas. Have a look yourself". Ruling 122 replaces
the unavailable conatus review with this direct review. The cone and speed
estimator are combined in the existing cone worktree. The bounded review passes: **36/36 current development cases, 4/4 Chrome AX
checks and the rebuilt release's 21-node frame receipt** meet their expected
verdicts. [`review-summary.json`](logs/self-review/review-summary.json) records
the counts, replacement control, complete rows and remaining integration limits.
The Max/50x runs started at 3.7-13.7% total CPU use; this does not claim an
entirely idle machine or remove the recorded load samples.

### Code and receipt findings

The feature cone retains the constructor helper before other startup code,
the loader's page-error gate, scenario exports and tree verbs, snapshots,
view-follow, role selects and local actions. The default host remains behind
`main-page`; standalone `product` and `remote` remain additive. The product-off
object is uninhabited and opens as `None`; remote-off rejects the operation
without a page error. The no-GPU lane reports zero device counts. The
conditional keep-list item for `MeaningJob` is inactive: no such type exists
in this candidate, and the cone did not remove one. No lexical functionality
is newly claimed by this review.

Merged the existing speed lane at `4b916a2e` into the cone in `8175b69e`. The
only source conflict was `web.rs`: the worker module remains in the shared
root, and its field, feed and startup calls move into `web_main.rs`. Both
hosts retain the worker, labelled main-thread fallback and ruled cap fallback.
The newer session seam, E1a and root Cargo lock remain unchanged. This is a
merge into the candidate, not into main; the other lane's checkout is untouched.

Two receipt defects needed correction:

- The effective-speed meter keeps 32 frames across a speed change. The old
  warmups mixed 1x and selected-speed samples. `c91f7b85` warms both phases
  for a full meter window in four fixtures. All assertions and ruled bars are
  unchanged. The earlier short-warmup 50x diagnostic passed, so it is not
  presented as a failure fixed by changing the fixture.
- With full warmup, the 500 ms slowed-Max plant still reached 2.571 times 1x.
  It was too weak to produce the intended negative case at steady state.
  `dd41d322` documents a 3000 ms configurable plant with the same <= 0.99
  assertion. The failed 500 ms control remains in the evidence. The stronger
  control passes in both builds: Max/marked-1x is 0.626 in the viewer and
  0.639 in default. Its verdict is separate in the final summary.

### Fresh validation and limits

[`bundles.json`](logs/self-review/bundles.json) binds both development builds
to runtime commit `8175b69e`, exact wasm-bindgen 0.2.129 and the locked web
dependency graph. Later code commits only alter scenario fixtures and the
physics plan; Rust runtime source remains unchanged. All eight wasm32 feature
configurations pass with warnings, recorded in
[`feature-checks.json`](logs/self-review/feature-checks.json). Focused native
`graphshell --no-default-features --features web --lib` passes 117 tests,
with four ignored; this does not claim the earlier full native/default suite.
The MPL header check and its planted-defect self-test pass.

Headed ThinkPad development checks cover both builds' slow, Max, 50x,
stall/slowed-Max/cap controls, speed selects, tree reader actions and its
missing-action control, role selects, framing/view-follow, the planted
page-error gate and worker/default/main/failure paths. The viewer also refuses
remote and saved edits cleanly and handles the loader's remote export without
a page error. One browser runs at a time with the recorded hardware Vulkan
flags. CPU samples, bundle and scenario hashes accompany each receipt. These
are bounded current receipts; live WebRTC joins, all historical product
scenarios, Firefox boot and a large-graph GPU crossover were not rerun.

Fresh Windows checks use Chrome's computed accessibility tree, read-only,
under the existing F68 exception. Both builds expose all 11 item groups with
Drag and Pin; removing Pin makes both checks fail for that reason. The
[`four-row summary`](logs/self-review/windows-ax/summary.json) and actual AX
trees are retained. Chrome ran without headless mode in a hidden window;
these are accessibility checks, not timing measurements. The Windows runner
now refuses occupied ports instead of killing a matching sink, accepts
absolute profile paths and launches its background browser hidden.

The rebuilt plain release WASM is 8,844,841 raw bytes and 2,028,647 bytes
brotli; generated JS is 14,194 bytes brotli. All six component assets total
**2,125,171 bytes brotli**, 4,840 above the earlier checkpoint. The four static
assets have unchanged hashes. [`release-payload.json`](logs/self-review/release-payload.json)
records hashes and build settings. The new 21-node frame receipt passes with live interval medians
16.6/17.1/16.6 ms and p95 23.9/21.6/17.3 ms. All nodes remain visible, the
four timing windows are visible and viewport/canvas sizes are recorded.
Physics-device work stays at zero. This is separate from the older plain/GPU
pair and cannot establish a causal GPU benefit. The first new release launch
was refused before browser startup because the preceding port's connections
were in TIME_WAIT, with no live listener; the recorded retry uses free port
9050. No product result is inferred from that launch refusal.

The numerical cap remains for Mark. The review does not claim a deployment
or merge into main. The tested baseline is `a59e4c47`; the shared remote-main
ref advanced during the review to `d041cc69`, adding smolweb projection work
and the E1b probe. Those three commits do not overlap this cone's changed
paths, but integration against that later head remains distinct from these
receipts. `origin/viewer-cone` and the measured tag remain at `6963ebe5`.

### Resource ownership after review

The existing cone worktree remains owned by P0 for integration, and the other
lane's existing worktree is preserved. Existing Cargo caches under
`C:/t/cargo-targets/mere/seiche-speed` (native) and its `web` child were reused
to avoid colliding with active builds in the primary checkout's shared target.
No Cargo home or new worktree was created. Local review bundles and four
fresh Chrome profiles remain under
`C:/Users/mark_/Code/testing/mere/viewer-cone/review`, owned by this review.
Their hashes and marker were checked and no Chrome or bindgen owner remained,
but automatic approval review rejected both the bounded computed-path cleanup
and the explicit-path cleanup with only "blocked by policy". The retained
paths and target ownership are recorded in
[`cleanup-local.json`](logs/self-review/cleanup-local.json). Source fixtures,
original measured local bundles and receipts are preserved.

The ThinkPad's 39 temporary profiles, three copied `pkg` bundles and three
transfer archives were removed after streaming the evidence locally and
checking the marker, hashes, absolute containment and lack of live owners.
[`cleanup-thinkpad.json`](logs/self-review/cleanup-thinkpad.json) records the
removed paths. Source fixtures, static sources and receipts remain. Receipt
browsers and sinks are stopped on both machines.

## Authorized integration

Mark: "authorized", following the direct review and its remaining integration
and cleanup steps (Ruling 123). The numerical cap is not selected by this reply.
Mere main now contains the viewer cone and speed estimator, pushed and verified
at **`ea9e6da74e2a0f13fc83411513b5d5ea871eb6a7`**. The tested runtime is
`ea38f335`: later code changes only update the physics plan. The primary
checkout fast-forwarded from clean `e7ec66af`; the push advanced remote main
from `57b4893d`. The measured remote viewer branch and `viewer-cone-measured`
tag remain at `6963ebe5`.

The new main baseline includes Genet `965b64e2`, the browser prerequisite,
smolweb accessibility projection and E1b. It merges cleanly into the candidate;
main's root manifest and lock are exact. The ignored standalone web lock
refreshes minimally offline, then all gates use locked/offline resolution and
the tracked portable configuration. Its before/after bytes and hashes are
recorded separately. No local path-patch configuration is used for these gates.

[`Integration summary`](logs/integration/summary.json) records:

- All eight wasm32 feature configurations pass, with warnings; both default
  and viewer development bundles and a plain release were rebuilt using exact
  wasm-bindgen 0.2.129. Focused native tests pass 117 with four ignored.
- **34 current headed development cases** pass their expected verdicts:
  viewer 18, default 16. They repeat the review's normal speed, stall/cap,
  tree/role/framing, error-gate, worker/fallback and clean-refusal coverage.
  The two 3000 ms slowed-Max cases passed in the prior review and are not
  repeated here. Their simulation, estimator, worker, bar and fixture sources
  are identical, with hashes in `unchanged-speed-source.json`. This is a
  bounded integration round, not a claim of the entire historical suite.
- Four fresh Chrome computed-AX checks pass, including the missing-Pin
  controls. The release's approved synthetic 21-node frame receipt passes:
  live interval medians are 16.7/16.7/16.7 ms, with all nodes visible, no hidden
  timing windows and zero physics-device work. Its first launch was refused
  before Chrome startup because an unrelated listener owned port 9090; the
  same scenario passes on verified-free 9120. The owner was not stopped.
- The fresh six assets total **2,147,689 B brotli**, using CLI 1.2.0 quality 11.
  WASM is 9,315,298 B raw and 2,051,137 B brotli; generated JS is 14,222 B
  brotli. This replaces the prior 2,125,171 B candidate size as the current
  cap basis, an increase of 22,518 B. Site data and HTTP overhead remain
  outside those six assets. The numerical cap and site deployment remain open.

### Integration cleanup

Local cleanup is complete. Bulk/forced deletion was initially rejected even
after authorization. Narrower native PowerShell operations on verified
individual generated files, empty directories and exact Chrome profile paths
succeeded without Force. Six disposable `pkg` bundles, five transfer archives,
eight Chrome profiles and generated Python bytecode are removed. Source
fixtures, static sources and receipt logs remain. The new ThinkPad round's
35 profiles, three copied bundles and two archives are also removed after
streaming evidence and confirming hashes and no live owners. The cleanup
records are linked from the integration summary.

The cone worktree and local branch are retired after the main push, clean-state
and ancestry checks. The Windows harness now defaults to the primary main
checkout; its configurable `-Web` path remains available for receipt bundles.
The updated harness parses successfully and its default directory exists.
Its original ignored development default bundle was
preserved, byte-identical, at
`C:/Users/mark_/Code/testing/mere/viewer-cone/measured-default-dev`, owned by
P0's historical receipts. The current standalone lock is preserved in
`logs/integration/web-lock-after.txt`. Earlier measured local bundles remain.
Other worktrees, including the original speed lane, are untouched. No Cargo
home was created. Existing shared native and web caches under
`C:/t/cargo-targets/mere/seiche-speed` are preserved for subsequent consumer
gates; no new Cargo target was created. Receipt browsers and sinks are stopped.
