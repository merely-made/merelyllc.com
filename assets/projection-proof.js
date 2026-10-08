// The Mere profile's projection proof. It reads four sibling artifacts
// (site canvas plan, Rulings 130-136): chirograph's V2 capture, scenotime's
// SceneTrace, the shelfmark that cites the capture, and the S1 host dataset
// that names everything.
//
// Until the first interaction it draws the captured scene, which needs no
// replay, beside the build-time reading. The first interaction, or a share
// link present at load, imports the site's graph Wasm (Rulings 138 and 140).
// Its ProjectionSession does the stack's work: chirograph's decoding and
// content address, the shelfmark check, scenotime's chained replay, and
// edit_history::History<SceneTrace> for position, undo, redo and
// truncate-on-commit. This script renders, handles controls and words links.

const SHARE_VERSION = "v3";
// The most steps this page keeps. Host policy (Ruling 15), not trace data;
// the session refuses the same bound.
const STEP_BOUND = 16;
const DATASET_SCHEMA = "scenomise.host-dataset/v1";
const DEFAULT_SELECTION = { kind: "node", id: "mere" };

const root = document.querySelector("[data-projection-proof]");

if (root) {
  queueMicrotask(() => {
    startProjectionProof(root).catch((error) => {
      const forcedFallback =
        new URLSearchParams(window.location.search).get("projection") === "no-scene";
      if (!forcedFallback) {
        console.error("Portable projection failed to initialize.", error);
      }
      root.dataset.state = "unavailable";
      root.dataset.ready = "false";
      root.querySelector("[data-projection-interface]").hidden = true;
      root.querySelector("[data-projection-fallback]").hidden = false;
      announce(
        root,
        "The portable scene could not initialize. The trace reading and the relationship lists remain available.",
      );
    });
  });
}

async function startProjectionProof(proofRoot) {
  const forcedMode = new URLSearchParams(window.location.search).get("projection");
  if (forcedMode === "no-scene") throw new Error("forced scene fallback");

  const fetchBytes = async (attribute) => {
    const response = await fetch(proofRoot.dataset[attribute]);
    if (!response.ok) throw new Error(`could not load ${proofRoot.dataset[attribute]}`);
    return new Uint8Array(await response.arrayBuffer());
  };
  const [capture, trace, shelfmark, dataset] = await Promise.all([
    fetchBytes("captureSrc"),
    fetchBytes("traceSrc"),
    fetchBytes("shelfmarkSrc"),
    fetchBytes("datasetSrc"),
  ]);
  const text = (bytes) => JSON.parse(new TextDecoder().decode(bytes));
  const proof = readProof(text(capture), text(trace), text(dataset));

  const replay = new Replay(proofRoot, { capture, trace, shelfmark }, forcedMode === "no-replay");
  const store = new ProofStore(proof, replay);
  const views = [...proofRoot.querySelectorAll("[data-projection-view]")].map(
    (element) => new ProjectionView(element, proof, store),
  );
  const controls = new ProjectionControls(proofRoot, proof, store);
  store.subscribe((state) => {
    views.forEach((view) => view.render(state));
    controls.render(state);
  });
  replay.onReady = () => store.loaded();

  // A share link counts as an interaction: it needs the trace replayed.
  let link = { state: "none", notice: null };
  if (new URLSearchParams(window.location.hash.slice(1)).has("projection-scene")) {
    try {
      await replay.ensure();
    } catch {
      return;
    }
    link = readSharedLink(store);
  }

  proofRoot.dataset.linkState = link.state;
  if (link.notice) controls.notify(link.notice);
  // The reading's no-script opening says the scene needs JavaScript; with the
  // script running, replay is a first interaction away (Ruling 140).
  const lead = proofRoot.querySelector("[data-projection-reading-lead]");
  if (!replay.session && lead) lead.textContent = lead.dataset.scriptedLead;
  proofRoot.querySelector("[data-projection-interface]").hidden = false;
  proofRoot.dataset.ready = "true";
  proofRoot.dataset.state = "ready";
  store.notify();
  announce(
    proofRoot,
    link.state === "restored"
      ? "Shared scene trace restored. Both projections replay the same capture and trace."
      : (link.notice ??
          `${proof.nodes.length} projects and ${proof.relations.length} relationships loaded from one Scenograph capture. Interact with the scene to load live replay.`),
  );
}

// ---------------------------------------------------------------------------
// Reading the artifacts for drawing. Checking them is the session's work.

function readProof(capture, trace, dataset) {
  const scene = capture?.scene;
  if (!Array.isArray(scene?.tables?.items) || !Array.isArray(trace?.steps)) {
    throw new Error("the capture or trace is not readable");
  }
  if (dataset?.schema !== DATASET_SCHEMA) throw new Error("unknown host dataset");
  const occurrences = new Map(
    dataset.dataset.occurrences.map((occurrence) => [occurrence.occurrence_id, occurrence]),
  );
  const field = (occurrence, name) => {
    const value = occurrence?.values?.[name];
    if (value?.kind !== "text" || !value.value) throw new Error(`a project has no ${name}`);
    return value.value;
  };
  const nodes = [];
  scene.tables.items.forEach((item) => {
    if (!item) return;
    const id = scene.tables.sources[item.source].id;
    const occurrence = occurrences.get(id);
    nodes.push({
      id,
      name: field(occurrence, "label"),
      class: field(occurrence, "class"),
      status: field(occurrence, "status"),
    });
  });
  const relations = scene.tables.relations.map((relation, index) => {
    const source = scene.tables.sources[scene.tables.items[relation.from].source].id;
    const target = scene.tables.sources[scene.tables.items[relation.to].source].id;
    const matches = dataset.relationships.filter(
      (disclosed) =>
        disclosed.from_occurrence === source &&
        disclosed.to_occurrence === target &&
        disclosed.kind === relation.kind,
    );
    if (matches.length !== 1) throw new Error(`relation ${index} is not disclosed exactly once`);
    return { index, id: matches[0].id, label: matches[0].label, source, target };
  });
  // The epoch in `scene` is rounded by JSON.parse. Nothing here reads it: the
  // session supplies every diff's epoch, and the page shows the session's.
  return { scene, stepCount: trace.steps.length, nodes, relations };
}

// Restores a v3 link, and explains a retired, stale or broken one politely.
function readSharedLink(store) {
  const session = store.session;
  const params = new URLSearchParams(window.location.hash.slice(1));
  const version = params.get("projection-scene");
  const fallback = (state, notice) => ({ state, notice });
  const broken = (why) =>
    fallback("broken", `This link's trace ${why}, so it cannot be restored. Showing the default trace.`);
  if (version === null) return fallback("none", null);
  if (version !== SHARE_VERSION) {
    return fallback(
      "retired",
      "This link uses a retired scene format, so it cannot be restored. Showing the default trace.",
    );
  }
  if (
    params.get("projection") !== session.captureAddress() ||
    params.get("expects-generation") !== session.generation()
  ) {
    return fallback(
      "stale",
      "This link cites an earlier capture of the scene, which this page no longer carries, so it cannot be restored. Showing the default trace.",
    );
  }
  let steps;
  if (params.has("trace")) {
    try {
      steps = decodeSteps(params.get("trace"));
    } catch {
      return broken("cannot be read");
    }
  }
  try {
    session.restore(steps, Number(params.get("position")));
  } catch (error) {
    let refusal = {};
    try {
      refusal = JSON.parse(error);
    } catch {
      throw error;
    }
    if (refusal.refusal === "unreadable") return broken("cannot be read");
    if (refusal.refusal === "too-long") {
      return fallback(
        "refused",
        `This link's trace is longer than the ${STEP_BOUND} steps this page keeps, so it cannot be restored. Showing the default trace.`,
      );
    }
    if (refusal.refusal === "broken") return broken(`breaks its chain at step ${refusal.step}`);
    return fallback(
      "broken",
      "This link names a step its trace does not have, so it cannot be restored. Showing the default trace.",
    );
  }
  store.refresh();
  try {
    store.annotations.forEach((annotation, index) => {
      if (annotation !== null) readSelection(store.proof, annotation, index);
    });
  } catch (error) {
    session.reset();
    store.refresh();
    return broken(`breaks its chain at step ${error.index + 1}`);
  }
  return { state: "restored", notice: null };
}

class SelectionError extends Error {
  constructor(index) {
    super(`step ${index}: the step selects nothing this scene carries`);
    this.index = index;
  }
}

function readSelection(proof, annotation, index) {
  const keys = annotation && typeof annotation === "object" ? Object.keys(annotation) : [];
  const known =
    keys.length === 2 &&
    ((annotation.kind === "node" && proof.nodes.some(({ id }) => id === annotation.id)) ||
      (annotation.kind === "edge" && proof.relations.some(({ id }) => id === annotation.id)));
  if (!known) throw new SelectionError(index);
  return { kind: annotation.kind, id: annotation.id };
}

// ---------------------------------------------------------------------------
// The graph Wasm, loaded once, on first interaction (Ruling 140).

class Replay {
  constructor(proofRoot, artifacts, forceFailure) {
    this.root = proofRoot;
    this.artifacts = artifacts;
    this.forceFailure = forceFailure;
    this.session = null;
    this.loading = null;
    this.onReady = null;
  }

  ensure() {
    this.loading ??= this.load();
    return this.loading;
  }

  async load() {
    const proofRoot = this.root;
    proofRoot.dataset.replay = "loading";
    proofRoot.setAttribute("aria-busy", "true");
    announce(proofRoot, "Loading the scene replay.");
    try {
      if (this.forceFailure) throw new Error("forced replay failure");
      const runtime = await import(proofRoot.dataset.graphRuntime);
      await runtime.default({
        module_or_path: new URL(proofRoot.dataset.graphWasm, window.location.href),
      });
      const { capture, trace, shelfmark } = this.artifacts;
      const session = new runtime.ProjectionSession(capture, trace, shelfmark);
      if (session.stepBound() !== STEP_BOUND) throw new Error("the step bounds disagree");
      this.session = session;
      this.onReady?.();
    } catch (error) {
      this.session = null;
      proofRoot.dataset.replay = "failed";
      proofRoot.dataset.state = "unavailable";
      proofRoot.dataset.ready = "false";
      proofRoot.removeAttribute("aria-busy");
      proofRoot.querySelector("[data-projection-interface]").hidden = true;
      proofRoot.querySelector("[data-projection-fallback]").hidden = false;
      const lead = proofRoot.querySelector("[data-projection-reading-lead]");
      if (lead) lead.textContent = "Live replay could not load, so this";
      const notice = proofRoot.querySelector("[data-projection-notice]");
      notice.textContent =
        "The scene replay could not load, so the scene cannot change here. The trace reading remains available.";
      notice.hidden = false;
      announce(proofRoot, notice.textContent);
      if (!this.forceFailure) console.warn("Projection replay unavailable:", error);
      throw error;
    }
    proofRoot.dataset.captureAddress = this.session.captureAddress();
    proofRoot.dataset.sceneEpoch = this.session.epoch();
    proofRoot.dataset.replay = "ready";
    proofRoot.removeAttribute("aria-busy");
    proofRoot.querySelector("[data-projection-fallback]").hidden = true;
    announce(proofRoot, "Scene replay ready.");
  }
}

// What both views draw: the captured scene before replay loads, then the
// session's scene where the page stands, plus a drag's preview.
class ProofStore {
  constructor(proof, replay) {
    this.proof = proof;
    this.replay = replay;
    this.annotations = [];
    this.transient = null;
    this.listeners = new Set();
    this.beforeDispatch = null;
    this.onRefused = null;
  }

  get session() {
    return this.replay.session;
  }

  get cursor() {
    return this.session ? this.session.position() : 0;
  }

  get length() {
    return this.session ? this.session.length() : this.proof.stepCount;
  }

  // Run an interaction now, or once the replay has loaded.
  act(action) {
    if (this.session) {
      action();
      return;
    }
    this.replay.ensure().then(action, () => {});
  }

  loaded() {
    this.refresh();
    this.annotations.forEach((annotation, index) => {
      if (annotation !== null) readSelection(this.proof, annotation, index);
    });
  }

  refresh() {
    this.annotations = JSON.parse(this.session.annotations());
  }

  subscribe(listener) {
    this.listeners.add(listener);
  }

  committedSnapshot() {
    if (!this.session) return { snapshot: this.proof.scene, selection: DEFAULT_SELECTION };
    const snapshot = JSON.parse(this.session.snapshot());
    let selection = DEFAULT_SELECTION;
    this.annotations.slice(0, this.cursor).forEach((annotation, index) => {
      if (annotation !== null) selection = readSelection(this.proof, annotation, index);
    });
    return { snapshot, selection };
  }

  snapshot() {
    const state = this.committedSnapshot();
    if (this.transient) {
      state.snapshot = this.transient.snapshot;
      if (this.transient.step.annotation !== undefined) {
        state.selection = readSelection(this.proof, this.transient.step.annotation, this.cursor);
      }
    }
    return state;
  }

  notify() {
    const state = this.snapshot();
    for (const listener of this.listeners) listener(state);
  }

  dispatch(step) {
    this.beforeDispatch?.();
    this.transient = null;
    if (!this.session.record(JSON.stringify(step))) {
      this.onRefused?.();
      this.notify();
      return false;
    }
    this.refresh();
    this.notify();
    return true;
  }

  setCursor(cursor) {
    this.transient = null;
    this.session.moveTo(clamp(Math.round(cursor), 0, this.length));
    this.notify();
  }

  preview(step) {
    this.transient = { step, snapshot: JSON.parse(this.session.preview(JSON.stringify(step))) };
    this.notify();
  }

  commitPreview() {
    if (!this.transient) return;
    const { step } = this.transient;
    this.transient = null;
    this.dispatch(step);
  }

  clearPreview() {
    this.transient = null;
    this.notify();
  }

  reset() {
    this.beforeDispatch?.();
    this.session.reset();
    this.refresh();
    this.transient = null;
    this.notify();
  }
}

// ---------------------------------------------------------------------------
// The two views and the controls.

class ProjectionView {
  constructor(element, proof, store) {
    this.element = element;
    this.kind = element.dataset.projectionView;
    this.stage = element.querySelector("[data-projection-stage]");
    this.edgeSvg = element.querySelector("[data-projection-edges]");
    this.edgeControls = element.querySelector("[data-projection-edge-controls]");
    this.nodeLayer = element.querySelector("[data-projection-nodes]");
    this.selection = element.querySelector("[data-projection-selection]");
    this.proof = proof;
    this.store = store;
    this.nodeButtons = new Map();
    this.edgePaths = new Map();
    this.edgeButtons = new Map();
    this.state = null;
    this.buildNodes();
    this.buildEdges();
    new ResizeObserver(() => this.updateGeometry()).observe(this.stage);
  }

  buildNodes() {
    for (const node of this.proof.nodes) {
      const button = document.createElement("button");
      button.type = "button";
      button.className = "projection-proof-node";
      button.dataset.projectionNode = node.id;
      button.dataset.projectionKind = this.kind;
      button.innerHTML =
        '<span class="projection-proof-node-mark" aria-hidden="true"></span>' +
        '<span class="projection-proof-node-initial" aria-hidden="true"></span>' +
        '<span class="projection-proof-node-label"></span>' +
        '<span class="projection-proof-node-fold" aria-hidden="true"></span>';
      button.querySelector(".projection-proof-node-initial").textContent =
        shortName(node.name);
      button.querySelector(".projection-proof-node-label").textContent = node.name;
      button.addEventListener("click", (event) => {
        if (event.detail === 0) {
          this.store.act(() =>
            this.store.dispatch(selectionStep("node", node.id, `Select ${node.name}`)),
          );
        }
      });
      this.installNodeDrag(button, node);
      this.nodeLayer.append(button);
      this.nodeButtons.set(node.id, button);
    }
  }

  buildEdges() {
    for (const relation of this.proof.relations) {
      const path = document.createElementNS("http://www.w3.org/2000/svg", "path");
      path.classList.add("projection-proof-edge");
      path.dataset.projectionEdge = relation.id;
      path.dataset.projectionKind = this.kind;
      this.edgeSvg.append(path);
      this.edgePaths.set(relation.id, path);

      const button = document.createElement("button");
      button.type = "button";
      button.className = "projection-proof-edge-control";
      button.dataset.projectionEdgeControl = relation.id;
      button.dataset.projectionKind = this.kind;
      button.innerHTML = '<span aria-hidden="true"></span>';
      button.addEventListener("click", () => {
        this.store.act(() =>
          this.store.dispatch(selectionStep("edge", relation.id, `Select ${relation.label}`)),
        );
      });
      this.edgeControls.append(button);
      this.edgeButtons.set(relation.id, button);
    }
  }

  installNodeDrag(button, node) {
    let drag = null;
    const pointAt = (event) => {
      const rect = this.stage.getBoundingClientRect();
      return {
        x: (event.clientX - rect.left) / rect.width,
        y: (event.clientY - rect.top) / rect.height,
      };
    };
    button.addEventListener("pointerdown", (event) => {
      if (event.button !== 0) return;
      event.preventDefault();
      this.store.act(() =>
        this.store.dispatch(selectionStep("node", node.id, `Select ${node.name}`)),
      );
      button.setPointerCapture(event.pointerId);
      drag = { pointerId: event.pointerId, moved: false, last: null };
      button.classList.add("is-dragging");
    });
    button.addEventListener("pointermove", (event) => {
      if (!drag || drag.pointerId !== event.pointerId) return;
      drag.moved = true;
      drag.last = pointAt(event);
      // Until the replay loads, the drag is remembered and committed on release.
      if (!this.store.session) return;
      const current = this.store.committedSnapshot().snapshot;
      this.store.preview(moveStep(current, node, drag.last.x, drag.last.y, this.proof));
    });
    button.addEventListener("pointerup", (event) => {
      if (!drag || drag.pointerId !== event.pointerId) return;
      button.releasePointerCapture(event.pointerId);
      button.classList.remove("is-dragging");
      const finished = drag;
      drag = null;
      if (!finished.moved) {
        if (this.store.session) this.store.clearPreview();
        return;
      }
      this.store.act(() => {
        if (this.store.transient) {
          this.store.commitPreview();
          return;
        }
        const current = this.store.committedSnapshot().snapshot;
        this.store.dispatch(moveStep(current, node, finished.last.x, finished.last.y, this.proof));
      });
    });
    button.addEventListener("pointercancel", () => {
      button.classList.remove("is-dragging");
      if (this.store.session) this.store.clearPreview();
      drag = null;
    });
    button.addEventListener("keydown", (event) => {
      const movement = {
        ArrowLeft: [-1, 0],
        ArrowRight: [1, 0],
        ArrowUp: [0, -1],
        ArrowDown: [0, 1],
      }[event.key];
      if (!movement || !this.state) return;
      event.preventDefault();
      const distance = event.shiftKey ? 0.08 : 0.035;
      this.store.act(() => {
        const { snapshot } = this.store.snapshot();
        const item = itemForSource(snapshot, node.id);
        if (!item) return;
        const position = normalizedPosition(this.proof.scene, item);
        this.store.dispatch(
          moveStep(
            snapshot,
            node,
            clamp(position.x + movement[0] * distance, 0.08, 0.92),
            clamp(position.y + movement[1] * distance, 0.1, 0.9),
            this.proof,
          ),
        );
      });
    });
  }

  render(state) {
    this.state = state;
    for (const node of this.proof.nodes) {
      const button = this.nodeButtons.get(node.id);
      const item = itemForSource(state.snapshot, node.id);
      const selected = state.selection.kind === "node" && state.selection.id === node.id;
      const folded = item ? channel(item, "fold") > 0 : false;
      const foldCount = folded ? dependencyIds(node.id, this.proof).size : 0;
      button.hidden = !item || !item.visible;
      if (item) {
        const position = normalizedPosition(this.proof.scene, item);
        button.style.left = `${position.x * 100}%`;
        button.style.top = `${position.y * 100}%`;
        button.dataset.x = position.x.toFixed(3);
        button.dataset.y = position.y.toFixed(3);
      }
      button.classList.toggle("is-selected", selected);
      button.setAttribute("aria-pressed", String(selected));
      button.setAttribute(
        "aria-label",
        `${node.name}, ${node.class}, ${node.status}. Drag or use arrow keys to move.`,
      );
      const fold = button.querySelector(".projection-proof-node-fold");
      fold.textContent = foldCount > 0 ? `+${foldCount}` : "";
      fold.hidden = foldCount === 0;
    }

    for (const metadata of this.proof.relations) {
      const relation = activeRelation(state.snapshot, metadata.index);
      const endpointHidden =
        !relation ||
        !activeItem(state.snapshot, relation.from)?.visible ||
        !activeItem(state.snapshot, relation.to)?.visible;
      const selected = state.selection.kind === "edge" && state.selection.id === metadata.id;
      const path = this.edgePaths.get(metadata.id);
      const button = this.edgeButtons.get(metadata.id);
      path.toggleAttribute("hidden", endpointHidden);
      path.classList.toggle("is-selected", selected);
      path.classList.toggle("is-curated-out", !relation);
      button.hidden = endpointHidden;
      button.classList.toggle("is-selected", selected);
      button.classList.toggle("is-curated-out", !relation);
      button.setAttribute("aria-pressed", String(selected));
      button.setAttribute(
        "aria-label",
        `${metadata.label}. ${relation ? "Active" : "Removed from scene"}. Select relationship.`,
      );
    }
    this.selection.textContent = `${selectionLabel(state.selection, this.proof)} selected`;
    this.updateGeometry();
  }

  updateGeometry() {
    if (!this.state) return;
    const rect = this.stage.getBoundingClientRect();
    if (rect.width === 0 || rect.height === 0) return;
    this.edgeSvg.setAttribute("viewBox", `0 0 ${rect.width} ${rect.height}`);
    for (const metadata of this.proof.relations) {
      const relation = activeRelation(this.state.snapshot, metadata.index);
      if (!relation) continue;
      const points = relation.points.map((point) => normalizedPoint(this.proof.scene, point));
      const geometry = routeGeometry(points, rect.width, rect.height);
      this.edgePaths.get(metadata.id).setAttribute("d", geometry.path);
      const control = this.edgeButtons.get(metadata.id);
      control.style.left = `${geometry.midpoint.x}px`;
      control.style.top = `${geometry.midpoint.y}px`;
    }
  }
}

class ProjectionControls {
  constructor(root, proof, store) {
    this.root = root;
    this.proof = proof;
    this.store = store;
    this.replayButton = root.querySelector('[data-projection-action="replay"]');
    this.foldButton = root.querySelector('[data-projection-action="fold"]');
    this.edgeButton = root.querySelector('[data-projection-action="edge"]');
    this.resetButton = root.querySelector('[data-projection-action="reset"]');
    this.shareButton = root.querySelector('[data-projection-action="share"]');
    this.cursor = root.querySelector("[data-projection-cursor]");
    this.cursorOutput = root.querySelector("[data-projection-cursor-output]");
    this.readout = root.querySelector("[data-projection-readout]");
    this.notice = root.querySelector("[data-projection-notice]");
    this.reducedMotion = window.matchMedia("(prefers-reduced-motion: reduce)");
    this.playback = null;
    store.beforeDispatch = () => this.stopReplay();
    store.onRefused = () => {
      this.root.dataset.stepRefused = "true";
      const message = `This page keeps at most ${STEP_BOUND} steps. Move back along the trace or reset it to record another.`;
      this.notify(message);
      announce(this.root, message);
    };
    this.install();
  }

  notify(message) {
    this.notice.textContent = message;
    this.notice.hidden = false;
  }

  install() {
    const act = (action) => this.store.act(action);
    this.replayButton.addEventListener("click", () => act(() => this.replay()));
    this.foldButton.addEventListener("click", () =>
      act(() => {
        const state = this.store.snapshot();
        if (state.selection.kind !== "node") return;
        const node = this.proof.nodes.find(({ id }) => id === state.selection.id);
        if (this.store.dispatch(foldStep(state.snapshot, node, this.proof))) {
          announce(this.root, "Scenotime applied the folded-scope scene diff.");
        }
      }),
    );
    this.edgeButton.addEventListener("click", () =>
      act(() => {
        const state = this.store.snapshot();
        if (state.selection.kind !== "edge") return;
        const metadata = relationMetadata(this.proof, state.selection.id);
        if (!metadata || !activeRelation(state.snapshot, metadata.index)) return;
        if (this.store.dispatch(removeRelationStep(metadata))) {
          announce(this.root, "Scenotime tombstoned the relationship in both projections.");
        }
      }),
    );
    this.resetButton.addEventListener("click", () =>
      act(() => {
        this.stopReplay();
        this.store.reset();
        announce(
          this.root,
          `Scene returned to revision ${this.proof.scene.revision} and the start of its supplied trace.`,
        );
      }),
    );
    this.cursor.addEventListener("input", () => {
      this.stopReplay();
      const value = Number(this.cursor.value);
      act(() => this.store.setCursor(value));
    });
    this.cursor.addEventListener("keydown", (event) => {
      const value = Number(this.cursor.value);
      const next = {
        Home: 0,
        End: this.store.length,
        ArrowLeft: Math.max(0, value - 1),
        ArrowDown: Math.max(0, value - 1),
        ArrowRight: Math.min(this.store.length, value + 1),
        ArrowUp: Math.min(this.store.length, value + 1),
      }[event.key];
      if (next === undefined) return;
      event.preventDefault();
      this.stopReplay();
      act(() => this.store.setCursor(next));
    });
    this.shareButton.addEventListener("click", () => act(() => this.share()));
  }

  render(state) {
    this.root.dataset.cursor = String(this.store.cursor);
    this.root.dataset.actionCount = String(this.store.length);
    this.root.dataset.sceneRevision = String(state.snapshot.revision);
    this.root.dataset.selectedKind = state.selection.kind;
    this.root.dataset.selectedId = state.selection.id;
    this.root.dataset.folded = this.proof.nodes
      .filter((node) => channel(itemForSource(state.snapshot, node.id), "fold") > 0)
      .map((node) => node.id)
      .join(",");
    this.cursor.max = String(this.store.length);
    this.cursor.value = String(this.store.cursor);
    this.cursorOutput.textContent = `${this.store.cursor} of ${this.store.length}`;
    this.readout.textContent = `${selectionLabel(state.selection, this.proof)} · revision ${state.snapshot.revision}`;

    if (state.selection.kind === "node") {
      const item = itemForSource(state.snapshot, state.selection.id);
      const children = dependencyIds(state.selection.id, this.proof);
      const folded = channel(item, "fold") > 0;
      this.foldButton.disabled = !item || children.size === 0;
      this.foldButton.textContent = folded ? "Expand dependencies" : "Fold dependencies";
    } else {
      this.foldButton.disabled = true;
      this.foldButton.textContent = "Fold dependencies";
    }

    if (state.selection.kind === "edge") {
      const metadata = relationMetadata(this.proof, state.selection.id);
      const active = metadata && activeRelation(state.snapshot, metadata.index);
      this.edgeButton.disabled = !active;
      this.edgeButton.textContent = active ? "Remove from scene" : "Removed from scene";
    } else {
      this.edgeButton.disabled = true;
      this.edgeButton.textContent = "Select an edge";
    }
  }

  replay() {
    this.stopReplay();
    if (this.reducedMotion.matches) {
      this.store.setCursor(this.store.length);
      announce(this.root, "Trace advanced to its final revision with reduced motion.");
      return;
    }
    this.store.setCursor(0);
    this.root.dataset.playing = "true";
    this.playback = window.setInterval(() => {
      if (this.store.cursor >= this.store.length) {
        this.stopReplay();
        announce(this.root, "Serialized Scenograph trace replayed in both projections.");
        return;
      }
      this.store.setCursor(this.store.cursor + 1);
    }, 520);
  }

  stopReplay() {
    if (this.playback !== null) window.clearInterval(this.playback);
    this.playback = null;
    this.root.dataset.playing = "false";
  }

  // A v3 link is the shelfmark's citation (the capture's address and the
  // generation it expects) plus the position, and the steps only when they
  // differ from the supplied trace.
  async share() {
    this.stopReplay();
    const session = this.store.session;
    const params = new URLSearchParams();
    params.set("projection-scene", SHARE_VERSION);
    params.set("projection", session.captureAddress());
    params.set("expects-generation", session.generation());
    params.set("position", String(this.store.cursor));
    const steps = session.sharedSteps();
    if (steps != null) params.set("trace", encodeSteps(steps));
    const url = new URL(window.location.href);
    url.hash = params.toString();
    window.history.replaceState(null, "", url);
    try {
      if (!navigator.clipboard?.writeText) throw new Error("clipboard unavailable");
      await navigator.clipboard.writeText(url.toString());
      announce(this.root, "Portable Scenograph scene link copied.");
    } catch {
      announce(this.root, "Portable scene link is ready in the address bar.");
    }
  }
}

// ---------------------------------------------------------------------------
// Steps the page records. A scene change names its operations only: the
// session builds the diff from the scene it is recorded onto.

function selectionStep(kind, id, label) {
  return { label, annotation: { kind, id } };
}

function moveStep(snapshot, node, x, y, proof) {
  const instance = instanceForSource(snapshot, node.id);
  if (instance < 0) throw new Error("move source is absent");
  const item = clone(activeItem(snapshot, instance));
  item.transform.translate = scenePoint(proof.scene, x, y);
  const operations = [{ UpdateItem: { index: instance, value: item } }];
  snapshot.tables.relations.forEach((relation, index) => {
    if (!relation || (relation.from !== instance && relation.to !== instance)) return;
    const updated = clone(relation);
    updated.points = [
      relation.from === instance
        ? clone(item.transform.translate)
        : clone(activeItem(snapshot, relation.from).transform.translate),
      relation.to === instance
        ? clone(item.transform.translate)
        : clone(activeItem(snapshot, relation.to).transform.translate),
    ];
    operations.push({ UpdateRelation: { index, value: updated } });
  });
  return { label: `Move ${node.name}`, operations };
}

function foldStep(snapshot, node, proof) {
  const instance = instanceForSource(snapshot, node.id);
  const rootItem = clone(activeItem(snapshot, instance));
  if (!rootItem) throw new Error("fold source is absent");
  const folded = channel(rootItem, "fold") > 0;
  rootItem.channels = rootItem.channels.filter(([name]) => name !== "fold");
  if (!folded) rootItem.channels.push(["fold", 1]);
  const operations = [{ UpdateItem: { index: instance, value: rootItem } }];
  for (const dependency of dependencyIds(node.id, proof)) {
    const child = instanceForSource(snapshot, dependency);
    if (child < 0) continue;
    const item = clone(activeItem(snapshot, child));
    item.visible = folded;
    operations.push({ UpdateItem: { index: child, value: item } });
  }
  return { label: `${folded ? "Expand" : "Fold"} ${node.name} dependencies`, operations };
}

function removeRelationStep(metadata) {
  return {
    label: `Remove ${metadata.label} from the scene`,
    operations: [{ TombstoneRelation: { index: metadata.index } }],
  };
}

// The steps travel as the session's own JSON, base64url-encoded, so the
// epoch's digits are never read as a JavaScript number.
function encodeSteps(json) {
  const bytes = new TextEncoder().encode(json);
  let binary = "";
  for (const byte of bytes) binary += String.fromCharCode(byte);
  return btoa(binary).replaceAll("+", "-").replaceAll("/", "_").replace(/=+$/, "");
}

function decodeSteps(value) {
  if (!value || value.length > 24000) throw new Error("trace is absent or oversized");
  const padded = value
    .replaceAll("-", "+")
    .replaceAll("_", "/")
    .padEnd(Math.ceil(value.length / 4) * 4, "=");
  const bytes = Uint8Array.from(atob(padded), (character) => character.charCodeAt(0));
  return new TextDecoder("utf-8", { fatal: true }).decode(bytes);
}

// ---------------------------------------------------------------------------
// Scene and label helpers.

function instanceForSource(snapshot, sourceId) {
  return snapshot.tables.items.findIndex((item) => {
    if (!item) return false;
    return snapshot.tables.sources[item.source]?.id === sourceId;
  });
}

function itemForSource(snapshot, sourceId) {
  return activeItem(snapshot, instanceForSource(snapshot, sourceId));
}

function activeItem(snapshot, index) {
  return index >= 0 ? (snapshot.tables.items[index] ?? null) : null;
}

function activeRelation(snapshot, index) {
  return snapshot.tables.relations[index] ?? null;
}

function relationMetadata(proof, id) {
  return proof.relations.find((relation) => relation.id === id) ?? null;
}

function dependencyIds(sourceId, proof) {
  return new Set(
    proof.relations
      .filter((relation) => relation.source === sourceId)
      .map((relation) => relation.target),
  );
}

function channel(item, name) {
  if (!item) return 0;
  return item.channels.find(([channelName]) => channelName === name)?.[1] ?? 0;
}

function normalizedPosition(baseline, item) {
  return normalizedPoint(baseline, item.transform.translate);
}

function normalizedPoint(baseline, point) {
  const bounds = baseline.tables.bounds;
  const width = Math.max(bounds.size.w, 1);
  const height = Math.max(bounds.size.h, 1);
  return {
    x: 0.1 + ((point.x - bounds.origin.x) / width) * 0.8,
    y: 0.1 + ((point.y - bounds.origin.y) / height) * 0.8,
  };
}

function scenePoint(baseline, x, y) {
  const bounds = baseline.tables.bounds;
  return {
    x: bounds.origin.x + ((clamp(x, 0.08, 0.92) - 0.1) / 0.8) * bounds.size.w,
    y: bounds.origin.y + ((clamp(y, 0.1, 0.9) - 0.1) / 0.8) * bounds.size.h,
  };
}

function routeGeometry(points, width, height) {
  const pixels = points.map((point) => ({ x: point.x * width, y: point.y * height }));
  const path = pixels
    .map((point, index) => `${index === 0 ? "M" : "L"} ${point.x} ${point.y}`)
    .join(" ");
  const middle = Math.max(0, Math.floor((pixels.length - 1) / 2));
  const a = pixels[middle];
  const b = pixels[Math.min(middle + 1, pixels.length - 1)];
  return { path, midpoint: { x: (a.x + b.x) / 2, y: (a.y + b.y) / 2 } };
}

function selectionLabel(selection, proof) {
  if (selection.kind === "node") {
    return proof.nodes.find((node) => node.id === selection.id)?.name ?? selection.id;
  }
  return relationMetadata(proof, selection.id)?.label ?? selection.id;
}

function shortName(name) {
  return name
    .split(/\s+/)
    .map((part) => part[0])
    .join("")
    .slice(0, 3)
    .toUpperCase();
}

function clone(value) {
  return structuredClone(value);
}

function announce(proofRoot, message) {
  proofRoot.querySelector("[data-projection-status]").textContent = message;
}

function clamp(value, minimum, maximum) {
  return Math.min(maximum, Math.max(minimum, Number.isFinite(value) ? value : minimum));
}
