const runtimeVersion = new URL(import.meta.url).search;
const {
  default: initWasm,
  layout_graph: layoutGraph,
  GraphPhysics,
} = await import(`./mer3ly_repo_graph.js${runtimeVersion}`);

const root = document.querySelector("[data-graph-sandbox]");
if (root) {
  startSandbox(root).catch((error) => {
    const fallback = root.querySelector("[data-sandbox-fallback]");
    if (fallback) {
      fallback.textContent =
        "The graph sandbox could not initialize. The repository map and semantic index remain available.";
    }
    root.dataset.sandboxState = "unavailable";
    console.warn("Mer3ly graph sandbox unavailable:", error);
  });
}

async function startSandbox(sandboxRoot) {
  const dataElement = document.querySelector("#graph-sandbox-data");
  if (!dataElement) throw new Error("graph sandbox data is absent");
  const authority = JSON.parse(dataElement.textContent);
  validateAuthority(authority);

  await initWasm({
    module_or_path: new URL(
      `./mer3ly_repo_graph_bg.wasm${runtimeVersion}`,
      import.meta.url,
    ),
  });
  const layout = JSON.parse(layoutGraph(JSON.stringify(authority)));
  const physics = new GraphPhysics(JSON.stringify(authority));
  const sandbox = new GraphSandbox(sandboxRoot, authority, layout, physics);
  sandbox.start();

  sandboxRoot.dataset.sandboxState = "ready";
  sandboxRoot.querySelector("[data-sandbox-fallback]").hidden = true;
  sandboxRoot.querySelector("[data-sandbox-interface]").hidden = false;
  announce(
    sandboxRoot,
    `${authority.nodes.length} heterogeneous actors and ${authority.edges.length} typed relations loaded into the Graphshell sandbox.`,
  );
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

class GraphSandbox {
  constructor(sandboxRoot, authority, layout, physics) {
    this.root = sandboxRoot;
    this.authority = authority;
    this.layout = layout;
    this.physics = physics;
    this.stage = sandboxRoot.querySelector("[data-sandbox-stage]");
    this.canvas = sandboxRoot.querySelector("[data-sandbox-canvas]");
    this.context = this.canvas.getContext("2d");
    this.nodeLayer = sandboxRoot.querySelector("[data-sandbox-nodes]");
    this.matrix = sandboxRoot.querySelector("[data-sandbox-matrix]");
    this.caption = sandboxRoot.querySelector("[data-sandbox-caption]");
    this.controls = new Map(
      [...sandboxRoot.querySelectorAll("[data-sandbox-control]")].map((control) => [
        control.dataset.sandboxControl,
        control,
      ]),
    );
    this.tangibleControl = sandboxRoot.querySelector("[data-sandbox-tangible]");
    this.pinControl = sandboxRoot.querySelector("[data-sandbox-pin]");
    this.arrangements = new Map(
      layout.arrangements.map((arrangement) => [arrangement.id, arrangement]),
    );
    this.currentArrangement = this.arrangements.has("graph_layout:stack")
      ? "graph_layout:stack"
      : layout.default_arrangement;
    this.scene = "graph";
    this.mobility = "anchored";
    this.backdrop = "ambient";
    this.physicsMode = "live";
    this.frame = null;
    this.frameById = new Map();
    this.nodeButtons = new Map();
    this.selectedId = authority.focus ?? authority.nodes[0].id;
    this.lastTime = performance.now();
    this.settleFrames = 0;
    this.animationFrame = null;
    this.drag = null;
    this.scale = 1;
  }

  start() {
    this.stage.dataset.sandboxMobility = this.mobility;
    this.installArrangementOptions();
    this.installNodes();
    this.installControls();
    this.buildMatrix();
    this.physics.setBackdrop(this.backdrop, false);
    this.applyArrangement();
    this.frame = JSON.parse(this.physics.frame());
    this.indexFrame();
    this.select(this.selectedId, false);
    this.resize();
    this.updateCaption();
    this.render();
    this.resizeObserver = new ResizeObserver(() => {
      this.resize();
      this.schedule();
    });
    this.resizeObserver.observe(this.stage);
    this.schedule();
  }

  installArrangementOptions() {
    const picker = this.controls.get("arrangement");
    picker.replaceChildren();
    for (const arrangement of this.layout.arrangements) {
      const option = document.createElement("option");
      option.value = arrangement.id;
      option.textContent =
        arrangement.id === "graph_layout:radial" ? "Neighborhood" : arrangement.name;
      option.title = arrangement.description;
      picker.append(option);
    }
    picker.value = this.currentArrangement;
  }

  installControls() {
    for (const [name, control] of this.controls) {
      control.addEventListener("change", () => this.changeControl(name, control.value));
    }
    this.tangibleControl.addEventListener("change", () => {
      this.physics.setBackdrop(this.backdrop, this.tangibleControl.checked);
      this.physicsMode = "live";
      this.controls.get("physics").value = "live";
      this.updateCaption();
      this.schedule();
    });
    this.pinControl.addEventListener("click", () => this.togglePin(this.selectedId));
  }

  changeControl(name, value) {
    if (name === "scene") {
      this.setScene(value);
      return;
    }
    if (name === "arrangement") {
      this.currentArrangement = value;
      this.applyArrangement();
      this.updateCaption();
      this.schedule();
      return;
    }
    if (name === "mobility") {
      this.mobility = value;
      this.stage.dataset.sandboxMobility = value;
      this.applyArrangement();
      this.updateInspector();
      this.updateCaption();
      this.schedule();
      return;
    }
    if (name === "backdrop") {
      this.backdrop = value;
      this.stage.dataset.sandboxBackdrop = value;
      const supportsCollision = value === "props" || value === "field";
      this.tangibleControl.disabled = !supportsCollision;
      if (!supportsCollision) this.tangibleControl.checked = false;
      this.physics.setBackdrop(value, this.tangibleControl.checked);
      this.updateCaption();
      this.schedule();
      return;
    }
    if (name === "physics") {
      this.physicsMode = value;
      this.settleFrames = 0;
      this.updateCaption();
      this.schedule();
    }
  }

  setScene(scene) {
    this.scene = scene;
    this.stage.dataset.sandboxScene = scene;
    const matrix = scene === "matrix";
    this.canvas.hidden = matrix;
    this.nodeLayer.hidden = matrix;
    this.matrix.hidden = !matrix;
    if (scene === "activity") {
      this.currentArrangement = "graph_layout:timeline";
      this.controls.get("arrangement").value = this.currentArrangement;
      this.applyArrangement();
    }
    this.controls.get("arrangement").disabled = matrix || scene === "activity";
    this.controls.get("mobility").disabled = matrix;
    this.controls.get("backdrop").disabled = matrix;
    this.controls.get("physics").disabled = matrix;
    this.tangibleControl.disabled =
      matrix || !(this.backdrop === "props" || this.backdrop === "field");
    this.updateNodeScenes();
    this.updateMatrixSelection();
    this.updateCaption();
    this.schedule();
  }

  applyArrangement() {
    const arrangement = this.arrangements.get(this.currentArrangement);
    if (!arrangement) return;
    this.physics.setArrangement(JSON.stringify(arrangement.nodes), this.mobility);
    this.frame = JSON.parse(this.physics.frame());
    this.indexFrame();
  }

  recomputeNeighborhood(focus) {
    this.authority.focus = focus;
    this.layout = JSON.parse(layoutGraph(JSON.stringify(this.authority)));
    this.arrangements = new Map(
      this.layout.arrangements.map((arrangement) => [arrangement.id, arrangement]),
    );
    if (this.currentArrangement === "graph_layout:radial") {
      this.applyArrangement();
    }
  }

  installNodes() {
    this.nodeLayer.replaceChildren();
    for (const node of this.authority.nodes) {
      const button = document.createElement("button");
      button.type = "button";
      button.className = `graph-sandbox-node class-${safeToken(node.class)} change-${safeToken(node.change)}`;
      button.dataset.sandboxNode = node.id;
      button.dataset.change = node.change;
      button.setAttribute("aria-label", `${node.name}, ${node.class}, ${node.status}`);
      const mark = document.createElement("span");
      mark.className = "graph-sandbox-node-mark";
      mark.setAttribute("aria-hidden", "true");
      mark.textContent = shortName(node.name);
      const label = document.createElement("span");
      label.className = "graph-sandbox-node-label";
      label.textContent = node.name;
      const badge = document.createElement("span");
      badge.className = "graph-sandbox-node-change";
      badge.textContent = node.change;
      button.append(mark, label, badge);
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

  updateNodeScenes() {
    for (const node of this.authority.nodes) {
      const button = this.nodeButtons.get(node.id);
      button.classList.toggle("is-change-muted", this.scene === "changes" && node.change === "stable");
      button.classList.toggle("is-activity", this.scene === "activity");
    }
  }

  startDrag(event, id, button) {
    if (event.button !== 0 || this.scene === "matrix") return;
    this.select(id, false);
    button.setPointerCapture(event.pointerId);
    this.drag = {
      id,
      pointerId: event.pointerId,
      button,
      startX: event.clientX,
      startY: event.clientY,
      moved: false,
    };
    const move = (moveEvent) => this.dragNode(moveEvent);
    const finish = (finishEvent) => {
      if (!this.drag || finishEvent.pointerId !== this.drag.pointerId) return;
      if (this.drag.moved) this.drag.button.dataset.dragged = "true";
      this.drag.button.removeEventListener("pointermove", move);
      this.drag.button.removeEventListener("pointerup", finish);
      this.drag.button.removeEventListener("pointercancel", finish);
      this.drag = null;
      this.updateInspector();
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
    this.selectedId = id;
    for (const [nodeId, button] of this.nodeButtons) {
      const selected = nodeId === id;
      button.classList.toggle("is-selected", selected);
      button.setAttribute("aria-pressed", String(selected));
    }
    if (this.currentArrangement === "graph_layout:radial") {
      this.recomputeNeighborhood(id);
    }
    this.updateInspector();
    this.updateMatrixSelection();
    if (speak) {
      const node = this.node(id);
      announce(this.root, `${node.name} selected. ${node.summary}`);
    }
    this.schedule();
  }

  togglePin(id) {
    if (!id || this.mobility === "frozen") return;
    const point = this.frameById.get(id);
    if (!point) return;
    if (this.physics.isPinned(id)) {
      this.physics.unpinNode(id);
    } else {
      this.physics.pinNode(id, point.x, point.y);
    }
    this.frame = JSON.parse(this.physics.frame());
    this.indexFrame();
    this.updateInspector();
    this.schedule();
  }

  node(id) {
    return this.authority.nodes.find((node) => node.id === id);
  }

  updateInspector() {
    const node = this.node(this.selectedId);
    if (!node) return;
    this.root.querySelector("[data-sandbox-inspector-title]").textContent = node.name;
    this.root.querySelector("[data-sandbox-inspector-summary]").textContent = node.summary;
    this.root.querySelector("[data-sandbox-primitive]").textContent = primitiveName(node.class);
    this.root.querySelector("[data-sandbox-script]").textContent = node.script;
    const pinned = this.physics.isPinned(node.id);
    this.root.querySelector("[data-sandbox-node-motion]").textContent = pinned
      ? this.mobility === "frozen"
        ? "frozen by scene"
        : "pinned by user"
      : this.mobility;
    this.pinControl.disabled = this.mobility === "frozen";
    this.pinControl.textContent =
      this.mobility === "frozen" ? "scene is frozen" : pinned ? "unpin selected" : "pin selected";
  }

  updateCaption() {
    const arrangement = this.arrangements.get(this.currentArrangement);
    const sceneText = {
      graph: "Graph shows actors and typed relations",
      changes: "Changes styles added, updated, stable, and removed actors",
      activity: "Activity binds the same actors to their source times",
      matrix: "Matrix replaces position with an exact relation lookup",
    }[this.scene];
    const arrangementName =
      this.currentArrangement === "graph_layout:radial"
        ? "Neighborhood"
        : arrangement?.name ?? "none";
    const collision = this.tangibleControl.checked ? "collidable" : "intangible";
    this.caption.textContent = `${sceneText}. ${arrangementName} slots; ${this.mobility} motion; ${this.backdrop} backdrop (${collision}); physics ${this.physicsMode}.`;
  }

  buildMatrix() {
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

  resize() {
    const rect = this.stage.getBoundingClientRect();
    const ratio = Math.min(window.devicePixelRatio || 1, 2);
    this.canvas.width = Math.max(1, Math.round(rect.width * ratio));
    this.canvas.height = Math.max(1, Math.round(rect.height * ratio));
    this.canvas.style.width = `${rect.width}px`;
    this.canvas.style.height = `${rect.height}px`;
    this.context.setTransform(ratio, 0, 0, ratio, 0, 0);
    this.scale = Math.min(rect.width / 820, rect.height / 640).toFixed(4);
    this.scale = Number(this.scale);
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
    if (this.scene !== "matrix" && this.physicsMode !== "paused") {
      const steps = this.physicsMode === "settle" ? 3 : 1;
      for (let step = 0; step < steps; step += 1) {
        this.frame = JSON.parse(this.physics.tick(dt / steps));
      }
      this.indexFrame();
      if (this.physicsMode === "settle") {
        this.settleFrames += 1;
        if (this.frame.at_rest && this.settleFrames > 18) {
          this.physicsMode = "paused";
          this.controls.get("physics").value = "paused";
          this.updateCaption();
        }
      }
    }
    this.render();
    if (this.scene !== "matrix" && this.physicsMode !== "paused") this.schedule();
  }

  render() {
    if (this.scene === "matrix") return;
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
      button.classList.toggle("is-pinned", position.pinned);
    }
  }

  drawCanvas() {
    const context = this.context;
    const rect = this.stage.getBoundingClientRect();
    context.clearRect(0, 0, rect.width, rect.height);
    this.drawBackdrop(context, rect);
    if (this.scene === "activity") this.drawActivityRail(context, rect);
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
        this.node(edge.source).change !== "stable" || this.node(edge.target).change !== "stable";
      context.save();
      context.lineWidth = selected ? 1.9 : 0.9;
      context.strokeStyle = selected
        ? "rgba(89, 60, 37, 0.9)"
        : this.scene === "changes" && changed
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

function primitiveName(className) {
  if (["document", "page", "note"].includes(className)) return "square document face + collider";
  if (className === "event") return "diamond event hull + collider";
  if (["device", "tool"].includes(className)) return "rounded device body + collider";
  if (["place", "person", "community"].includes(className)) return "circular actor + collider";
  return "circular software actor + collider";
}

function shortName(value) {
  const words = value.trim().split(/\s+/);
  if (words.length > 1) return words.slice(0, 2).map((word) => word[0]).join("").toUpperCase();
  return value.slice(0, 2).toUpperCase();
}

function safeToken(value) {
  return String(value).toLowerCase().replace(/[^a-z0-9_-]/g, "-");
}

function humanize(value) {
  return value.replaceAll("_", " ");
}

function announce(sandboxRoot, message) {
  const status = sandboxRoot.querySelector("[data-sandbox-status]");
  if (status) status.textContent = message;
}
