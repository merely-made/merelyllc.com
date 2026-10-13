// The site owns its published data and the frozen reading. Graphshell owns
// the live graph. Its qualified package is imported unchanged on demand.
const root = document.querySelector('[data-graph-sandbox]');
if (root) {
  const open = root.querySelector('[data-sandbox-open]');
  const lead = root.querySelector('[data-sandbox-reading-lead]');
  const status = root.querySelector('[data-sandbox-status]');
  const surface = root.querySelector('[data-sandbox-interface]');
  const frozen = root.querySelector('[data-sandbox-frozen]');
  const query = new URLSearchParams(location.search);
  let started = false;
  const say = (text) => { status.textContent = text; };
  // Legacy sandbox scenes describe a different model. Never silently discard
  // them or reinterpret their authored positions as a Graphshell document.
  const legacy = location.hash.startsWith('#graphshell-scene=');
  const eligible = () => !matchMedia('(max-width: 760px), (pointer: coarse)').matches && !!navigator.gpu;
  const refuse = () => {
    root.dataset.sandboxState = 'unavailable';
    surface.hidden = true;
    surface.replaceChildren();
    lead.textContent = lead.dataset.failedLead;
    open.hidden = true;
    say('The live graph is unavailable. The static graph, matrix, source history and project links remain available.');
  };
  async function start(event) {
    if (event.type === 'keydown' && !['Enter', ' '].includes(event.key)) return;
    if (event.target.closest('a') || started || legacy || !eligible()) return;
    started = true;
    root.dataset.sandboxState = 'loading';
    root.setAttribute('aria-busy', 'true');
    open.disabled = true;
    say('Loading the live graph…');
    let observer, timer;
    const controller = new AbortController();
    const deadline = new Promise((_, reject) => {
      timer = setTimeout(() => { controller.abort(); reject(new Error('Viewer startup timed out')); }, 45000);
    });
    try {
      await Promise.race([deadline, (async () => {
        if (query.get('graph-sandbox') === 'no-wasm') throw new Error('Forced fallback');
        if (!await navigator.gpu.requestAdapter()) throw new Error('WebGPU adapter unavailable');
        const response = await fetch(root.dataset.sandboxHistorySrc, {signal: controller.signal});
        if (!response.ok) throw new Error(`History HTTP ${response.status}`);
        const dataset = await response.text();
        if (controller.signal.aborted) return;
        const style = document.createElement('link');
        style.rel = 'stylesheet'; style.href = root.dataset.viewerStyle;
        const styled = new Promise((resolve, reject) => {
          style.onload = resolve; style.onerror = () => reject(new Error('Viewer stylesheet unavailable'));
        });
        document.head.append(style);
        await Promise.all([styled, import(root.dataset.sandboxRuntime)]);
        if (controller.signal.aborted) return;
        const viewer = document.createElement('graphshell-tree');
        viewer.className = 'site-graphshell-viewer';
        viewer.setAttribute('data-presenter', 'inspector');
        viewer.setAttribute('data-project-link-field', 'project_url');
        viewer.setAttribute('data-source-link-field', 'github_url');
        // Set data before connection: shared mount must not accept ?dataset=
        // as authority for the published site's graph.
        viewer.setAttribute('data-dataset', dataset);
        const ready = new Promise(resolve => {
          observer = new MutationObserver(() => {
            if (viewer.dataset.ready === 'true') resolve();
          });
          observer.observe(viewer, {attributes: true, attributeFilter: ['data-ready']});
        });
        surface.style.visibility = 'hidden'; surface.hidden = false;
        surface.append(viewer);
        await ready;
        if (controller.signal.aborted) return;
        surface.style.visibility = '';
        root.dataset.sandboxState = 'ready';
        open.hidden = true;
        say('Live graph ready. The static reading below remains available.');
        viewer.scrollIntoView({block: 'nearest'});
      })()]);
    } catch { controller.abort(); refuse(); }
    finally { clearTimeout(timer); observer?.disconnect(); root.removeAttribute('aria-busy'); }
  }
  if (legacy) {
    say('This link contains a legacy sandbox scene, which this viewer cannot import. Its scene has not been applied; the current static reading is shown.');
  } else if (eligible()) {
    lead.textContent = lead.dataset.scriptedLead;
    open.hidden = false;
    frozen.addEventListener('pointerdown', start);
    frozen.addEventListener('keydown', start);
    open.addEventListener('click', start);
  } else {
    say('Static reading shown. The live graph is available in desktop browsers with WebGPU.');
  }
}
