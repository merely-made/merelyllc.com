> Superseded 2026-10-07: Mark asked Codex to perform this review directly.
> The completed review is in [RESUME.md](RESUME.md#candidate-review).
> The request below preserves the prior candidate and review scope.

# Viewer cone: review request for Conatus, physics, seiche status

The resumed P0 lane is ready for your pre-main review under site canvas
Ruling 109. Evidence is in `RESUME.md` and its linked JSON receipts.

Mere candidate: `viewer-cone` at `e9678fdd`, rebased onto freshly fetched
`origin/main` at `a59e4c47`. The four cone patches compare equal with
`git range-diff`; two further commits only supply Exhibit A in the two inert
stand-ins and main's inherited framing test file. The old measured checkpoint
remains at `viewer-cone-measured`
(`6963ebe5`), which also remains `origin/viewer-cone`. Nothing is merged or
force-pushed. The candidate is available in the existing local worktree
`C:/Users/mark_/Code/worktrees/mere-viewer-cone`.

Please check:

1. The keep-list: startup page-error gate and constructor helper; tree receipt
   verbs and snapshots; view-follow and role selects; local action surface;
   lexical fallback. Default host moved into `web_main.rs`, and the viewer's
   `connect_remote` and `run_scenario` exports remain usable.
2. Speed-lane ordering. The ThinkPad receipts use the old `6963ebe5` bundles:
   all five release rows pass unchanged, the planted stall is detected, and
   four development rows miss their original bars. The physics catalog now
   records the own-1x fast bar and half-of-the-lesser 50x bar; those changes
   are still owned by `seiche-speed-estimator`. Do not waive those misses or
   change its assertions in this cone. Your earlier note says three worker
   lines must move from `web.rs` to `web_main.rs` once the cone lands. Settle
   merge ordering and run the combined candidate against the ruled bars.
3. Ruling 120: both 21-node release frame receipts pass; 60 Hz median cadence,
   zero physics-device submissions in both builds. Mark's "Proceed" accepts
   the plain initial-site viewer (Ruling 121). Numerical payload cap is open.
4. The license check and planted-defect self-test now pass. Besides the cone
   headers, the candidate repairs the incomplete Exhibit A header inherited
   in `crates/system/framing/src/tests.rs` at `a59e4c47`. No behavior changes.

Payload at the measured checkpoint: WASM 2,024,097 bytes brotli, generated JS
13,904, loader 5,881, stylesheet 2,017, font 74,038, tree HTML 394. Total
2,120,331 bytes for those six assets, excluding site data and HTTP overhead.
Twiggy item attribution is recorded for all three measured bundles; symbols
are stripped, so its anonymous indices do not establish crate-level cost.
These sizes and all headed receipts belong to `6963ebe5`, not a rebuilt
post-rebase bundle. All eight candidate wasm32 feature combinations compile with warnings.
Fresh compile checks are recorded separately
in `logs/integration-checks.json`. A read-only merge preview against the local
`seiche-speed-estimator` at `4b916a2e` finds a conflict in `web.rs`; tree and
Cargo changes auto-merge. Keep the worker module shared and move its three
host uses into `web_main.rs` while retaining the cone cfgs and viewer exports.

No review response has been received through Codex. This request is prepared
for the existing Claude chat; it does not claim that the chat has reviewed it.
