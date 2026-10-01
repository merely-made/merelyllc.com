# Graphshell as the site canvas

**Date:** 2026-09-30
**Status:** assessment. Ruling 1 recorded; the first round of forks is open
(Open decisions). No code has changed.

## Purpose

Mere's projection grammar adoption plan ruled on 2026-09-01 that Graphshell's
first job on mer3ly.net is to supersede the site's own canvas, the repository
graph sandbox, and to serve as the site index. The same ruling called this a
new objective needing its own assessment. This document is that assessment.

It answers two questions: what Graphshell must be able to do before the
sandbox can retire, and where each of those capabilities lives.

**Related:**
[canvas stage unification](2026-09-01_canvas_stage_unification_plan.md),
whose open decision this answers;
[browser delivery](2026-08-16_browser_delivery_plan.md);
in mere, the projection grammar adoption plan
(`design_docs/mere_docs/implementation_strategy/2026-08-15_projection_grammar_adoption_plan.md`,
"Apps embedded in the site"), the consumer survey
(`design_docs/mere_docs/research/2026-08-16_mer3ly_stack_consumer_survey.md`),
and the Graphshell one-tree plan
(`design_docs/mere_docs/implementation_strategy/2026-09-25_graphshell_one_tree_plan.md`).

## Rulings

### Ruling 1 (2026-09-30): the site is a consumer, and has no exceptions

Put as: does mer3ly count as a consumer whose asks open gates, given that the
2026-09-01 framing called the graph view a stress-test toy? The adoption plan
had left this unsettled.

Mark: "my feeling is, the site counts as a consumer, but it should be consuming
or creating stack capabilities, not special exceptions."

What follows: every behaviour the replacement canvas needs is either an
existing stack capability the site consumes, or a new stack capability created
for it, in a crate that another consumer could use unchanged. Nothing
site-shaped goes into Graphshell. A Graphshell build that hardcodes mer3ly data,
schema, or behaviour does not satisfy this plan.

*Reading, not ruled:* the same test applies in reverse. Site-local code that
duplicates a stack capability is retired as part of this work rather than kept
beside it. Ruling 1 applies to the graph sandbox. Whether it also reaches
`message-path-lab` and `projection-proof` is Open decision 4.

## Findings (2026-09-30)

Sources: mer3ly at `c1b8ab1` (2026-09-24), mere at `bd5912fb` (2026-09-30),
both clean on `main`.

### What the sandbox is

- `crates/repo-graph` is 4,149 lines of Rust (3,856 in `lib.rs`, 293 in
  `arrangement.rs`), shipped as a 1.31 MB wasm with a 24.9 KB binding. The
  sandbox page logic is `assets/graph-sandbox.js`, 1,878 lines and 68.8 KB,
  against the loader's 72 KiB size guard.
- It pins mere at `d82afa17` (2026-09-03). mere is 512 commits ahead.
- The rendering, the camera, and the gestures are all in JavaScript. The Rust is
  computation only, which is the split the browser delivery plan exists to end.

### Inventory: site-local code against its stack home

| Site-local surface | Stack home today | Verdict |
| --- | --- | --- |
| `GraphPhysics`: pins, the anchored/free split, backdrops (`lib.rs:364`) | `seiche`, and `mere::canvas::PhysicsBoard`, which Graphshell's practice proof already uses for dragging | consume |
| Arrangement catalog: `radial_rings`, `stack_layers`, timeline lanes (`arrangement.rs`, `lib.rs:2443-2695`) | `scenomise` families: `axial::timeline`, `stack`; `arrangements` registry | consume. The site copies are retired, or upstreamed where they differ |
| Shelfmark compose and resolve (`lib.rs:1018-1315`) | `incipit::ShelfmarkV1`, founded at mere `6cc014c4` | consume. The site keeps only its adapter |
| `mer3ly.graphshell-scene-state/v1` URL-hash wire | shelfmark. The 2026-08-16 ruling made the citation delta and the sidecar one record | consume. The site schema is retired |
| Reading and representation registries | `cartography` | already consumed |
| Portable projection plus receipts, `mer3ly.portable-projection/v1` (`lib.rs:1798-2344`) | `sceno`, `scenotime`, the receipts plan | the schema is site-named. **Create** a neutral one, or drop the site name |
| Two-reading matrix, `mer3ly.two-reading-matrix/v1` (`lib.rs:539-1018`) | none. No stack crate carries a matrix view | **create**, or drop (Open decision 3) |
| Source-time history control, `diff_graphs` (`lib.rs:1441`) | `scenotime` diffs; no history view in Graphshell's browser build | **create** |
| Accessible matrix HTML (`lib.rs:899`) and the authority-derived static index | `graphshell-client` frozen realization (`frozen.rs`), built for exactly this | consume (Open decision 2) |
| Rendering, camera, gestures, hit testing (`graph-sandbox.js`) | `graphshell-web` over `cambium-genet-web-host`, netrender, and Seiche | consume |

### What Graphshell's browser build offers

- `graphshell-web` exports `mount(root)`, and the page uses a
  `<graphshell-view>` element. The component carries its own markup,
  `include_str!`'d, with every id prefixed `gs-` and every lookup scoped to the
  root. `web/embed.html` proves it sits inside a hostile host page without
  touching the page's ids or keys. Embedding is solved.
- It already runs on `cambium-genet-web-host`. Since 2026-09-26 (one-tree plan
  phase 1), the host mirrors the Cambium tree into the DOM with ARIA, so the
  canvas is no longer opaque to a screen reader. Phase 4 of that plan is
  approved and in progress, with stack performance and live physics left open.
- **Stylo is gone.** The web lockfile resolves 638 packages, none of them Stylo
  or Servo, and `genet-livery` is present. The browser delivery plan's biggest
  payload lever has been pulled. The size has not been re-measured. The last
  figure, 3.8 MB gzipped, predates the change, and the local `pkg/` holds a
  71.9 MB debug build.

### The two gaps that are not mer3ly's to fill alone

1. **No mount-time dataset.** The practice proof feeds `ProjectionDataset`
   from `include_str!("../web/fixtures/woodshed-comparison.json")`
   (`ports/graphshell/src/web_practice.rs:37`). Graphshell's browser build has
   no way for a host page to hand it a dataset. Its other data paths are a
   local Mere store in IndexedDB and a remote session over WebRTC, and the
   second needs a peer that GitHub Pages cannot run. Woodshed's precedent is
   the right one: the product owns the export, and Graphshell takes no
   dependency on product crates. That precedent is still waiting on the
   input seam.
2. **`ProjectionDataset` has no relations.** It carries a source, a revision,
   typed fields, and occurrences (`projection_compile.rs:128-142`). A
   repository graph is mostly its relations: 10 edges across the 8 rendered
   nodes the current smoke asserts (`c1b8ab1`). The seam needs relations, or
   the site feeds Graphshell through something other than `ProjectionDataset`
   (Open decision 1).

*Reading, not ruled:* both gaps are capabilities for any static host, not for
mer3ly. A docs site, a published Woodshed set, and merelyllc.com's embeds all
need "hand the component a disclosed graph without a peer". That is why they
pass Ruling 1.

### Standing constraints the replacement inherits

- The whole site stays readable without JavaScript, WebAssembly, or WebGPU.
- The Pages artifact is validated by `authority -- validate-artifact`, and its
  hashes go into receipts.
- Artifacts are committed today. The browser delivery plan ruled that
  application builds ship as release assets once they are megabytes in size.

## Plan (draft; the phases depend on the Open decisions)

**P0: instruments.** Repin mer3ly to current mere. Measure
`graphshell-web`'s size-profile release build, raw and gzipped, with
and without Livery. Then the payload question is answered with a number
rather than the 2026-08-16 figure.
Done when both numbers are recorded here with the build command, and the
repinned site passes `cargo test --locked` and the smoke suite unchanged.

**P1: the dataset seam (stack, in mere).** A host page hands the mounted
component a disclosed dataset with relations. This means data at mount time
plus a fetchable URL, with the source, revision, and identity validation the
practice proof already performs. The practice fixture moves onto that seam, and
its `include_str!` goes.
Done when the practice proof runs from a host-supplied dataset with its
receipts unchanged, a dataset with relations renders edges, and a stale
revision or an unknown field fails explicitly.

**P2: the site exporter (mer3ly).** The authority emits its repositories and
relations as a disclosed dataset at build time, hashed into the artifact
receipt.
Done when `validate-artifact` covers the dataset, and its generation matches
the authority's.

**P3: parity.** Each row of the inventory reaches its verdict: consumed,
created in the stack, or dropped by ruling. The shelfmark links in circulation
keep resolving, or their retirement is ruled.
Done when the smoke suite's projection assertions (8 nodes, 10 edges, the
selection, and the share link) pass against Graphshell, and the frozen
realization serves the no-script reader.

**P4: retirement.** `graph-sandbox.js` and the site-local Rust it drove are
removed. The canvas stage plan's expiry condition is met, and that plan is
closed.
Done when no page emits the sandbox, and `crates/repo-graph` holds only the
exporter.

## Open decisions

1. **The data seam's shape.** (a) `ProjectionDataset` grows relations, and
   the site exports into it the way Woodshed does. (b) The site emits a
   static, read-only Mere graph snapshot that Graphshell mounts the way it
   mounts a store. (c) The site is served from a live resident. Pages cannot
   host one, so (c) waits on C5 public rendezvous.
2. **The no-script reader.** Does the site's own authority-derived static index
   give way to the stack's frozen realization rendered at build time, or does
   the index stay as the site's page and the frozen realization serve only the
   canvas?
3. **Features with no stack home.** These are the two-reading matrix, the
   source-time history control, and the site-named projection schema. For each:
   create it in the stack, or drop it from the site.
4. **Ruling 1's reach.** Does "no special exceptions" also cover
   `message-path-lab` and `projection-proof`? The 2026-09-01 canvas stage plan
   called them site-native explanatory instruments.

## Progress

- 2026-09-30: assessment written; Ruling 1 recorded.
