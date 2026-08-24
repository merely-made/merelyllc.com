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
        "The portable scene could not initialize. The semantic relationship lists remain available.",
      );
    });
  });
}

async function startProjectionProof(proofRoot) {
  const artifactElement = document.querySelector("#mere-projection-artifact");
  if (!artifactElement) throw new Error("missing portable projection artifact");
  const artifact = JSON.parse(artifactElement.textContent);
  validateArtifact(artifact);

  const forcedMode = new URLSearchParams(window.location.search).get("projection");
  if (forcedMode === "no-scene") throw new Error("forced scene fallback");

  const restored = parseSharedScene(artifact);
  const store = new PortableSceneStore(
    artifact,
    restored?.steps ?? artifact.default_trace,
    restored?.cursor ?? 0,
  );
  const views = [...proofRoot.querySelectorAll("[data-projection-view]")].map(
    (element) => new ProjectionView(element, artifact, store),
  );
  const controls = new ProjectionControls(proofRoot, artifact, store);
  store.subscribe((state) => {
    views.forEach((view) => view.render(state));
    controls.render(state);
  });

  proofRoot.querySelector("[data-projection-fallback]").hidden = true;
  proofRoot.querySelector("[data-projection-interface]").hidden = false;
  proofRoot.dataset.ready = "true";
  proofRoot.dataset.state = "ready";
  store.notify();
  announce(
    proofRoot,
    restored
      ? "Shared Scenograph trace restored. Both projections consumed the same snapshot and diffs."
      : `${artifact.nodes.length} projects and ${artifact.relations.length} relationships loaded from one Scenograph score and scene snapshot.`,
  );
}

class PortableSceneStore {
  constructor(artifact, steps, cursor) {
    this.artifact = artifact;
    this.baseline = clone(artifact.snapshot);
    this.defaultSteps = clone(artifact.default_trace);
    this.steps = clone(steps);
    this.cursor = clamp(Math.round(cursor), 0, steps.length);
    this.transientStep = null;
    this.listeners = new Set();
    this.beforeDispatch = null;
    validateTrace(artifact, this.steps);
  }

  subscribe(listener) {
    this.listeners.add(listener);
  }

  committedSnapshot() {
    const snapshot = clone(this.baseline);
    let selection = { kind: "node", id: "mere" };
    for (const step of this.steps.slice(0, this.cursor)) {
      selection = applyStep(snapshot, selection, step, this.artifact);
    }
    return { snapshot, selection };
  }

  snapshot() {
    const state = this.committedSnapshot();
    if (this.transientStep) {
      state.selection = applyStep(
        state.snapshot,
        state.selection,
        this.transientStep,
        this.artifact,
      );
    }
    return state;
  }

  notify() {
    const state = this.snapshot();
    for (const listener of this.listeners) listener(state);
  }

  dispatch(step) {
    this.beforeDispatch?.();
    const current = this.snapshot();
    applyStep(clone(current.snapshot), current.selection, step, this.artifact);
    if (this.steps.length >= 16 && this.cursor === this.steps.length) return false;
    this.steps = this.steps.slice(0, this.cursor);
    this.steps.push(clone(step));
    this.cursor = this.steps.length;
    this.transientStep = null;
    this.notify();
    return true;
  }

  setCursor(cursor) {
    this.transientStep = null;
    this.cursor = clamp(Math.round(cursor), 0, this.steps.length);
    this.notify();
  }

  preview(step) {
    const current = this.committedSnapshot();
    applyStep(clone(current.snapshot), current.selection, step, this.artifact);
    this.transientStep = clone(step);
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
    this.steps = clone(this.defaultSteps);
    this.cursor = 0;
    this.transientStep = null;
    this.notify();
  }
}

class ProjectionView {
  constructor(element, artifact, store) {
    this.element = element;
    this.kind = element.dataset.projectionView;
    this.stage = element.querySelector("[data-projection-stage]");
    this.edgeSvg = element.querySelector("[data-projection-edges]");
    this.edgeControls = element.querySelector("[data-projection-edge-controls]");
    this.nodeLayer = element.querySelector("[data-projection-nodes]");
    this.selection = element.querySelector("[data-projection-selection]");
    this.artifact = artifact;
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
    for (const node of this.artifact.nodes) {
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
      this.installNodeDrag(button, node.id);
      this.nodeLayer.append(button);
      this.nodeButtons.set(node.id, button);
    }
  }

  buildEdges() {
    for (const relation of this.artifact.relations) {
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
        this.store.dispatch(selectionStep("edge", relation.id, `Select ${relation.id}`));
      });
      this.edgeControls.append(button);
      this.edgeButtons.set(relation.id, button);
    }
  }

  installNodeDrag(button, sourceId) {
    let drag = null;
    button.addEventListener("pointerdown", (event) => {
      if (event.button !== 0) return;
      event.preventDefault();
      this.store.dispatch(selectionStep("node", sourceId, `Select ${sourceId}`));
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
          sourceId,
          (event.clientX - rect.left) / rect.width,
          (event.clientY - rect.top) / rect.height,
          this.artifact,
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
      const item = itemForSource(this.state.snapshot, sourceId);
      if (!item) return;
      const position = normalizedPosition(this.artifact.snapshot, item);
      const distance = event.shiftKey ? 0.08 : 0.035;
      this.store.dispatch(
        moveStep(
          this.state.snapshot,
          sourceId,
          clamp(position.x + movement[0] * distance, 0.08, 0.92),
          clamp(position.y + movement[1] * distance, 0.1, 0.9),
          this.artifact,
        ),
      );
    });
  }

  render(state) {
    this.state = state;
    for (const node of this.artifact.nodes) {
      const button = this.nodeButtons.get(node.id);
      const item = itemForSource(state.snapshot, node.id);
      const selected = state.selection.kind === "node" && state.selection.id === node.id;
      const folded = item ? channel(item, "fold") > 0 : false;
      const foldCount = folded ? dependencyIds(node.id, this.artifact).size : 0;
      button.hidden = !item || !item.visible;
      if (item) {
        const position = normalizedPosition(this.artifact.snapshot, item);
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

    for (const metadata of this.artifact.relations) {
      const relation = activeRelation(state.snapshot, metadata.index);
      const endpointHidden =
        !relation ||
        !activeItem(state.snapshot, relation.from)?.visible ||
        !activeItem(state.snapshot, relation.to)?.visible;
      const selected =
        state.selection.kind === "edge" && state.selection.id === metadata.id;
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
        `${relationLabel(metadata, this.artifact)}. ${relation ? "Active" : "Removed from scene"}. Select relationship.`,
      );
    }
    this.selection.textContent = `${selectionLabel(state.selection, this.artifact)} selected`;
    this.updateGeometry();
  }

  updateGeometry() {
    if (!this.state) return;
    const rect = this.stage.getBoundingClientRect();
    if (rect.width === 0 || rect.height === 0) return;
    this.edgeSvg.setAttribute("viewBox", `0 0 ${rect.width} ${rect.height}`);
    for (const metadata of this.artifact.relations) {
      const relation = activeRelation(this.state.snapshot, metadata.index);
      if (!relation) continue;
      const points = relation.points.map((point) =>
        normalizedPoint(this.artifact.snapshot, point),
      );
      const geometry = routeGeometry(points, rect.width, rect.height);
      this.edgePaths.get(metadata.id).setAttribute("d", geometry.path);
      const control = this.edgeButtons.get(metadata.id);
      control.style.left = `${geometry.midpoint.x}px`;
      control.style.top = `${geometry.midpoint.y}px`;
    }
  }
}

class ProjectionControls {
  constructor(root, artifact, store) {
    this.root = root;
    this.artifact = artifact;
    this.store = store;
    this.replayButton = root.querySelector('[data-projection-action="replay"]');
    this.foldButton = root.querySelector('[data-projection-action="fold"]');
    this.edgeButton = root.querySelector('[data-projection-action="edge"]');
    this.resetButton = root.querySelector('[data-projection-action="reset"]');
    this.shareButton = root.querySelector('[data-projection-action="share"]');
    this.cursor = root.querySelector("[data-projection-cursor]");
    this.cursorOutput = root.querySelector("[data-projection-cursor-output]");
    this.readout = root.querySelector("[data-projection-readout]");
    this.reducedMotion = window.matchMedia("(prefers-reduced-motion: reduce)");
    this.playback = null;
    store.beforeDispatch = () => this.stopReplay();
    this.install();
  }

  install() {
    this.replayButton.addEventListener("click", () => this.replay());
    this.foldButton.addEventListener("click", () => {
      const state = this.store.snapshot();
      if (state.selection.kind !== "node") return;
      this.store.dispatch(foldStep(state.snapshot, state.selection.id, this.artifact));
      announce(this.root, "Scenotime applied the folded-scope scene diff.");
    });
    this.edgeButton.addEventListener("click", () => {
      const state = this.store.snapshot();
      if (state.selection.kind !== "edge") return;
      const metadata = relationMetadata(this.artifact, state.selection.id);
      if (!metadata || !activeRelation(state.snapshot, metadata.index)) return;
      this.store.dispatch(removeRelationStep(state.snapshot, metadata));
      announce(this.root, "Scenotime tombstoned the relationship in both projections.");
    });
    this.resetButton.addEventListener("click", () => {
      this.stopReplay();
      this.store.reset();
      announce(this.root, "Scene returned to revision one and the start of its supplied trace.");
    });
    this.cursor.addEventListener("input", () => {
      this.stopReplay();
      this.store.setCursor(Number(this.cursor.value));
    });
    this.cursor.addEventListener("keydown", (event) => {
      const value = Number(this.cursor.value);
      const next = {
        Home: 0,
        End: this.store.steps.length,
        ArrowLeft: Math.max(0, value - 1),
        ArrowDown: Math.max(0, value - 1),
        ArrowRight: Math.min(this.store.steps.length, value + 1),
        ArrowUp: Math.min(this.store.steps.length, value + 1),
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
    this.root.dataset.actionCount = String(this.store.steps.length);
    this.root.dataset.sceneRevision = String(state.snapshot.revision);
    this.root.dataset.selectedKind = state.selection.kind;
    this.root.dataset.selectedId = state.selection.id;
    this.root.dataset.folded = this.artifact.nodes
      .filter((node) => channel(itemForSource(state.snapshot, node.id), "fold") > 0)
      .map((node) => node.id)
      .join(",");
    this.cursor.max = String(this.store.steps.length);
    this.cursor.value = String(this.store.cursor);
    this.cursorOutput.textContent = `${this.store.cursor} of ${this.store.steps.length}`;
    this.readout.textContent = `${selectionLabel(state.selection, this.artifact)} · revision ${state.snapshot.revision}`;

    if (state.selection.kind === "node") {
      const item = itemForSource(state.snapshot, state.selection.id);
      const children = dependencyIds(state.selection.id, this.artifact);
      const folded = channel(item, "fold") > 0;
      this.foldButton.disabled = !item || children.size === 0;
      this.foldButton.textContent = folded ? "Expand dependencies" : "Fold dependencies";
    } else {
      this.foldButton.disabled = true;
      this.foldButton.textContent = "Fold dependencies";
    }

    if (state.selection.kind === "edge") {
      const metadata = relationMetadata(this.artifact, state.selection.id);
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
      this.store.setCursor(this.store.steps.length);
      announce(this.root, "Trace advanced to its final revision with reduced motion.");
      return;
    }
    this.store.setCursor(0);
    this.root.dataset.playing = "true";
    this.playback = window.setInterval(() => {
      if (this.store.cursor >= this.store.steps.length) {
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

  async share() {
    this.stopReplay();
    const params = new URLSearchParams();
    params.set("projection-scene", "v2");
    params.set("authority", this.artifact.authority_sha256);
    params.set("trace", encodeTrace(this.store.steps));
    params.set("cursor", String(this.store.cursor));
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

function selectionStep(kind, id, label) {
  return { label, selection: { kind, id }, diff: null };
}

function diffStep(label, diff) {
  return { label, selection: null, diff };
}

function moveStep(snapshot, sourceId, x, y, artifact) {
  const instance = instanceForSource(snapshot, sourceId);
  if (instance < 0) throw new Error("move source is absent");
  const item = clone(activeItem(snapshot, instance));
  item.transform.translate = scenePoint(artifact.snapshot, x, y);
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
  return diffStep(`Move ${sourceId}`, nextDiff(snapshot, operations));
}

function foldStep(snapshot, sourceId, artifact) {
  const instance = instanceForSource(snapshot, sourceId);
  const rootItem = clone(activeItem(snapshot, instance));
  if (!rootItem) throw new Error("fold source is absent");
  const folded = channel(rootItem, "fold") > 0;
  rootItem.channels = rootItem.channels.filter(([name]) => name !== "fold");
  if (!folded) rootItem.channels.push(["fold", 1]);
  const operations = [{ UpdateItem: { index: instance, value: rootItem } }];
  for (const dependency of dependencyIds(sourceId, artifact)) {
    const child = instanceForSource(snapshot, dependency);
    if (child < 0) continue;
    const item = clone(activeItem(snapshot, child));
    item.visible = folded;
    operations.push({ UpdateItem: { index: child, value: item } });
  }
  return diffStep(
    `${folded ? "Expand" : "Fold"} ${sourceId} dependencies`,
    nextDiff(snapshot, operations),
  );
}

function removeRelationStep(snapshot, metadata) {
  return diffStep(
    `Remove ${metadata.id} from the scene`,
    nextDiff(snapshot, [{ TombstoneRelation: { index: metadata.index } }]),
  );
}

function nextDiff(snapshot, operations) {
  return {
    epoch: snapshot.epoch,
    base: snapshot.revision,
    revision: snapshot.revision + 1,
    operations,
  };
}

function applyStep(snapshot, selection, step, artifact) {
  if (!step || typeof step !== "object" || typeof step.label !== "string") {
    throw new Error("invalid projection step");
  }
  let nextSelection = selection;
  if (step.selection) {
    const ids =
      step.selection.kind === "node"
        ? new Set(artifact.nodes.map(({ id }) => id))
        : new Set(artifact.relations.map(({ id }) => id));
    if (!ids.has(step.selection.id)) throw new Error("unknown projection selection");
    nextSelection = clone(step.selection);
  }
  if (step.diff) applySceneDiff(snapshot, step.diff);
  return nextSelection;
}

function applySceneDiff(snapshot, diff) {
  if (diff.epoch !== snapshot.epoch) throw new Error("scene diff has wrong epoch");
  if (diff.revision <= snapshot.revision) return;
  if (diff.base !== snapshot.revision || diff.revision <= diff.base) {
    throw new Error("scene diff has a missing or invalid base revision");
  }
  for (const operation of diff.operations) {
    if (operation.UpdateItem) {
      const { index, value } = operation.UpdateItem;
      requireActive(snapshot.tables.items, index, "item");
      snapshot.tables.items[index] = clone(value);
    } else if (operation.UpdateRelation) {
      const { index, value } = operation.UpdateRelation;
      requireActive(snapshot.tables.relations, index, "relation");
      snapshot.tables.relations[index] = clone(value);
    } else if (operation.TombstoneRelation) {
      const { index } = operation.TombstoneRelation;
      requireActive(snapshot.tables.relations, index, "relation");
      snapshot.tables.relations[index] = null;
    } else {
      throw new Error("unsupported scene operation");
    }
  }
  snapshot.revision = diff.revision;
  validateSnapshot(snapshot);
}

function validateArtifact(artifact) {
  if (
    artifact?.schema !== "mer3ly.portable-projection/v1" ||
    artifact?.adapter !== "mer3ly.repository-graph/v1" ||
    artifact?.score?.version !== 4 ||
    !Array.isArray(artifact.nodes) ||
    !Array.isArray(artifact.relations) ||
    !Array.isArray(artifact.default_trace)
  ) {
    throw new Error("invalid portable projection artifact");
  }
  // Score version 2 added holds: authored placements the solver honored ahead
  // of the arrangement. Their effect is already baked into the snapshot this
  // viewer renders, so nothing here re-applies them; the shape is checked so a
  // malformed hold is refused rather than ignored.
  if (artifact.score.holds !== undefined && !Array.isArray(artifact.score.holds)) {
    throw new Error("invalid portable projection artifact");
  }
  validateSnapshot(artifact.snapshot);
  if (
    artifact.score.items.length !== artifact.nodes.length ||
    artifact.snapshot.tables.items.filter(Boolean).length !== artifact.nodes.length ||
    artifact.snapshot.tables.relations.length !== artifact.relations.length
  ) {
    throw new Error("portable projection counts diverge");
  }
  validateTrace(artifact, artifact.default_trace);
}

function validateTrace(artifact, steps) {
  if (!Array.isArray(steps) || steps.length > 16) throw new Error("invalid trace length");
  const snapshot = clone(artifact.snapshot);
  let selection = { kind: "node", id: "mere" };
  for (const step of steps) selection = applyStep(snapshot, selection, step, artifact);
}

function validateSnapshot(snapshot) {
  const tables = snapshot?.tables;
  if (
    !Number.isInteger(snapshot?.epoch) ||
    !Number.isInteger(snapshot?.revision) ||
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

function parseSharedScene(artifact) {
  const params = new URLSearchParams(window.location.hash.slice(1));
  if (params.get("projection-scene") !== "v2") return null;
  if (params.get("authority") !== artifact.authority_sha256) return null;
  try {
    const steps = decodeTrace(params.get("trace") ?? "");
    validateTrace(artifact, steps);
    return {
      steps,
      cursor: clamp(Number(params.get("cursor")), 0, steps.length),
    };
  } catch {
    return null;
  }
}

function encodeTrace(steps) {
  const bytes = new TextEncoder().encode(JSON.stringify(steps));
  let binary = "";
  for (const byte of bytes) binary += String.fromCharCode(byte);
  return btoa(binary).replaceAll("+", "-").replaceAll("/", "_").replace(/=+$/, "");
}

function decodeTrace(value) {
  if (!value || value.length > 24000) throw new Error("trace is absent or oversized");
  const padded = value.replaceAll("-", "+").replaceAll("_", "/").padEnd(
    Math.ceil(value.length / 4) * 4,
    "=",
  );
  const binary = atob(padded);
  const bytes = Uint8Array.from(binary, (character) => character.charCodeAt(0));
  return JSON.parse(new TextDecoder().decode(bytes));
}

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
  return index >= 0 ? snapshot.tables.items[index] ?? null : null;
}

function activeRelation(snapshot, index) {
  return snapshot.tables.relations[index] ?? null;
}

function relationMetadata(artifact, id) {
  return artifact.relations.find((relation) => relation.id === id) ?? null;
}

function dependencyIds(sourceId, artifact) {
  return new Set(
    artifact.relations
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
  return {
    path,
    midpoint: { x: (a.x + b.x) / 2, y: (a.y + b.y) / 2 },
  };
}

function selectionLabel(selection, artifact) {
  if (selection.kind === "node") {
    return artifact.nodes.find((node) => node.id === selection.id)?.name ?? selection.id;
  }
  const relation = relationMetadata(artifact, selection.id);
  return relation ? relationLabel(relation, artifact) : selection.id;
}

function relationLabel(relation, artifact) {
  const source = artifact.nodes.find((node) => node.id === relation.source)?.name ?? relation.source;
  const target = artifact.nodes.find((node) => node.id === relation.target)?.name ?? relation.target;
  return `${source} ${relation.kind.replaceAll("_", " ")} ${target}`;
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
  return JSON.parse(JSON.stringify(value));
}

function announce(proofRoot, message) {
  proofRoot.querySelector("[data-projection-status]").textContent = message;
}

function clamp(value, minimum, maximum) {
  return Math.min(maximum, Math.max(minimum, Number.isFinite(value) ? value : minimum));
}
