# Canvas stage unification

## Purpose

Decide whether the site's draggable canvases should share code, and record the
evidence either way. The question arose from a real defect: a phone could not
scroll past the Graphshell sandbox, and the same defect existed in every other
canvas on the site.

The scroll policy half is already landed. This plan covers the remaining
question — whether a shared JavaScript stage kit is worth building — and
recommends against it for now, with the conditions that would change that.

## What was already fixed

Each stage suppressed native touch behaviour with `touch-action: none` while
implementing no gesture of its own, so every one of them was a region a phone
could neither scroll past nor pan. Three live stages now carry one policy: the
stage yields the vertical scroll, and its nodes narrow that back to `none` so
one-finger dragging still works. The sandbox additionally gained two-finger pan
and pinch and a modifier-held wheel zoom, because it is the only surface with a
camera.

`.repository-graph-stage` was left alone: it appears in `assets/site.css` and
nowhere else, and no page emits it. It is dead style, and removing it is its own
small change rather than a silent rider on this one.

## The load-bearing finding

The canvases are not the same kind of thing. They share a substrate, not a
model, and the substrate is thinner than it looks.

Their coordinate spaces disagree. `message-path-lab` positions nodes as
percentages clamped to 7–93. `projection-proof` uses 0–1 fractions clamped to
0.08–0.92. The sandbox uses world units behind a camera with scale, zoom and
pan. The first two clamp their content inside the stage by construction, which
means **they have no off-stage content and therefore no use for a camera at
all**. For them the entire gesture question reduces to not eating the page
scroll, which is a CSS policy and now costs four lines.

Their state models disagree more. `message-path-lab` mutates the DOM directly.
`projection-proof` dispatches into a store with preview and commit semantics.
The sandbox drives a wasm physics engine and reads frames back out. Only the
sandbox renders to a real `<canvas>` with a device-pixel transform; the other
two position DOM nodes.

What is genuinely duplicated is pointer-capture node dragging and a
`ResizeObserver` that recomputes geometry — roughly thirty lines, written three
times, in files of 423, 818 and 1878 lines.

## The lifespan problem

The 2026-08-16 browser delivery plan states that every interactive surface on
the site is a JavaScript re-creation of an application's behaviour, and that
Cambium gaining a browser host replaces that division for applications that
warrant it. Its ranked application order does not name these three surfaces,
because they are site-native explanatory instruments rather than ported
applications.

But the sandbox is the exception that matters here. It re-creates Mere canvas
behaviour in JavaScript over a Rust computation core, which is precisely the
division that plan exists to end. So the surface carrying by far the most
complex stage code is also the one most likely to be superseded.

That inverts the usual case for extraction. A kit spanning all three would
inherit the sandbox's camera, physics coupling and device-pixel handling for
whichever surface has the shortest remaining life, while the two surfaces that
will certainly outlive it are the two whose stage code is already trivial.

## Recommendation

Do not build the kit now. The policy fix captured the defect that motivated the
question, and it captured it for every stage at once. What remains to share is
about thirty lines across three files whose common abstraction would be
load-bearing for the least durable of them.

The integration cost is also real and specific to this repo. There is no
bundler; scripts ship as plain content-addressed files. A shared module means a
new entry in `BASE_FILES` and the artifact digest wiring, a load-order
dependency between two classic scripts or a concatenation step, and it lands
inside the sandbox loader's size guard, which sits at 68.9 KB against 72 KiB.

## What would change this

Any one of these makes the kit worth revisiting:

- A fourth draggable stage is proposed. Three implementations is tolerable
  duplication; four is a pattern asking for a home.
- The sandbox's browser-host replacement is settled and scheduled, which
  removes the complex consumer from the kit's scope and leaves an extraction
  across simple, durable surfaces.
- A stage needs a camera for a real reason — content that legitimately exceeds
  its frame, rather than positions clamped to fit.

## Done conditions

For the landed policy half:

- Every live stage computes `touch-action: pan-y`, and every node under one
  computes `none`. Asserted per stage in the smoke suite rather than once,
  since each canvas carries its own rule.
- The sandbox's gestures hold their arithmetic: a two-finger drag translates by
  the pointer delta over the view scale, and a pinch holds its anchor.
- One finger anywhere on a stage leaves the page scroll alone. This is the
  invariant most likely to be broken by a later stage-level handler, so it is
  asserted as a negative rather than assumed.

## Open decisions

- Whether `.repository-graph-stage` and its `[data-graph-node-form]` variants
  should be deleted outright. They are unreferenced, and deleting them would
  shrink the stylesheet that the repository page's payload budget measures.
- Whether the sandbox is in scope for the Cambium browser host at all, or stays
  a JavaScript instrument because it explains the graph rather than being an
  application. The answer sets this plan's expiry.
