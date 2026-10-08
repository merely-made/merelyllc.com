// The V4 bench drives retinue's radio-mirror: the firmware's own Controller,
// renderer, press classifier, LED intent, and text projection, compiled to
// WebAssembly. This file only wires page controls to it. It keeps no page
// table and no controller logic of its own; the RNode, Meshtastic, and
// MeshCore handoff notes are site HTML in the page, not firmware screens.

const runtimeVersion = new URL(import.meta.url).search;
const SCENARIOS_SCHEMA = "mer3ly.radio-bench-scenarios/v1";
const KEY_BUTTONS = { a: "a", b: "b" };

const benches = [...document.querySelectorAll("[data-radio-simulator]")];
if (benches.length > 0) {
  start().catch((error) => {
    benches.forEach((bench) => {
      bench.dataset.ready = "unavailable";
      const fallback = bench.querySelector("[data-radio-fallback]");
      if (fallback) {
        fallback.hidden = false;
        fallback.textContent =
          "The radio-mirror runtime could not start. The rendered screens below remain available.";
      }
    });
    console.warn("radio-mirror unavailable:", error);
  });
}

async function start() {
  const { default: initWasm, RadioMirror } = await import(
    `./radio_mirror.js${runtimeVersion}`
  );
  await initWasm({
    module_or_path: new URL(`./radio_mirror_bg.wasm${runtimeVersion}`, import.meta.url),
  });
  const documentElement = document.getElementById("radio-mirror-scenarios");
  if (!documentElement) throw new Error("radio-mirror scenario documents are absent");
  const documents = JSON.parse(documentElement.textContent);
  if (documents.schema !== SCENARIOS_SCHEMA) {
    throw new Error(`unsupported scenario documents ${documents.schema}`);
  }
  benches.forEach((bench) => new RadioBench(bench, RadioMirror, documents));
}

class RadioBench {
  constructor(root, RadioMirror, documents) {
    this.root = root;
    this.RadioMirror = RadioMirror;
    this.documents = documents;
    this.screen = root.querySelector("[data-radio-screen]");
    this.staticImage = root.querySelector("[data-radio-static]");
    this.canvas = root.querySelector("[data-radio-canvas]");
    this.context = this.canvas.getContext("2d");
    this.text = root.querySelector("[data-radio-text]");
    this.led = root.querySelector("[data-radio-led]");
    this.handoffs = [...root.querySelectorAll("[data-radio-handoff]")];
    this.helps = [...root.querySelectorAll("[data-radio-help]")];
    this.firmware = root.querySelector("[data-radio-firmware]");
    this.scenario = root.querySelector("[data-radio-scenario]");
    this.input = root.querySelector("[data-radio-input]");
    this.buttons = [...root.querySelectorAll("[data-radio-action]")];
    this.fallback = root.querySelector("[data-radio-fallback]");
    this.mirror = null;
    this.held = new Set();

    this.firmware.addEventListener("change", () => this.applyControls());
    this.scenario.addEventListener("change", () => this.reset());
    this.input.addEventListener("change", () => this.applyControls());
    this.buttons.forEach((button) => {
      button.addEventListener("click", () => this.press(button.dataset.radioAction));
    });

    // With the screen focused, A and B are the radio's buttons as raw edges,
    // timed by the firmware's own press classifier.
    this.screen.tabIndex = 0;
    this.screen.setAttribute("role", "group");
    this.screen.setAttribute(
      "aria-label",
      "Radio screen. Hold the A or B key to press the radio's buttons.",
    );
    this.screen.addEventListener("keydown", (event) => this.key(event, true));
    this.screen.addEventListener("keyup", (event) => this.key(event, false));
    this.screen.addEventListener("blur", () => this.releaseKeys());

    this.fallback.hidden = true;
    this.staticImage.hidden = true;
    this.canvas.hidden = false;
    this.reset();
    root.dataset.ready = "true";
  }

  get retinue() {
    return this.firmware.value === "retinue";
  }

  get profile() {
    return this.input.value === "two" ? "two-button" : "one-button";
  }

  reset() {
    this.releaseKeys();
    const scenario = this.documents.scenarios[this.scenario.value];
    const mirror = new this.RadioMirror(this.documents.surface, this.profile);
    mirror.set_local_json(JSON.stringify(scenario.local));
    mirror.set_host_json(scenario.host === null ? undefined : JSON.stringify(scenario.host));
    this.mirror?.free();
    this.mirror = mirror;
    this.root.dataset.lastAction = "none";
    this.applyControls();
  }

  applyControls() {
    const two = this.input.value === "two";
    this.root.dataset.inputFace = this.input.value;
    this.root.dataset.firmwareOwner = this.retinue ? "retinue" : "upstream";
    this.mirror.set_input(this.profile);
    this.helps.forEach((help) => {
      help.hidden = help.dataset.radioHelp !== this.input.value;
    });
    this.buttons.forEach((button) => {
      button.hidden = button.dataset.requiresTwo === "true" && !two;
      button.disabled = !this.retinue;
    });
    this.render();
  }

  press(event) {
    if (!this.retinue) return;
    this.root.dataset.lastAction = this.mirror.press(event);
    this.render();
  }

  key(event, pressed) {
    const button = KEY_BUTTONS[event.key.toLowerCase()];
    if (!button || !this.retinue || event.altKey || event.ctrlKey || event.metaKey) return;
    event.preventDefault();
    if (pressed && (event.repeat || this.held.has(button))) return;
    if (pressed) this.held.add(button);
    else this.held.delete(button);
    this.edge(button, pressed);
  }

  releaseKeys() {
    if (!this.mirror) return;
    [...this.held].forEach((button) => this.edge(button, false));
    this.held.clear();
  }

  edge(button, pressed) {
    const completed = this.mirror.edge(button, pressed, Math.floor(performance.now()) >>> 0);
    if (completed) {
      const [event, action] = completed.split(" ");
      this.root.dataset.lastEvent = event;
      this.root.dataset.lastAction = action;
      this.render();
    }
  }

  setReading(lines) {
    this.text.replaceChildren(
      ...lines.map((line) => {
        const item = document.createElement("li");
        item.textContent = line;
        return item;
      }),
    );
  }

  render() {
    if (!this.retinue) {
      const owner = this.firmware.value;
      this.canvas.hidden = true;
      let note = "";
      this.handoffs.forEach((handoff) => {
        handoff.hidden = handoff.dataset.radioHandoff !== owner;
        if (!handoff.hidden) note = handoff.textContent.replace(/\s+/g, " ").trim();
      });
      this.screen.dataset.screenName = `handoff:${owner}`;
      this.led.dataset.ledState = "off";
      this.setReading([note]);
      return;
    }

    this.handoffs.forEach((handoff) => {
      handoff.hidden = true;
    });
    this.canvas.hidden = false;
    const mirror = this.mirror;
    const image = new ImageData(mirror.rgba(), mirror.width, mirror.height);
    this.context.putImageData(image, 0, 0);
    this.screen.dataset.screenName = mirror.screen();
    this.screen.dataset.panelLit = String(mirror.panel_lit);
    this.screen.dataset.brightness = String(mirror.brightness);
    // The firmware signals activity for radio frames and host traffic, not
    // for button presses, so the bench shows the idle intent: off, or the
    // fault pattern while a fault stands.
    this.led.dataset.ledState = mirror.led("idle");
    this.setReading(mirror.text().split("\n"));
  }
}
