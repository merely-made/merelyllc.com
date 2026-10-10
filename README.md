# Mer3ly

The source for the Merely public site at [merelyllc.com](https://merelyllc.com/): a
small static Rust build (Cambium views serialized to HTML) plus a WebGPU
repository-graph client, deployed to GitHub Pages. The complete site remains
readable without JavaScript, WebAssembly, or WebGPU.

## Status (2026-10-09)

Live and deployed. The pages.yml workflow rebuilds, validates, browser-smokes,
and deploys on main pushes, manual dispatch, and a daily schedule.

- Ships home, the repository map with a semantic index and an optional
  Mere-arranged WebGPU graph, project showcase profiles, the community-radio
  page, and the open radio device catalog (added 2026-08-06).
- The repository graph is driven from live GitHub org data with normalized
  topic metadata (2026-08-11) and embeds a Graphshell scene sandbox with
  portable, history-aware scenes (2026-08-12).
- An authority binary validates content manifests, the metadata cache, and
  the exact Pages artifact (file set, structured data, absence of secrets
  and personal data), emitting SHA-256 JSON receipts.
- Eight dated plans live in `docs/`. The first five cover the shipped site;
  browser delivery, canvas stage unification, and
  [Graphshell as the site canvas](docs/2026-09-30_graphshell_site_canvas_plan.md)
  cover the replacement of the site-owned interactive surfaces.
- P0 viewer review and provider integration are complete: the viewer cone and speed estimator are
  merged into Mere main at `ea9e6da7`, with fresh browser, accessibility and
  frame receipts. The six-asset plain viewer payload is 2,147,689 B brotli;
  Ruling 157 now sets the delivery caps (64 KiB gzip first load; on interaction,
  viewer Wasm 3.25 MiB gzip, glue 20 KiB, mount 8 KiB, history 128 KiB). The
  measurement above is the historical P0 baseline. The site still ships its own sandbox;
  the remaining sandbox exports and Graphshell viewer cutover stay open. See the
  [integration receipt](docs/handoffs/2026-10-07_viewer_cone/RESUME.md#authorized-integration).
- The build now emits `repository-host-dataset.json` from the reconciled public
  authority: 21 projects and 30 explained relationships in the versioned S1
  envelope. It records scope and a content-derived revision; this is published
  repository-family context, not a resolved Cargo dependency closure. Native
  parser/compiler acceptance is [recorded](docs/receipts/site/2026-10-07_host_dataset/receipt.json).
  The site's Graphshell viewer mount remains ahead.
- /repos/ opens on a first view frozen at build time (Ruling 157, 2026-10-09):
  the default reading through Graphshell's frozen reader, the default matrix
  through the shared two-reading-matrix adapter as a `FrozenGrid` (S2), and the
  source history as text through scenomise's evaluator (S3). The history comes
  from `repository-host-history.json`, a `scenomise.host-dataset/v2` envelope
  of the merged authority checkpoints. Only a small mount script loads up
  front; the site's own sandbox, its glue and the graph Wasm load on the first
  interaction with the frozen view, or at once for a share link.
- P2 has replaced three surfaces. The V4 device bench runs radio-mirror
  (2026-10-07). The community-radio page's message path lab (2026-10-08) reads
  retinue-sim's generated cold and warm route traces, committed under
  `content/retinue-traces/` with their provenance, and draws the selected
  radio's TRAFFIC page through radio-mirror. The site links no
  Reticulum-licensed crate. The Mere profile's projection proof (2026-10-08)
  reads Mere's shared artifacts instead of a site schema: a chirograph V2
  capture, a scenotime scene trace and an incipit shelfmark citing the
  capture, labelled from the host dataset. Its renderer is still the site's
  own lightweight script; the Graphshell viewer is not mounted there.

`content/*.toml` is the editorial authority for repository roles, summaries,
relations, showcases, and the device catalog; the GitHub listing is the live
membership authority. `content/retinue-traces/` holds generated files, not
editorial ones: regenerate them in a retinue checkout with the commands in its
`provenance.toml`, then update the revision and hashes there (`authority
validate` checks them).

## Use

```powershell
cargo run --locked --bin site                       # generate the static site
cargo run --locked --bin authority -- validate      # validate content
.\scripts\refresh-public-metadata.ps1               # refresh the GitHub metadata cache
.\scripts\build-repo-graph.ps1                      # rebuild the committed Wasm graph runtime
.\scripts\build-radio-mirror.ps1                    # rebuild the committed radio-mirror runtime
```

Verify with `cargo test --locked` and the browser smoke (`npm ci
--ignore-scripts`, `npx playwright install chromium`, `npm run smoke`).
For installed Microsoft Edge on Windows, set `$env:MER3LY_BROWSER_CHANNEL =
"msedge"` before `npm run smoke`; the receipt records the selected channel.
The default and Pages workflow use bundled Chromium. The
[workspace upstream refresh](docs/2026-10-08_workspace_upstream_refresh.md)
records dependency qualification and the separate Vello adoption lane.

`authority -- validate-artifact . <artifact-root>` checks an exact Pages
artifact and emits a hashed receipt.

## License

Source MPL-2.0. Original prose and site artwork CC BY 4.0; imported project
screenshots retain their source licenses. The V4 radio bench and the message
path lab run retinue's MPL-2.0 `radio-mirror` and `radio-face` at a pinned
revision, and the lab reads traces generated by retinue's MPL-2.0
`retinue-sim`; the source pointers are in `CONTENT_LICENSE.md`. See [`LICENSE`](LICENSE) and
[`CONTENT_LICENSE.md`](CONTENT_LICENSE.md).

---

*This README was generated by AI and will be edited by the author upon
release.*
