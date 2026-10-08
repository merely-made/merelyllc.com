# Mer3ly content license

Except where a source or attribution says otherwise, original prose and
original site artwork in this repository are licensed by Merely LLC under the
[Creative Commons Attribution 4.0 International
license](https://creativecommons.org/licenses/by/4.0/).

When reusing that material, attribute it to **Merely LLC**, link to
`https://merelyllc.com/`, link to the CC BY 4.0 license, and indicate whether you
made changes.

Project screenshots under `assets/showcase/` retain the license declared by
their source repositories. Their source URLs, descriptions, and licenses are
recorded in `content/showcases.toml`. This repository does not grant rights in
third-party material that may appear inside a screenshot.

The V4 radio bench is built from retinue's `radio-mirror` and `radio-face`
crates, which are MPL-2.0, at retinue revision
`6aa78fc0d0ddd30e94db59470f30f47e669321dc`. That covers the committed runtime
`assets/radio_mirror.js` and `assets/radio_mirror_bg.wasm`, and the screen
images the site build renders into `radio-mirror/`. Their source is
<https://github.com/merely-made/retinue/tree/6aa78fc0d0ddd30e94db59470f30f47e669321dc/crates/radio-mirror>
and
<https://github.com/merely-made/retinue/tree/6aa78fc0d0ddd30e94db59470f30f47e669321dc/crates/radio-face>.
The vendored `embedded-graphics` they link is MIT OR Apache-2.0. The status
documents in `crates/radio-mirror/fixtures/` are copied unchanged from the
same revision. These screens are firmware output, not original site artwork,
and CC BY 4.0 does not apply to them. The build links no Reticulum-licensed
retinue crate.

The message path lab on the community-radio page reads route traces and face
tracks under `content/retinue-traces/` (published as `retinue-traces/`). They
are the output of retinue's `retinue-sim` lab example, which is MPL-2.0, run
outside this site at retinue revision
`6aa78fc0d0ddd30e94db59470f30f47e669321dc`; `provenance.toml` beside them
records the commands and each file's SHA-256, and the source is
<https://github.com/merely-made/retinue/tree/6aa78fc0d0ddd30e94db59470f30f47e669321dc/crates/retinue-sim>.
The files are generated data, not original site prose, and CC BY 4.0 does not
apply to them. `retinue-sim` links the `retinue` crate, which is under the
Reticulum License, but the traces contain no part of that crate's source: they
hold node names, simulated times, event kinds, frame header facts, hashes the
run derived, and status counters. The Reticulum License's conditions attach to
copies or substantial portions of its software and to uses of that software,
so they apply to whoever runs the generator, not to these committed outputs;
the site itself builds and runs no Reticulum-licensed code. The lab's screen
images, rendered into `radio-mirror/` from these documents, are firmware output
as above.

The Merely name, company mark, project names, and other source-identifying
marks are not licensed as trademarks by CC BY 4.0 or MPL-2.0.

Source code is separately licensed under MPL-2.0; see `LICENSE`.
