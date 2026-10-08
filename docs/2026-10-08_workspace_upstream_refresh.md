# Workspace upstream refresh

**Policy, 2026-10-08:** Mark included third-party dependencies and maintained
fork upstreams. Use latest stable releases and compatible upstream changes
where the architecture has not radically changed. Vello has a separate scoped
adoption lane because of its role throughout the graphics stack.

## Inventory and completed updates

The audit covered 22 primary repositories and 17 existing crate checkouts,
including configured parent remotes. Nineteen maintained root Cargo workspaces
expose 250 packages. Historical receipt manifests and archived source pins are
evidence, not upgrade targets. The Hocket and Ringdown origin pointers return
404 and remain historical; this refresh does not recreate those repositories.

Eleven checkouts were fast-forwarded: mer3ly, Mere, knot-editor, Retinue,
Woodshed, iroh, nexus, prns, weave, WPT, and Xilem. Iroh also advanced from its
fork to upstream `d4490fcfde8e4dc7260a44a2a982c654748595e2`, then published that
fast-forward. This establishes ancestry, not product integration acceptance.
Woodshed's tracked and untracked Redshank work was hash-checked before and after
its disjoint fast-forward. Other concurrent edits and local commits remain owned
by their original lanes.

| Update | Published revision | Qualification |
| --- | --- | --- |
| Turquet sha2 0.11 | `9ca8fe7f5589aa37824ce12d0766831d3074bcf4` | Verify-feature check and 134 tests across 34 executables; canonical SHA-256 controls in both verifier entrypoints. |
| Netrender pollster 1.0 | `5a56b09849eddd70ff5b79c3512b0c2e1ec2bf07` | Default workspace suite: 343 passed, two existing ignores, 66 executables; includes automated headless GPU/image composition. |
| Boa stable v0.22 | `494ae680a10704722df3c98335073245b7088a24` on `genet` | 1,142 engine/GC tests and pre-push format/lint gates. Genet public-source adapter: 26 tests and default browser Wasm compile passed. |
| Separate Vello scope | Netrender `65a25c967` | Source/ownership inventory and provider-to-consumer gates; no renderer migration implemented. |

Netrender's receipt does not cover every CPU/Hybrid feature combination or
headed host acceptance. Turquet's checksum change handles sha2's new digest
array representation explicitly. Existing numerical tests remain intact.

## Site and scripting qualification

The site aligns all 18 active Mere Git declarations to
`7a5bedd13c776b43101672349841bd77132bc377`, with one Mere source in both
lockfiles. Cambium/document contract versions and Playwright 1.64.0 are updated
with their locks. Qualification passed: 73 site tests, 39 graph tests, nine
radio-mirror tests, formatting and all-target lint checks with warnings denied
for all three crates, and the locked site check.

The graph Wasm was regenerated against the public immutable Mere pin. Two
same-host builds and bindgen runs using the reusable target produced identical
output. This is not a pair of cold builds or cross-host evidence. The promoted
committed Windows build's Wasm SHA-256 is
`44ce0e5971e3dd37649b65642ecaa6810bfcf1ec6cc6c94b86002d6f14d97b2b`;
the generated JavaScript interface is unchanged. Raw Wasm size is 1,367,745 bytes;
the site's actual gzip payload gates passed.

The exact Pages artifact and native portable projection consumer passed.
Local browser smoke passed in headless Microsoft Edge 156, recording its channel,
including the 21-project/30-relationship graph. Bundled Chromium 156 could not
launch on Windows because of a side-by-side configuration error.
`MER3LY_BROWSER_CHANNEL=msedge` selects installed Edge locally; the default
remains bundled Chromium. The [Pages workflow](https://github.com/merely-made/merelyllc.com/actions/runs/37858985653)
passed its headed Chromium smoke and deployed the checked artifact for
`f151b45336315ad63b3bca5613c6ad9ece5a9566`. All Rust tests/lints, both Wasm
reproducibility checks, exact-artifact validation, and projection acceptance
passed on that deployment runner. Its acceptance artifact is retained with
the machine-local evidence. The Linux runner's same-source Wasm rebuild
is 1,367,785 bytes with SHA-256
`7b6476416d572dc475a48fc0e4afc596237a402cfb5a1f2b2c70ebd7b0be8071`. Its two builds match each other; the cross-host output
differs from the committed Windows build.

Windows portability fixes retain the validation rules: license and source pin
tests parse TOML package entries rather than depending on LF lockfiles;
digest-verified trace and radio fixture JSON retain exact committed bytes through
Git attributes. Radio metadata resolves outside ignored local path redirects.
The radio provider pin remains `6aa78fc0d0ddd30e94db59470f30f47e669321dc`;
archival fixtures keep their pins.

Boa's stable v0.22 is published while retaining `1.0.0-dev`, required by Genet's
exact dependency contract. Finalization waiters, caller-realm provenance, narrow
ArrayBuffer transfer, and GC semantics remain intact. Genet pins engine and GC
to the same published revision; browser targets enable the required `js` host
glue. Public-source adapter tests and default browser Wasm compilation passed.

Clean Livery/scripted DOM/scripted worker compilation passed on immutable Genet
`e84f9c7`, whose code matched the then-primary `6cb2284`. Incoming paragraph
layout changes at `15713014e2e` subsequently overlapped unfinished primary
`text.rs` edits. The Boa update was merged with that published revision in a
focused worktree. The combined source passed all 26 adapter tests, default
browser Wasm compilation, and clean Livery/scripted DOM/scripted worker
compilation against the published Boa source.
Genet main now publishes this integration at `cca45fc7a5c`. Concurrent forms,
runtime, and layout work remains outside this publication.
See Genet's [stable Boa receipt](https://github.com/merely-made/genet/blob/main/design_docs/2026-10-08_boa_stable_upstream_refresh.md).

## Deliberate migration boundaries

| Family | Current decision and required next evidence |
| --- | --- |
| Vello | [Separate adoption lane](https://github.com/merely-made/netrender/blob/main/netrender-notes/2026-10-08_vello_upstream_lane.md): preserve append and buffer recovery through Classic research-crate moves and Hybrid-to-GPU rename; qualify all backends and actual consumers. |
| Crabslab | Latest stable v0.6.6 is already contained. Parent main deletes files carrying owned GPU allocation changes; that rewrite is outside the compatible refresh. |
| p2panda | Latest stable v0.7.1 is already contained. Maintained `mere-p2panda-net-0.7.5` remains coherent; registry 0.7.2 is not an upgrade of that fork. |
| Vano, arboard, Piccolo, Emissary | Contain the fetched parent line. Preserve maintained fork interfaces. |
| Renderling | Contains the parent line; three pre-existing local commits remain unpublished by this lane. |
| rust-gpu | Two owned compiler-version gate commits and 46 missing parent-main commits require a compiler/nightly qualification lane. Preserve the prerelease gate. |
| Genet and Mere renderer pins | Update provider first, then consumer. Genet and Mere currently share Netrender's `9607d16f` source; a one-sided repin would duplicate the family. Newer Genet includes paragraph layout and the stable Boa refresh. Re-measure that provider before adopting it in Mere; the documentation-only comparison is historical. |
| Registry API families | Mere's active `2026-10-08_dependency_currency_plan.md` records itself as the next owner-specific stage (D8), updating Netrender, then Genet, then Mere. Its Rapier/compatible update work is active. Linebender text moves with Vello; other API families earn their own consumer receipts. |

Genet declares Rust 1.86 while selected wgpu/Vello/Parley packages require newer
compilers. Cargo's MSRV-filtered suggestions can therefore offer misleading
downgrades. Keep package/toolchain ownership explicit and never replace a
qualified graphics source with an older family merely to satisfy that filter.
Servo/WebRender parent pointers are historical references after Genet's and
Netrender's architectural pivots, not blanket merge targets.

## Evidence and resource ownership

The machine-local evidence is under `Code/testing/workspace-upstreams`: fetch
inventories, ancestry comparisons, registry dry runs, source controls, logs,
qualified lock snapshots, publication receipts, and cleanup records. They are
development evidence and do not belong in the public site payload.

Stable Cargo targets under `C:\t\cargo-targets` are retained for ordinary
reuse. The first Boa pre-push hook also used the pre-existing shared
`C:\t\graphshell-target` inherited from the environment; subsequent explicit
builds use approved stable targets. That shared target has other owners.

The three isolated Cargo homes completed their gates, but verified cache
cleanup was rejected by automatic approval review with "blocked by policy",
including a retry after explicit removal authorization:
`C:\t\cargo-homes\turquet-upstreams`, `C:\t\cargo-homes\netrender-upstreams`,
and `C:\t\cargo-homes\mere-upstreams`. They remain cache data owned by this
refresh; local build owners have finished.

The only new worktree, `Code/worktrees/genet-upstreams`, was created for the
actual incoming paragraph/layout collision, qualified, published, and removed.
The primary Genet checkout retains its unfinished layout/forms/runtime changes
and local `a26cd7b` head. Published main is `cca45fc7a5c`; a primary fast-forward
remains pending because incoming `text.rs` changes overlap that unfinished work.
Under the subsequent removal authorization, three more clean worktrees whose
heads are contained in published main were removed: `mere-l3`,
`mere-sceno-editor`, and `netrender-deps`. Their ignored configuration, locks,
and generated files were archived and hash-verified before removal. Active
worktrees, unpublished commits, and Woodshed source work remain preserved.
The unregistered `mere-proof42` directory is empty; automatic approval review
also rejected its removal with "blocked by policy".
