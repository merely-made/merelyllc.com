const runtimeVersion = new URL(import.meta.url).search;
const SCENE_STATE_SCHEMA = "mer3ly.graphshell-scene-state/v1";
const REGISTRY_SCHEMA = "mere.graph-representation-registry/v1";
const READING_REGISTRY_SCHEMA = "mere.graph-reading-registry/v1";
const READING_FACES = new Map([
  ["graph", "identity"],
  ["changes", "delta"],
  ["activity", "signal"],
  ["neighbors", "orbit"],
  ["matrix", "table"],
]);
const ENVIRONMENT_MODES = [
  { id: "clear", label: "Clear", backdrop: "clear", collidable: false },
  { id: "ambient", label: "Ambient", backdrop: "ambient", collidable: false },
  { id: "props", label: "Props", backdrop: "props", collidable: false },
  { id: "props-tangible", label: "Props · tangible", backdrop: "props", collidable: true },
  { id: "field", label: "Field", backdrop: "field", collidable: false },
  { id: "field-tangible", label: "Field · tangible", backdrop: "field", collidable: true },
];
const {
  default: initWasm,
  layout_graph: layoutGraph,
  project_reading: projectReading,
  reading_registry: readingRegistry,
  representation_registry: representationRegistry,
  portable_projection_with_placement: portableProjectionWithPlacement,
  GraphPhysics,
} = await import(`./mer3ly_repo_graph.js${runtimeVersion}`);

const root = document.querySelector("[data-graph-sandbox]");
if (root) {
  startSandbox(root).catch((error) => {
    const fallback = root.querySelector("[data-sandbox-fallback]");
    if (fallback) {
      fallback.textContent =
        "The graph sandbox could not initialize. The semantic repository index remains available.";
    }
    root.dataset.sandboxState = "unavailable";
    console.warn("Mer3ly graph sandbox unavailable:", error);
  });
}

async function startSandbox(sandboxRoot) {
  if (new URLSearchParams(window.location.search).get("graph-sandbox") === "no-wasm") {
    throw new Error("forced Graphshell fallback");
  }
  const specimenElement = document.querySelector("#graph-sandbox-data");
  const repositoryElement = document.querySelector("#repository-graph-data");
  if (!specimenElement || !repositoryElement) {
    throw new Error("graph sandbox authorities are absent");
  }

  const specimen = JSON.parse(specimenElement.textContent);
  const repositories = JSON.parse(repositoryElement.textContent);
  validateAuthority(specimen);
  validateAuthority(repositories);

  await initWasm({
    module_or_path: new URL(
      `./mer3ly_repo_graph_bg.wasm${runtimeVersion}`,
      import.meta.url,
    ),
  });
  const registry = JSON.parse(representationRegistry());
  validateRegistry(registry);
  const readings = JSON.parse(readingRegistry());
  validateReadingRegistry(readings);
  const datasets = buildDatasets(specimen, repositories);
  const sharedState = decodeSceneState(window.location.hash);
  const sandbox = new GraphSandbox(
    sandboxRoot,
    datasets,
    registry,
    readings,
    sharedState,
  );
  sandbox.start();

  sandboxRoot.dataset.sandboxState = "ready";
  sandboxRoot.dataset.sandboxSceneSchema = SCENE_STATE_SCHEMA;
  sandboxRoot.querySelector("[data-sandbox-fallback]").hidden = true;
  sandboxRoot.querySelector("[data-sandbox-interface]").hidden = false;
  announce(
    sandboxRoot,
    `${sandbox.authority.nodes.length} actors and ${sandbox.authority.edges.length} typed relations loaded from ${sandbox.datasetLabel()}.`,
  );
}

function buildDatasets(specimen, repositories) {
  const snapshots = collapseEquivalentSnapshots(
    (repositories.history?.checkpoints ?? [])
      .filter((checkpoint) => checkpoint.availability === "available")
      .map((checkpoint) => ({ cursor: checkpoint.cursor, graph: checkpoint.graph })),
  );
  return new Map([
    [
      "live",
      {
        id: "live",
        label: "merely-made feed",
        graph: graphOnly(repositories),
        snapshots,
      },
    ],
    [
      "specimen",
      {
        id: "specimen",
        label: "heterogeneous specimen",
        graph: graphOnly(specimen),
        snapshots: [],
      },
    ],
  ]);
}

function collapseEquivalentSnapshots(snapshots) {
  const meaningful = [];
  for (const snapshot of snapshots) {
    const previous = meaningful.at(-1);
    if (previous && graphSignature(previous.graph) === graphSignature(snapshot.graph)) {
      meaningful[meaningful.length - 1] = snapshot;
    } else {
      meaningful.push(snapshot);
    }
  }
  return meaningful;
}

function graphSignature(graph) {
  const nodes = graph.nodes
    .map((node) => `${node.id}:${nodeSignature(node)}`)
    .sort()
    .join("|");
  const edges = graph.edges
    .map((edge) => [edge.id, edge.source, edge.target, edge.kind, edge.provenance].join(":"))
    .sort()
    .join("|");
  return `${nodes}\u0001${edges}`;
}

function graphOnly(authority) {
  return {
    schema: authority.schema,
    ...(authority.focus ? { focus: authority.focus } : {}),
    nodes: authority.nodes.map((node) => ({ ...node })),
    edges: authority.edges.map((edge) => ({ ...edge })),
  };
}

function validateAuthority(authority) {
  if (authority.schema !== "mer3ly.repo-graph/v1") {
    throw new Error("graph sandbox authority schema mismatch");
  }
  if (!Array.isArray(authority.nodes) || !authority.nodes.length) {
    throw new Error("graph sandbox has no actors");
  }
  const ids = new Set(authority.nodes.map(({ id }) => id));
  if (ids.size !== authority.nodes.length) {
    throw new Error("graph sandbox actor ids are not unique");
  }
  for (const edge of authority.edges) {
    if (!ids.has(edge.source) || !ids.has(edge.target)) {
      throw new Error(`graph sandbox relation ${edge.id} has an unknown endpoint`);
    }
  }
}

function validateRegistry(registry) {
  if (
    registry.schema !== REGISTRY_SCHEMA ||
    !Array.isArray(registry.profiles) ||
    !registry.fallback
  ) {
    throw new Error("Mere representation registry schema mismatch");
  }
}

function validateReadingRegistry(registry) {
  if (
    registry.schema !== READING_REGISTRY_SCHEMA ||
    !Array.isArray(registry.profiles) ||
    !registry.profiles.length
  ) {
    throw new Error("Mere reading registry schema mismatch");
  }
}

class GraphSandbox {
  constructor(sandboxRoot, datasets, registry, readingRegistry, sharedState) {
    this.root = sandboxRoot;
    this.datasets = datasets;
    this.registry = registry;
    this.readingRegistry = readingRegistry;
    this.readings = new Map(
      readingRegistry.profiles.map((reading) => [reading.id, reading]),
    );
    this.sharedState = sharedState;
    this.stage = sandboxRoot.querySelector("[data-sandbox-stage]");
    this.canvas = sandboxRoot.querySelector("[data-sandbox-canvas]");
    this.context = this.canvas.getContext("2d");
    this.nodeLayer = sandboxRoot.querySelector("[data-sandbox-nodes]");
    this.matrix = sandboxRoot.querySelector("[data-sandbox-matrix]");
    this.caption = sandboxRoot.querySelector("[data-sandbox-caption]");
    this.controlActors = new Map(
      [...sandboxRoot.querySelectorAll("[data-sandbox-cycle]")].map((control) => [
        control.dataset.sandboxCycle,
        control,
      ]),
    );
    this.historyGroup = sandboxRoot.querySelector("[data-sandbox-history-control]");
    this.historyControl = sandboxRoot.querySelector("[data-sandbox-history]");
    this.historyStatus = sandboxRoot.querySelector("[data-sandbox-history-status]");
    this.shareControl = sandboxRoot.querySelector("[data-sandbox-share]");
    this.shareStatus = sandboxRoot.querySelector("[data-sandbox-share-status]");
    this.exportControl = sandboxRoot.querySelector("[data-sandbox-export]");
    this.exportStatus = sandboxRoot.querySelector("[data-sandbox-export-status]");
    this.datasetId = "live";
    this.historyIndex = 0;
    this.scene = readingRegistry.profiles[0].id;
    this.currentArrangement =
      readingRegistry.profiles[0].default_arrangement ?? "graph_layout:stack";
    this.mobility = "anchored";
    this.backdrop = "ambient";
    this.collidable = false;
    this.selectedId = null;
    this.pinsByDataset = new Map();
    this.frame = null;
    this.frameById = new Map();
    this.nodeButtons = new Map();
    this.lastTime = performance.now();
    this.animationFrame = null;
    this.drag = null;
    this.scale = 1;
  }

  start() {
    this.installControlActors();
    this.applySharedState(this.sharedState);
    this.setLatestHistoryUnlessShared();
    this.loadAuthority();
    this.resize();
    this.resizeObserver = new ResizeObserver(() => {
      this.resize();
      this.schedule();
    });
    this.resizeObserver.observe(this.stage);
    this.schedule();
  }

  dataset() {
    return this.datasets.get(this.datasetId);
  }

  datasetLabel() {
    return this.dataset()?.label ?? this.datasetId;
  }

  readingProfile() {
    return this.readings.get(this.scene) ?? this.readingRegistry.profiles[0];
  }

  isMatrix() {
    return this.readingProfile().surface === "relation_matrix";
  }

  currentPins() {
    if (!this.pinsByDataset.has(this.datasetId)) {
      this.pinsByDataset.set(this.datasetId, new Map());
    }
    return this.pinsByDataset.get(this.datasetId);
  }

  setLatestHistoryUnlessShared() {
    const snapshots = this.dataset()?.snapshots ?? [];
    if (!snapshots.length) {
      this.historyIndex = 0;
      return;
    }
    if (!this.sharedState?.source) this.historyIndex = snapshots.length - 1;
  }

  applySharedState(state) {
    if (!state || state.schema !== SCENE_STATE_SCHEMA) return;
    if (this.datasets.has(state.dataset)) this.datasetId = state.dataset;
    if (this.readings.has(state.reading)) {
      this.scene = state.reading;
    }
    if (typeof state.arrangement === "string") {
      this.currentArrangement = state.arrangement;
    }
    if (["anchored", "free"].includes(state.motion)) {
      this.mobility = state.motion;
    } else if (state.motion === "frozen") {
      this.mobility = "anchored";
    }
    if (["clear", "ambient", "props", "field"].includes(state.backdrop?.kind)) {
      this.backdrop = state.backdrop.kind;
      this.collidable = Boolean(state.backdrop.collidable);
    }
    if (typeof state.selection === "string") this.selectedId = state.selection;

    const snapshots = this.dataset()?.snapshots ?? [];
    const sourceIndex = snapshots.findIndex(
      ({ cursor }) =>
        cursor.source === state.source?.source && cursor.commit === state.source?.commit,
    );
    this.historyIndex = sourceIndex >= 0 ? sourceIndex : Math.max(0, snapshots.length - 1);

    const pins = new Map();
    for (const pin of Array.isArray(state.pins) ? state.pins.slice(0, 64) : []) {
      if (
        typeof pin.id === "string" &&
        Number.isFinite(pin.x) &&
        Number.isFinite(pin.y)
      ) {
        pins.set(pin.id, {
          x: clamp(pin.x, -2000, 2000),
          y: clamp(pin.y, -2000, 2000),
        });
      }
    }
    this.pinsByDataset.set(this.datasetId, pins);
  }

  authorityForState() {
    const dataset = this.dataset();
    const snapshots = dataset.snapshots;
    const current = snapshots[this.historyIndex]?.graph ?? dataset.graph;
    const previous = this.historyIndex > 0 ? snapshots[this.historyIndex - 1].graph : null;
    return JSON.parse(
      projectReading(
        JSON.stringify({
          reading: this.scene,
          current,
          previous,
          focus: this.selectedId ?? current.focus ?? null,
        }),
      ),
    );
  }

  loadAuthority() {
    this.authority = this.authorityForState();
    validateAuthority(this.authority);
    this.layout = JSON.parse(layoutGraph(JSON.stringify(this.authority)));
    this.arrangements = new Map(
      this.layout.arrangements.map((arrangement) => [arrangement.id, arrangement]),
    );
    const reading = this.readingProfile();
    if (
      reading.arrangement_locked &&
      reading.default_arrangement &&
      this.arrangements.has(reading.default_arrangement)
    ) {
      this.currentArrangement = reading.default_arrangement;
    } else if (!this.arrangements.has(this.currentArrangement)) {
      this.currentArrangement =
        reading.default_arrangement && this.arrangements.has(reading.default_arrangement)
          ? reading.default_arrangement
          : this.layout.default_arrangement;
    }

    this.physics = new GraphPhysics(JSON.stringify(this.authority));
    this.physics.setBackdrop(this.backdrop, this.collidable);
    this.installNodes();
    this.buildMatrix();
    this.applyArrangement();
    if (!this.authority.nodes.some(({ id }) => id === this.selectedId)) {
      this.selectedId = this.authority.focus ?? this.authority.nodes[0].id;
    }
    this.select(this.selectedId, false);
    this.syncControlActors();
    this.applySceneVisibility();
    this.updateHistoryControl();
    this.updateCaption();
    this.root.dataset.sandboxDataset = this.datasetId;
    this.root.dataset.sandboxSource = this.sourceLabel();
  }

  installControlActors() {
    for (const [name, control] of this.controlActors) {
      control.addEventListener("click", (event) => {
        this.cycleControl(name, event.shiftKey ? -1 : 1);
      });
      control.addEventListener("keydown", (event) => {
        const delta = ["ArrowLeft", "ArrowUp"].includes(event.key)
          ? -1
          : ["ArrowRight", "ArrowDown"].includes(event.key)
            ? 1
            : 0;
        if (delta !== 0) {
          event.preventDefault();
          this.cycleControl(name, delta);
        }
      });
    }
    this.historyControl.addEventListener("input", () => {
      this.historyIndex = Number(this.historyControl.value);
      this.loadAuthority();
      this.schedule();
    });
    this.shareControl.addEventListener("click", () => this.copySceneLink());
    this.exportControl.addEventListener("click", () => this.copyPortableProjection());
  }

  controlOptions(name) {
    if (name === "dataset") {
      return [...this.datasets].map(([id, dataset]) => ({ id, label: dataset.label }));
    }
    if (name === "reading") {
      return this.readingRegistry.profiles.map(({ id, label }) => ({ id, label }));
    }
    if (name === "arrangement") {
      return [...(this.arrangements ?? new Map()).values()].map(({ id, name: label }) => ({
        id,
        label: id === "graph_layout:radial" ? "Neighborhood" : label,
      }));
    }
    if (name === "mobility") {
      return [
        { id: "anchored", label: "Anchored" },
        { id: "free", label: "Free" },
      ];
    }
    if (name === "environment") return ENVIRONMENT_MODES;
    return [];
  }

  controlValue(name) {
    if (name === "dataset") return this.datasetId;
    if (name === "reading") return this.scene;
    if (name === "arrangement") return this.currentArrangement;
    if (name === "mobility") return this.mobility;
    if (name === "environment") {
      return (
        ENVIRONMENT_MODES.find(
          ({ backdrop, collidable }) =>
            backdrop === this.backdrop && collidable === this.collidable,
        )?.id ?? "ambient"
      );
    }
    return "";
  }

  cycleControl(name, delta) {
    const options = this.controlOptions(name);
    if (!options.length) return;
    const current = options.findIndex(({ id }) => id === this.controlValue(name));
    const next = (Math.max(0, current) + delta + options.length) % options.length;
    this.changeControl(name, options[next].id);
  }

  changeControl(name, value) {
    if (name === "dataset") {
      this.datasetId = this.datasets.has(value) ? value : "live";
      const snapshots = this.dataset().snapshots;
      this.historyIndex = Math.max(0, snapshots.length - 1);
      this.selectedId = null;
      this.loadAuthority();
      this.schedule();
      return;
    }
    if (name === "reading") {
      this.scene = value;
      const reading = this.readingProfile();
      if (reading.default_arrangement) {
        this.currentArrangement = reading.default_arrangement;
      }
      this.loadAuthority();
      this.schedule();
      return;
    }
    if (name === "arrangement") {
      this.currentArrangement = value;
      this.applyArrangement();
      this.updateCaption();
      this.syncControlActors();
      this.schedule();
      return;
    }
    if (name === "mobility") {
      this.mobility = value;
      this.stage.dataset.sandboxMobility = value;
      this.applyArrangement();
      this.updateSelectionFaces();
      this.updateCaption();
      this.syncControlActors();
      this.schedule();
      return;
    }
    if (name === "environment") {
      const environment = ENVIRONMENT_MODES.find(({ id }) => id === value);
      if (!environment) return;
      this.backdrop = environment.backdrop;
      this.collidable = environment.collidable;
      this.stage.dataset.sandboxBackdrop = this.backdrop;
      this.physics.setBackdrop(this.backdrop, this.collidable);
      this.updateCaption();
      this.syncControlActors();
      this.schedule();
    }
  }

  syncControlActors() {
    for (const [name, control] of this.controlActors) {
      const options = this.controlOptions(name);
      const value = this.controlValue(name);
      const label = options.find(({ id }) => id === value)?.label ?? value;
      control.dataset.sandboxControlValue = value;
      control.querySelector("[data-sandbox-cycle-value]").textContent = label;
      const kind = control.querySelector(".graph-sandbox-control-kind").textContent;
      control.setAttribute(
        "aria-label",
        `${kind}: ${label}. Activate for the next value; use arrow keys for either direction.`,
      );
    }
    this.stage.dataset.sandboxScene = this.scene;
    this.stage.dataset.sandboxFace = READING_FACES.get(this.scene) ?? "identity";
    this.stage.dataset.sandboxMobility = this.mobility;
    this.stage.dataset.sandboxBackdrop = this.backdrop;
  }

  applySceneVisibility() {
    const reading = this.readingProfile();
    const matrix = reading.surface === "relation_matrix";
    this.canvas.hidden = matrix;
    this.nodeLayer.hidden = matrix;
    this.matrix.hidden = !matrix;
    this.controlActors.get("arrangement").disabled = matrix || reading.arrangement_locked;
    this.controlActors.get("mobility").disabled = matrix;
    this.controlActors.get("environment").disabled = matrix;
    this.updateNodeScenes();
    this.updateMatrixSelection();
  }

  updateHistoryControl() {
    const snapshots = this.dataset().snapshots;
    this.historyGroup.hidden = snapshots.length < 2;
    if (!snapshots.length) {
      this.historyStatus.textContent = "authored specimen";
      return;
    }
    this.historyControl.min = "0";
    this.historyControl.max = String(snapshots.length - 1);
    this.historyControl.value = String(this.historyIndex);
    this.historyStatus.textContent = this.sourceLabel();
  }

  sourceLabel() {
    const snapshot = this.dataset()?.snapshots?.[this.historyIndex];
    if (!snapshot) return "authored specimen";
    const date = snapshot.cursor.committed_at.slice(0, 10);
    return `${date} · ${snapshot.cursor.commit.slice(0, 7)}`;
  }

  applyArrangement() {
    const arrangement = this.arrangements.get(this.currentArrangement);
    if (!arrangement) return;
    this.physics.setArrangement(JSON.stringify(arrangement.nodes), this.mobility);
    for (const [id, point] of this.currentPins()) {
      if (this.authority.nodes.some((node) => node.id === id)) {
        this.physics.pinNode(id, point.x, point.y);
      }
    }
    this.frame = JSON.parse(this.physics.frame());
    this.indexFrame();
  }

  recomputeNeighborhood(focus) {
    this.authority.focus = focus;
    this.layout = JSON.parse(layoutGraph(JSON.stringify(this.authority)));
    this.arrangements = new Map(
      this.layout.arrangements.map((arrangement) => [arrangement.id, arrangement]),
    );
    if (this.currentArrangement === "graph_layout:radial") this.applyArrangement();
  }

  installNodes() {
    this.nodeLayer.replaceChildren();
    this.nodeButtons.clear();
    for (const node of this.authority.nodes) {
      const profile = this.profileFor(node.class);
      const button = document.createElement("button");
      button.type = "button";
      button.className = [
        "graph-sandbox-node",
        `class-${safeToken(node.class)}`,
        `primitive-${safeToken(profile.primitive.body)}`,
        `change-${safeToken(node.change)}`,
      ].join(" ");
      button.dataset.sandboxNode = node.id;
      button.dataset.change = node.change;
      button.dataset.primitive = profile.primitive.id;
      button.dataset.face = READING_FACES.get(this.scene) ?? "identity";
      button.setAttribute("aria-label", `${node.name}, ${node.class}, ${node.status}`);
      button.append(...this.nodeFace(node, profile));
      button.addEventListener("click", () => {
        if (!button.dataset.dragged) this.select(node.id);
        delete button.dataset.dragged;
      });
      button.addEventListener("dblclick", (event) => {
        event.preventDefault();
        this.togglePin(node.id);
      });
      button.addEventListener("keydown", (event) => this.handleNodeKey(event, node.id));
      button.addEventListener("pointerdown", (event) => this.startDrag(event, node.id, button));
      this.nodeLayer.append(button);
      this.nodeButtons.set(node.id, button);
    }
    this.updateNodeScenes();
  }

  nodeFace(node, profile) {
    const face = READING_FACES.get(this.scene) ?? "identity";
    const mark = document.createElement("span");
    mark.className = "graph-sandbox-node-mark";
    mark.setAttribute("aria-hidden", "true");
    if (face === "delta") {
      mark.textContent = { added: "+", updated: "~", removed: "−", stable: "·" }[
        node.change
      ];
    } else if (face === "signal") {
      mark.textContent = compactDate(node.pushed_at);
    } else {
      mark.textContent = shortName(node.name);
    }

    const label = document.createElement("span");
    label.className = "graph-sandbox-node-label";
    label.textContent = node.name;

    const meta = document.createElement("span");
    meta.className = "graph-sandbox-node-meta";
    if (face === "delta") meta.textContent = node.change;
    else if (face === "signal") meta.textContent = node.status;
    else if (face === "orbit") {
      meta.textContent = node.id === this.authority.focus ? "focus" : "one relation";
    } else meta.textContent = node.class;

    const detail = document.createElement("span");
    detail.className = "graph-sandbox-node-detail";
    detail.textContent = `${node.summary} ${profile.primitive.label}; ${behaviorText(profile)}. ${this.mobility} motion. Press P or double-click to pin.`;
    return [mark, label, meta, detail];
  }

  updateNodeScenes() {
    const emphasis = this.readingProfile().emphasis;
    for (const node of this.authority.nodes) {
      const button = this.nodeButtons.get(node.id);
      button?.classList.toggle(
        "is-change-muted",
        emphasis === "change" && node.change === "stable",
      );
      button?.classList.toggle("is-activity", emphasis === "activity");
      button?.classList.toggle(
        "is-reading-focus",
        emphasis === "focus_distance" && node.id === this.authority.focus,
      );
    }
    this.updateSelectionFaces();
  }

  startDrag(event, id, button) {
    if (event.button !== 0 || this.isMatrix()) return;
    if (
      this.readingProfile().actor_scope === "focus_and_neighbors" &&
      this.authority.focus !== id
    ) {
      this.select(id);
      return;
    }
    this.select(id, false);
    button.setPointerCapture(event.pointerId);
    this.drag = {
      id,
      pointerId: event.pointerId,
      button,
      startX: event.clientX,
      startY: event.clientY,
      moved: false,
      persistent: this.currentPins().has(id),
    };
    const move = (moveEvent) => this.dragNode(moveEvent);
    const finish = (finishEvent) => {
      if (!this.drag || finishEvent.pointerId !== this.drag.pointerId) return;
      const finished = this.drag;
      if (finished.moved) finished.button.dataset.dragged = "true";
      finished.button.removeEventListener("pointermove", move);
      finished.button.removeEventListener("pointerup", finish);
      finished.button.removeEventListener("pointercancel", finish);
      if (finished.moved && !finished.persistent) {
        this.physics.unpinNode(finished.id);
        this.frame = JSON.parse(this.physics.frame());
        this.indexFrame();
      }
      this.drag = null;
      this.updateSelectionFaces();
      this.schedule();
    };
    button.addEventListener("pointermove", move);
    button.addEventListener("pointerup", finish);
    button.addEventListener("pointercancel", finish);
  }

  dragNode(event) {
    if (!this.drag || event.pointerId !== this.drag.pointerId) return;
    if (Math.hypot(event.clientX - this.drag.startX, event.clientY - this.drag.startY) > 3) {
      this.drag.moved = true;
    }
    if (!this.drag.moved) return;
    const point = this.worldFromPointer(event);
    if (this.drag.persistent) this.currentPins().set(this.drag.id, point);
    this.physics.pinNode(this.drag.id, point.x, point.y);
    this.frame = JSON.parse(this.physics.frame());
    this.indexFrame();
    this.schedule();
  }

  handleNodeKey(event, currentId) {
    const index = this.authority.nodes.findIndex(({ id }) => id === currentId);
    let next = null;
    if (event.key === "ArrowRight" || event.key === "ArrowDown") next = index + 1;
    if (event.key === "ArrowLeft" || event.key === "ArrowUp") next = index - 1;
    if (event.key === "Home") next = 0;
    if (event.key === "End") next = this.authority.nodes.length - 1;
    if (event.key.toLowerCase() === "p") {
      event.preventDefault();
      this.togglePin(currentId);
      return;
    }
    if (next !== null) {
      event.preventDefault();
      const normalized = (next + this.authority.nodes.length) % this.authority.nodes.length;
      this.nodeButtons.get(this.authority.nodes[normalized].id).focus();
    }
  }

  select(id, speak = true) {
    if (!this.nodeButtons.has(id)) return;
    const node = this.node(id);
    if (
      this.readingProfile().actor_scope === "focus_and_neighbors" &&
      this.authority.focus !== id
    ) {
      this.selectedId = id;
      this.loadAuthority();
      if (speak) announce(this.root, `${node.name} is now the neighborhood focus.`);
      this.schedule();
      return;
    }
    this.selectedId = id;
    for (const [nodeId, button] of this.nodeButtons) {
      const selected = nodeId === id;
      button.classList.toggle("is-selected", selected);
      button.setAttribute("aria-pressed", String(selected));
    }
    if (this.currentArrangement === "graph_layout:radial") {
      this.recomputeNeighborhood(id);
    }
    this.updateSelectionFaces();
    this.updateMatrixSelection();
    if (speak) {
      announce(this.root, `${node.name} selected. ${node.summary}`);
    }
    this.schedule();
  }

  togglePin(id) {
    if (!id) return;
    const point = this.frameById.get(id);
    if (!point) return;
    if (this.currentPins().has(id)) {
      this.currentPins().delete(id);
      this.physics.unpinNode(id);
    } else {
      const pinned = { x: point.x, y: point.y };
      this.currentPins().set(id, pinned);
      this.physics.pinNode(id, pinned.x, pinned.y);
    }
    this.frame = JSON.parse(this.physics.frame());
    this.indexFrame();
    this.updateSelectionFaces();
    this.schedule();
  }

  node(id) {
    return this.authority.nodes.find((node) => node.id === id);
  }

  profileFor(className) {
    return (
      this.registry.profiles.find((profile) => profile.classes.includes(className)) ??
      this.registry.fallback
    );
  }

  updateSelectionFaces() {
    for (const node of this.authority.nodes) {
      const button = this.nodeButtons.get(node.id);
      if (!button) continue;
      const pinned = this.currentPins().has(node.id);
      button.classList.toggle("is-pinned", pinned);
      const profile = this.profileFor(node.class);
      const detail = button.querySelector(".graph-sandbox-node-detail");
      if (detail) {
        const motion = pinned ? "pinned" : this.mobility;
        detail.textContent = `${node.summary} ${profile.primitive.label}; ${behaviorText(profile)}. ${motion} motion. Press P or double-click to pin.`;
      }
    }
  }

  updateCaption() {
    const arrangement = this.arrangements.get(this.currentArrangement);
    const reading = this.readingProfile();
    const sceneText =
      this.scene === "changes" && this.datasetId === "specimen"
        ? "Authored transition states in the specimen"
        : reading.description;
    const arrangementName =
      this.currentArrangement === "graph_layout:radial"
        ? "Neighborhood"
        : arrangement?.name ?? "none";
    const collision = this.collidable ? "collidable" : "intangible";
    const face = READING_FACES.get(this.scene) ?? "identity";
    this.caption.textContent = `${this.datasetLabel()} · ${this.sourceLabel()}. ${sceneText}. ${face} face; ${arrangementName} arrangement; ${this.mobility}; ${this.backdrop} field (${collision}).`;
  }

  buildMatrix() {
    this.matrix.replaceChildren();
    const nodes = this.authority.nodes;
    this.matrix.style.setProperty("--matrix-size", String(nodes.length));
    const corner = document.createElement("span");
    corner.className = "graph-sandbox-matrix-corner";
    corner.textContent = "from ↓ / to →";
    this.matrix.append(corner);
    for (const node of nodes) this.matrix.append(this.matrixHeader(node, "column"));
    for (const source of nodes) {
      this.matrix.append(this.matrixHeader(source, "row"));
      for (const target of nodes) {
        const edge = this.authority.edges.find(
          (candidate) => candidate.source === source.id && candidate.target === target.id,
        );
        const cell = document.createElement("button");
        cell.type = "button";
        cell.className = "graph-sandbox-matrix-cell";
        cell.dataset.matrixSource = source.id;
        cell.dataset.matrixTarget = target.id;
        cell.textContent = edge ? "→" : "·";
        cell.title = edge
          ? `${source.name} ${humanize(edge.kind)} ${target.name}`
          : `${source.name} has no direct relation to ${target.name}`;
        cell.setAttribute("aria-label", cell.title);
        cell.addEventListener("click", () => this.select(edge ? target.id : source.id));
        if (edge) cell.classList.add("has-relation");
        this.matrix.append(cell);
      }
    }
  }

  matrixHeader(node, axis) {
    const button = document.createElement("button");
    button.type = "button";
    button.className = `graph-sandbox-matrix-header is-${axis}`;
    button.dataset.matrixNode = node.id;
    button.textContent = node.name;
    button.addEventListener("click", () => this.select(node.id));
    return button;
  }

  updateMatrixSelection() {
    for (const element of this.matrix.querySelectorAll(
      "[data-matrix-node], [data-matrix-source], [data-matrix-target]",
    )) {
      element.classList.toggle(
        "is-selected",
        element.dataset.matrixNode === this.selectedId ||
          element.dataset.matrixSource === this.selectedId ||
          element.dataset.matrixTarget === this.selectedId,
      );
    }
  }

  sceneState() {
    const source = this.dataset().snapshots[this.historyIndex]?.cursor ?? {
      source: "mer3ly/specimen",
      commit: "authored",
      committed_at: "static",
    };
    return {
      schema: SCENE_STATE_SCHEMA,
      dataset: this.datasetId,
      source,
      reading: this.scene,
      arrangement: this.currentArrangement,
      motion: this.mobility,
      backdrop: { kind: this.backdrop, collidable: this.collidable },
      selection: this.selectedId,
      pins: [...this.currentPins()].map(([id, point]) => ({ id, ...point })),
    };
  }

  async copySceneLink() {
    const encoded = encodeSceneState(this.sceneState());
    const url = new URL(window.location.href);
    url.hash = `graphshell-scene=${encoded}`;
    window.history.replaceState(null, "", url);
    let copied = false;
    try {
      await navigator.clipboard.writeText(url.toString());
      copied = true;
    } catch {
      copied = false;
    }
    this.shareStatus.textContent = copied
      ? "portable scene copied"
      : "portable scene written to this URL";
    announce(this.root, this.shareStatus.textContent);
  }

  // Sharing hands over a citation; this hands over the thing it cites.
  //
  // The score carries the visitor's pins as holds, the solver honors them ahead
  // of the arrangement, and the artifact only comes back if it consumed
  // cleanly, so a reported pin count is a checked one rather than a claim.
  async copyPortableProjection() {
    let artifact;
    try {
      artifact = portableProjectionWithPlacement(
        JSON.stringify(this.authority),
        JSON.stringify(this.sceneState()),
      );
    } catch (error) {
      this.exportStatus.textContent = `projection refused: ${error}`;
      announce(this.root, this.exportStatus.textContent);
      return;
    }
    const held = JSON.parse(artifact).score.holds?.length ?? 0;
    let copied = false;
    try {
      await navigator.clipboard.writeText(artifact);
      copied = true;
    } catch {
      copied = false;
    }
    const pins = held === 1 ? "1 pin" : `${held} pins`;
    this.exportStatus.textContent = copied
      ? `portable projection copied, ${pins} honored`
      : `portable projection built, ${pins} honored, clipboard refused`;
    announce(this.root, this.exportStatus.textContent);
  }

  resize() {
    const rect = this.stage.getBoundingClientRect();
    const ratio = Math.min(window.devicePixelRatio || 1, 2);
    this.canvas.width = Math.max(1, Math.round(rect.width * ratio));
    this.canvas.height = Math.max(1, Math.round(rect.height * ratio));
    this.canvas.style.width = `${rect.width}px`;
    this.canvas.style.height = `${rect.height}px`;
    this.context.setTransform(ratio, 0, 0, ratio, 0, 0);
    this.scale = Number(Math.min(rect.width / 820, rect.height / 640).toFixed(4));
  }

  screenPoint(point) {
    const rect = this.stage.getBoundingClientRect();
    return {
      x: rect.width * 0.5 + point.x * this.scale,
      y: rect.height * 0.5 + point.y * this.scale,
    };
  }

  worldFromPointer(event) {
    const rect = this.stage.getBoundingClientRect();
    return {
      x: (event.clientX - rect.left - rect.width * 0.5) / this.scale,
      y: (event.clientY - rect.top - rect.height * 0.5) / this.scale,
    };
  }

  indexFrame() {
    this.frameById = new Map(this.frame.nodes.map((node) => [node.id, node]));
  }

  schedule() {
    if (this.animationFrame !== null) return;
    this.animationFrame = requestAnimationFrame((time) => this.animate(time));
  }

  animate(time) {
    this.animationFrame = null;
    const dt = Math.min(Math.max((time - this.lastTime) / 1000, 1 / 120), 1 / 24);
    this.lastTime = time;
    if (!this.isMatrix()) {
      this.frame = JSON.parse(this.physics.tick(dt));
      this.indexFrame();
    }
    this.render();
    if (!this.isMatrix() && (!this.frame.at_rest || this.backdrop === "field")) {
      this.schedule();
    }
  }

  render() {
    if (this.isMatrix()) return;
    this.drawCanvas();
    for (const node of this.authority.nodes) {
      const position = this.frameById.get(node.id);
      const button = this.nodeButtons.get(node.id);
      if (!position) {
        button.hidden = true;
        continue;
      }
      button.hidden = false;
      const point = this.screenPoint(position);
      button.style.transform = `translate(${point.x}px, ${point.y}px) translate(-50%, -50%)`;
      button.classList.toggle("is-pinned", this.currentPins().has(node.id));
    }
  }

  drawCanvas() {
    const context = this.context;
    const rect = this.stage.getBoundingClientRect();
    context.clearRect(0, 0, rect.width, rect.height);
    this.drawBackdrop(context, rect);
    if (this.readingProfile().emphasis === "activity") this.drawActivityRail(context, rect);
    this.drawEdges(context);
    this.drawProps(context);
  }

  drawBackdrop(context, rect) {
    if (this.backdrop === "ambient") {
      const gradient = context.createRadialGradient(
        rect.width * 0.46,
        rect.height * 0.42,
        20,
        rect.width * 0.5,
        rect.height * 0.5,
        rect.width * 0.58,
      );
      gradient.addColorStop(0, "rgba(191, 145, 88, 0.1)");
      gradient.addColorStop(1, "rgba(58, 50, 39, 0)");
      context.fillStyle = gradient;
      context.fillRect(0, 0, rect.width, rect.height);
    }
    if (this.backdrop === "field") {
      const center = this.screenPoint({ x: 0, y: 0 });
      context.save();
      context.strokeStyle = "rgba(171, 112, 66, 0.2)";
      context.setLineDash([4, 9]);
      for (const radius of [80, 145, 215]) {
        context.beginPath();
        context.arc(center.x, center.y, radius * this.scale, 0, Math.PI * 2);
        context.stroke();
      }
      context.restore();
    }
  }

  drawEdges(context) {
    for (const edge of this.authority.edges) {
      const source = this.frameById.get(edge.source);
      const target = this.frameById.get(edge.target);
      if (!source || !target) continue;
      const a = this.screenPoint(source);
      const b = this.screenPoint(target);
      const selected = edge.source === this.selectedId || edge.target === this.selectedId;
      const changed =
        this.node(edge.source)?.change !== "stable" ||
        this.node(edge.target)?.change !== "stable";
      context.save();
      context.lineWidth = selected ? 1.9 : 0.9;
      context.strokeStyle = selected
        ? "rgba(89, 60, 37, 0.9)"
        : this.readingProfile().emphasis === "change" && changed
          ? "rgba(156, 76, 45, 0.5)"
          : "rgba(77, 67, 53, 0.22)";
      if (edge.provenance === "derived") context.setLineDash([4, 5]);
      context.beginPath();
      context.moveTo(a.x, a.y);
      context.lineTo(b.x, b.y);
      context.stroke();
      context.restore();
    }
  }

  drawActivityRail(context, rect) {
    context.save();
    context.strokeStyle = "rgba(83, 69, 48, 0.26)";
    context.lineWidth = 1;
    context.beginPath();
    context.moveTo(rect.width * 0.08, rect.height * 0.5);
    context.lineTo(rect.width * 0.92, rect.height * 0.5);
    context.stroke();
    context.restore();
  }

  drawProps(context) {
    if (!this.frame?.props) return;
    context.save();
    context.fillStyle = "rgba(155, 102, 62, 0.12)";
    context.strokeStyle = "rgba(101, 71, 45, 0.38)";
    context.lineWidth = 1;
    for (const prop of this.frame.props) {
      const point = this.screenPoint(prop);
      context.save();
      context.translate(point.x, point.y);
      context.rotate(prop.rotation);
      context.beginPath();
      if (prop.shape === "ball") {
        context.arc(0, 0, prop.radius * this.scale, 0, Math.PI * 2);
      } else if (prop.shape === "square") {
        const half = prop.half * this.scale;
        context.rect(-half, -half, half * 2, half * 2);
      } else if (prop.points.length) {
        prop.points.forEach(([x, y], index) => {
          const px = x * this.scale;
          const py = y * this.scale;
          if (index === 0) context.moveTo(px, py);
          else context.lineTo(px, py);
        });
        context.closePath();
      }
      context.fill();
      context.stroke();
      context.restore();
    }
    context.restore();
  }
}

function nodeSignature(node) {
  return [node.name, node.class, node.status, node.pushed_at].join("\u0000");
}

function behaviorText(profile) {
  return profile.behaviors
    .map(({ gesture, behavior }) => `${humanize(gesture)}: ${humanize(behavior)}`)
    .join(" · ");
}

function encodeSceneState(state) {
  const bytes = new TextEncoder().encode(JSON.stringify(state));
  let binary = "";
  for (const byte of bytes) binary += String.fromCharCode(byte);
  return btoa(binary).replaceAll("+", "-").replaceAll("/", "_").replaceAll("=", "");
}

function decodeSceneState(hash) {
  const match = hash.match(/^#graphshell-scene=([A-Za-z0-9_-]+)$/);
  if (!match) return null;
  try {
    const base64 = match[1].replaceAll("-", "+").replaceAll("_", "/");
    const padded = base64.padEnd(Math.ceil(base64.length / 4) * 4, "=");
    const binary = atob(padded);
    const bytes = Uint8Array.from(binary, (character) => character.charCodeAt(0));
    return JSON.parse(new TextDecoder().decode(bytes));
  } catch {
    return null;
  }
}

function shortName(value) {
  const words = value.trim().split(/\s+/);
  if (words.length > 1) {
    return words
      .slice(0, 2)
      .map((word) => word[0])
      .join("")
      .toUpperCase();
  }
  return value.slice(0, 2).toUpperCase();
}

function compactDate(value) {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return "--/--";
  return `${String(date.getUTCMonth() + 1).padStart(2, "0")}/${String(
    date.getUTCDate(),
  ).padStart(2, "0")}`;
}

function safeToken(value) {
  return String(value).toLowerCase().replace(/[^a-z0-9_-]/g, "-");
}

function humanize(value) {
  return String(value).replaceAll("_", " ");
}

function clamp(value, minimum, maximum) {
  return Math.min(Math.max(value, minimum), maximum);
}

function announce(sandboxRoot, message) {
  const status = sandboxRoot.querySelector("[data-sandbox-status]");
  if (status) status.textContent = message;
}
