# Browser delivery

## Purpose

Ship the Merely applications as real applications in a browser, not as
reimplementations of their views. Cross-platform including the web is a product
thesis, not a demo: the browser is the cheapest way to distribute the apps and
the fastest loop for refining them.

The site already embeds Rust. It embeds it as computation only: `crates/repo-graph`
is 936 KB of arrangements, cartography, sceno, scenomise, scenotime and seiche
with no renderer in it, and the drawing happens in `assets/graph-sandbox.js`.
Every interactive surface on the site today is a JavaScript re-creation of an
application's behaviour. This plan replaces that division for applications that
warrant it.

## The load-bearing finding

The work belongs in Cambium, not in each application.

Six of seven candidate applications already consume Cambium: turnstone,
isometry, hocket, cleromancy, mesocosm, woodshed. Only paredros does not. An
application that gains a browser target on its own gains it once; Cambium
gaining a browser host gives it to all six.

The precedent is Woodshed's. It donated `cambium-genet-winit-host` on
2026-08-09 and became its first consumer; its binary fell from 1728 to 211
lines of host code. The same move applies here with a different donor.

## Architecture

An earlier draft of this plan named `graphshell-web` as the donor. Reading the
code falsified that. Its chrome is stateless: `build_chrome_scene` constructs a
fresh `ScriptedDom` and a fresh `GenetAppRunner` on every call, lays out, emits
a paint list and returns a Scene. Its event wiring routes into the content
layer, Mere's `Canvas`, and the chrome receives no pointer or keyboard input at
all. So it holds no retained layout, no hit testing, no input routing into
Cambium and no accessibility. What it can donate is `web_gpu.rs`, 108 lines that
already delegate to `genet-render-host`, plus a `requestAnimationFrame` loop.

The donor is `cambium-genet-winit-host` itself. It carried 3544 lines and only
64 winit references, so the machinery was already close to windowing-neutral;
it had simply never been asked to prove it.

Target shape, siblings under `genet/components/cambium/`:

| Crate | Role |
|---|---|
| `cambium-genet-host` | the neutral machinery: input vocabulary, routing, layout, paint, accessibility |
| `cambium-genet-winit-host` | winit event source over it |
| `cambium-genet-web-host` | DOM event source over it |

Cambium already survives the browser. It is in `graphshell-web`'s
`wasm32-unknown-unknown` graph today, with `meristem`, `sprigging`,
`genet-layout` and `genet-render-host` beside it.

### Three seams, not one

| Seam | Winit side | Browser side |
|---|---|---|
| Input | `winit::keyboard` | DOM events |
| Surface | `genet_winit_host::SurfaceHost` | `genet_render_host::{RenderCore, WindowSurface}` |
| Accessibility | `cambium-winit-a11y`, an AccessKit tree attached to a window handle | the rendered DOM itself |

The third is not a port. AccessKit builds a parallel tree the platform queries;
a browser's accessibility is the document already on screen. The host's use of
it is small, `A11yHost::new`, one `sync` returning requests, and a drain, and
the crux is `sync`'s `window` parameter: a browser has no handle to attach to.
The neutral signature drops it and each event source holds its own.

### Landed so far

`cambium-genet-host` exists and carries the input vocabulary, key lowering
including the assistive-input path, caret movement, and spatial-navigation
scoring. `cambium-genet-winit-host` converts winit input at one boundary and
routes neutral presses; `HostState` holds neutral modifiers. Woodshed's
application code no longer imports `winit::keyboard`. 52 tests green across the
two crates, with woodshed building and passing against the changed API.

Open: the accessibility request vocabulary, then `Host` itself, then the DOM
event source.

## Payload, and the lever that moves it

Measured on `graphshell-web`, `wasm32-unknown-unknown`:

| Build | Raw | Gzipped |
|---|---|---|
| Debug artifact committed 2026-08-06 | 104 MB | |
| Release, default profile | 21.4 MB | |
| Release, size profile (`opt-level="s"`, thin LTO, `strip="symbols"`, `codegen-units=1`) | 12.7 MB | 3.8 MB |
| `repo-graph`, for comparison | 936 KB | 333 KB |

The size profile costs 19m 29s to build. It is a local cost: the site commits
artifacts rather than building them in the Pages workflow.

The graph holds 399 distinct crates, fourteen of which are Servo or Stylo:
`genet-stylo`, `genet-stylo-atoms`, `genet-stylo-dom`, `genet-stylo-static-prefs`,
`genet-stylo-traits`, `servo-base`, `servo-config`, `servo-config-macro`,
`servo-malloc-size-of`, `servo-pixels`, `servo-url`, `stylo_derive`,
`stylo_malloc_size_of`, `stylo_taffy`. A full CSS engine is riding along.

An application that is not the whole web does not need it. Livery is the exit
and it is further along than the browser question has noticed: an 87-property
Cambium lane catalog generating a typed `ComputedValues`, a resolver covering
declaration and shorthand parsing, selector matching, cascade ordering,
inheritance and media evaluation, with the Cambium integration in `genet-livery`
carrying a `LayoutDom` adapter, a concrete style plane and a standalone Taffy
box path with neutral paint emission. Its own manifest names no Stylo.

Livery states the split this plan needs: the first lane is Cambium structural
UI, and fullweb documents continue on Genet Stylo. Applications sit on the
first lane. Livery is therefore the payload lever, and the browser is the
strongest argument yet for finishing it.

Until Livery drives Cambium's views, every browser application carries the
Servo CSS engine. That is the difference between a 3.8 MB download and
something plausibly under a megabyte.

## Collaboration

Joining someone's document must be possible.

iroh compiles to `wasm32-unknown-unknown` with wasm-bindgen. Browsers cannot
send UDP from inside the sandbox, so every browser connection is relayed. The
relay cannot decrypt what it carries; connections stay end to end encrypted.

Two facts about the present state:

- `graphshell-web`'s wasm graph contains no networking at all. Not iroh, not
  p2panda, not quinn. Collaboration is additive work, not a port.
- `murm/transport` describes itself as the transport "for the Mere browser" and
  is iroh over QUIC. It is native in practice.

GitHub Pages is static, so the relay lives somewhere else. `iroh-relay` is a
single binary wanting a public IP, a DNS name and TCP 80/443, with ACME TLS
built in. One relay carries up to 60,000 concurrent connections; two in
separate regions is the production shape, and clients fall back between them
automatically.

### People as their own relay

The relay being a plain binary makes owner-run relays a first-class option
rather than a workaround. A person who wants their collaboration to touch
nothing of ours points their peers at their own relay. This matches the
sovereignty posture the rest of the stack already takes, and it moves the
infrastructure cost off Merely for the people who care most about it.

Merely still runs relays, because a default that requires standing up a server
is not a default.

## Memory64 posture

Deferred, and not for toolchain reasons.

| | |
|---|---|
| `wasm64-unknown-unknown` | Tier 3; no prebuilt std, needs `build-std` on nightly |
| `panic = "unwind"` | unsupported on the target |
| wasm-bindgen | supports it; `usize` and pointers lowered through an f64 JS ABI |
| Chrome / Firefox | 133+ / 134+ |
| Safari / WebKit | unsupported, every version through 26.5-TP |
| Global reach | 72.08% |

WebKit has shipped nothing in roughly eighteen months since Chrome. So mem32 is
not a fallback that retires when Safari catches up; it is a co-primary, and it
is the one covering every iPhone. This is the framing the Vano work already
uses: Boa is the durable primary for WebKit and iOS, not a fallback, because
Nova's `Value` is word-size asserted.

The concrete thing mem64 buys in this stack is Nova instead of Boa. That driver
is not live here: `graphshell-web`'s wasm graph contains no JavaScript engine at
all. Whether wgpu, genet, netrender and Stylo build for wasm64 is unmeasured,
and measuring it needs nightly and `build-std`.

Revisit when a JavaScript engine lands in a browser application, or when an
application demonstrably wants more than 4 GB.

## Application order

Ranked by what each one teaches, with the owner's assessment recorded:

1. **woodshed**, the obvious second. Most developed, and the donor of the
   desktop host it would now follow into the browser. Its audio is `cpal`, so
   it carries the WebAudio problem.
2. **hocket**, same audio problem, same value in solving it. Woodshed and
   hocket share `audio-primitives`, so the backend seam is solved once.
3. **cleromancy**, admissible once its design is settled.
4. **isometry**, conditional on whether people can join instances. That is the
   same relay question as collaboration, so it follows that work rather than
   leading it.
5. **turnstone**. Not the hardest once it is not carrying the whole web; like
   the rest it is GUI work and refinement.
6. **mesocosm** and **paredros**: not settled, not beyond prototype. Out of
   scope until they are.

Audio is the first real challenge and it lands immediately. It is a backend
swap from `cpal` to WebAudio behind the existing `woodshed-audio` seam, not a
recompile.

## Costs

### Engineering

- Extract the Cambium web host from `graphshell-web`'s GPU and event halves.
  Paid once; six applications collect it.
- A `web` feature cone and a presenter crate per application. Small per
  application, and the shape is already proven twice.
- WebAudio backend behind the `woodshed-audio` seam. Shared by woodshed and
  hocket through `audio-primitives`.
- Browser transport: iroh over wasm, plus whatever session and presence model
  joining implies. The largest single item, and the one with no existing proof
  in this stack.
- Livery far enough to drive Cambium views without Stylo. Already in flight and
  independently motivated; the browser makes it urgent rather than merely
  desirable.

### Payload

3.8 MB gzipped per application with Stylo, on the measured Graphshell figure.
Plausibly under a megabyte once Livery displaces it, unmeasured until it does.

### Infrastructure

- Two `iroh-relay` instances in separate regions is the production shape. Each
  is a small public Linux host with ports 80 and 443 open. Commodity pricing;
  the recurring cost is per month and per region, not per user, until 60,000
  concurrent connections per relay is in sight.
- Owner-run relays cost Merely nothing and cost the owner a host.
- Cloudflare is usable, but not for `iroh-relay` as shipped: Workers and
  Durable Objects would mean writing a relay rather than running the one that
  exists. Worth pricing only if the commodity host proves wrong.

### Operating

- 19m 29s per size-profiled application build, locally.
- Artifacts ship as release assets rather than committed files. Pushing the
  init and then releasing keeps the site repository from absorbing 12.7 MB per
  rebuild, which at the current cadence would pass 1 GB inside a hundred
  rebuilds.
- Two artifacts per application if mem64 is ever taken, which is one of several
  reasons it is deferred.

## Done conditions

- A named Cambium web host exists as a sibling to `cambium-genet-winit-host`,
  and `graphshell-web` consumes it rather than hand-rolling its own.
- A second application runs in a browser through that host, with its native
  build unchanged.
- The browser build of at least one application resolves and lays out without
  Stylo in its dependency graph.
- Two people, in two browsers, on two machines, edit one thing and both see it.
- A person can point that collaboration at a relay they run.
- Each shipped application has a static readable form that needs no JavaScript,
  WebAssembly or WebGPU, per the site's standing constraint.

## Open decisions

- Which repository owns the Cambium web host: genet beside the winit host is
  the obvious answer, and this plan assumes it.
- Whether Merely's default relays are Merely-run or a named third party.
- Whether isometry's joining model is peer-to-peer through the relay or an
  instance a host publishes.
