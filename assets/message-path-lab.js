// The message path lab reads retinue-sim's generated route traces (Rulings 7,
// 9 and 127). Every step is an event in a committed trace; nothing here
// authors a route, a frame or a delivery. The selected radio's screen is
// retinue's radio-mirror drawing the firmware's own TRAFFIC page from that
// radio's face-track documents (Ruling 8). Route and delivery are the site's
// ledger prose, generated from the trace at build time: firmware has no ROUTE
// or DELIVERED screen. Radio positions are site layout, not geography.

const runtimeVersion = new URL(import.meta.url).search;
const MANIFEST_SCHEMA = "mer3ly.message-path-traces/v1";
const ROUTE_SCHEMA = "retinue-sim.route-trace/v1";
const FACE_SCHEMA = "retinue-sim.face-track/v1";
const SHARE_VERSION = "v2";
const DEFAULT_LOCAL = JSON.stringify({ schema: "radio-mirror.local/v1" });
const PLAY_INTERVAL_MS = 250;
const MAX_PRESSES = 8;

const root = document.querySelector("[data-message-path-lab]");
if (root) {
  start(root).catch((error) => {
    root.dataset.ready = "unavailable";
    const status = root.querySelector("[data-path-status]");
    if (status) {
      status.textContent =
        "The trace reader could not start. The cold-start trace's ledger and screen below remain readable.";
    }
    console.warn("message path lab unavailable:", error);
  });
}

async function start(root) {
  const manifestElement = document.getElementById("message-path-traces");
  if (!manifestElement) throw new Error("the trace manifest is absent");
  const manifest = JSON.parse(manifestElement.textContent);
  if (manifest.schema !== MANIFEST_SCHEMA) {
    throw new Error(`unsupported trace manifest ${manifest.schema}`);
  }
  const { default: initWasm, RadioMirror } = await import(
    `./radio_mirror.js${runtimeVersion}`
  );
  await initWasm({
    module_or_path: new URL(`./radio_mirror_bg.wasm${runtimeVersion}`, import.meta.url),
  });
  const lab = new MessagePathLab(root, manifest, RadioMirror);
  await lab.restore();
}

function seconds(t) {
  return t % 100 === 0
    ? `${Math.floor(t / 1000)}.${Math.floor((t % 1000) / 100)} s`
    : `${Math.floor(t / 1000)}.${String(t % 1000).padStart(3, "0")} s`;
}

function listed(items) {
  if (items.length <= 1) return items.join("");
  return `${items.slice(0, -1).join(", ")} and ${items.at(-1)}`;
}

function capitalized(text) {
  return text.charAt(0).toUpperCase() + text.slice(1);
}

async function fetchJson(href) {
  const response = await fetch(href);
  if (!response.ok) throw new Error(`${href}: ${response.status}`);
  return response.json();
}

// One scenario: its route trace and face track, with each node's faces
// indexed by event for "latest entry at or before this step".
class LoadedScenario {
  constructor(entry, trace, faces) {
    if (trace.schema !== ROUTE_SCHEMA || trace.scenario !== entry.name) {
      throw new Error(`${entry.id}: unexpected route trace ${trace.schema} ${trace.scenario}`);
    }
    if (
      faces.schema !== FACE_SCHEMA ||
      faces.route_trace !== trace.schema ||
      faces.trace_sha256 !== entry.trace_sha256 ||
      trace.events.length !== entry.events
    ) {
      throw new Error(`${entry.id}: the face track does not pair with its route trace`);
    }
    this.entry = entry;
    this.trace = trace;
    this.milestones = entry.milestones;
    this.milestoneText = new Map(entry.milestones.map((row) => [row.event, row.text]));
    this.faces = new Map();
    for (const face of faces.entries) {
      if (!this.faces.has(face.node)) this.faces.set(face.node, []);
      this.faces.get(face.node).push(face);
    }
    this.frames = new Map();
    trace.events.forEach((event) => {
      if (event.kind === "transmit") this.frames.set(event.frame, event);
    });
  }

  faceAt(node, step) {
    let latest = null;
    for (const face of this.faces.get(node) ?? []) {
      if (face.event > step) break;
      latest = face;
    }
    return latest;
  }

  latestMilestone(step) {
    let latest = null;
    for (const row of this.milestones) {
      if (row.event > step) break;
      latest = row;
    }
    return latest;
  }
}

class MessagePathLab {
  constructor(root, manifest, RadioMirror) {
    this.root = root;
    this.manifest = manifest;
    this.labels = new Map(manifest.nodes.map((node) => [node.name, node.label]));
    this.loaded = new Map();
    this.scenario = null;
    this.step = manifest.default.step;
    this.node = manifest.default.node;
    this.playback = null;
    this.reducedMotion = window.matchMedia("(prefers-reduced-motion: reduce)");

    this.select = root.querySelector("[data-path-scenario]");
    this.stepInput = root.querySelector("[data-path-step]");
    this.stepOutput = root.querySelector("[data-path-step-output]");
    this.status = root.querySelector("[data-path-status]");
    this.notice = root.querySelector("[data-path-notice]");
    this.route = root.querySelector("[data-path-route]");
    this.stage = root.querySelector("[data-path-stage]");
    this.links = root.querySelector("[data-path-links]");
    this.packet = root.querySelector("[data-path-packet]");
    this.ledger = root.querySelector("[data-path-ledger]");
    this.ledgerCount = root.querySelector("[data-path-ledger-count]");
    this.screenNode = root.querySelector("[data-path-screen-node]");
    this.screen = root.querySelector("[data-path-screen]");
    this.staticImage = root.querySelector("[data-path-static]");
    this.canvas = root.querySelector("[data-path-canvas]");
    this.context = this.canvas.getContext("2d");
    this.screenText = root.querySelector("[data-path-screen-text]");
    this.nodes = new Map(
      [...root.querySelectorAll("[data-lab-node]")].map((node) => [node.dataset.labNode, node]),
    );
    this.edges = [...root.querySelectorAll("[data-lab-edge]")];

    // One radio, its page reached through the firmware's own button logic.
    this.mirror = new RadioMirror(manifest.surface, "one-button");
    this.showPage(DEFAULT_LOCAL, undefined);

    this.select.addEventListener("change", () => {
      this.stopPlayback();
      this.load(this.select.value, null)
        .then(() => this.render(true))
        .catch((error) => {
          this.status.textContent = "That trace could not be loaded.";
          console.warn("message path trace unavailable:", error);
        });
    });
    this.stepInput.addEventListener("input", () => {
      this.stopPlayback();
      this.setStep(this.stepInput.value);
    });
    const actions = {
      play: () => this.play(),
      previous: () => this.setStep(this.step - 1),
      next: () => this.setStep(this.step + 1),
      "previous-milestone": () => this.jumpMilestone(-1),
      "next-milestone": () => this.jumpMilestone(1),
      share: () => this.share(),
    };
    for (const button of root.querySelectorAll("[data-path-action]")) {
      button.addEventListener("click", () => {
        if (button.dataset.pathAction !== "play") this.stopPlayback();
        actions[button.dataset.pathAction]?.();
      });
    }
    for (const [name, node] of this.nodes) {
      node.addEventListener("click", () => {
        this.stopPlayback();
        this.node = name;
        this.render(false);
        this.status.textContent = `Showing the ${this.label(name)}'s own screen at ${seconds(
          this.event().t,
        )}.`;
      });
    }
    window.addEventListener("visibilitychange", () => {
      if (document.hidden) this.stopPlayback();
    });
    new ResizeObserver(() => this.drawLinks()).observe(this.stage);
  }

  label(name) {
    return (this.labels.get(name) ?? `${name} radio`).toLowerCase();
  }

  the(name) {
    return `the ${this.label(name)}`;
  }

  event() {
    return this.scenario.trace.events[this.step];
  }

  // Presses the radio's one button until the firmware shows the lab's page.
  showPage(localJson, hostJson) {
    this.mirror.set_local_json(localJson);
    this.mirror.set_host_json(hostJson);
    let presses = 0;
    while (this.mirror.screen() !== this.manifest.page) {
      if (presses === MAX_PRESSES) {
        throw new Error(`the radio did not reach ${this.manifest.page}`);
      }
      this.mirror.press("a-short");
      presses += 1;
    }
  }

  async load(id, step) {
    const entry = this.manifest.scenarios.find((scenario) => scenario.id === id);
    if (!entry) throw new Error(`unknown scenario ${id}`);
    if (!this.loaded.has(id)) {
      const [trace, faces] = await Promise.all([fetchJson(entry.route), fetchJson(entry.face)]);
      this.loaded.set(id, new LoadedScenario(entry, trace, faces));
    }
    this.scenario = this.loaded.get(id);
    const last = this.scenario.trace.events.length - 1;
    this.step = step === null ? last : Math.min(Math.max(step, 0), last);
    this.select.value = id;
    this.stepInput.max = String(last);
    this.buildLedger();
  }

  // Restores a v2 share link, explains an older or unknown one, and
  // otherwise starts where the static reading stands.
  async restore() {
    const params = new URLSearchParams(window.location.hash.slice(1));
    const version = params.get("message-path");
    let scenario = this.manifest.default.scenario;
    let step = this.manifest.default.step;
    let notice = "";
    if (version === SHARE_VERSION) {
      const entry = this.manifest.scenarios.find((item) => item.id === params.get("scenario"));
      if (entry && entry.trace_sha256.startsWith(params.get("trace") ?? "-")) {
        scenario = entry.id;
        const parsed = Number(params.get("step"));
        step = Number.isInteger(parsed) ? parsed : null;
        if (this.nodes.has(params.get("node"))) this.node = params.get("node");
      } else {
        notice =
          "This link names a trace this page no longer carries, so it cannot be restored. Showing the cold-start trace.";
      }
    } else if (version !== null) {
      notice =
        "This link is from an older version of the lab, which played an authored story rather than a generated trace, so it cannot be restored. Showing the cold-start trace.";
    }
    await this.load(scenario, step);
    this.root.dataset.linkState = notice ? (version === SHARE_VERSION ? "unknown-trace" : "retired") : "none";
    if (notice) {
      this.notice.textContent = notice;
      this.notice.hidden = false;
    }
    this.staticImage.hidden = true;
    this.canvas.hidden = false;
    this.root.dataset.ready = "true";
    this.root.dataset.playing = "false";
    this.render(Boolean(notice) || version === SHARE_VERSION);
  }

  setStep(next, announce = true) {
    const last = this.scenario.trace.events.length - 1;
    this.step = Math.min(Math.max(Math.round(Number(next)), 0), last);
    this.render(announce);
  }

  jumpMilestone(direction) {
    const rows = this.scenario.milestones;
    const target =
      direction > 0
        ? rows.find((row) => row.event > this.step)
        : rows.findLast((row) => row.event < this.step);
    if (target) this.setStep(target.event);
  }

  stopPlayback() {
    if (this.playback !== null) {
      window.clearInterval(this.playback);
      this.playback = null;
    }
    this.root.dataset.playing = "false";
  }

  // Plays one trace event per tick, not simulated time: the warm trace spans
  // 760 s, most of it quiet, so the ledger rows are where it is read.
  play() {
    this.stopPlayback();
    const last = this.scenario.trace.events.length - 1;
    if (this.reducedMotion.matches) {
      this.setStep(last);
      this.status.textContent = `${this.status.textContent} Motion is reduced, so the trace jumps to its end.`;
      return;
    }
    this.setStep(0);
    this.root.dataset.playing = "true";
    this.playback = window.setInterval(() => {
      if (this.step >= last) {
        this.stopPlayback();
        return;
      }
      // While playing, only ledger rows are announced.
      const next = this.step + 1;
      this.setStep(next, this.scenario.milestoneText.has(next));
    }, PLAY_INTERVAL_MS);
  }

  async share() {
    const params = new URLSearchParams(window.location.hash.slice(1));
    for (const key of ["blocked", "positions"]) params.delete(key);
    params.set("message-path", SHARE_VERSION);
    params.set("trace", this.scenario.entry.trace_sha256.slice(0, 12));
    params.set("scenario", this.scenario.entry.id);
    params.set("step", String(this.step));
    params.set("node", this.node);
    window.history.replaceState(null, "", `#${params.toString()}`);
    try {
      await navigator.clipboard.writeText(window.location.href);
      this.status.textContent = "Link to this trace step copied.";
    } catch {
      this.status.textContent = "Link to this trace step is ready in the address bar.";
    }
  }

  buildLedger() {
    const rows = this.scenario.milestones;
    this.ledgerCount.textContent = `${rows.length} rows · ${this.scenario.trace.events.length} events`;
    this.ledger.replaceChildren(
      ...rows.map((row) => {
        const item = document.createElement("li");
        item.className = "message-path-event";
        item.dataset.labEvent = String(row.event);
        item.dataset.kind = row.kind;
        const button = document.createElement("button");
        button.type = "button";
        const time = document.createElement("span");
        time.className = "message-path-event-index";
        time.textContent = seconds(this.scenario.trace.events[row.event].t);
        const copy = document.createElement("span");
        copy.dataset.pathEventCopy = "";
        copy.textContent = row.text;
        button.append(time, copy);
        button.addEventListener("click", () => {
          this.stopPlayback();
          this.setStep(row.event);
        });
        item.append(button);
        return item;
      }),
    );
  }

  // The reading for an event the ledger does not name: an announce on the
  // air, or a radio hearing a frame.
  describe(event) {
    const ledger = this.scenario.milestoneText.get(this.step);
    if (ledger) return ledger;
    if (event.kind === "transmit") {
      const subject = event.packet.destination_node;
      let text =
        event.origin === "forward"
          ? `${this.the(event.node)} relays ${subject ? `${this.the(subject)}'s` : "an"} announce`
          : `${this.the(event.node)} transmits its own announce`;
      text += event.heard_by.length
        ? `. Heard by ${listed(event.heard_by.map((name) => this.the(name)))}.`
        : ". No radio hears it.";
      if (event.blocked.length) {
        text += ` Behind the cut, ${listed(event.blocked.map((name) => this.the(name)))} ${
          event.blocked.length === 1 ? "does" : "do"
        } not hear it.`;
      }
      return capitalized(text);
    }
    if (event.kind === "receive") {
      const frame = this.scenario.frames.get(event.frame);
      const what = frame
        ? `${this.the(frame.node)}'s ${frame.packet.packet_type.replace("_", " ")}`
        : `frame ${event.frame}`;
      const effects = event.effects.map((effect) => {
        switch (effect.effect) {
          case "learned":
            return `learns a route to ${this.the(effect.destination)}`;
          case "link_up":
            return "brings a link up";
          case "link_down":
            return "takes a link down";
          case "data":
            return "takes in its data";
          case "resource":
            return "takes in a resource";
          default:
            return effect.effect.replace("_", " ");
        }
      });
      return capitalized(
        `${this.the(event.node)} hears ${what}${effects.length ? ` and ${listed(effects)}` : ""}.`,
      );
    }
    return capitalized(event.kind.replaceAll("_", " "));
  }

  latestDelivery() {
    const events = this.scenario.trace.events;
    for (let index = this.step; index >= 0; index -= 1) {
      if (events[index].kind === "delivered") return { index, event: events[index] };
    }
    return null;
  }

  cutEdges() {
    const cuts = [];
    for (const event of this.scenario.trace.events.slice(0, this.step + 1)) {
      if (event.kind === "cut") cuts.push([event.a, event.b]);
    }
    return cuts;
  }

  render(announce) {
    const event = this.event();
    const events = this.scenario.trace.events;
    this.root.dataset.scenario = this.scenario.entry.id;
    this.root.dataset.step = String(this.step);
    this.root.dataset.node = this.node;
    this.root.dataset.eventKind = event.kind;
    this.stepInput.value = String(this.step);
    this.stepOutput.textContent = `${this.step + 1} of ${events.length} · ${seconds(event.t)}`;

    const delivery = this.latestDelivery();
    if (delivery) {
      const row = this.scenario.milestoneText.get(delivery.index);
      this.route.textContent = `Latest delivery, at ${seconds(delivery.event.t)}: ${row}`;
    } else {
      this.route.textContent = "No message has been delivered yet at this step.";
    }

    const path = delivery?.event.path ?? [];
    const onPath = (a, b) =>
      path.some((name, index) => {
        const next = path[index + 1];
        return (name === a && next === b) || (name === b && next === a);
      });
    const cuts = this.cutEdges();
    const isCut = (a, b) =>
      cuts.some(([x, y]) => (x === a && y === b) || (x === b && y === a));
    const transmitting = event.kind === "transmit" ? event : null;
    for (const edge of this.edges) {
      const { from, to } = edge.dataset;
      const touches = (name) =>
        transmitting &&
        ((from === transmitting.node && to === name) || (to === transmitting.node && from === name));
      edge.classList.toggle("is-route", onPath(from, to));
      edge.classList.toggle("is-cut", isCut(from, to));
      edge.classList.toggle(
        "is-active",
        Boolean(transmitting?.heard_by.some((name) => touches(name))),
      );
      edge.classList.toggle(
        "is-refused",
        Boolean(transmitting?.blocked.some((name) => touches(name))),
      );
    }

    const actor = event.node ?? event.from ?? null;
    for (const [name, node] of this.nodes) {
      node.classList.toggle("is-active", name === actor);
      node.classList.toggle("is-heard", Boolean(transmitting?.heard_by.includes(name)));
      node.classList.toggle("is-selected", name === this.node);
      node.setAttribute("aria-pressed", String(name === this.node));
    }

    const current = this.scenario.latestMilestone(this.step);
    for (const item of this.ledger.children) {
      const index = Number(item.dataset.labEvent);
      const isCurrent = current !== null && index === current.event;
      item.classList.toggle("is-current", isCurrent);
      item.classList.toggle("is-complete", current !== null && index < current.event);
      if (isCurrent) item.setAttribute("aria-current", "step");
      else item.removeAttribute("aria-current");
      if (isCurrent) this.keepInView(item);
    }

    this.drawScreen();
    if (announce) this.status.textContent = `${seconds(event.t)} · ${this.describe(event)}`;
    this.drawLinks();
  }

  // Scrolls the ledger, not the page, to keep its current row in view.
  keepInView(item) {
    const top = item.offsetTop;
    const bottom = top + item.offsetHeight;
    if (top < this.ledger.scrollTop) this.ledger.scrollTop = top;
    else if (bottom > this.ledger.scrollTop + this.ledger.clientHeight) {
      this.ledger.scrollTop = bottom - this.ledger.clientHeight;
    }
  }

  // The selected radio's own screen after this step. The channel republishes
  // its host snapshot on every beat and each document is valid for
  // `valid_for_secs` (15 s), so the radio re-receives the node's latest
  // document as simulated time advances: it is re-set at every step rather
  // than aged out between the trace's sparse state changes.
  drawScreen() {
    const face = this.scenario.faceAt(this.node, this.step);
    this.mirror.set_local_json(face ? JSON.stringify(face.local) : DEFAULT_LOCAL);
    this.mirror.set_host_json(face ? JSON.stringify(face.host) : undefined);
    const image = new ImageData(this.mirror.rgba(), this.mirror.width, this.mirror.height);
    this.context.putImageData(image, 0, 0);
    this.screen.dataset.screenName = this.mirror.screen();
    this.screen.dataset.faceEvent = face ? String(face.event) : "none";
    this.screenNode.textContent = `${this.labels.get(this.node)} · its own screen`;
    this.screenText.replaceChildren(
      ...this.mirror
        .text()
        .split("\n")
        .map((line) => {
          const item = document.createElement("li");
          item.textContent = line;
          return item;
        }),
    );
  }

  drawLinks() {
    const stageRect = this.stage.getBoundingClientRect();
    if (stageRect.width === 0 || stageRect.height === 0) return;
    this.links.setAttribute("viewBox", `0 0 ${stageRect.width} ${stageRect.height}`);
    const centre = (name) => {
      const rect = this.nodes.get(name).getBoundingClientRect();
      return [
        rect.left + rect.width / 2 - stageRect.left,
        rect.top + rect.height / 2 - stageRect.top,
      ];
    };
    for (const edge of this.edges) {
      const [x1, y1] = centre(edge.dataset.from);
      const [x2, y2] = centre(edge.dataset.to);
      edge.setAttribute("x1", x1);
      edge.setAttribute("y1", y1);
      edge.setAttribute("x2", x2);
      edge.setAttribute("y2", y2);
    }
    // The stage's observer fires once before the first trace has loaded.
    const event = this.scenario ? this.event() : null;
    if (event?.kind === "transmit") {
      const [x, y] = centre(event.node);
      this.packet.setAttribute("cx", x);
      this.packet.setAttribute("cy", y);
      this.packet.removeAttribute("hidden");
    } else {
      this.packet.setAttribute("hidden", "hidden");
    }
  }
}
