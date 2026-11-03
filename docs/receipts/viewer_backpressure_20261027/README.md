# Viewer readback backpressure repair

The retained Linux fixture exposed a failure in the published viewer: cold GPU
work delayed the first quality-counter result by about 14 seconds while the
browser host submitted 256 downloads within about five seconds. Vello correctly
refused the exhausted queue, and an unchecked producer path panicked before the
GPU caught up. The original failure is retained in
`../reconciliation-20261027/linux-viewer.json`; the extended timing probe is
[run 40622039994](https://github.com/merely-made/merelyllc.com/actions/runs/40622039994).

The repaired browser host polls completed curve-quality results before admitting
another frame. Pending redraws survive the wait, and the animation loop also
drains counters for a final static frame. Provider failures keep their typed
refusal path. Vello's quality limits and original dependency pins are retained.
The site mount also tolerates a retained page without a static-reading disclosure.

The package uses Mere `14b91e3df2e239924bdb3d7b88a58d00f40871ae`, whose baseline
`0e360eacca7d71541a50f9f0f3a4f043ae39e3b6` is tree-identical to the former
`56859e30292405e682f28f3841b76e4f141acf73`. The runtime delta is confined to the
browser host's mount module. The original branch and failed package remain in
recovery storage. Current Mere main also receives the guard on its retained
legacy surface path; current presentation drivers keep their existing owner of
frame acceptance.

The browser regression delays quality-map delivery by five seconds, requires
bounded pending downloads and resumed progress, and observes Rust unmapping the
completed counters. The full viewer suite runs separately without injected delay on the generated
site pages in `tests/fixtures/viewer-site-20261027`; all thirteen cases pass.
The historical fixture remains unchanged for the dedicated regression.
The source-bound qualification record names the exact package hashes, browser,
run and results. Linux Chromium with SwiftShader is software-GPU browser evidence;
it does not establish physical GPU or Windows acceptance.

The site builder's stale Mere revision was separately replaced with its exact
history-rewrite twin (`d20eb555` to `9bba595c`); their source trees are identical.
No dependency package versions were changed by that pin repair.

Accepted run: [40625364082](https://github.com/merely-made/merelyllc.com/actions/runs/40625364082).
The native site suite passes 110 tests. The Wasm gzip6 size is 3,280,694 bytes
against the unchanged 3,407,872-byte limit.
