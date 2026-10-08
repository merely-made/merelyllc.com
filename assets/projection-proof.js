// The Mere profile's projection proof. It reads four sibling artifacts
// (site canvas plan, Rulings 130-136): chirograph's V2 capture, scenotime's
// SceneTrace, the shelfmark that cites the capture, and the S1 host dataset
// that names everything. It replays the trace as `SceneTrace::snapshot_at`
// does and moves through it as `edit_history::History<SceneTrace>` does.

const SHARE_VERSION = "v3";
// The most steps this page keeps. Host policy (Ruling 15), not trace data.
const STEP_BOUND = 16;
// sceno's Score wire at the site's Mere pin; a newer score is refused.
const SCORE_VERSION = 5;
const CAPTURE_VERSION = 2;
const TRACE_VERSION = 1;
const SHELFMARK_SCHEMA = "mere.shelfmark/1";
const AUTHORITY_ROLE = "authority";
const ADAPTER = "mer3ly.repository-graph/v1";
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
  const [captureBytes, traceBytes, shelfmarkBytes, datasetBytes] = await Promise.all([
    fetchBytes("captureSrc"),
    fetchBytes("traceSrc"),
    fetchBytes("shelfmarkSrc"),
    fetchBytes("datasetSrc"),
  ]);
  const text = (bytes) => new TextDecoder().decode(bytes);
  const proof = readProof(
    captureBytes,
    parseLossless(text(captureBytes)),
    parseLossless(text(traceBytes)),
    parseLossless(text(shelfmarkBytes)),
    parseLossless(text(datasetBytes)),
  );

  const link = readSharedLink(proof);
  const store = new ProofStore(proof, link.trace ?? proof.trace, link.position ?? 0);
  const views = [...proofRoot.querySelectorAll("[data-projection-view]")].map(
    (element) => new ProjectionView(element, proof, store),
  );
  const controls = new ProjectionControls(proofRoot, proof, store);
  store.subscribe((state) => {
    views.forEach((view) => view.render(state));
    controls.render(state);
  });

  proofRoot.dataset.captureAddress = proof.address;
  proofRoot.dataset.sceneEpoch = proof.epoch;
  proofRoot.dataset.linkState = link.state;
  if (link.notice) controls.notify(link.notice);
  proofRoot.querySelector("[data-projection-fallback]").hidden = true;
  proofRoot.querySelector("[data-projection-interface]").hidden = false;
  proofRoot.dataset.ready = "true";
  proofRoot.dataset.state = "ready";
  store.notify();
  announce(
    proofRoot,
    link.state === "restored"
      ? "Shared scene trace restored. Both projections replay the same capture and trace."
      : (link.notice ??
          `${proof.nodes.length} projects and ${proof.relations.length} relationships loaded from one Scenograph capture.`),
  );
}

// ---------------------------------------------------------------------------
// Reading the four artifacts.

function readProof(captureBytes, capture, trace, shelfmark, dataset) {
  if (capture?.version !== CAPTURE_VERSION) throw new Error("not a V2 projection capture");
  const { score, authority, scene } = capture;
  if (
    !score ||
    !Number.isInteger(score.version) ||
    score.version > SCORE_VERSION ||
    !Array.isArray(score.items) ||
    (score.holds !== undefined && !Array.isArray(score.holds))
  ) {
    throw new Error("the capture's score is absent or newer than this reader");
  }
  if (authority?.adapter !== ADAPTER || !String(authority.sha256).startsWith("ni:///sha-256;")) {
    throw new Error("the capture names an unknown authority");
  }
  // chirograph's check, then the site's convention (Ruling 132).
  if (!sameInteger(score.generation, authority.generation)) {
    throw new Error("the capture's score and authority disagree about the generation");
  }
  if (!sameInteger(scene?.epoch, authority.generation)) {
    throw new Error("the captured scene's epoch is not the authority generation");
  }
  validateSnapshot(scene);

  // Ruling 135: the shelfmark cites these bytes, this authority and this
  // generation.
  const address = blake3Hex(captureBytes);
  const input = shelfmark?.inputs?.[AUTHORITY_ROLE];
  if (
    shelfmark?.schema !== SHELFMARK_SCHEMA ||
    shelfmark.projection !== address ||
    input?.authority?.adapter !== authority.adapter ||
    input.authority.record !== authority.sha256 ||
    input.reading !== authority.schema ||
    input.expects_generation !== String(authority.generation)
  ) {
    throw new Error("the shelfmark does not cite this capture");
  }

  if (
    trace?.version !== TRACE_VERSION ||
    Object.keys(trace).some((key) => !["version", "base", "steps"].includes(key)) ||
    stringifyLossless(trace.base) !== stringifyLossless(scene)
  ) {
    throw new Error("the scene trace does not start from the captured scene");
  }
  const defaultTrace = new SceneTrace(scene, trace.steps ?? []);

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

  const proof = {
    address,
    epoch: String(authority.generation),
    scene,
    shelfmark,
    trace: defaultTrace,
    nodes,
    relations,
  };
  defaultTrace.steps.forEach((step, index) => {
    if (step.annotation !== undefined) readSelection(proof, step.annotation, index);
  });
  return proof;
}

// Restores a v3 link, and explains a retired, stale or broken one politely.
function readSharedLink(proof) {
  const params = new URLSearchParams(window.location.hash.slice(1));
  const version = params.get("projection-scene");
  const fallback = (state, notice) => ({ state, notice, trace: null, position: null });
  if (version === null) return fallback("none", null);
  if (version !== SHARE_VERSION) {
    return fallback(
      "retired",
      "This link uses a retired scene format, so it cannot be restored. Showing the default trace.",
    );
  }
  const input = proof.shelfmark.inputs[AUTHORITY_ROLE];
  if (
    params.get("projection") !== proof.shelfmark.projection ||
    params.get("expects-generation") !== input.expects_generation
  ) {
    return fallback(
      "stale",
      "This link cites an earlier capture of the scene, which this page no longer carries, so it cannot be restored. Showing the default trace.",
    );
  }
  let trace = proof.trace;
  if (params.has("trace")) {
    let steps;
    try {
      steps = decodeSteps(params.get("trace"));
    } catch {
      return fallback(
        "broken",
        "This link's trace cannot be read, so it cannot be restored. Showing the default trace.",
      );
    }
    if (!Array.isArray(steps) || steps.length > STEP_BOUND) {
      return fallback(
        "refused",
        `This link's trace is longer than the ${STEP_BOUND} steps this page keeps, so it cannot be restored. Showing the default trace.`,
      );
    }
    try {
      trace = new SceneTrace(proof.scene, steps);
      trace.steps.forEach((step, index) => {
        if (step.annotation !== undefined) readSelection(proof, step.annotation, index);
      });
    } catch (error) {
      const where = error instanceof TraceError ? ` at step ${error.index + 1}` : "";
      return fallback(
        "broken",
        `This link's trace breaks its chain${where}, so it cannot be restored. Showing the default trace.`,
      );
    }
  }
  const position = Number(params.get("position"));
  if (!Number.isInteger(position) || position < 0 || position > trace.length) {
    return fallback(
      "broken",
      "This link names a step its trace does not have, so it cannot be restored. Showing the default trace.",
    );
  }
  return { state: "restored", notice: null, trace, position };
}

function readSelection(proof, annotation, index) {
  const keys = annotation && typeof annotation === "object" ? Object.keys(annotation) : [];
  const known =
    keys.length === 2 &&
    ((annotation.kind === "node" && proof.nodes.some(({ id }) => id === annotation.id)) ||
      (annotation.kind === "edge" && proof.relations.some(({ id }) => id === annotation.id)));
  if (!known) throw new TraceError(index, "the step selects nothing this scene carries");
  return { kind: annotation.kind, id: annotation.id };
}

// ---------------------------------------------------------------------------
// The trace and its history.

class TraceError extends Error {
  constructor(index, reason) {
    super(`step ${index}: ${reason}`);
    this.index = index;
  }
}

// scenotime's SceneTrace: a base and steps, validated as a chain when built,
// replayed from the base on request. It keeps no cursor.
class SceneTrace {
  constructor(base, steps) {
    if (!Array.isArray(steps)) throw new TraceError(0, "steps are not a list");
    this.base = base;
    this.steps = steps;
    const head = clone(base);
    steps.forEach((step, index) => advance(head, index, step));
  }

  get length() {
    return this.steps.length;
  }

  snapshotAt(position) {
    if (!Number.isInteger(position) || position < 0 || position > this.steps.length) {
      throw new Error(`position ${position} is out of range`);
    }
    const head = clone(this.base);
    this.steps.slice(0, position).forEach((step, index) => advance(head, index, step));
    return head;
  }

  appended(step) {
    advance(this.snapshotAt(this.steps.length), this.steps.length, step);
    return new SceneTrace(this.base, [...this.steps, step]);
  }
}

// One step, refused unless it chains: a repeated or older diff is a broken
// chain here (Ruling 134), not the no-op it is on a live wire.
function advance(head, index, step) {
  if (
    !step ||
    typeof step !== "object" ||
    typeof step.label !== "string" ||
    Object.keys(step).some((key) => !["label", "diff", "annotation"].includes(key))
  ) {
    throw new TraceError(index, "not a trace step");
  }
  const diff = step.diff;
  if (diff === undefined || diff === null) return;
  if (!sameInteger(diff.epoch, head.epoch)) throw new TraceError(index, "wrong epoch");
  if (diff.base !== head.revision) throw new TraceError(index, "missing base");
  if (!Number.isSafeInteger(diff.revision) || diff.revision <= diff.base) {
    throw new TraceError(index, "invalid revision");
  }
  if (!Array.isArray(diff.operations)) throw new TraceError(index, "no operations");
  try {
    for (const operation of diff.operations) {
      if (operation.UpdateItem) {
        const { index: slot, value } = operation.UpdateItem;
        requireActive(head.tables.items, slot, "item");
        head.tables.items[slot] = clone(value);
      } else if (operation.UpdateRelation) {
        const { index: slot, value } = operation.UpdateRelation;
        requireActive(head.tables.relations, slot, "relation");
        head.tables.relations[slot] = clone(value);
      } else if (operation.TombstoneRelation) {
        const { index: slot } = operation.TombstoneRelation;
        requireActive(head.tables.relations, slot, "relation");
        head.tables.relations[slot] = null;
      } else {
        throw new Error("unsupported scene operation");
      }
    }
    head.revision = diff.revision;
    validateSnapshot(head);
  } catch (error) {
    throw new TraceError(index, error.message);
  }
}

// edit_history::History<SceneTrace>, kept as the longest trace and a
// position: moving back or forward is undo and redo, and committing after a
// move truncates there, clearing redo. Nothing falls off silently; the step
// bound is refused with a message instead.
class TraceHistory {
  constructor(trace, position) {
    this.trace = trace;
    this.position = position;
  }

  get current() {
    return new SceneTrace(this.trace.base, this.trace.steps.slice(0, this.position));
  }

  moveTo(position) {
    this.position = clamp(Math.round(position), 0, this.trace.length);
  }

  commit(step) {
    if (this.position + 1 > STEP_BOUND) return false;
    this.trace = this.current.appended(step);
    this.position = this.trace.length;
    return true;
  }
}

class ProofStore {
  constructor(proof, trace, position) {
    this.proof = proof;
    this.history = new TraceHistory(trace, position);
    this.transientStep = null;
    this.listeners = new Set();
    this.beforeDispatch = null;
    this.onRefused = null;
  }

  get cursor() {
    return this.history.position;
  }

  get length() {
    return this.history.trace.length;
  }

  get steps() {
    return this.history.trace.steps;
  }

  subscribe(listener) {
    this.listeners.add(listener);
  }

  committedSnapshot() {
    const snapshot = this.history.trace.snapshotAt(this.cursor);
    let selection = DEFAULT_SELECTION;
    this.steps.slice(0, this.cursor).forEach((step, index) => {
      if (step.annotation !== undefined) selection = readSelection(this.proof, step.annotation, index);
    });
    return { snapshot, selection };
  }

  snapshot() {
    const state = this.committedSnapshot();
    if (this.transientStep) {
      advance(state.snapshot, this.cursor, this.transientStep);
      if (this.transientStep.annotation !== undefined) {
        state.selection = readSelection(this.proof, this.transientStep.annotation, this.cursor);
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
    this.transientStep = null;
    if (!this.history.commit(step)) {
      this.onRefused?.();
      this.notify();
      return false;
    }
    this.notify();
    return true;
  }

  setCursor(cursor) {
    this.transientStep = null;
    this.history.moveTo(cursor);
    this.notify();
  }

  preview(step) {
    advance(this.committedSnapshot().snapshot, this.cursor, step);
    this.transientStep = step;
    this.notify();
  }

  commitPreview() {
    if (!this.transientStep) return;
    const step = this.transientStep;
    this.transientStep = null;
    this.dispatch(step);
  }

  clearPreview() {
    this.transientStep = null;
    this.notify();
  }

  reset() {
    this.beforeDispatch?.();
    this.history = new TraceHistory(this.proof.trace, 0);
    this.transientStep = null;
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
          this.store.dispatch(selectionStep("node", node.id, `Select ${node.name}`));
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
        this.store.dispatch(selectionStep("edge", relation.id, `Select ${relation.label}`));
      });
      this.edgeControls.append(button);
      this.edgeButtons.set(relation.id, button);
    }
  }

  installNodeDrag(button, node) {
    let drag = null;
    button.addEventListener("pointerdown", (event) => {
      if (event.button !== 0) return;
      event.preventDefault();
      this.store.dispatch(selectionStep("node", node.id, `Select ${node.name}`));
      button.setPointerCapture(event.pointerId);
      drag = { pointerId: event.pointerId, moved: false };
      button.classList.add("is-dragging");
    });
    button.addEventListener("pointermove", (event) => {
      if (!drag || drag.pointerId !== event.pointerId) return;
      const rect = this.stage.getBoundingClientRect();
      drag.moved = true;
      const current = this.store.committedSnapshot().snapshot;
      this.store.preview(
        moveStep(
          current,
          node,
          (event.clientX - rect.left) / rect.width,
          (event.clientY - rect.top) / rect.height,
          this.proof,
        ),
      );
    });
    button.addEventListener("pointerup", (event) => {
      if (!drag || drag.pointerId !== event.pointerId) return;
      button.releasePointerCapture(event.pointerId);
      button.classList.remove("is-dragging");
      if (drag.moved) this.store.commitPreview();
      else this.store.clearPreview();
      drag = null;
    });
    button.addEventListener("pointercancel", () => {
      button.classList.remove("is-dragging");
      this.store.clearPreview();
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
      const item = itemForSource(this.state.snapshot, node.id);
      if (!item) return;
      const position = normalizedPosition(this.proof.scene, item);
      const distance = event.shiftKey ? 0.08 : 0.035;
      this.store.dispatch(
        moveStep(
          this.state.snapshot,
          node,
          clamp(position.x + movement[0] * distance, 0.08, 0.92),
          clamp(position.y + movement[1] * distance, 0.1, 0.9),
          this.proof,
        ),
      );
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
    this.replayButton.addEventListener("click", () => this.replay());
    this.foldButton.addEventListener("click", () => {
      const state = this.store.snapshot();
      if (state.selection.kind !== "node") return;
      const node = this.proof.nodes.find(({ id }) => id === state.selection.id);
      if (this.store.dispatch(foldStep(state.snapshot, node, this.proof))) {
        announce(this.root, "Scenotime applied the folded-scope scene diff.");
      }
    });
    this.edgeButton.addEventListener("click", () => {
      const state = this.store.snapshot();
      if (state.selection.kind !== "edge") return;
      const metadata = relationMetadata(this.proof, state.selection.id);
      if (!metadata || !activeRelation(state.snapshot, metadata.index)) return;
      if (this.store.dispatch(removeRelationStep(state.snapshot, metadata))) {
        announce(this.root, "Scenotime tombstoned the relationship in both projections.");
      }
    });
    this.resetButton.addEventListener("click", () => {
      this.stopReplay();
      this.store.reset();
      announce(
        this.root,
        `Scene returned to revision ${this.proof.scene.revision} and the start of its supplied trace.`,
      );
    });
    this.cursor.addEventListener("input", () => {
      this.stopReplay();
      this.store.setCursor(Number(this.cursor.value));
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
      this.store.setCursor(next);
    });
    this.shareButton.addEventListener("click", () => this.share());
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
    const params = new URLSearchParams();
    params.set("projection-scene", SHARE_VERSION);
    params.set("projection", this.proof.shelfmark.projection);
    params.set(
      "expects-generation",
      this.proof.shelfmark.inputs[AUTHORITY_ROLE].expects_generation,
    );
    params.set("position", String(this.store.cursor));
    const steps = stringifyLossless(this.store.steps);
    if (steps !== stringifyLossless(this.proof.trace.steps)) {
      params.set("trace", encodeSteps(steps));
    }
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
// Steps the page records.

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
  return { label: `Move ${node.name}`, diff: nextDiff(snapshot, operations) };
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
  return {
    label: `${folded ? "Expand" : "Fold"} ${node.name} dependencies`,
    diff: nextDiff(snapshot, operations),
  };
}

function removeRelationStep(snapshot, metadata) {
  return {
    label: `Remove ${metadata.label} from the scene`,
    diff: nextDiff(snapshot, [{ TombstoneRelation: { index: metadata.index } }]),
  };
}

function nextDiff(snapshot, operations) {
  return {
    epoch: snapshot.epoch,
    base: snapshot.revision,
    revision: snapshot.revision + 1,
    operations,
  };
}

function validateSnapshot(snapshot) {
  const tables = snapshot?.tables;
  if (
    !isInteger(snapshot?.epoch) ||
    !Number.isSafeInteger(snapshot?.revision) ||
    !Array.isArray(tables?.sources) ||
    !Array.isArray(tables?.spaces) ||
    !Array.isArray(tables?.items) ||
    !Array.isArray(tables?.item_order) ||
    !Array.isArray(tables?.relations) ||
    tables.items.length !== tables.item_order.length ||
    !tables.spaces[0]
  ) {
    throw new Error("invalid scene snapshot");
  }
  for (const item of tables.items.filter(Boolean)) {
    requireActive(tables.sources, item.source, "source");
    requireActive(tables.spaces, item.space, "space");
  }
  for (const relation of tables.relations.filter(Boolean)) {
    requireActive(tables.items, relation.from, "relation start");
    requireActive(tables.items, relation.to, "relation end");
    requireActive(tables.spaces, relation.space, "relation space");
  }
}

function requireActive(table, index, label) {
  if (!Number.isInteger(index) || index < 0 || index >= table.length || !table[index]) {
    throw new Error(`${label} slot is absent`);
  }
}

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
  return parseLossless(new TextDecoder().decode(bytes));
}

// ---------------------------------------------------------------------------
// Lossless JSON: the scene epoch and the generation exceed 2^53, so an
// integer a JavaScript number cannot hold exactly is read as a BigInt and
// written back as the same digits.

function parseLossless(text) {
  let at = 0;
  const fail = () => {
    throw new SyntaxError(`invalid JSON at ${at}`);
  };
  const space = () => {
    while (" \t\n\r".includes(text[at]) && at < text.length) at += 1;
  };
  const token = (pattern) => {
    pattern.lastIndex = at;
    const match = pattern.exec(text);
    if (!match) fail();
    at = pattern.lastIndex;
    return match;
  };
  const string = () => JSON.parse(token(/"(?:[^\x00-\x1f"\\]|\\(?:["\\/bfnrt]|u[0-9a-fA-F]{4}))*"/y)[0]);
  const value = () => {
    space();
    const next = text[at];
    if (next === "{") {
      at += 1;
      const object = {};
      space();
      if (text[at] === "}") return (at += 1), object;
      for (;;) {
        space();
        const key = string();
        space();
        if (text[at++] !== ":") fail();
        Object.defineProperty(object, key, {
          value: value(),
          enumerable: true,
          writable: true,
          configurable: true,
        });
        space();
        const end = text[at++];
        if (end === "}") return object;
        if (end !== ",") fail();
      }
    }
    if (next === "[") {
      at += 1;
      const array = [];
      space();
      if (text[at] === "]") return (at += 1), array;
      for (;;) {
        array.push(value());
        space();
        const end = text[at++];
        if (end === "]") return array;
        if (end !== ",") fail();
      }
    }
    if (next === '"') return string();
    for (const [word, literal] of [["true", true], ["false", false], ["null", null]]) {
      if (text.startsWith(word, at)) return (at += word.length), literal;
    }
    const [number, fraction, exponent] = token(/-?(?:0|[1-9]\d*)(\.\d+)?([eE][+-]?\d+)?/y);
    if (fraction || exponent) return Number(number);
    return Number.isSafeInteger(Number(number)) ? Number(number) : BigInt(number);
  };
  const result = value();
  space();
  if (at !== text.length) fail();
  return result;
}

function stringifyLossless(value) {
  if (typeof value === "bigint") return value.toString();
  if (value === null || typeof value !== "object") return JSON.stringify(value);
  if (Array.isArray(value)) return `[${value.map(stringifyLossless).join(",")}]`;
  return `{${Object.entries(value)
    .filter(([, entry]) => entry !== undefined)
    .map(([key, entry]) => `${JSON.stringify(key)}:${stringifyLossless(entry)}`)
    .join(",")}}`;
}

function isInteger(value) {
  return typeof value === "bigint" || Number.isSafeInteger(value);
}

function sameInteger(left, right) {
  return isInteger(left) && isInteger(right) && BigInt(left) === BigInt(right);
}

// ---------------------------------------------------------------------------
// BLAKE3, for the capture's content address (chirograph's ContentHash).

const BLAKE3_IV = Uint32Array.of(
  0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
);
const BLAKE3_PERMUTATION = [2, 6, 3, 10, 7, 0, 4, 13, 1, 11, 12, 5, 9, 14, 15, 8];

function blake3Compress(cv, block, counter, length, flags) {
  const s = new Uint32Array(16);
  s.set(cv, 0);
  s.set(BLAKE3_IV.subarray(0, 4), 8);
  s[12] = counter;
  s[13] = Math.floor(counter / 0x100000000);
  s[14] = length;
  s[15] = flags;
  const rotate = (x, n) => (x >>> n) | (x << (32 - n));
  const g = (a, b, c, d, x, y) => {
    s[a] = s[a] + s[b] + x;
    s[d] = rotate(s[d] ^ s[a], 16);
    s[c] = s[c] + s[d];
    s[b] = rotate(s[b] ^ s[c], 12);
    s[a] = s[a] + s[b] + y;
    s[d] = rotate(s[d] ^ s[a], 8);
    s[c] = s[c] + s[d];
    s[b] = rotate(s[b] ^ s[c], 7);
  };
  let m = block;
  for (let round = 0; round < 7; round += 1) {
    g(0, 4, 8, 12, m[0], m[1]);
    g(1, 5, 9, 13, m[2], m[3]);
    g(2, 6, 10, 14, m[4], m[5]);
    g(3, 7, 11, 15, m[6], m[7]);
    g(0, 5, 10, 15, m[8], m[9]);
    g(1, 6, 11, 12, m[10], m[11]);
    g(2, 7, 8, 13, m[12], m[13]);
    g(3, 4, 9, 14, m[14], m[15]);
    const permuted = m;
    m = Uint32Array.from(BLAKE3_PERMUTATION, (index) => permuted[index]);
  }
  for (let index = 0; index < 8; index += 1) {
    s[index] ^= s[index + 8];
    s[index + 8] ^= cv[index];
  }
  return s;
}

function blake3Hex(bytes) {
  const [CHUNK_START, CHUNK_END, PARENT, ROOT] = [1, 2, 4, 8];
  const words = (offset, length) => {
    const block = new Uint32Array(16);
    for (let index = 0; index < length; index += 1) {
      block[index >> 2] |= bytes[offset + index] << (8 * (index & 3));
    }
    return block;
  };
  const chain = (output) =>
    blake3Compress(output.cv, output.block, output.counter, output.length, output.flags).slice(0, 8);
  const chunkOutput = (chunk) => {
    const start = chunk * 1024;
    const end = Math.min(bytes.length, start + 1024);
    const blocks = Math.max(1, Math.ceil((end - start) / 64));
    let cv = BLAKE3_IV;
    for (let index = 0; ; index += 1) {
      const offset = start + index * 64;
      const length = Math.min(64, end - offset);
      const output = {
        cv,
        block: words(offset, length),
        counter: chunk,
        length,
        flags: (index === 0 ? CHUNK_START : 0) | (index === blocks - 1 ? CHUNK_END : 0),
      };
      if (index === blocks - 1) return output;
      cv = chain(output);
    }
  };
  const parent = (left, right) => {
    const block = new Uint32Array(16);
    block.set(left, 0);
    block.set(right, 8);
    return { cv: BLAKE3_IV, block, counter: 0, length: 64, flags: PARENT };
  };
  const chunks = Math.max(1, Math.ceil(bytes.length / 1024));
  const stack = [];
  for (let chunk = 0; chunk < chunks - 1; chunk += 1) {
    let cv = chain(chunkOutput(chunk));
    for (let total = chunk + 1; (total & 1) === 0; total >>= 1) cv = chain(parent(stack.pop(), cv));
    stack.push(cv);
  }
  let output = chunkOutput(chunks - 1);
  while (stack.length) output = parent(stack.pop(), chain(output));
  const digest = blake3Compress(output.cv, output.block, output.counter, output.length, output.flags | ROOT);
  let hex = "";
  for (let index = 0; index < 32; index += 1) {
    hex += ((digest[index >> 2] >>> (8 * (index & 3))) & 0xff).toString(16).padStart(2, "0");
  }
  return hex;
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
