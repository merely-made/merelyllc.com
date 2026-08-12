const runtimeVersion = new URL(import.meta.url).search;
const { default: initWasm, layout_graph: layoutGraph } = await import(
  `./mer3ly_repo_graph.js${runtimeVersion}`
);

class GraphUnavailable extends Error {}

const MORPH_DURATION_MS = 640;
const TIMELINE_ARRANGEMENT = "graph_layout:timeline";
const SHARED_SCENE_KEY = "repository-scene";
const SHARED_SCENE_VERSION = "v1";
const SCENE_PROFILES = new Map([
  [
    "graph_layout:radial",
    {
      form: "medallion",
      scaffold: "orbits",
      edgeOpacity: 1,
      selectedEdgeOpacity: 1,
      canvasNodes: true,
      caption: "Constellation medallions · relationships remain fully drawn",
    },
  ],
  [
    "graph_layout:stack",
    {
      form: "tile",
      scaffold: "index",
      edgeOpacity: 0.42,
      selectedEdgeOpacity: 0.9,
      canvasNodes: false,
      caption: "Directed layers · topology supplies slots while relationships stay explicit",
    },
  ],
  [
    "graph_layout:grid",
    {
      form: "tile",
      scaffold: "index",
      edgeOpacity: 0.14,
      selectedEdgeOpacity: 0.72,
      canvasNodes: false,
      caption: "Index tiles · relationships surface around the selected repository",
    },
  ],
  [
    "graph_layout:phyllotaxis",
    {
      form: "seed",
      scaffold: "field",
      edgeOpacity: 0.34,
      selectedEdgeOpacity: 0.86,
      canvasNodes: false,
      caption: "Seeds in a phyllotactic field · labels open on attention",
    },
  ],
  [
    "graph_layout:timeline",
    {
      form: "flag",
      scaffold: "timeline",
      edgeOpacity: 0.08,
      selectedEdgeOpacity: 0.74,
      canvasNodes: false,
      caption: "Repository strips · exact push times anchored to one chronological rail",
    },
  ],
  [
    "graph_layout:kanban",
    {
      form: "card",
      scaffold: "lanes",
      edgeOpacity: 0.05,
      selectedEdgeOpacity: 0.68,
      canvasNodes: false,
      caption: "Repository cards · lanes follow public project status",
    },
  ],
  [
    "graph_layout:penrose",
    {
      form: "facet",
      scaffold: "tessellation",
      edgeOpacity: 0.16,
      selectedEdgeOpacity: 0.76,
      canvasNodes: false,
      caption: "Faceted repositories · the field reads as a tessellation",
    },
  ],
  [
    "graph_layout:lsystem",
    {
      form: "leaf",
      scaffold: "branches",
      edgeOpacity: 0.1,
      selectedEdgeOpacity: 0.78,
      canvasNodes: false,
      caption: "Leaves on a generated path · semantic links remain available on focus",
    },
  ],
]);
const root = document.querySelector("[data-repository-graph]");

if (root) {
  startRepositoryGraph(root).catch((error) => {
    showFallback(
      root,
      error instanceof GraphUnavailable
        ? error.message
        : "The interactive map could not initialize. The complete repository index remains available below.",
    );
    console.warn("Mer3ly repository graph unavailable:", error);
  });
}

async function startRepositoryGraph(graphRoot) {
  const forcedMode = new URLSearchParams(window.location.search).get("graph");
  if (forcedMode === "no-webgpu") {
    throw new GraphUnavailable(
      "WebGPU is unavailable in this browser. The complete repository index remains available below.",
    );
  }
  if (!navigator.gpu) {
    throw new GraphUnavailable(
      "WebGPU is unavailable in this browser. The complete repository index remains available below.",
    );
  }

  const authorityElement = document.querySelector("#repository-graph-data");
  if (!authorityElement) {
    throw new GraphUnavailable(
      "The interactive map has no public graph data. The complete repository index remains available below.",
    );
  }
  const authority = JSON.parse(authorityElement.textContent);

  if (forcedMode === "no-wasm") {
    throw new GraphUnavailable(
      "WebAssembly is unavailable in this browser. The complete repository index remains available below.",
    );
  }
  await initWasm({
    module_or_path: new URL(
      `./mer3ly_repo_graph_bg.wasm${runtimeVersion}`,
      import.meta.url,
    ),
  });
  const layout = JSON.parse(layoutGraph(JSON.stringify(authority)));
  validateProjection(authority, layout);
  const history = validateHistory(authority);

  if (forcedMode === "init-failure") {
    throw new GraphUnavailable(
      "The interactive map could not initialize. The complete repository index remains available below.",
    );
  }

  const renderer = await RepositoryGraphRenderer.create(graphRoot, layout);
  const sceneMessage = installGraphControls(graphRoot, renderer, layout, authority, history);
  renderer.fit();
  renderer.schedule();

  graphRoot.dataset.graphState = "ready";
  graphRoot.querySelector("[data-graph-fallback]").hidden = true;
  graphRoot.querySelector("[data-graph-interface]").hidden = false;
  announce(
    graphRoot,
    sceneMessage ??
      `${layout.nodes.length} repositories and ${layout.edges.length} relationships arranged by Mere. Select a repository node to inspect it.`,
  );
}

function validateProjection(authority, layout) {
  validateAuthorityGraph(authority);
  if (layout.schema !== "mer3ly.repo-graph-layout/v2") {
    throw new Error("repository graph schema mismatch");
  }
  if (layout.authority_schema !== authority.schema) {
    throw new Error("repository graph authority mismatch");
  }
  const authorityNodes = authority.nodes.map(({ id }) => id).sort();
  const layoutNodes = layout.nodes.map(({ id }) => id).sort();
  const authorityEdges = authority.edges.map(({ id }) => id).sort();
  const layoutEdges = layout.edges.map(({ id }) => id).sort();
  if (
    JSON.stringify(authorityNodes) !== JSON.stringify(layoutNodes) ||
    JSON.stringify(authorityEdges) !== JSON.stringify(layoutEdges)
  ) {
    throw new Error("repository graph projection lost authority records");
  }
  if (
    !Array.isArray(layout.arrangements) ||
    !layout.arrangements.some(({ id }) => id === layout.default_arrangement)
  ) {
    throw new Error("repository graph has no default arrangement");
  }
  for (const arrangement of layout.arrangements) {
    if (!SCENE_PROFILES.has(arrangement.id)) {
      throw new Error(`repository arrangement ${arrangement.id} has no scene profile`);
    }
    const arrangementNodes = arrangement.nodes.map(({ id }) => id).sort();
    if (JSON.stringify(authorityNodes) !== JSON.stringify(arrangementNodes)) {
      throw new Error(`repository arrangement ${arrangement.id} lost nodes`);
    }
    if (
      arrangement.nodes.some(
        ({ x, y }) => !Number.isFinite(x) || !Number.isFinite(y),
      )
    ) {
      throw new Error(`repository arrangement ${arrangement.id} has invalid positions`);
    }
  }
}

function validateAuthorityGraph(graph) {
  if (
    graph.schema !== "mer3ly.repo-graph/v1" ||
    !Array.isArray(graph.nodes) ||
    !Array.isArray(graph.edges)
  ) {
    throw new Error("repository graph schema mismatch");
  }
  const nodeIds = new Set(graph.nodes.map(({ id }) => id));
  if (
    nodeIds.size !== graph.nodes.length ||
    graph.nodes.some(({ id, pushed_at }) =>
      typeof id !== "string" || typeof pushed_at !== "string",
    ) ||
    graph.edges.some(
      ({ id, source, target }) =>
        typeof id !== "string" ||
        !nodeIds.has(source) ||
        !nodeIds.has(target),
    )
  ) {
    throw new Error("repository graph contains invalid source records");
  }
}

function validateHistory(authority) {
  if (authority.history === undefined) {
    return null;
  }
  const history = authority.history;
  if (
    history.schema !== "mer3ly.repository-git-history/v1" ||
    !Array.isArray(history.checkpoints)
  ) {
    throw new Error("repository graph history schema mismatch");
  }
  for (const checkpoint of history.checkpoints) {
    if (
      !checkpoint.cursor ||
      typeof checkpoint.cursor.source !== "string" ||
      typeof checkpoint.cursor.commit !== "string" ||
      typeof checkpoint.cursor.committed_at !== "string"
    ) {
      throw new Error("repository graph history has an invalid cursor");
    }
    if (checkpoint.availability === "available") {
      validateAuthorityGraph(checkpoint.graph);
    } else if (
      checkpoint.availability !== "unavailable" ||
      typeof checkpoint.reason !== "string"
    ) {
      throw new Error("repository graph history has an invalid checkpoint");
    }
  }
  return history;
}

function showFallback(graphRoot, message) {
  graphRoot.dataset.graphState = "unavailable";
  const fallback = graphRoot.querySelector("[data-graph-fallback]");
  const graphInterface = graphRoot.querySelector("[data-graph-interface]");
  if (fallback) {
    fallback.hidden = false;
    fallback.textContent = message;
  }
  if (graphInterface) {
    graphInterface.hidden = true;
  }
  announce(graphRoot, message);
}

function announce(graphRoot, message) {
  const status = graphRoot.querySelector("[data-graph-status]");
  if (status) {
    status.textContent = message;
  }
}

class RepositoryGraphRenderer {
  static async create(graphRoot, layout) {
    const canvas = graphRoot.querySelector("canvas");
    const stage = graphRoot.querySelector("[data-graph-stage]");
    const nodeLayer = graphRoot.querySelector("[data-graph-nodes]");
    const adapter = await navigator.gpu.requestAdapter({
      powerPreference: "low-power",
    });
    if (!adapter) {
      throw new GraphUnavailable(
        "WebGPU could not provide a graphics adapter. The complete repository index remains available below.",
      );
    }
    const device = await adapter.requestDevice();
    const context = canvas.getContext("webgpu");
    if (!context) {
      throw new GraphUnavailable(
        "WebGPU could not create a canvas. The complete repository index remains available below.",
      );
    }
    return new RepositoryGraphRenderer(
      graphRoot,
      stage,
      canvas,
      nodeLayer,
      device,
      context,
      layout,
    );
  }

  constructor(graphRoot, stage, canvas, nodeLayer, device, context, layout) {
    this.graphRoot = graphRoot;
    this.stage = stage;
    this.canvas = canvas;
    this.nodeLayer = nodeLayer;
    this.device = device;
    this.context = context;
    this.layout = layout;
    this.arrangements = new Map(
      layout.arrangements.map((arrangement) => [arrangement.id, arrangement]),
    );
    this.currentArrangement = layout.default_arrangement;
    this.sceneProfile = SCENE_PROFILES.get(this.currentArrangement);
    this.sceneLayer = document.createElement("div");
    this.sceneLayer.className = "repository-graph-scene";
    this.sceneLayer.dataset.graphScene = "";
    this.sceneLayer.setAttribute("aria-hidden", "true");
    this.stage.prepend(this.sceneLayer);
    this.scaffoldItems = [];
    this.positions = new Map(
      this.arrangements
        .get(this.currentArrangement)
        .nodes.map(({ id, x, y }) => [id, { x, y }]),
    );
    this.morph = null;
    this.reducedMotion = prefersReducedMotion();
    this.format = navigator.gpu.getPreferredCanvasFormat();
    this.scale = 1;
    this.panX = 0;
    this.panY = 0;
    this.selectedId = layout.focus;
    this.frame = null;
    this.userAdjusted = false;
    this.edgeBuffer = null;
    this.nodeBuffer = null;
    this.edgePipeline = this.createEdgePipeline();
    this.nodePipeline = this.createNodePipeline();

    this.context.configure({
      device: this.device,
      format: this.format,
      alphaMode: "premultiplied",
    });
    this.installNodeButtons();
    this.applySceneProfile(this.currentArrangement);
    this.installPointerControls();
    this.resizeObserver = new ResizeObserver(() => {
      if (!this.userAdjusted) {
        this.fit();
      }
      this.schedule();
    });
    this.resizeObserver.observe(this.stage);
    document.addEventListener("visibilitychange", () => {
      if (!document.hidden) {
        this.schedule();
      }
    });
    this.device.lost.then(() => {
      showFallback(
        this.graphRoot,
        "The WebGPU device was lost. The complete repository index remains available below.",
      );
    });
  }

  replaceLayout(layout) {
    if (this.morph) {
      this.advanceMorph(performance.now());
    }
    const previousArrangement = this.currentArrangement;
    const previousSelection = this.selectedId;
    const nextArrangement =
      layout.arrangements.some(({ id }) => id === previousArrangement)
        ? previousArrangement
        : layout.default_arrangement;
    this.alignMobileOrientation(previousArrangement, nextArrangement);
    const previousPositions = new Map(this.positions);
    this.layout = layout;
    this.arrangements = new Map(
      layout.arrangements.map((arrangement) => [arrangement.id, arrangement]),
    );
    this.currentArrangement = nextArrangement;
    this.selectedId = layout.nodes.some(({ id }) => id === previousSelection)
      ? previousSelection
      : layout.focus;
    this.graphRoot.dataset.graphArrangement = nextArrangement;
    this.graphRoot.dataset.graphEngine = this.arrangements.get(nextArrangement).engine;
    this.nodeLayer.replaceChildren();
    this.installNodeButtons();
    this.applySceneProfile(nextArrangement);

    const arrangement = this.arrangements.get(nextArrangement);
    const target = new Map(
      arrangement.nodes.map(({ id, x, y }) => [id, { x, y }]),
    );
    this.positions = new Map(
      [...target].map(([id, position]) => [
        id,
        previousPositions.get(id) ?? position,
      ]),
    );
    if (this.reducedMotion) {
      this.positions = target;
      this.morph = null;
      this.fit();
      this.graphRoot.dataset.graphMorphing = "false";
      return;
    }
    this.morph = {
      arrangement,
      from: new Map(this.positions),
      target,
      startedAt: performance.now(),
      announcement: "",
    };
    this.graphRoot.dataset.graphMorphing = "true";
    this.schedule();
  }

  applySceneProfile(arrangementId) {
    const profile = SCENE_PROFILES.get(arrangementId);
    if (!profile) {
      throw new Error(`repository arrangement ${arrangementId} has no scene profile`);
    }
    this.sceneProfile = profile;
    this.graphRoot.dataset.graphNodeForm = profile.form;
    this.graphRoot.dataset.graphScaffold = profile.scaffold;
    this.sceneLayer.dataset.graphScene = profile.scaffold;
    const caption = this.graphRoot.querySelector("[data-graph-scene-caption]");
    if (caption) {
      caption.textContent = profile.caption;
    }
    this.rebuildSceneScaffold();
  }

  rebuildSceneScaffold() {
    this.sceneLayer.replaceChildren();
    this.scaffoldItems = [];
    const scaffold = this.sceneProfile.scaffold;
    if (scaffold === "orbits") {
      for (const scale of [0.38, 0.68, 1]) {
        const element = sceneElement("repository-graph-orbit");
        this.sceneLayer.append(element);
        this.scaffoldItems.push({ element, scale });
      }
      return;
    }
    if (scaffold === "timeline") {
      const datedNodes = [...this.layout.nodes].sort(
        (left, right) =>
          left.pushed_at.localeCompare(right.pushed_at) || left.id.localeCompare(right.id),
      );
      const firstDate = datedNodes[0].pushed_at.slice(0, 10);
      const lastDate = datedNodes.at(-1).pushed_at.slice(0, 10);
      const rail = sceneElement("repository-graph-timeline-rail");
      const label = sceneElement(
        "repository-graph-scene-label",
        formatGraphDateRange(firstDate, lastDate),
      );
      rail.append(label);
      this.sceneLayer.append(rail);
      this.scaffoldItems.push({
        kind: "timeline-rail",
        element: rail,
        nodeIds: datedNodes.map(({ id }) => id),
      });
      for (const node of datedNodes) {
        const stem = sceneElement("repository-graph-timeline-stem");
        const anchor = sceneElement("repository-graph-timeline-anchor");
        this.sceneLayer.append(stem, anchor);
        this.scaffoldItems.push({
          kind: "timeline-anchor",
          element: stem,
          anchor,
          nodeId: node.id,
        });
      }
      return;
    }
    if (scaffold === "lanes") {
      const statuses = new Map();
      for (const node of this.layout.nodes) {
        if (!statuses.has(node.status)) statuses.set(node.status, []);
        statuses.get(node.status).push(node.id);
      }
      for (const [status, nodeIds] of statuses) {
        const label = sceneElement("repository-graph-scene-label", humanize(status));
        const element = sceneElement("repository-graph-kanban-lane");
        element.dataset.status = status;
        element.append(label);
        this.sceneLayer.append(element);
        this.scaffoldItems.push({ element, nodeIds });
      }
      return;
    }
    if (scaffold === "tessellation") {
      for (const node of this.layout.nodes) {
        const element = sceneElement("repository-graph-facet-cell");
        this.sceneLayer.append(element);
        this.scaffoldItems.push({ element, nodeId: node.id });
      }
      return;
    }
    if (scaffold === "branches") {
      for (let index = 1; index < this.layout.nodes.length; index += 1) {
        const element = sceneElement("repository-graph-branch");
        this.sceneLayer.append(element);
        this.scaffoldItems.push({
          element,
          sourceId: this.layout.nodes[index - 1].id,
          targetId: this.layout.nodes[index].id,
        });
      }
    }
  }

  updateSceneScaffold() {
    const scaffold = this.sceneProfile.scaffold;
    const nodeById = new Map(this.layout.nodes.map((node) => [node.id, node]));
    const screenFor = (id) => {
      const node = nodeById.get(id);
      return node ? this.screenPosition(node) : null;
    };
    if (scaffold === "orbits") {
      const center = screenFor(this.layout.focus);
      if (!center) return;
      const radius = Math.max(
        ...this.layout.nodes.map((node) => {
          const point = this.screenPosition(node);
          return Math.hypot(point.x - center.x, point.y - center.y);
        }),
        1,
      );
      for (const item of this.scaffoldItems) {
        const diameter = radius * item.scale * 2;
        positionBox(item.element, {
          left: center.x - diameter * 0.5,
          top: center.y - diameter * 0.5,
          width: diameter,
          height: diameter,
        });
      }
      return;
    }
    if (scaffold === "timeline") {
      this.sceneLayer.dataset.orientation = "horizontal";
      const rail = this.scaffoldItems.find(({ kind }) => kind === "timeline-rail");
      const points = rail.nodeIds.map(screenFor).filter(Boolean);
      const xs = points.map(({ x }) => x);
      const railY = this.panY;
      const left = Math.max(Math.min(...xs) - 18, 12);
      positionBox(rail.element, {
        left,
        top: railY,
        width: Math.max(...xs) + 18 - left,
        height: 2,
      });
      for (const item of this.scaffoldItems) {
        if (item.kind !== "timeline-anchor") continue;
        const point = screenFor(item.nodeId);
        if (!point) continue;
        positionBox(item.element, {
          left: point.x,
          top: Math.min(point.y, railY),
          width: 1,
          height: Math.max(Math.abs(point.y - railY), 1),
        });
        item.anchor.style.left = `${point.x}px`;
        item.anchor.style.top = `${railY}px`;
      }
      return;
    }
    if (scaffold === "lanes") {
      for (const item of this.scaffoldItems) {
        const points = item.nodeIds.map(screenFor).filter(Boolean);
        const xs = points.map(({ x }) => x);
        const ys = points.map(({ y }) => y);
        const horizontal = this.stage.clientWidth < 480;
        const paddingX = horizontal ? 22 : 58;
        const paddingY = horizontal ? 34 : 48;
        positionBox(item.element, {
          left: Math.min(...xs) - paddingX,
          top: Math.min(...ys) - paddingY,
          width: Math.max(...xs) - Math.min(...xs) + paddingX * 2,
          height: Math.max(...ys) - Math.min(...ys) + paddingY * 2,
        });
      }
      return;
    }
    if (scaffold === "tessellation") {
      for (const item of this.scaffoldItems) {
        const point = screenFor(item.nodeId);
        if (!point) continue;
        item.element.style.left = `${point.x}px`;
        item.element.style.top = `${point.y}px`;
      }
      return;
    }
    if (scaffold === "branches") {
      for (const item of this.scaffoldItems) {
        const source = screenFor(item.sourceId);
        const target = screenFor(item.targetId);
        if (source && target) positionLine(item.element, source, target);
      }
    }
  }

  createEdgePipeline() {
    const module = this.device.createShaderModule({
      label: "Mer3ly repository edge shader",
      code: `
        struct VertexInput {
          @location(0) position: vec2f,
          @location(1) color: vec4f,
        };
        struct VertexOutput {
          @builtin(position) position: vec4f,
          @location(0) color: vec4f,
        };
        @vertex fn vertex_main(input: VertexInput) -> VertexOutput {
          var output: VertexOutput;
          output.position = vec4f(input.position, 0.0, 1.0);
          output.color = input.color;
          return output;
        }
        @fragment fn fragment_main(input: VertexOutput) -> @location(0) vec4f {
          return input.color;
        }
      `,
    });
    return this.device.createRenderPipeline({
      label: "Mer3ly repository edge pipeline",
      layout: "auto",
      vertex: {
        module,
        entryPoint: "vertex_main",
        buffers: [
          {
            arrayStride: 24,
            attributes: [
              { shaderLocation: 0, offset: 0, format: "float32x2" },
              { shaderLocation: 1, offset: 8, format: "float32x4" },
            ],
          },
        ],
      },
      fragment: {
        module,
        entryPoint: "fragment_main",
        targets: [
          {
            format: this.format,
            blend: {
              color: {
                srcFactor: "src-alpha",
                dstFactor: "one-minus-src-alpha",
                operation: "add",
              },
              alpha: {
                srcFactor: "one",
                dstFactor: "one-minus-src-alpha",
                operation: "add",
              },
            },
          },
        ],
      },
      primitive: { topology: "line-list" },
    });
  }

  createNodePipeline() {
    const module = this.device.createShaderModule({
      label: "Mer3ly repository node shader",
      code: `
        struct VertexInput {
          @location(0) position: vec2f,
          @location(1) local: vec2f,
          @location(2) color: vec4f,
        };
        struct VertexOutput {
          @builtin(position) position: vec4f,
          @location(0) local: vec2f,
          @location(1) color: vec4f,
        };
        @vertex fn vertex_main(input: VertexInput) -> VertexOutput {
          var output: VertexOutput;
          output.position = vec4f(input.position, 0.0, 1.0);
          output.local = input.local;
          output.color = input.color;
          return output;
        }
        @fragment fn fragment_main(input: VertexOutput) -> @location(0) vec4f {
          let distance = length(input.local);
          if (distance > 1.0) {
            discard;
          }
          let rim = smoothstep(0.72, 0.96, distance);
          let rim_color = vec4f(0.22, 0.08, 0.07, 1.0);
          return mix(input.color, rim_color, rim);
        }
      `,
    });
    return this.device.createRenderPipeline({
      label: "Mer3ly repository node pipeline",
      layout: "auto",
      vertex: {
        module,
        entryPoint: "vertex_main",
        buffers: [
          {
            arrayStride: 32,
            attributes: [
              { shaderLocation: 0, offset: 0, format: "float32x2" },
              { shaderLocation: 1, offset: 8, format: "float32x2" },
              { shaderLocation: 2, offset: 16, format: "float32x4" },
            ],
          },
        ],
      },
      fragment: {
        module,
        entryPoint: "fragment_main",
        targets: [
          {
            format: this.format,
            blend: {
              color: {
                srcFactor: "src-alpha",
                dstFactor: "one-minus-src-alpha",
                operation: "add",
              },
              alpha: {
                srcFactor: "one",
                dstFactor: "one-minus-src-alpha",
                operation: "add",
              },
            },
          },
        ],
      },
      primitive: { topology: "triangle-list" },
    });
  }

  installNodeButtons() {
    this.nodeButtons = new Map();
    for (const node of this.layout.nodes) {
      const button = document.createElement("button");
      button.type = "button";
      button.className = `repository-graph-node class-${node.class} status-${node.status}`;
      button.dataset.graphNodeId = node.id;
      button.setAttribute("aria-label", `${node.name}, ${node.class}, ${node.status}`);
      button.setAttribute("aria-pressed", "false");
      button.innerHTML = `
        <span class="repository-graph-node-mark" aria-hidden="true"><span class="repository-graph-node-initial">${escapeMarkup(shortName(node.name))}</span></span>
        <span class="repository-graph-node-label" aria-hidden="true">${escapeMarkup(node.name)}</span>
        <span class="repository-graph-node-meta repository-graph-node-date" aria-hidden="true">${escapeMarkup(formatGraphDate(node.pushed_at.slice(0, 10)))}</span>
        <span class="repository-graph-node-meta repository-graph-node-status" aria-hidden="true">${escapeMarkup(humanize(node.status))}</span>
      `;
      button.addEventListener("click", () => this.select(node.id));
      button.addEventListener("dblclick", () => this.open(node.id));
      button.addEventListener("focus", () => this.select(node.id));
      button.addEventListener("keydown", (event) => {
        this.handleNodeKey(event, node.id);
      });
      this.nodeLayer.append(button);
      this.nodeButtons.set(node.id, button);
    }
    this.select(this.selectedId, false);
  }

  installPointerControls() {
    let drag = null;
    this.stage.addEventListener("pointerdown", (event) => {
      if (event.target.closest("[data-graph-node-id]")) {
        return;
      }
      drag = {
        id: event.pointerId,
        x: event.clientX,
        y: event.clientY,
      };
      this.stage.setPointerCapture(event.pointerId);
      this.stage.classList.add("is-panning");
    });
    this.stage.addEventListener("pointermove", (event) => {
      if (!drag || drag.id !== event.pointerId) {
        return;
      }
      this.panBy(event.clientX - drag.x, event.clientY - drag.y);
      drag.x = event.clientX;
      drag.y = event.clientY;
    });
    const finishDrag = (event) => {
      if (!drag || drag.id !== event.pointerId) {
        return;
      }
      drag = null;
      this.stage.classList.remove("is-panning");
    };
    this.stage.addEventListener("pointerup", finishDrag);
    this.stage.addEventListener("pointercancel", finishDrag);
    this.stage.addEventListener(
      "wheel",
      (event) => {
        event.preventDefault();
        const rect = this.stage.getBoundingClientRect();
        this.zoomBy(event.deltaY < 0 ? 1.12 : 0.89, {
          x: event.clientX - rect.left,
          y: event.clientY - rect.top,
        });
      },
      { passive: false },
    );
  }

  handleNodeKey(event, currentId) {
    const index = this.layout.nodes.findIndex(({ id }) => id === currentId);
    let next = null;
    if (event.key === "ArrowRight" || event.key === "ArrowDown") {
      next = (index + 1) % this.layout.nodes.length;
    } else if (event.key === "ArrowLeft" || event.key === "ArrowUp") {
      next = (index - 1 + this.layout.nodes.length) % this.layout.nodes.length;
    } else if (event.key === "Home") {
      next = 0;
    } else if (event.key === "End") {
      next = this.layout.nodes.length - 1;
    } else if (event.key === "Enter") {
      event.preventDefault();
      this.open(currentId);
      return;
    }
    if (next !== null) {
      event.preventDefault();
      const nextNode = this.layout.nodes[next];
      this.nodeButtons.get(nextNode.id).focus();
    }
  }

  select(id, announceSelection = true) {
    if (!this.nodeButtons.has(id)) {
      return;
    }
    this.selectedId = id;
    for (const [nodeId, button] of this.nodeButtons) {
      const selected = nodeId === id;
      button.classList.toggle("is-selected", selected);
      button.setAttribute("aria-pressed", String(selected));
    }
    if (announceSelection) {
      const node = this.layout.nodes.find((candidate) => candidate.id === id);
      const outgoing = this.layout.edges.filter((edge) => edge.source === id).length;
      const incoming = this.layout.edges.filter((edge) => edge.target === id).length;
      announce(
        this.graphRoot,
        `${node.name} selected. ${outgoing} outgoing and ${incoming} incoming relationships.`,
      );
    }
    this.schedule();
  }

  open(id = this.selectedId) {
    const target = document.querySelector(`#repo-${CSS.escape(id)}`);
    if (!target) {
      return;
    }
    const projectHref = target.dataset.projectHref;
    if (projectHref) {
      window.location.assign(projectHref);
    }
  }

  morphTo(arrangementId) {
    const arrangement = this.arrangements.get(arrangementId);
    if (!arrangement || arrangementId === this.currentArrangement) {
      return;
    }
    if (this.morph) {
      this.advanceMorph(performance.now());
    }
    this.alignMobileOrientation(this.currentArrangement, arrangementId);
    const target = new Map(
      arrangement.nodes.map(({ id, x, y }) => [id, { x, y }]),
    );
    this.currentArrangement = arrangementId;
    this.graphRoot.dataset.graphArrangement = arrangementId;
    this.graphRoot.dataset.graphEngine = arrangement.engine;
    this.applySceneProfile(arrangementId);

    if (this.reducedMotion) {
      this.positions = target;
      this.morph = null;
      this.fit();
      this.graphRoot.dataset.graphMorphing = "false";
      this.schedule();
      announce(
        this.graphRoot,
        `${arrangement.name} arrangement. ${arrangement.description}`,
      );
      return;
    }

    this.morph = {
      arrangement,
      from: new Map(
        [...this.positions].map(([id, position]) => [id, { ...position }]),
      ),
      target,
      startedAt: performance.now(),
    };
    this.graphRoot.dataset.graphMorphing = "true";
    announce(this.graphRoot, `Morphing into the ${arrangement.name} arrangement.`);
    this.schedule();
  }

  advanceMorph(timestamp) {
    if (!this.morph) {
      return false;
    }
    const progress = clamp(
      (timestamp - this.morph.startedAt) / MORPH_DURATION_MS,
      0,
      1,
    );
    const eased = 1 - (1 - progress) ** 3;
    for (const [id, target] of this.morph.target) {
      const start = this.morph.from.get(id) ?? target;
      this.positions.set(id, {
        x: start.x + (target.x - start.x) * eased,
        y: start.y + (target.y - start.y) * eased,
      });
    }
    if (progress < 1) {
      return true;
    }
    const { arrangement, target, announcement } = this.morph;
    this.positions = target;
    this.morph = null;
    this.fit();
    this.graphRoot.dataset.graphMorphing = "false";
    if (announcement !== "") {
      announce(
        this.graphRoot,
        `${arrangement.name} arrangement. ${arrangement.description}`,
      );
    }
    return false;
  }

  fit() {
    const width = Math.max(this.stage.clientWidth, 1);
    const height = Math.max(this.stage.clientHeight, 1);
    const positions = this.layout.nodes.map((node) => this.layoutPosition(node));
    const xs = positions.map(({ x }) => x);
    const ys = positions.map(({ y }) => y);
    const minX = Math.min(...xs);
    const maxX = Math.max(...xs);
    const minY = Math.min(...ys);
    const maxY = Math.max(...ys);
    const margin = width < 480 ? 54 : 90;
    const worldWidth = Math.max(maxX - minX, 1);
    const worldHeight = Math.max(maxY - minY, 1);
    this.scale = clamp(
      Math.min(
        (width - margin * 2) / worldWidth,
        (height - margin * 2) / worldHeight,
      ),
      0.2,
      2.5,
    );
    this.panX = width * 0.5 - ((minX + maxX) * 0.5) * this.scale;
    this.panY = height * 0.5 - ((minY + maxY) * 0.5) * this.scale;
    this.userAdjusted = false;
    this.schedule();
  }

  zoomBy(factor, center = null) {
    const width = Math.max(this.stage.clientWidth, 1);
    const height = Math.max(this.stage.clientHeight, 1);
    const pivot = center ?? { x: width * 0.5, y: height * 0.5 };
    const nextScale = clamp(this.scale * factor, 0.18, 4);
    const worldX = (pivot.x - this.panX) / this.scale;
    const worldY = (pivot.y - this.panY) / this.scale;
    this.scale = nextScale;
    this.panX = pivot.x - worldX * nextScale;
    this.panY = pivot.y - worldY * nextScale;
    this.userAdjusted = true;
    this.schedule();
  }

  panBy(x, y) {
    this.panX += x;
    this.panY += y;
    this.userAdjusted = true;
    this.schedule();
  }

  schedule() {
    if (document.hidden || this.frame !== null) {
      return;
    }
    this.frame = requestAnimationFrame((timestamp) => {
      this.frame = null;
      const morphing = this.advanceMorph(timestamp);
      this.draw();
      if (morphing) {
        this.schedule();
      }
    });
  }

  screenPosition(node) {
    const position = this.layoutPosition(node);
    return {
      x: position.x * this.scale + this.panX,
      y: position.y * this.scale + this.panY,
    };
  }

  layoutPosition(node) {
    const position = this.positions.get(node.id) ?? node;
    if (
      this.stage.clientWidth < 480 &&
      this.currentArrangement !== TIMELINE_ARRANGEMENT
    ) {
      return { x: position.y, y: -position.x };
    }
    return position;
  }

  alignMobileOrientation(previousArrangement, nextArrangement) {
    if (this.stage.clientWidth >= 480) return;
    const previousIsTimeline = previousArrangement === TIMELINE_ARRANGEMENT;
    const nextIsTimeline = nextArrangement === TIMELINE_ARRANGEMENT;
    if (previousIsTimeline === nextIsTimeline) return;
    this.positions = new Map(
      [...this.positions].map(([id, position]) => [
        id,
        nextIsTimeline
          ? { x: position.y, y: -position.x }
          : { x: -position.y, y: position.x },
      ]),
    );
  }

  draw() {
    const width = Math.max(this.stage.clientWidth, 1);
    const height = Math.max(this.stage.clientHeight, 1);
    const pixelRatio = Math.min(window.devicePixelRatio || 1, 2);
    const pixelWidth = Math.max(Math.round(width * pixelRatio), 1);
    const pixelHeight = Math.max(Math.round(height * pixelRatio), 1);
    if (this.canvas.width !== pixelWidth || this.canvas.height !== pixelHeight) {
      this.canvas.width = pixelWidth;
      this.canvas.height = pixelHeight;
    }

    const nodeById = new Map(this.layout.nodes.map((node) => [node.id, node]));
    const toClip = ({ x, y }) => ({
      x: (x / width) * 2 - 1,
      y: 1 - (y / height) * 2,
    });
    const edgeVertices = [];
    for (const edge of this.layout.edges) {
      const source = nodeById.get(edge.source);
      const target = nodeById.get(edge.target);
      if (!source || !target) {
        continue;
      }
      const connected = edge.source === this.selectedId || edge.target === this.selectedId;
      const opacity = connected
        ? this.sceneProfile.selectedEdgeOpacity
        : this.sceneProfile.edgeOpacity;
      const color = edgeColor(edge, opacity);
      const sourceClip = toClip(this.screenPosition(source));
      const targetClip = toClip(this.screenPosition(target));
      edgeVertices.push(sourceClip.x, sourceClip.y, ...color);
      edgeVertices.push(targetClip.x, targetClip.y, ...color);
    }

    const nodeVertices = [];
    for (const node of this.layout.nodes) {
      const screen = this.screenPosition(node);
      if (this.sceneProfile.canvasNodes) {
        const selected = node.id === this.selectedId;
        const radius = selected ? 14 : 10;
        const color = nodeColor(node.class, selected);
        const corners = [
          [-1, -1],
          [1, -1],
          [1, 1],
          [-1, -1],
          [1, 1],
          [-1, 1],
        ];
        for (const [localX, localY] of corners) {
          const clip = toClip({
            x: screen.x + localX * radius,
            y: screen.y + localY * radius,
          });
          nodeVertices.push(clip.x, clip.y, localX, localY, ...color);
        }
      }
      const button = this.nodeButtons.get(node.id);
      button.style.left = `${screen.x}px`;
      button.style.top = `${screen.y}px`;
    }
    this.updateSceneScaffold();

    this.edgeBuffer?.destroy();
    this.nodeBuffer?.destroy();
    this.edgeBuffer = createVertexBuffer(
      this.device,
      new Float32Array(edgeVertices),
      "Mer3ly repository edges",
    );
    this.nodeBuffer = createVertexBuffer(
      this.device,
      new Float32Array(nodeVertices),
      "Mer3ly repository nodes",
    );

    const encoder = this.device.createCommandEncoder({
      label: "Mer3ly repository graph commands",
    });
    const pass = encoder.beginRenderPass({
      label: "Mer3ly repository graph pass",
      colorAttachments: [
        {
          view: this.context.getCurrentTexture().createView(),
          clearValue: { r: 0.949, g: 0.929, b: 0.875, a: 1 },
          loadOp: "clear",
          storeOp: "store",
        },
      ],
    });
    if (edgeVertices.length > 0) {
      pass.setPipeline(this.edgePipeline);
      pass.setVertexBuffer(0, this.edgeBuffer);
      pass.draw(edgeVertices.length / 6);
    }
    if (nodeVertices.length > 0) {
      pass.setPipeline(this.nodePipeline);
      pass.setVertexBuffer(0, this.nodeBuffer);
      pass.draw(nodeVertices.length / 8);
    }
    pass.end();
    this.device.queue.submit([encoder.finish()]);
  }
}

function installGraphControls(graphRoot, renderer, layout, authority, history) {
  const controls = graphRoot.querySelector("[data-graph-controls]");
  let historyScrubber = null;
  controls.addEventListener("click", (event) => {
    const button = event.target.closest("button[data-graph-action]");
    if (!button) {
      return;
    }
    const action = button.dataset.graphAction;
    if (action === "zoom-in") renderer.zoomBy(1.2);
    if (action === "zoom-out") renderer.zoomBy(0.82);
    if (action === "fit") renderer.fit();
    if (action === "pan-left") renderer.panBy(36, 0);
    if (action === "pan-right") renderer.panBy(-36, 0);
    if (action === "pan-up") renderer.panBy(0, 36);
    if (action === "pan-down") renderer.panBy(0, -36);
    if (action === "open") renderer.open();
    if (action === "return-live") historyScrubber?.showLive();
    if (action === "share") sceneShare?.share();
  });
  const reduced = prefersReducedMotion();
  renderer.reducedMotion = reduced;
  graphRoot.dataset.reducedMotion = String(reduced);
  graphRoot.dataset.graphEngine = layout.engine;
  graphRoot.dataset.graphArrangement = layout.default_arrangement;
  graphRoot.dataset.graphMorphing = "false";

  const arrangementPicker = controls.querySelector("[data-graph-arrangement]");
  for (const arrangement of layout.arrangements) {
    const option = document.createElement("option");
    option.value = arrangement.id;
    option.textContent = arrangement.name;
    option.title = arrangement.description;
    arrangementPicker.append(option);
  }
  for (const arrangement of layout.unavailable_arrangements) {
    const option = document.createElement("option");
    option.value = arrangement.id;
    option.textContent = `${arrangement.name} · needs data`;
    option.title = arrangement.reason;
    option.disabled = true;
    arrangementPicker.append(option);
  }
  arrangementPicker.value = layout.default_arrangement;
  arrangementPicker.addEventListener("change", () => {
    renderer.morphTo(arrangementPicker.value);
  });
  historyScrubber = installHistoryScrubber(graphRoot, renderer, authority, history);
  const sceneShare = installSceneShare(graphRoot, renderer, historyScrubber);
  return sceneShare?.restore() ?? null;
}

function installHistoryScrubber(graphRoot, renderer, authority, history) {
  const controls = graphRoot.querySelector("[data-graph-history-controls]");
  if (!controls || !history) {
    return null;
  }
  const snapshots = history.checkpoints.filter(
    ({ availability }) => availability === "available",
  );
  if (snapshots.length === 0) {
    return null;
  }
  const range = controls.querySelector("[data-graph-history]");
  const status = controls.querySelector("[data-graph-history-status]");
  const liveValue = snapshots.length;
  const unavailable = history.checkpoints.length - snapshots.length;
  let currentValue = liveValue;
  controls.hidden = false;
  range.max = String(liveValue);
  range.value = String(liveValue);

  const describe = (value) => {
    if (value === liveValue) {
      return unavailable > 0
        ? `Live authority · ${unavailable} earlier commits lack an authority graph`
        : "Live authority";
    }
    const cursor = snapshots[value].cursor;
    return `Committed ${formatGraphDate(cursor.committed_at.slice(0, 10))} · ${cursor.source}`;
  };
  const apply = (value) => {
    if (!Number.isInteger(value) || value < 0 || value > liveValue) {
      return;
    }
    range.value = String(value);
    status.textContent = describe(value);
    range.setAttribute("aria-valuetext", status.textContent);
    if (value === currentValue) {
      return;
    }
    currentValue = value;
    const source = value === liveValue ? authority : snapshots[value].graph;
    const layout = JSON.parse(layoutGraph(JSON.stringify(source)));
    validateProjection(source, layout);
    renderer.replaceLayout(layout);
    announce(
      graphRoot,
      value === liveValue
        ? "Returned to the live repository authority."
        : `${status.textContent}. The current arrangement and selected repository are preserved where available.`,
    );
  };

  range.addEventListener("input", () => apply(Number(range.value)));
  range.addEventListener("keydown", (event) => {
    const value = Number(range.value);
    const next = {
      Home: 0,
      End: liveValue,
      ArrowLeft: Math.max(0, value - 1),
      ArrowDown: Math.max(0, value - 1),
      ArrowRight: Math.min(liveValue, value + 1),
      ArrowUp: Math.min(liveValue, value + 1),
    }[event.key];
    if (next === undefined) {
      return;
    }
    event.preventDefault();
    apply(next);
  });
  apply(liveValue);
  return {
    showLive: () => apply(liveValue),
    current: () =>
      currentValue === liveValue
        ? { kind: "live" }
        : { kind: "cursor", cursor: snapshots[currentValue].cursor },
    showCursor: (cursor) => {
      const index = snapshots.findIndex(
        ({ cursor: available }) =>
          available.source === cursor.source && available.commit === cursor.commit,
      );
      if (index < 0) {
        return false;
      }
      apply(index);
      return true;
    },
  };
}

function installSceneShare(graphRoot, renderer, historyScrubber) {
  const sceneFromFragment = () => {
    const params = new URLSearchParams(window.location.hash.slice(1));
    if (!params.has(SHARED_SCENE_KEY)) {
      return null;
    }
    if (params.get(SHARED_SCENE_KEY) !== SHARED_SCENE_VERSION) {
      return { error: "This repository scene link uses an unsupported version." };
    }
    const arrangement = params.get("arrangement");
    const selected = params.get("selected");
    const source = params.get("source");
    const commit = params.get("commit");
    if (!arrangement || !selected || Boolean(source) !== Boolean(commit)) {
      return { error: "This repository scene link is incomplete." };
    }
    if (source && (!isPublicSceneValue(source) || !isPublicSceneValue(commit))) {
      return { error: "This repository scene link has an invalid public source cursor." };
    }
    return { arrangement, selected, source, commit };
  };

  const restore = () => {
    const scene = sceneFromFragment();
    if (!scene) {
      return null;
    }
    if (scene.error) {
      return scene.error;
    }
    if (scene.source && !historyScrubber?.showCursor(scene)) {
      return "The requested repository source cursor is unavailable in this public artifact.";
    }
    if (!renderer.arrangements.has(scene.arrangement)) {
      return "The requested repository arrangement is unavailable.";
    }
    if (!renderer.nodeButtons.has(scene.selected)) {
      return "The requested repository selection is unavailable at this source cursor.";
    }
    renderer.morphTo(scene.arrangement);
    renderer.select(scene.selected, false);
    return "Shared repository scene restored from its public source cursor.";
  };

  const share = async () => {
    const scene = {
      arrangement: renderer.currentArrangement,
      selected: renderer.selectedId,
      ...(historyScrubber?.current() ?? { kind: "live" }),
    };
    const params = new URLSearchParams([[SHARED_SCENE_KEY, SHARED_SCENE_VERSION]]);
    params.set("arrangement", scene.arrangement);
    params.set("selected", scene.selected);
    if (scene.kind === "cursor") {
      params.set("source", scene.cursor.source);
      params.set("commit", scene.cursor.commit);
    }
    const url = new URL(window.location.href);
    url.hash = params.toString();
    window.history.replaceState(null, "", url);
    try {
      if (!navigator.clipboard?.writeText) {
        throw new Error("clipboard unavailable");
      }
      await navigator.clipboard.writeText(url.toString());
      announce(graphRoot, "Shareable repository scene link copied to the clipboard.");
    } catch {
      announce(graphRoot, "Shareable repository scene link is ready in the address bar.");
    }
  };

  return { restore, share };
}

function isPublicSceneValue(value) {
  return /^[a-z0-9][a-z0-9._/-]*$/i.test(value);
}

function createVertexBuffer(device, data, label) {
  const buffer = device.createBuffer({
    label,
    size: Math.max(data.byteLength, 4),
    usage: GPUBufferUsage.VERTEX | GPUBufferUsage.COPY_DST,
  });
  if (data.byteLength > 0) {
    device.queue.writeBuffer(buffer, 0, data);
  }
  return buffer;
}

function edgeColor(edge, opacity = 1) {
  if (edge.kind === "host_for") return [0.55, 0.1, 0.08, 0.82 * opacity];
  if (edge.kind === "renders_with") return [0.76, 0.44, 0.12, 0.82 * opacity];
  if (edge.provenance === "curated") return [0.7, 0.42, 0.13, 0.78 * opacity];
  return [0.22, 0.4, 0.5, 0.52 * opacity];
}

function nodeColor(repositoryClass, selected) {
  const colors = {
    product: [0.55, 0.1, 0.09, 1],
    platform: [0.16, 0.34, 0.44, 1],
    foundation: [0.74, 0.45, 0.15, 1],
    tool: [0.38, 0.34, 0.29, 1],
  };
  const color = colors[repositoryClass] ?? colors.tool;
  return selected
    ? color.map((channel, index) =>
        index === 3 ? channel : Math.min(channel * 1.18, 1),
      )
    : color;
}

function shortName(name) {
  if (name === "Merely organization profile") return "M";
  if (name === "Mer3ly") return "M3";
  const pieces = name.split(/[\s-]+/).filter(Boolean);
  if (pieces.length > 1) {
    return pieces
      .slice(0, 2)
      .map((piece) => piece[0])
      .join("")
      .toUpperCase();
  }
  return name.slice(0, 2).toUpperCase();
}

function escapeMarkup(value) {
  return value
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");
}

function sceneElement(className, text = "") {
  const element = document.createElement("div");
  element.className = className;
  element.textContent = text;
  return element;
}

function positionBox(element, { left, top, width, height }) {
  element.style.left = `${left}px`;
  element.style.top = `${top}px`;
  element.style.width = `${Math.max(width, 1)}px`;
  element.style.height = `${Math.max(height, 1)}px`;
}

function positionLine(element, source, target) {
  const dx = target.x - source.x;
  const dy = target.y - source.y;
  element.style.left = `${source.x}px`;
  element.style.top = `${source.y}px`;
  element.style.width = `${Math.hypot(dx, dy)}px`;
  element.style.transform = `rotate(${Math.atan2(dy, dx)}rad)`;
}

function average(values) {
  return values.reduce((sum, value) => sum + value, 0) / Math.max(values.length, 1);
}

function humanize(value) {
  return value.replaceAll("_", " ").replace(/\b\w/g, (letter) => letter.toUpperCase());
}

function formatGraphDate(value) {
  const match = /^(\d{4})-(\d{2})-(\d{2})$/.exec(value);
  if (!match) return value;
  const date = new Date(Date.UTC(Number(match[1]), Number(match[2]) - 1, Number(match[3])));
  return date.toLocaleDateString("en-US", {
    month: "short",
    day: "numeric",
    timeZone: "UTC",
  });
}

function formatGraphDateRange(first, last) {
  if (first === last) return formatGraphDate(first);
  const firstYear = first.slice(0, 4);
  const lastYear = last.slice(0, 4);
  if (firstYear === lastYear) {
    return `${formatGraphDate(first)}–${formatGraphDate(last)} ${lastYear}`;
  }
  return `${formatGraphDate(first)} ${firstYear}–${formatGraphDate(last)} ${lastYear}`;
}

function prefersReducedMotion() {
  return (
    window.matchMedia("(prefers-reduced-motion: reduce)").matches ||
    new URLSearchParams(window.location.search).get("motion") === "reduce"
  );
}

function clamp(value, minimum, maximum) {
  return Math.min(Math.max(value, minimum), maximum);
}
