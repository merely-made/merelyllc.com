# Mer3ly

The public site for [Merely](https://mer3ly.net/). It is a small static Rust
build: Cambium views construct Genet `ScriptedDom` documents, which are
serialized into ordinary HTML. The repository page progressively adds a small
Mere-arranged WebGPU graph over the same validated repository and relation
authority.

The complete site remains readable without JavaScript, WebAssembly, or WebGPU.
Source CSS, graph runtime, social previews, and site identity live under
`assets/`. Local generated previews live under ignored `html/`; production
writes and validates an exact temporary artifact.

Source code is licensed under MPL-2.0. Original Mer3ly prose and site artwork
are available under CC BY 4.0; imported project screenshots retain their source
repository licenses. See [`LICENSE`](LICENSE) and
[`CONTENT_LICENSE.md`](CONTENT_LICENSE.md).

## Build

Refresh the committed baseline of reduced public GitHub metadata when
authenticated `gh` access is available:

```powershell
.\scripts\refresh-public-metadata.ps1
```

The refresh validates a complete temporary snapshot before replacing the
cache. A failed refresh leaves the last valid public snapshot in place.

When changing the repository graph client, rebuild its committed Wasm runtime:

```powershell
.\scripts\build-repo-graph.ps1
```

The script compiles the nested client crate against a pinned Mere revision,
runs `wasm-bindgen`, copies the deployable module into `assets/`, and removes
its temporary Cargo target.

Generate the static home, community-radio, device catalog, repository map, and
project profiles with:

```powershell
cargo run --locked --bin site
```

Write to a different directory with:

```powershell
cargo run --locked --bin site -- --output path/to/output
```

## Verify

```powershell
cargo test --locked
cargo test --manifest-path crates/repo-graph/Cargo.toml --locked
cargo clippy --locked --all-targets -- -D warnings
cargo run --locked --bin authority -- validate
cargo run --locked --bin authority -- validate-metadata
cargo run --manifest-path crates/repo-graph/Cargo.toml --locked --bin projection-receipt -- html/projection-scene.json
npm ci --ignore-scripts
npx playwright install chromium
npm run smoke
```

The browser smoke serves generated `html/` locally and checks discovery files,
project social metadata and structured data, the home, community-radio, and
device pages, and desktop, mobile, reduced-motion, and WebGPU-fallback paths. Set
`MER3LY_SITE_DIR` to check another generated site directory.

`authority validate-artifact` accepts a Pages artifact root after the
repository root. It enforces the exact public file set, public authority and
graph counts, canonical sitemap coverage, project and device structured
metadata, device purchase state, favicon identity, displayed metadata timestamp,
Wasm header, reduced GitHub links, and the absence of secrets, personal data
patterns, local paths, and private network addresses. The artifact is a
conventional, self-contained site root: `index.html`, `repos/index.html`,
`projects/<id>/index.html`, `devices/index.html`,
`devices/<id>/index.html`, `radio.html`, `sitemap.xml`, `robots.txt`, their
approved styles, showcase images and runtime assets, the serialized
`projection-scene.json` Scenograph score/snapshot/trace, and `CNAME` all live
directly beneath the supplied directory.
The exact public contact
`markik@mer3ly.net` is allowed; other contact addresses are rejected. The
command emits a JSON receipt with SHA-256 hashes:

```powershell
cargo run --locked --bin authority -- validate-artifact . .tmp/pages-artifact
```

## Deployment

[`pages.yml`](.github/workflows/pages.yml) refreshes the reduced public
metadata cache, rebuilds and validates the exact static artifact, runs a
headed Chromium smoke under a virtual display, and deploys that artifact to
GitHub Pages. It runs after relevant changes reach `main`, on manual dispatch,
and on a daily schedule. The committed metadata file is a reviewable baseline;
each deployment refreshes a complete temporary snapshot before rendering. The
build has read-only repository permission; only the separate deployment job
receives the Pages and identity grants. The graph runtime is built twice and
must have identical hashes on the deployment host before either output can be
published. GitHub Pages is the sole deployment origin. Cloudflare may proxy the
custom domain for DNS, TLS, and HSTS, but does not serve a second site bundle.

## Plans

- [Live repository graph and Merely organization migration](docs/2026-07-29_live_repos_graph_and_org_migration_plan.md)
- [Open radio device catalog](docs/2026-08-06_device_catalog_plan.md)
- [Merely project showcase](docs/2026-07-30_project_showcase_plan.md)
- [Discovery and sharing](docs/2026-07-30_discovery_and_sharing_plan.md)
- [Authority reconciliation](docs/2026-07-31_authority_reconciliation_plan.md)
- [Site acceptance receipts](docs/receipts/site/README.md)
