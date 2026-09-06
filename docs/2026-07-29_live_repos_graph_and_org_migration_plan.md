# Live repository graph and Merely organization migration

**Status:** M6 automated metadata and Pages deployment accepted; plan complete
**Date:** 2026-07-29
**Authority:** This is the canonical plan for the public repository graph on
Mer3ly and for moving Merely-owned repositories into the `merely-made`
organization.

**Follow-on, 2026-09-02:** the completed M0-M6 work below remains the public
graph and transfer receipt. The new semantic-boundary and topology review is
owned by
`mere/design_docs/mere_docs/implementation_strategy/2026-09-02_platform_boundary_and_repository_topology_plan.md`;
future authorized GitHub mutations return here for execution receipts.

## Outcome

Mer3ly gains a `/repos/` page with two projections of one repository model:

1. semantic HTML that lists every public Merely repository and its concrete
   relationships; and
2. an optional live Mere graph that lets visitors explore the same nodes and
   edges.

The relevant project repositories move from `mark-ik` to `merely-made` in
verified batches. The transfer must not split Cargo's view of a source
repository, publish private material, or make the public site depend on WebGPU
or Wasm merely to remain readable.

## Ownership

Mer3ly owns:

- the public repository manifest and relation ledger;
- the GitHub metadata cache used by the site;
- the semantic repository page;
- the optional live graph client;
- the migration ledger and receipts.

The `merely-made/.github` repository continues to own only
`profile/README.md`, as required by that repository's local instructions. It
may link to `https://mer3ly.net/repos/` after the page is public, but it does
not own this plan.

Each source repository continues to own:

- its code, licensing, provenance, releases, and publication claims;
- its own sensitive-information remediation;
- its dependency and lockfile updates;
- its build and test receipts.

No repository transfer is implied by merging this plan. Transfers are
separate external actions, authorized and executed one batch at a time.

## Current evidence snapshot

This snapshot is evidence for planning, not a permanent inventory.

- `merely-made` currently contains two public repositories:
  `.github` and `mer3ly-net`.
- Thirteen Merely project repositories under `Code/repos/` are public under
  `mark-ik`: `genet`, `hocket`, `isometry`, `mere`, `netrender`, `retinue`,
  `smolweb`, `turnstone`, `wavicle`, `wgpu-graft`, `wgpu-scry`, `wgpu-weld`,
  and `woodshed`.
- Those thirteen repositories currently contain 235 tracked files referring
  to `mark-ik`, including 102 `Cargo.toml` files. GitHub redirects make a
  transfer survivable, but they do not make a mixed Cargo source graph safe.
- The local `Code/crates/` area also contains maintained forks or donor
  checkouts with `mark-ik` remotes: `arboard`, `boa`, `emissary`, `iroh`,
  `p2panda`, `piccolo`, `stylo`, `vano`, and `xilem`. The `vano` checkout's
  origin still uses the old `mark-ik/nova` redirect. Active Cargo manifests
  also name the public `iroh-address-lookups` and `swarm-discovery` forks,
  which have no checkout in the inspected local fork area. None of these are
  admitted to the organization migration until the fork gate below classifies
  them.
- `merecat` and `strophe` appear in current product language or local testing
  material but do not have an authoritative repository in the inspected
  `Code/repos/` inventory. The site must not invent repository nodes for them.
- M3 replaced the generated, roughly 1.1 MB runtime bundle with 28,025 bytes of
  base HTML and CSS plus a 177,124-byte social image.
- GitHub's legacy Pages build completes from `main`, while the public domain is
  Cloudflare-fronted. HTTPS works, but plain HTTP still returns 200 and
  GitHub cannot issue its own certificate through the current DNS boundary.
- The generated footer contains only the intentionally public company
  location, domain, and GitHub organization contact.

Refresh this evidence before the first transfer and store the output as a
receipt. Do not treat the counts above as a live ledger.

## Product boundary

### Cambium

Cambium authors the page views. It already constructs semantic element trees
in a retained `ScriptedDom`.

### Genet

Genet provides the DOM serializer, static-profile parse/layout checks, and
headed rendering receipts. The deployed baseline remains ordinary HTML and
CSS. Visitors do not need Genet, WebGPU, or JavaScript to read the site.

### Mere

Mere supplies the graph truth and portable graph-canvas projection for the
enhanced repository view. The Mer3ly graph client should be a thin Wasm
consumer of reusable Mere graph/canvas seams, not a copy of Graphshell's full
application host.

The HTML list and the graph must consume the same repository and relation
records. Neither projection may become a second source of truth.

## Planned repository seams

The exact Rust module split may move during implementation, but ownership must
remain recognizable:

```text
Cargo.toml
src/
  main.rs                    static site build entrypoint
  site.rs                    shared document shell, metadata, navigation
  pages/
    home.rs
    radio.rs
    repos.rs                 semantic repository projection
  repositories.rs           typed repository and relation records
  github_metadata.rs         build-time public metadata merge
content/
  repositories.toml         curated public repository records
  relations.toml            curated non-Cargo relationships
assets/
  site.css                   shared responsive styles
ops/
  org-migration.toml         operational transfer ledger, not site content
scripts/
  inventory-repositories.ps1 read-only local/GitHub inventory
html/                        generated Pages output during the first cut
docs/
  receipts/
    site/
    org-transfer/
```

Generated Cargo targets, metadata scratch files, tokens, local absolute paths,
and headed test output stay ignored. Human-sized HTML and screenshot receipts
may be committed when they are the acceptance evidence for a milestone.

## Repository model

`content/repositories.toml` gives every public project a stable, owner-neutral
id. GitHub ownership may change without changing the graph identity.

Required repository fields:

- `id`
- `github_slug`
- `name`
- `summary`
- `class`: `foundation`, `platform`, `product`, `tool`, or `maintained-fork`
- `status`: `active`, `prototype`, `reference`, `research`, or `archived`
- `license`
- `homepage`
- `public`

`content/relations.toml` stores typed edges:

- `depends_on`
- `contains`
- `reference_app_for`
- `host_for`
- `uses_ui_from`
- `renders_with`
- `fork_of`

Every edge records:

- stable source and target ids;
- whether it is `derived` or `curated`;
- a source path, package id, or short evidence note;
- the last verification date.

Cargo dependencies and GitHub fork ancestry are derived evidence. Product
roles such as `reference_app_for` are curated evidence. A derived edge must
not silently become a product claim.

## Migration ledger

`ops/org-migration.toml` tracks operational state separately from public site
content. Each candidate records:

- current and target GitHub slugs;
- classification and intended transfer batch;
- default branch and current head;
- license/provenance result;
- sensitive-information result;
- Pages, Packages, Actions, release, deploy-key, and webhook state;
- count of old-owner manifest and documentation references;
- transfer status;
- verification receipt path.

The ledger may say `hold` without hiding an otherwise public repository from
the site. Moving ownership and documenting the project are related, but not
the same decision.

## Milestones

### M0: Establish the authority files

Add the typed repository model, relation model, migration ledger, and
read-only inventory script.

The initial inventory must distinguish:

1. Merely products and platforms;
2. reusable foundation repositories;
3. maintained upstream forks;
4. donor/reference checkouts;
5. local testing and generated material.

**Done when**

- every current `merely-made` and selected `mark-ik` public repository has one
  ledger entry;
- repository ids and GitHub slugs are unique;
- every relation endpoint exists;
- missing products such as Merecat or Strophe are recorded as unresolved
  product references rather than fabricated repositories;
- the inventory receipt records current visibility, default branch, Pages,
  Packages, Actions, license detection, and old-owner reference counts.

**Stop rule:** do not transfer any repository before M0 is committed and its
inventory has been reviewed.

### M1: Run the publication gate

Audit each transfer candidate independently.

The gate covers:

- intended public scope and repository identity;
- license and inherited-source provenance;
- current tree and Git history for credentials, local paths, usernames,
  personal contact details, private hostnames, IP addresses, device ids, radio
  coordinates, or other operational material;
- GitHub Actions permissions, secrets, environments, deploy keys, webhooks,
  Pages, Packages, releases, and Marketplace actions;
- machine-local `.cargo/config.toml` or tooling paths;
- branch protection, rulesets, collaborators, and organization defaults;
- current default branch pushed and recoverable.

`.gitignore` prevents future accidental additions. It does not remove tracked
files or prior commits. Tracked sensitive material requires deletion and, when
the exposure warrants it, credential rotation or a separately authorized
history rewrite.

**Done for one repository when**

- the ledger names the public scope and license;
- the current tree has no accidental sensitive material;
- any historical finding has an explicit remediation decision;
- Pages/Packages/Actions transfer behavior is known;
- the intended default branch and release state are pushed;
- the receipt contains commands and results without reproducing secrets.

**Stop rules**

- stop on uncertain license or donor provenance;
- stop on an unrotated credential or unidentified private endpoint;
- stop when the target organization already contains the target name or fork
  network;
- stop when a Pages site, package, or Marketplace action lacks a migration
  decision;
- do not transfer a maintained upstream fork merely because it has a
  `mark-ik` remote.

#### M1 receipt: 2026-07-29

The publication audit covers all 13 transfer candidates at their pushed
default-branch heads.

- All current candidate trees pass the redacted HEAD secret scan and contain
  no local profile path, local machine name, configured contact address, or
  first-party private-network match.
- Full-history scanning found only reviewed historical material in Netrender,
  Genet, Mere, Retinue, and Isometry. Genet's large count is concentrated in
  inherited WPT and Servo test material. No history rewrite is authorized;
  current trees were sanitized and public upstream history is retained.
- Netrender, Genet, Mere, Turnstone, Woodshed, and Wavicle received separate
  remediation commits and were pushed before the receipt was generated.
- The target organization has no target-name or fork-network collision for
  any candidate.
- Repository settings contain no repository or environment secrets,
  variables, deploy keys, webhooks, Pages sites, releases, branch protection,
  repository rulesets, or Marketplace action metadata. Genet has one empty,
  unprotected Actions environment.
- The authenticated package inventory covers container, npm, Maven, RubyGems,
  and NuGet packages. It found no packages associated with the transfer
  candidates.
- The target organization does not have organization rulesets because that
  feature is unavailable on its current GitHub plan. Repository-level
  rulesets are also absent.

All 13 candidates are ready with no publication-gate blockers. M2 begins with
the foundation batch and must preserve one canonical Cargo source identity
before the platform batch starts.

### M2: Transfer repositories in dependency-aware batches

Proposed batches:

1. **Foundation:** `netrender`, `smolweb`, `wavicle`, `wgpu-graft`,
   `wgpu-scry`, `wgpu-weld`.
2. **Platform:** `genet`, `mere`, `retinue`.
3. **Products:** `turnstone`, `woodshed`, `hocket`, `isometry`.
4. **Maintained forks:** separately authorized after the M1 fork decision.

Within a batch, transfer providers before consumers. GitHub redirects remain
useful during the transition, but old and new Git URLs must not persist as two
active Cargo sources.

For each repository:

1. record the pre-transfer head and settings receipt;
2. execute the GitHub transfer after explicit authorization;
3. update local `origin`;
4. update root package `repository` and homepage metadata;
5. update Git dependencies and URL-keyed `[patch]` tables;
6. regenerate the lockfile intentionally;
7. update badges, documentation, release automation, and external links;
8. run `cargo metadata` and the repository's focused verification wall;
9. verify old GitHub and Git remote URLs redirect;
10. save `docs/receipts/org-transfer/<repo>.md`.

Do not create a new repository at the old `mark-ik/<name>` location. GitHub
warns that doing so permanently removes the transfer redirect.

**Batch done when**

- every transferred repository resolves at `merely-made/<name>`;
- local and CI remotes use the canonical URL;
- no active manifest or URL-keyed patch table uses the old owner for a
  transferred dependency;
- `cargo metadata` contains one intended source identity per shared git
  package;
- lockfiles and focused checks pass;
- old repository URLs redirect and any Pages/package repair is complete.

**Stop rule:** do not begin the next batch while a consumer can resolve both
old-owner and new-owner URLs for the same package family.

#### M2 foundation receipt: 2026-07-29

The foundation batch transferred `netrender`, `smolweb`, `wavicle`,
`wgpu-graft`, `wgpu-scry`, and `wgpu-weld` to `merely-made`.

- All six local origins use the canonical organization URL. Their former web,
  API, and Git URLs redirect to the transferred repositories and resolve the
  same `main` head.
- Package metadata, documentation links, Git dependencies, and tracked locks
  use `merely-made` for every transferred foundation source.
- Genet and Mere were not transferred in this batch, but their foundation
  dependency references were canonicalized and pushed before product locks
  were regenerated.
- Turnstone's ignored local lock and ignored Cargo patch files were updated
  locally but remain outside Git. Woodshed, Hocket, and Isometry received
  clean-checkout lock regeneration with zero old foundation sources.
- The source-identity stop rule found a temporary old/new Netrender split
  while Woodshed still resolved the previous Genet commit. Pushing Genet and
  Mere first and regenerating the product locks removed the split.
- Wgpu-graft's stale CI package name was corrected to `grafting`; its minimal
  feature check now explicitly enables `wgpu-29`.
- Focused verification passed for all foundation repositories and affected
  consumers. This includes Turnstone's 154 library tests and Isometry's
  workspace-wide all-features/all-targets check.

The six repository receipts under `docs/receipts/org-transfer/` record heads,
settings, redirects, commits, and verification. Stop before the platform
ownership batch so this boundary remains reviewable.

#### M2 platform receipt: 2026-07-29

The platform batch transferred `genet`, `mere`, and `retinue` to
`merely-made`.

- All three local origins use the canonical organization URL. Their former
  web, API, and Git URLs redirect to the transferred repositories and resolve
  the same `main` head.
- Package metadata, documentation links, Git dependencies, URL-keyed patch
  tables, and active lock sources use `merely-made` for all three platforms.
- Genet, Retinue, and Mere were committed and pushed in provider order. Mere's
  clean metadata resolves Genet at `2955d41c` and Retinue at `7439a79d`.
- Hocket, Isometry, Woodshed, and Turnstone locks resolve one Genet source at
  `2955d41c` and one Mere source at `d5af0618`. Turnstone's lock remains
  ignored; the other three locks are tracked.
- Cargo's first clean pass had to materialize Genet's large Servo and
  standards-fixture worktrees under the new Git source identity. Final locked
  offline metadata passed after that one-time cache work.
- Focused verification passed for Genet, Mere, Retinue, Hocket, Isometry,
  Woodshed, and Turnstone. This includes Retinue's complete host test suite,
  Woodshed's 167 tests and 4 doctests, Turnstone's 158 library tests with 4
  ignored, and Isometry's workspace-wide all-features/all-targets check.

The three platform receipts under `docs/receipts/org-transfer/` record heads,
settings, redirects, source pins, commits, and verification. Stop before the
product ownership batch so this boundary remains reviewable.

#### M2 product receipt: 2026-07-30

The product batch transferred `woodshed`, `hocket`, `turnstone`, and
`isometry` to `merely-made`.

- Woodshed transferred first because Hocket consumes its `audio-primitives`
  package. Hocket's tracked lock now resolves that package from Woodshed
  commit `5011b91a`.
- All four local origins use the canonical organization URL. Their former
  web, API, and Git URLs redirect to the transferred repositories and resolve
  the same `main` head.
- Package metadata, documentation links, release-feed identities, and active
  lock sources use `merely-made` for all four products.
- Mere's luggage tests exposed a stale owner expectation in its GitHub feed
  fixtures. Commit `d18cfdf7` updates both the active Hocket feed references
  and their expected owner values.
- Tracked repositories outside the historical Mer3ly receipts contain zero
  `mark-ik/woodshed`, `mark-ik/hocket`, `mark-ik/turnstone`, or
  `mark-ik/isometry` references.
- Focused verification passed for all affected products and Mere. This
  includes Hocket's 38 model tests, Turnstone's 158 library tests with 4
  ignored, Isometry's workspace-wide all-features/all-targets check, and all
  34 luggage library tests plus its binary test and doctest.

The four product receipts under `docs/receipts/org-transfer/` record heads,
settings, redirects, commits, and verification. The transfer sequence stops
before the maintained-fork review; none of the fork-review holds moved.

#### M2 maintained-fork review receipt: 2026-07-30

The review kept ten thin forks under `mark-ik` and admitted Vano to
`merely-made`.

- All eleven are genuine GitHub forks with explicit upstream parents and no
  target-name or fork-network collision in `merely-made`.
- Vano crossed the organization threshold because its data-oriented engine
  architecture is integral to Genet and its maintained branch carries a
  substantial independent project surface.
- Arboard, Boa, Iroh Address Lookups, P2panda, Piccolo, Stylo, and Swarm
  Discovery remain personal patch carriers. Emissary remains an evaluation
  patch, Iroh is an upstream mirror, and Xilem's former Woodshed branches are
  retired from active manifests.
- Future fork admissions use the same test: integral to a Merely project and
  substantial enough to present an independent maintained surface.
- Redacted current-tree and fork-only history scans found no credential,
  machine-profile path, machine name, configured contact address, or
  first-party private endpoint. Two P2panda detector matches are inherited
  variable names, not credential material.
- Vano's integration line fast-forwarded `main`; project commit `1816f328`
  established its identity and organization metadata before transfer.
- Genet now resolves `merely-made/vano` while preserving Vano commit
  `22b42989` in the ignored local lock. A clean-path locked offline check
  compiled that commit from the canonical repository.

The aggregate fork review receipt is
`docs/receipts/org-transfer/2026-07-30_fork_review.md`; Vano's repository
receipt is `docs/receipts/org-transfer/vano.md`. No other fork transferred,
rewrote history, deleted a repository, or changed archival state.

### M3: Replace the generated site bundle with a static Cambium build

Create a small Rust site builder. Cambium views build `ScriptedDom` trees;
Genet's HTML serializer emits deployment documents.

Preserve the current visual language while replacing the bundled runtime:

- shared document shell and navigation;
- real titles, descriptions, canonical URLs, Open Graph metadata, and
  structured data;
- external stylesheet rather than repeated inline declarations;
- semantic headings, landmarks, lists, links, diagrams, and tables;
- responsive behavior at narrow and regular widths;
- explicit public-contact content.

The first cut may continue to write `html/` while deployment is stabilized.
The desired endpoint is an Actions-built Pages artifact with `CNAME` preserved
and HTTPS enforced after domain verification.

**Done when**

- home and radio pages are generated from maintainable source;
- both remain readable with JavaScript disabled;
- Genet parses and lays out both pages at 375, 420, 900, and 1440 CSS pixels;
- headed screenshots at those widths have no clipping, overlap, or horizontal
  page overflow;
- the public footer contains only intentionally approved contact details;
- the base HTML, CSS, and repository data stay below 200 KiB uncompressed,
  excluding images, fonts, and the optional graph bundle;
- the current React/bundler manifest is gone;
- Pages serves the custom domain over enforced HTTPS.

**M3 result, 2026-07-30:** The source, size, privacy, Genet layout, headed
browser, and public-deployment gates passed. At initial acceptance, the last
condition was only partly met because HTTP did not redirect and GitHub could
not enable its certificate while Cloudflare fronted the domain. Cloudflare's
`Always Use HTTPS` setting closed that gate later the same day. The exact
receipt is
[`docs/receipts/site/2026-07-30_m3_static_site.md`](receipts/site/2026-07-30_m3_static_site.md).
M4 was not started.

### M4: Publish the semantic `/repos/` page

Render the repository manifest as ordinary HTML first.

Each repository entry shows:

- name, summary, status, class, and license;
- canonical GitHub and homepage links;
- latest public metadata timestamp;
- concrete incoming and outgoing relationships;
- explicit labels for curated and derived relationships.

GitHub metadata is fetched during the trusted build, cached as a generated
input, and reduced to public fields. The deployed browser receives no GitHub
credential.

**Done when**

- `/repos/` is useful before JavaScript or Wasm loads;
- every visible repository and edge comes from the typed authority files;
- stale or failed GitHub refreshes retain the last valid public cache and show
  its timestamp;
- private repositories and authenticated-only metadata cannot enter the
  generated artifact;
- links, filters, and relation summaries work by keyboard and screen reader;
- the narrow-width receipt has no horizontal overflow.

**M4 result, 2026-07-30:** `/repos/` now projects all 16 public repositories
and 25 typed relations as semantic static HTML. The reduced GitHub cache,
atomic stale-cache fallback, native class filters, responsive Genet and headed
receipts, repository-topic taxonomy, and public deployment passed. The exact
receipt is
[`docs/receipts/site/2026-07-30_m4_repository_index.md`](receipts/site/2026-07-30_m4_repository_index.md).
M5 was not started.

### M5: Add the live Mere graph

Build a thin `mer3ly-repo-graph` Wasm client over Mere's reusable graph and
canvas seams.

The client:

- reads the same public repository and relation records as the HTML page;
- projects repository class, status, and relation kind into an accessible
  legend;
- supports focus, keyboard traversal, pan, zoom, selection, and opening the
  corresponding semantic entry;
- pauses animation when hidden or settled;
- respects reduced-motion preferences;
- loads only after the semantic page is present;
- fails closed to the HTML list when WebGPU, Wasm, or initialization is
  unavailable.

Genet may render the graph client for headed receipts, but it is not part of
the production browser requirement.

**Done when**

- HTML and graph projections contain identical repository ids and edge ids;
- selecting a graph node focuses or opens its semantic repository entry;
- every edge style has a text equivalent;
- graph failure leaves the complete repository list available;
- mobile, keyboard, reduced-motion, WebGPU-unavailable, and ordinary headed
  paths have explicit receipts;
- the graph has no dependency on Graphshell application state, browser
  history, Personae identity, or a resident Mere host.

**M5 result, 2026-07-30:** `/repos/` now progressively loads a 16-node,
25-edge WebGPU map laid out by Mere's pinned radial arrangement. Keyboard,
pointer, pan, zoom, reduced-motion, narrow-layout, and forced WebGPU, Wasm,
and initialization failure paths passed headed review. All failures retain the
complete semantic index. The exact receipt is
[`docs/receipts/site/2026-07-30_m5_live_repository_graph.md`](receipts/site/2026-07-30_m5_live_repository_graph.md).
M6 was not started.

### M6: Automate metadata and deployment

Add a Pages workflow that:

1. validates the authority files;
2. refreshes public GitHub metadata with the workflow's scoped token;
3. builds the Cambium static site;
4. builds the optional Mere graph bundle;
5. checks generated output for secrets, personal data patterns, local absolute
   paths, and private repository slugs;
6. runs static and headed smoke receipts;
7. deploys the exact checked artifact.

Use scheduled refresh and manual dispatch first. Cross-repository dispatch can
follow only if it does not require a broad personal token.

**Done when**

- the deployed artifact is reproducible from one pinned source revision;
- workflow permissions are read-only except for the Pages deployment grant;
- no token or authenticated API response is included in the artifact;
- a failed metadata refresh or graph build cannot replace the last good site
  with a partial page;
- the page displays its metadata refresh timestamp.

**M6 result, 2026-07-30:** Scheduled and manually dispatched GitHub Actions now
refresh the reduced public metadata, verify the authority, reproduce the pinned
Mere graph build, validate and smoke the exact self-contained site artifact,
and deploy it through GitHub Pages. Cloudflare proxies that origin for DNS,
TLS, HTTPS redirection, and HSTS. Its former Worker custom domain was detached,
and the public edge was hash-matched to the checked Pages artifact. The exact
receipt is
[`docs/receipts/site/2026-07-30_m6_automated_pages_deployment.md`](receipts/site/2026-07-30_m6_automated_pages_deployment.md).

## Verification ladder

Do not collapse these into one claim:

1. **Model-valid:** authority files parse and all ids/edges resolve.
2. **Static-valid:** generated HTML parses, contains required metadata, and is
   useful without scripts.
3. **Genet-rendered:** the static pages pass Genet parse/layout receipts.
4. **Browser-rendered:** headed desktop and mobile screenshots are clean.
5. **Graph-wired:** the Wasm graph consumes the same ids and edges.
6. **Graph-headed:** keyboard, pointer, fallback, and reduced-motion paths work
   in a real browser.
7. **Migrated:** GitHub ownership, redirects, Cargo source identity, CI, Pages,
   and packages are all verified for the completed batch.

## Security and privacy rules

- Public site builds consume only public repository metadata.
- Never place a personal access token in JavaScript, Wasm, committed config, or
  a Pages artifact.
- Do not publish local filesystem paths, usernames, private hostnames,
  addresses, network topology, device identifiers, exact radio sites, or
  unapproved personal contact details.
- Do not serialize a personal Mere graph to make the repository page.
  Construct a bounded public graph from the repository authority files.
- Store security receipts as results and hashes. Do not copy discovered secret
  values into Markdown.
- History rewriting, repository deletion, visibility changes, and GitHub
  transfers remain explicitly authorized operations.

## Open decisions

These do not block M0:

1. Whether Mer3ly's source and editorial content receive an explicit license.
2. Which public contact channel replaces the current personal footer details.
3. Whether the final Pages deployment commits generated `html/` or deploys
   only an Actions artifact. The recommended endpoint is the artifact.
4. Whether repository metadata refreshes on a schedule alone or later receives
   narrow cross-repository dispatch events.

M6 resolved decisions 3 and 4: deployment uses only the checked Actions
artifact, with scheduled refresh and manual dispatch. Cross-repository
dispatch remains unimplemented.

The public contact decision was resolved on 2026-07-30:
`markik@mer3ly.net` is the approved address. The artifact validator permits
that exact address and continues to reject other email addresses.

Maintained-fork ownership was resolved on 2026-07-30: Vano belongs in
`merely-made`; the ten thinner forks stay under `mark-ik`. Re-run the
integral-and-substantial test as those forks evolve.

## Authorization history

M0 was the first implementation slice. It changed no GitHub ownership and
deployed nothing. It created the authority files, migration ledger, validation,
inventory script, and an evidence receipt.

### M0 receipt: 2026-07-29

- The public authority contains 15 repositories and 24 relations.
- The migration ledger contains 26 repositories: 13 transfer candidates, 11
  maintained-fork holds, and 2 unresolved products.
- `cargo test` passes 2 authority tests; `cargo clippy --all-targets --
  -D warnings` and `authority validate` pass.
- The sanitized inventory receipt covers all 26 migration records, identifies
  2 extra local donor/tooling repositories, and reports 0 drift findings.
- Public GitHub package pages listed no packages. M1 later confirmed that result
  through the authenticated package API across all supported registries.

M0 was reviewed and accepted on 2026-07-29, then committed as `965fc2a`. No
GitHub ownership, Pages deployment, or site implementation changed in M0.

M1 was reviewed and accepted on 2026-07-29. The refreshed authenticated audit
reported 13 ready candidates and 0 blockers. M2 was executed through the
dependency-aware batches and their stop rules.

The maintained-fork review was authorized on 2026-07-30. It transferred Vano,
retained ten thinner forks under `mark-ik`, and canonicalized Genet's Vano
source without changing its locked commit.

M3 was implemented and accepted on 2026-07-30. Commit `b3e99e4` replaced the
generated runtime site with the static Cambium/Genet build. The public domain
serves the result through Cloudflare. Enabling Cloudflare's `Always Use HTTPS`
setting later that day closed the remaining deployment gate with direct 301
redirects at the edge.

M4 was implemented and accepted on 2026-07-30. Commit `0b1ddff` published the
semantic repository index and validated public metadata cache. All 16 public
organization repositories received bounded, project-specific GitHub topics.
Pages run `30518337035` deployed the page successfully. Work stopped before
the optional live Mere graph in M5. The HTTPS redirect closure was verified
against the repository page as well as the home and community-radio routes.

M5 was implemented and accepted on 2026-07-30. Commit `80351f6` added the
optional Mere-arranged WebGPU graph while retaining the complete semantic
repository index and explicit failure paths.

M6 was implemented and accepted on 2026-07-30. Commits `4d4be38`, `1e23c17`,
`b2a52fa`, and `248aecc` added the artifact validator, scheduled/manual Pages
workflow, deployment-host reproducibility check, and conventional root
artifact. Pages run `30557137565` deployed source
`248aecc90f7c8f09212135cdaa50aef7e01c1e60`. The Cloudflare Worker custom
domain was removed, and the proxied public site matched the checked Pages
artifact at the home, repository, and community-radio routes.
