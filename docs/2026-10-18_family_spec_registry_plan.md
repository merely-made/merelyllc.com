# Family spec registry

**Date:** 2026-10-18
**Status:** in progress. R0 (seed), R1 (validator), R2 (one home) and R3
(coverage report) landed 2026-10-18; D1–D6 ruled as proposed. R4 has partial
measurements; the remaining sections describe possibilities and dependencies.
**Data:** [`content/specs.toml`](../content/specs.toml)
**Rendered view:** the Merely Spec Atlas,
https://claude.ai/artifact/HFe3GfjV3SmvqyipkDzW5R (private until shared)
**Builds on:** `mere/design_docs/2026-08-24_standards_survey_brief.md` (verdict
grammar, 174 verdicts), `mere/design_docs/mere_docs/research/2026-07-05_w3c_standards_architecture_review.md`
(web-platform area verdicts), `genet/design_docs/2026-09-07_standards_to_features_ledger.md`
(consumer rows and WPT counts).

## Purpose

Mark, 2026-10-18: "it seems more and more to me that the point of the stack is
to implement those specs", and then "every one of the apps in addition to the
platform engines … there is a relevant spec or specs". The atlas survey
confirmed it for every repository in the family. Most of the family's specs
are not W3C's, though.

What exists today is three partial ledgers. Each is shaped by the question it
was written to answer:

- **genet's ledger** admits a row only when a consumer names it ("A feature
  nobody has asked for is not a row"). It covers 16 rows of a web platform that
  has 615 specs.
- **mere's survey** is the workspace's named home for cross-repo standards
  findings (its scope note, under DOC_POLICY §2). Its 174 verdicts were
  gathered around castellan, fleece, tabard, pelt and the apps. They are a
  markdown table whose consumer column is truncated and whose repo is implied
  by the prose.
- **the W3C review** grades thirteen web-platform areas and ten compositor
  areas, not specs.

None of them can answer "which specs does repo X answer to, how deep is it,
and what measures it". The registry answers that for every repo, spec-first:
a spec is in the registry because a repo's product falls within its scope,
whether or not a consumer has asked for it yet. Consumer relationships explain
why a spec may matter; they neither gate its recording nor impose a work order.

## Findings

Verified 2026-10-18 against genet `6f00922`, mere main, the merely-made
org listing, and the catalogs fetched that day.

- **F1. Every repository has relevant specs; about half ship one as the
  product.** Spec-as-product: genet, netfetcher (now a genet crate), vano,
  bole, didgeridoo, smolweb, turquet, wavicle, emblem, knot-editor, woodshed,
  cleromancy, dramatis, mere-linked-data, the site. Spec through dependencies:
  netrender, weld, scry, graft, turnstone. Thin: hocket (a woodshed port),
  isocosm's simulation core, pipit, mora. Only Merely's own grammars have no
  external spec: scenograph, chirograph, platen, servitor, armillary, radio-face
  and the UI internals.
- **F2. WHATWG, not W3C, owns the engine's core.** HTML, DOM, Fetch, URL,
  Encoding and Streams account for 24,756 of the 33,824 passing subtests in the
  2026-09-29 Vano census (144,238 reported).
- **F3. Outside the browser, specs are mostly IETF or community.** Seeded spec
  rows by body: W3C 168, community 91, IETF 59, IAU/IERS 15, WHATWG 15,
  vendor 13, ISO/IEC 11, Khronos 7, NIST 6, ITU 6, Ecma 4, FIDO 4,
  freedesktop 4, Unicode 1. The transport survey found about 70% of its specs
  are community protocol docs (Reticulum, LXMF, Meshtastic, MeshCore, tinySSB),
  qualified against pinned reference peers rather than spec-derived vectors.
- **F4. Genet touches 123 of 615 browser specs.** "Touches" means a WPT pass,
  an IDL interface named in `components/`, or a CSS property named in Livery
  source. Livery names 325 of webref's 758 CSS properties. The 2026-09-29
  census covered 82 shards and no CSS. The last CSS map is 2026-08-24 and lives
  on o-pc.
- **F5. The W3C TR catalog lists 1,720 documents** (318 REC, 528 Notes, 333
  retired). About 414 live documents fall outside browser-specs. The ones that
  fit this family are the semantic-web, credentials, annotation, social-web,
  timed-text, PLS/SSML and accessibility families.
- **F6. Verdicts belong to a consumer, not to a spec.** The survey grades MLS
  as SKIP for the vault and ADOPT for moot, WebRTC twice, and CSS Color 4,
  DTCG and the Public Suffix List twice. So the registry records verdicts on
  claims (spec × repo).
- **F7. The survey table has 175 rows**, one more than its stated 174, with
  23 WATCH against 22 stated. Its §7.1 re-read lines up with the 59 PULL rows
  in order, which is how the seed applied it.
- **F8. Docs lag code in both directions.** fleece ships Web Annotation
  selectors and tabard emits DTCG 2025.10, while the survey still lists both as
  to-do. Both genet and mere have a crate named `fleece`: genet's extracts
  (microdata, JSON-LD), mere's bridges Web Annotation. TLS 1.3 is now RFC 9846
  (July 2026); the seed uses that designator.
- **F9. Conformance suites exist but aren't wired.** WPT `fetch/` for
  netfetcher, Lua 5.4 tests for didgeridoo, test262 `intl402` and Annex B for
  vano and bole, the WebGL CTS for netrender, Djot's reference tests for knot
  and nematic, the W3C Web Annotation tests for fleece and Redshank, and
  RDF test suites for mere-linked-data. 195 seeded full or partial claims sit on
  specs whose `suite` is `none`.

## Model

`content/specs.toml` (schema `mer3ly.spec-registry/v1`) holds three tables.
Repository ids are the migration-ledger ids in `ops/org-migration.toml`, which
covers every organization repository; the site catalog is a subset of it.
`family` is reserved for a verdict with no single consumer.

```toml
# Illustrative.
[[spec]]
id = "fetch"                 # browser-specs or W3C TR shortname; rfcNNNN; else a stable slug
name = "Fetch Standard"
body = "WHATWG"              # see Bodies
designator = "…"             # version, RFC number or pin, when the id doesn't say it
catalog = "browser-specs"    # browser-specs | w3c-tr | rfc | none
url = "https://fetch.spec.whatwg.org/"
suite = "wpt:fetch"          # wpt:<paths> | test262[:dir] | <name> | reference:<peer> | none
requires = ["streams"]       # optional: prerequisites beyond the catalog's normative references

[[claim]]
spec = "fetch"
repo = "genet"
crate = "netfetcher"         # optional
relation = "implements"      # implements | via-dep | should | adjacent
depth = "partial"            # full | partial | stub | planned | none
verdict = "ADOPT"            # ADOPT | PULL | SKIP | WATCH (survey grammar) | split | open
evidence = "one line: a test path, a file, or the reason"
measured = { on = "2026-09-29", scope = "…", engine = "vano", wpt_pass = 2016, wpt_total = 5901 }
measured_from = { path = "spec-results.toml", format = "spec-results" }  # instead of measured; see below
consumers = ["genet:design_docs/2026-09-07_standards_to_features_ledger.md#row-7"]
unlocks = [{ feature = "page identity from canonical URLs", consumer = "mere: search surface wiring plan" }]
source = "atlas-2026-10-18"
verified_on = "2026-10-18"

[[scope]]                     # a repo whose scope is a whole catalog
repo = "genet"
catalog = "browser-specs"
categories = ["browser"]
```

**Bodies:** W3C, WHATWG, IETF, Ecma, Khronos, Unicode, ISO/IEC, IAU/IERS, NIST,
ITU, FIDO, freedesktop, vendor, community, and `merely` for a published house
spec (D5).

**Verdicts** use the survey's grammar unchanged, including Mark's 2026-09-23
PULL: "design for it now, implement in order". This preserves the dated design
judgment and its rationale, without assigning an implementation sequence.
`split` marks a seeded row whose
source graded parts of a bundle differently. `open` means nobody has ruled yet.

## Row rules

1. A spec is a row when any repository's product falls within its scope. A
   `[[scope]]` row covers a whole catalog without listing it: catalog specs in
   scope with no claim are reported as unclaimed, not written out.
2. One claim per spec, repository and crate. The verdict lives on the claim.
3. Counts are copied, never estimated, and carry their date, engine and scope
   (genet's ledger rule). A later measurement replaces the block; git keeps the
   history.
4. A `full` or `partial` claim names its suite. `none` is allowed, and is
   itself a finding that R3 reports.
5. Every claim carries `verified_on`. The survey's §1 lesson is that a
   standards inventory decays silently in months; R3 reports claims older than
   the D4 window.
6. Consumers supply relevance and dependency cues (R4), not a work order or
   an admission gate.
7. Evidence is a test, a file or a copied count, never a receipt file.
8. An `unlocks` entry names a feature and the consumer that exists, or the
   plan that names it (genet's ledger rule for its "Unlocks" column).
   Prerequisites are spec-level facts: the catalog snapshot carries each
   browser spec's normative references (from webref), and `requires` adds
   edges the catalogs don't know.

## Seed (R0)

Generated once from four inputs, then edited by hand:

| Source | Claims | What it contributed |
|---|---:|---|
| mere survey §7 + §7.1 | 262 | verdicts and consumer text; repo inferred from the crate names in the consumer text |
| atlas repo surveys | 244 | one claim per spec for 24 repositories, bundles split |
| catalog join | 123 | genet claims with WPT, IDL and CSS counts |
| genet ledger | 5 new, 15 rows attached | `consumers` links on genet claims |

After merging duplicates: 404 specs and 585 claims. Verdicts: 332 open, 89
ADOPT, 63 PULL, 56 SKIP, 36 WATCH, 9 split. Depths: 218 partial, 124 full,
128 none, 73 planned, 42 stub. Nine survey refusals with no consumer (OPAQUE,
SRP, X-Wing, FIPS 140-3, the assurance regimes, JATS, Nym, the ICC/HDR bundle,
Lottie) sit on `family`.

Known seed defects, all R2 work: survey bundles kept as one row each (AX.25 +
KISS + APRS, the nine unspecified small-web protocols, the post-quantum set),
slug ids on survey rows that have no catalog shortname, and inferred repo
attribution where the consumer text names several crates.

## Implementation history and remaining possibilities

### R1: Validator

`cargo run --bin authority -- validate` parses `content/specs.toml` and fails
on any of these:

- a duplicate spec id, or a malformed one;
- a claim whose spec or repo does not resolve;
- an enum value outside the lists above;
- a `measured` block without `on`, `scope` or a count, or a count above its total;
- a duplicate (spec, repo, crate) claim;
- a malformed `suite`;
- an `rfc` spec whose id is not `rfcNNNN`;
- a spec with no claim.

Done when validate passes on main, and a test with one bad row of each kind
fails with that row's id. **Landed 2026-10-18**: `src/specs.rs`, wired into
`authority validate`, and `tests/m12_spec_registry.rs` with ten tests.

### R2: One home

- mere survey §7 and §7.1 tables are replaced by a pointer to the registry.
  The survey's prose stays as the rationale, and its sections cite registry ids.
- genet's ledger keeps its "Unlocks" prose, and each row cites registry ids.
  The ledger's admission rule is retired in favour of row rule 1, if D2
  passes.
- The W3C review's area verdicts become verdicts on genet claims.
- Bundles split, slug ids replaced with catalog ids where one exists, and
  `split` verdicts ruled.

Done when the survey holds no verdict table, every ledger row names a
registry id, and no seeded bundle or `split` remains. **Landed 2026-10-18**:
mere `224fa70d6` (survey §7 and §7.1 point here) and `9bb2ad801` (review
note), genet `b2667f664b8` and `1141804b62c` (ledger registry column, admission
rule retired).

### R3: Coverage report

`authority specs` prints, from the registry alone:

- each repo's claims by depth and verdict;
- unclaimed specs inside each `[[scope]]`;
- claims past the D4 freshness window;
- `full` or `partial` claims whose suite is `none`;
- specs with a known suite that no claim measures.

The atlas page is regenerated from this output.

Done when the report runs without network and the atlas's §3 and §4 tables
come from it. **Landed 2026-10-18**:
- `authority specs` prints the report and `authority specs-json` emits it with
  the parsed registry (`src/spec_coverage.rs`). Unclaimed specs come from a
  committed browser-specs 5.3.0 snapshot, `content/spec-catalogs/browser-specs.json`
  (CC0-1.0), which R5 refreshes.
- `scripts/spec-atlas/build.py` turns that JSON into the atlas page, and its
  output is byte-identical to the published atlas.
- `tests/m13_spec_coverage.rs` has four tests, plus a date round-trip unit test.

### R4: Suite wiring

Suites identified in the investigation, with execution belonging to the repo
that implements the relevant behavior. This list records opportunities, without
assigning lanes or an order:

- WPT `fetch/` against netfetcher;
- Lua 5.4 tests for didgeridoo;
- test262 `intl402` and Annex B in the vano and bole runs;
- the WebGL CTS for netrender's WebGL2 route;
- Djot reference tests for knot and nematic;
- the Web Annotation tests for fleece;
- the W3C RDF test suites for mere-linked-data;
- a fresh CSS reftest census for genet.

A suite wired into its owning repo's test gate, with a copied count in the
claim's `measured` block, supplies durable evidence for that claim.

### R5: Catalog refresh

A command re-reads browser-specs, the W3C TR API and webref and diffs them
against the registry. It reports new specs inside a scope, status changes, and
superseded designators such as RFC 8446 → RFC 9846. Validate never touches the
network.

A useful refresh would expose the registry rows affected by a catalog change.
The accepted monthly-publication direction could use that output. This records
a possible mechanism, without assigning the implementation or installing a schedule.

## Decision record

The following proposals were accepted on 2026-10-18 (see Rulings). Their original
wording is retained as decision history rather than a fresh request.

- **D1. Home.** Proposed: `merelyllc.com/content/specs.toml`, beside
  `repositories.toml` and `relations.toml`, guarded by `authority validate`.
  The alternative is `mere/design_docs`, the survey's current home, which has
  no validator and is not the family map.
- **D2. Admission.** Proposed: spec-first (row rule 1), retiring genet's
  consumer gate. Consumers keep ordering the work.
- **D3. Grammar.** Proposed: the survey's ADOPT/PULL/SKIP/WATCH,
  family-wide, on claims, plus `open` and `split` for seeded rows.
- **D4. Freshness.** Proposed: R3 reports claims older than 90 days.
- **D5. House specs.** Proposed: chirograph, scenograph and the other
  published wires get `body = "merely"` rows when they ship a written spec, so
  the family's own formats are held to the same rules.
- **D6. Site.** Should merelyllc.com publish the atlas from the registry?
  Mark, 2026-10-18, on the atlas: "i am tempted to say, we should have this on
  merelyllc.com and refresh it monthly. but eh. there may be some errors". That
  is a lean, not a ruling. Proposed: yes, after R2 clears the seed defects,
  rebuilt by R5's refresh on a monthly schedule beside the daily deploy.

## Rulings

- **2026-10-18, D1–D6.** Mark: "Accept D1–D6 as proposed in the registry plan,
  and start R1, the validator." So: the registry lives in
  `content/specs.toml` (D1); admission is spec-first (D2); the survey grammar
  applies family-wide on claims (D3); claims are stale after 90 days (D4);
  published house specs take `body = "merely"` (D5); and the site publishes the
  atlas from the registry after R2, refreshed monthly by R5 (D6).

## Progress

- 2026-10-18: R0. Seeded 404 specs and 585 claims into `content/specs.toml`
  from the atlas, the catalog join, genet's ledger and mere's survey. The file
  parses with `toml` 0.9; nothing reads it yet.
- 2026-10-18: corrected Misfin. smolweb and mere's errand and comms implement
  it, and Turnstone takes it through comms by ruling U2. The atlas had copied
  the "fails" row from Turnstone's plan status table, which predates U2. That
  row is a seed-defect class R2 should look for: status tables that later
  rulings have overtaken.
- 2026-10-18: R1 landed. Repo ids resolve against the migration ledger, not
  `content/repositories.toml`. A site repository needs a matching
  `github-metadata.json` entry (`tests/m4_repositories.rs` asserts the counts
  are equal), and that cache is refreshed only by the Pages workflow's
  PowerShell script. The ledger already lists `boa-fork` (bole) and
  `piccolo-fork` (didgeridoo), and the registry now uses those ids. dramatis was
  in neither file, so it gained a ledger record from the GitHub API (head
  `cf562f4`, MPL-2.0). The 91 `measured` blocks holding only IDL or CSS name
  counts gained `on` and `scope`, which row rule 3 requires. The full test
  suite, clippy and rustfmt pass.
- 2026-10-18: R2 landed. The registry was regenerated from the same inputs
  plus mere's W3C review, with the hand corrections since R0 carried forward.
  It now holds 503 specs and 735 claims. Verdicts: 284 open, 176 ADOPT,
  128 PULL, 92 SKIP, 55 WATCH, none split.
  - Every survey row has a curated mapping. About fifty bundled rows split into
    one spec each, taking §7.1's per-part grades (WebRTC ADOPT / SIP SKIP;
    KISS ADOPT / AX.25 and APRS SKIP; RIFF/WAVE ADOPT / BWF and BW64 PULL;
    did:key and did:plc ADOPT / did:web PULL).
  - The RFC-number rule had filed "Audio metadata tag formats" under FLAC's
    RFC 9639, because the row cites Vorbis comments' RFC section. It is now
    APEv2, Vorbis comments and ID3v2.
  - The MLS rows separate castellan's SKIP from the transport's ADOPT.
  - Merge rule: a ruled verdict beats `open`. Between ruled verdicts the later
    verification wins (DOC_POLICY §2, newer is more authoritative), so the
    survey's grades override the review's.
  - Kept as one spec each, because each is one document or one model used as a
    whole: Gemini and gemtext, Scroll and scrolltext, IAU 2006/2000A, TLE with
    SGP4, the TT/UT1/UTC/TAI time scales, WebKitGTK with WPE, DMA-BUF with DRM
    fourcc.
  - The R2 generator is not committed: it reads catalogs fetched for the atlas.
    From here the registry is edited by hand, and R5 owns catalog diffs.
- 2026-10-18: R3 landed. The first report (registry plus the browser-specs 5.3.0
  snapshot) finds:
  - 446 of genet's 623 in-scope browser specs unclaimed;
  - one registry id the catalog dropped (`sanitizer-api`, gone between
    browser-specs 4 and 5.3.0);
  - 30 stale claims, all W3C-review verdicts dated 2026-07-05;
  - 219 full or partial claims on specs with no suite;
  - 65 specs whose suite no claim measures. These are R4's candidate list:
    test262 and `intl402` for vano and bole, the Lua 5.4 tests, the Unicode
    break tests, the WebGL CTS, Djot's and CommonMark's reference tests, the
    W3C RDF suites, and reference-peer vectors for the radio protocols.
- 2026-10-18: R4 lane landed: Lua 5.4 tests for didgeridoo. lua-5.4.9-tests run
  per file from `cargo test --test lua_suite` (didgeridoo 63928fa). 32 files:
  1 pass, 1 skips itself without ltests, 22 fail, 8 cannot-load. The
  `lua-5.4`/`piccolo-fork` claim carries `files_all_pass = 1, files = 32`.

## Recovering the analysis

2026-10-19: Mark clarified the purpose of preserving this analysis: future
judgment benefits from recoverable findings and dependencies, without inheriting
assignments or a prescribed order. The registry is a map of claims and evidence,
not a queue. The unfinished possibilities in this document carry no commitment
to implement them. ADOPT/PULL/SKIP/WATCH record design judgments under their
existing definitions and verification dates; they do not schedule future work.

The durable material is in these locations:

- `content/specs.toml`: spec identities, consumers, dated claims and measurements.
- `cargo run --bin authority -- specs`: offline coverage report using the
  committed catalog. `specs-json` includes the underlying registry rows.
- `authority validate`: checks registry structure and references.
- `scripts/spec-atlas/build.py`: renders the report data as the atlas.
- Mere's standards survey and W3C review: the reasoning behind the verdicts.
- Genet's standards ledger: consumer relationships and historical WPT counts.
- `didgeridoo/LUA54_GAP_PLAN.md`: Lua findings, interpretation and dependencies.
  The local clone is `crates/piccolo`, and its registry repo id is `piccolo-fork`.
  Its 1/32 raw file result is limited by early missing APIs that mask later tests.

An entry point for another conversation is: "Read the family spec registry and
its analysis notes; show me the evidence and dependencies around [spec/repo]."
The source and dates make it possible to reassess a claim without repeating the
whole survey. They do not determine what is worth doing at that later moment.

R5 describes how a catalog refresh could work. Monthly site publication was
accepted as a design direction; neither an installed schedule nor a future
assignment follows from these notes.

## Cues: prerequisites and unlocks (2026-10-21)

Mark, 2026-10-21, on how findings should read: "telling yourself to do work in
the future is less appealing than telling them what's wrong and what would
improve the stack." A claim already says what is wrong; `unlocks` says what
would improve, and `requires` connects gaps to the features they hold back.

- The snapshot gained each catalog spec's normative references from
  w3c/webref `ed/refs` at 14d0b42 (MIT): 501 of 623 specs. They are mapped to
  catalog and registry ids, without RFC 2119 and RFC 8174, which define
  requirement keywords, not prerequisites.
- 16 specs carry hand-written `requires`, for example LXMF needs Reticulum and
  ActivityPub needs Activity Streams and JSON-LD.
- 32 claims carry `unlocks`. They are seeded from genet's ledger column, with
  its consumers on record, and from the atlas's strong fits, each naming an
  existing consumer.
- `authority specs` lists cues. A cue is a claim short of `full` whose gap
  holds back a named feature, either directly or as a prerequisite of another
  spec the same repository claims. The first report lists 63.
- A spec with more than 25 normative references is an umbrella; 25 is the
  catalog's 90th percentile, and HTML has 139. Its references aren't read as
  prerequisites of its features, which would otherwise put CSP and Mixed
  Content behind HTML workers. Hand-written `requires` always count.

## Counts that arrive by themselves (2026-10-22)

Mark asked for a roadmap that "automatically and predictably updates and
shows the progress as we make commits", and chose to begin with the counts.

- A claim's `measured_from` names a results file in the repository that runs
  the suite: `{ path, format, ref?, key?, repo? }`. Two formats are read.
  `spec-results` (`merely.spec-results/v1`) is one `[[result]]` per spec, with
  a `measured` block in the registry's shape. `test262-metrics` is the test262
  runner's `{ results: { pass }, total }`. A claim with `measured_from` may not
  also carry `measured`.
- `authority spec-sources` lists the files, resolving slug and branch through
  the migration ledger. `scripts/refresh-spec-results.py` fetches each one at
  the latest commit that touched it into `content/spec-results/`.
  `sources.json` records that commit and its date. The Pages workflow runs the
  refresh after the metadata refresh on every push and daily build, then
  validates. The committed cache is the baseline for local builds.
- Every command but `spec-sources` fills those claims from the cache. Each
  count's `on` is the source commit's date, and its scope names the slug,
  path and commit.
- Five claims read this way:
  - vano: `ecma262` from `tests/metrics.json` (CI-gated) and `ecma402` from
    `tests/metrics-skipped.json#intl402`;
  - bole: `ecma262` and `ecma402` from `spec-results.toml` on its `genet` branch
    (`4f50841e`), mirroring `docs/test262.md`;
  - didgeridoo: `lua-5.4` from `spec-results.toml` (`a2648e4`).
- didgeridoo's suite test fails when its file disagrees with a run, and
  `LUA_SUITE_RECORD=1` rewrites it. So its published count follows every tested
  commit, not someone's memory to copy it.
- Still hand-copied: genet's WPT census and name counts, whose sources are
  long runs and session-time joins. They have no small results file yet.
- `tests/m14_spec_results.rs` has five tests: the committed cache applies, a
  fixture overlay carries its source commit, a missing source fails, sources
  resolve through the ledger, and `measured` with `measured_from` is rejected.

## The roadmap page (2026-10-22)

`/roadmap/` reads the registry as features. Each `unlocks` entry is a feature.
Its chain is the claim that unlocks it plus that spec's prerequisites claimed by
the same repository, under the same rule as the cues. The page and its data are
rebuilt on every deploy from `content/specs.toml` and the fetched results, so a
commit that changes a claim or a suite's results file shows on the next deploy.

- **First view.** A frozen scene: the roadmap graph solved by the repository
  graph's solver (`mer3ly_repo_graph`) and read by graphshell-client's frozen
  reader, styled like `/repos/`'s. Below it sit the features with their chains
  (depth, counts, registry lines, consumer plans), every count with its source
  commit, and coverage by repository.
- **Live map.** Graphshell's qualified viewer, loaded on request where WebGPU
  exists, draws `spec-roadmap-dataset.json`, a `scenomise.host-dataset/v1`
  envelope. Repositories claim specs, specs require specs and specs unlock
  features. `detail_url` and `source_url` link each node to its spec document,
  its registry line or the plan that names it.
- **Sizes.** First seed: 46 features on 64 specs, of which 5 are complete; 119
  scene items and 194 relationships. The page is about 18 KB gzip, under
  Ruling 157's 64 KiB first-load cap, and the dataset about 15 KB.
- **Publication rule.** Links follow the artifact gate. GitHub links go only to
  catalogued merely-made repositories, so bole's and didgeridoo's source commits
  show as text. `validate-artifact` rebuilds the page and dataset and requires
  the published bytes to match.
- **Code.** `src/spec_roadmap.rs` (model, scene, dataset),
  `src/pages/roadmap.rs`, `assets/roadmap-mount.js`, and
  `tests/m15_roadmap.rs` (three tests).
- **Out of scope:** a history slider over registry checkpoints, like
  `/repos/`'s v2 history, and the live viewer's generic "project" wording in its
  inspector.
