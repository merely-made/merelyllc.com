// The repositories page's only up-front script (site canvas plan, Ruling 157).
//
// /repos/ opens on a first view frozen at build time: the default reading,
// its matrix and the source history, read by mere's shared readers. The live
// sandbox (graph-sandbox.js, its glue and the graph Wasm) loads on the first
// pointer or keyboard action on that view, or at once when a share link is
// present, as the Mere profile's projection proof loads its replay (Ruling
// 140). Until then the frozen view stands in, opening with the scripted line;
// if the sandbox cannot load, the frozen view stays, saying so.

const root = document.querySelector("[data-graph-sandbox]");
if (root) prepare(root);

function prepare(sandboxRoot) {
  const frozen = sandboxRoot.querySelector("[data-sandbox-frozen]");
  const lead = sandboxRoot.querySelector("[data-sandbox-reading-lead]");
  const open = sandboxRoot.querySelector("[data-sandbox-open]");
  const forcedFailure =
    new URLSearchParams(window.location.search).get("graph-sandbox") === "no-wasm";
  let loading = null;

  // Keys that only move focus or modify another key are not an action.
  const ignored = new Set(["Tab", "Shift", "Control", "Alt", "Meta", "Escape"]);
  const onPointer = () => load();
  const onKey = (event) => {
    if (!ignored.has(event.key)) load();
  };
  const detach = () => {
    frozen?.removeEventListener("pointerdown", onPointer);
    frozen?.removeEventListener("keydown", onKey);
  };

  function load() {
    loading ??= start();
    return loading;
  }

  async function start() {
    detach();
    sandboxRoot.dataset.sandboxState = "loading";
    sandboxRoot.setAttribute("aria-busy", "true");
    if (open) {
      open.disabled = true;
      open.textContent = "loading the live sandbox…";
    }
    announce(sandboxRoot, "Loading the live sandbox.");
    try {
      if (forcedFailure) throw new Error("forced Graphshell fallback");
      const { mountSandbox } = await import(sandboxRoot.dataset.sandboxRuntime);
      await mountSandbox(sandboxRoot);
    } catch (error) {
      sandboxRoot.dataset.sandboxState = "unavailable";
      sandboxRoot.querySelector("[data-sandbox-interface]").hidden = true;
      if (frozen) frozen.hidden = false;
      if (lead) lead.textContent = lead.dataset.failedLead;
      if (open) open.hidden = true;
      announce(
        sandboxRoot,
        "The live sandbox could not load. The frozen reading and the semantic repository index remain available.",
      );
      if (!forcedFailure) console.warn("Mer3ly graph sandbox unavailable:", error);
    } finally {
      sandboxRoot.removeAttribute("aria-busy");
    }
  }

  if (lead) lead.textContent = lead.dataset.scriptedLead;
  // A share link counts as an interaction: it needs the live sandbox.
  if (window.location.hash.startsWith("#graphshell-scene=")) {
    load();
    return;
  }
  if (open) {
    open.hidden = false;
    open.addEventListener("click", () => load());
  }
  frozen?.addEventListener("pointerdown", onPointer);
  frozen?.addEventListener("keydown", onKey);
  announce(sandboxRoot, "Interact with the graph to load the live sandbox.");
}

function announce(sandboxRoot, message) {
  const status = sandboxRoot.querySelector("[data-sandbox-status]");
  if (status) status.textContent = message;
}
