import assert from 'node:assert/strict';
import {checkBrowsingRoutes} from './check-browsing-routes.mjs';
import {captureViewer} from './capture-viewer.mjs';
import path from 'node:path';
import {writeFile} from 'node:fs/promises';

// Runs inside the caller's finite browser lease. Every case gets a fresh page
// and closes it; failure evidence belongs to the caller's receipt directory.
export async function checkQualifiedViewer(browser, baseUrl, receiptRoot) {
  const results = {cases: {}, screenshots: []};
  async function check(name, options, run) {
    const page = await browser.newPage({viewport: {width: 1440, height: 1000}, ...options});
    const errors = [], consoleErrors = [];
    page.on('console', message => { if (message.type() === 'error') consoleErrors.push(message.text()); });
    page.on('pageerror', error => errors.push(String(error)));
    try {
      await run(page);
      assert.deepEqual(errors, [], `${name}: uncaught page errors`);
      if (['desktop', 'touch-live', 'legacy-share', 'no-script', 'mobile', 'tablet', 'no-webgpu'].includes(name)) {
        assert.deepEqual(consoleErrors, [], `${name}: console errors`);
      }
      results.cases[name] = 'pass';
      await writeFile(path.join(receiptRoot, 'viewer-progress.json'), JSON.stringify(results, null, 2));
    } catch (error) {
      const state = await page.evaluate(() => ({hidden: document.hidden, focused: document.hasFocus(), title: document.title, state: document.querySelector('[data-graph-sandbox]')?.dataset.sandboxState, ready: document.querySelector('graphshell-tree')?.dataset.ready, connected: document.querySelector('graphshell-tree')?.isConnected, active: document.activeElement?.outerHTML, mirror: document.querySelector('[data-cambium-mirror]')?.textContent, captureDiagnostics: window.__captureDiagnostics, browsingRoutes: window.__browsingRouteEvidence, touchDiagnostics: window.__touchDiagnostics, semantics: [...document.querySelectorAll('graphshell-tree [role]')].map(e => ({role: e.getAttribute('role'), label: e.getAttribute('aria-label'), text: e.textContent?.slice(0,120)}))})).catch(() => null);
      await writeFile(path.join(receiptRoot, `viewer-${name}-failure.json`), JSON.stringify({error: String(error), errors, consoleErrors, completedEvidence: results, state}, null, 2));
      await page.screenshot({path: path.join(receiptRoot, `viewer-${name}-failure.png`), fullPage: true}).catch(() => {});
      await page.screenshot({path: path.join(receiptRoot, `viewer-${name}-failure-viewport.png`)}).catch(() => {});
      throw error;
    } finally { await page.close(); }
  }
  const heavy = page => page.evaluate(() => performance.getEntriesByType('resource').filter(r => /\/graphshell\//.test(r.name)).map(r => r.name));
  async function staticReading(page) {
    assert.equal(await page.locator('[data-sandbox-frozen]').isVisible(), true);
    assert.ok(await page.locator('.frozen-grid td').count() > 0);
    assert.ok(await page.locator('[data-sandbox-history-revision]').count() > 0);
    assert.ok(await page.locator('[data-repository-id]').count() > 0);
    assert.equal(await page.evaluate(() => document.documentElement.scrollWidth > innerWidth + 1), false);
  }
  const goto = page => page.goto(`${baseUrl}/repos/`, {waitUntil: 'networkidle'});
  const activate = page => page.getByRole('button', {name: 'Open the live graph', exact: true}).click();
  const state = (page, expected) => page.waitForFunction(expected => document.querySelector('[data-graph-sandbox]').dataset.sandboxState === expected, expected, {timeout: 60000, polling: 100});

  await check('desktop', {}, async page => {
    const diagnostics = [];
    page.on('console', msg => { if (msg.type() === 'error') diagnostics.push(msg.text()); });
    await page.goto(`${baseUrl}/repos/?overlay_observe=1&dataset=/untrusted-dataset.json`, {waitUntil: 'networkidle'});
    const title = await page.title();
    await staticReading(page);
    assert.deepEqual(await heavy(page), []);
    const authority = await page.locator('#repository-graph-data').evaluate(el => JSON.parse(el.textContent));
    results.repositories = authority.nodes.length;
    results.relationships = authority.edges.length;
    await activate(page);
    await state(page, 'ready');
    const viewer = page.locator('graphshell-tree');
    assert.equal(await viewer.getAttribute('data-ready'), 'true');
    assert.equal(await page.title(), title);
    await staticReading(page);
    await viewer.getByRole('button', {name: 'Choose projects', exact: true}).press('Enter');
    const choices = viewer.getByRole('button', {name: /^Select /});
    await choices.first().waitFor({state: 'attached'});
    assert.equal(await choices.count(), results.repositories);
    const selectedProject = authority.nodes[0];
    await viewer.getByRole('button', {name: `Select ${selectedProject.name}`, exact: true}).focus();
    await page.keyboard.press('Enter');
    await viewer.getByRole('button', {name: 'Manage selection', exact: true}).waitFor();
    assert.equal(await viewer.getByRole('button', {name: 'Clear project selection', exact: true}).count(), 0);
    await viewer.getByRole('link', {name: 'Project page', exact: true}).first().waitFor({state: 'attached'});
    assert.ok(await viewer.getByRole('link', {name: 'Project page', exact: true}).count() > 0);
    // Canvas links expose accessible actions, not native anchor hrefs. Bind
    // the expectation to the public disclosure and verify actual navigation.
    const history = JSON.parse(await viewer.getAttribute('data-dataset'));
    const disclosed = history.revisions.at(-1).dataset.occurrences.find(item => item.occurrence_id === selectedProject.id);
    const projectLink = disclosed.values.project_url.value;
    assert.match(projectLink, /^https:\/\/merelyllc\.com\/projects\/[a-z0-9-]+\/$/);
    assert.equal(new URL(projectLink).pathname, `/projects/${selectedProject.id}/`);
    assert.equal((await page.request.get(new URL(new URL(projectLink).pathname, baseUrl).href)).status(), 200);
    results.selected_project = projectLink;
    await viewer.scrollIntoViewIfNeeded();
    const panel = await viewer.getByRole('region', {name: 'Project inspector', exact: true}).boundingBox();
    const graph = viewer.getByRole('group', {name: /^\d+ of \d+ shown$/});
    const graphBox = await graph.boundingBox();
    assert.ok(panel && graphBox && panel.x >= graphBox.x + graphBox.width - 2, 'inspector beside graph');
    await viewer.getByRole('button', {name: 'Graph menu', exact: true}).press('Enter');
    await viewer.getByRole('dialog', {name: 'Graph commands', exact: true}).waitFor();
    await viewer.getByRole('button', {name: 'Close commands', exact: true}).press('Enter');
    await viewer.getByRole('dialog', {name: 'Graph commands', exact: true}).waitFor({state: 'detached'});
    const y = await page.evaluate(() => scrollY);
    await page.mouse.move(graphBox.x + graphBox.width / 2, graphBox.y + graphBox.height / 2);
    await page.mouse.wheel(0, -100);
    await page.waitForTimeout(250);
    assert.equal(await page.evaluate(() => scrollY), y, 'graph wheel must not scroll document');
    const adapter = await page.evaluate(async () => {
      const a = await navigator.gpu.requestAdapter();
      return a ? {vendor: a.info.vendor, architecture: a.info.architecture, device: a.info.device, description: a.info.description} : null;
    });
    assert.ok(adapter, 'viewer requires WebGPU adapter');
    if (process.env.MER3LY_REQUIRE_HARDWARE === 'true') assert.ok(adapter.vendor === 'amd' && !/swiftshader/i.test(JSON.stringify(adapter)), 'local acceptance needs AMD hardware');
    results.adapter = adapter;
    results.browsing_routes = await checkBrowsingRoutes(page, viewer, {pauseForCapture: true});
    // A pointer context menu can change the active subject. Re-establish the
    // disclosed subject before checking its canonical navigation.
    if (await viewer.getByRole('button', {name: 'Manage selection', exact: true}).count()) {
      await viewer.getByRole('button', {name: 'Manage selection', exact: true}).press('Enter');
      await viewer.getByRole('button', {name: 'Clear project selection', exact: true}).press('Enter');
      await viewer.getByRole('region', {name: 'Project inspector', exact: true}).getByRole('status').filter({hasText: /^0 selected/}).waitFor();
    }
    if (await viewer.getByRole('button', {name: 'Choose projects', exact: true}).count()) {
      await viewer.getByRole('button', {name: 'Choose projects', exact: true}).press('Enter');
    }
    const selectedChoice = viewer.getByRole('button', {name: `Select ${selectedProject.name}`, exact: true});
    await selectedChoice.waitFor();
    if (await selectedChoice.getAttribute('aria-pressed') !== 'true') await selectedChoice.press('Enter');
    await page.waitForFunction(name => [...document.querySelectorAll('graphshell-tree [role="button"]')].some(el => el.textContent === `Select ${name}` && el.getAttribute('aria-pressed') === 'true'), selectedProject.name);
    await viewer.getByRole('link', {name: 'Project page', exact: true}).first().waitFor();
    await viewer.getByRole('button', {name: 'Hide project list', exact: true}).press('Enter');
    await selectedChoice.waitFor({state: 'detached'});
    const hideSelection = viewer.getByRole('button', {name: 'Hide selection controls', exact: true});
    if (await hideSelection.count()) {
      await hideSelection.press('Enter');
      await viewer.getByRole('button', {name: 'Manage selection', exact: true}).waitFor();
    }
    await viewer.getByRole('link', {name: 'Project page', exact: true}).first().waitFor();
    results.capture_bounds = await captureViewer(page, viewer, path.join(receiptRoot, 'viewer-desktop-component.png'));
    await page.screenshot({path: path.join(receiptRoot, 'viewer-desktop-viewport.png'), timeout: 90000});
    // Preserve the visible viewer first. SwiftShader can need more than the
    // default 30 seconds to capture the entire long repository document.
    // A capture timeout still fails acceptance; this is a finite capture budget.
    await page.screenshot({path: path.join(receiptRoot, 'viewer-desktop.png'), fullPage: true, timeout: 90000});
    results.screenshots.push('viewer-desktop.png', 'viewer-desktop-component.png', 'viewer-desktop-viewport.png');
    assert.deepEqual(diagnostics, [], 'live viewer console errors');
    results.resources = await heavy(page);
    assert.ok(results.resources.some(url => url.includes('graphshell_web_bg.wasm')));
    assert.equal(await page.evaluate(() => performance.getEntriesByType('resource').some(r => r.name.includes('untrusted-dataset'))), false);
    // Exercise the real canonical navigation against the frozen local artifact,
    // including subsequent assets, rather than mixing in a deployed revision.
    await page.route('https://merelyllc.com/**', async route => {
      const url = new URL(route.request().url());
      const response = await page.request.get(new URL(url.pathname + url.search, baseUrl).href);
      await route.fulfill({response});
    });
    await viewer.getByRole('link', {name: 'Project page', exact: true}).first().press('Enter');
    await page.waitForURL(new URL(projectLink, baseUrl).href);
    assert.equal(await page.locator('h1').count(), 1);
  });
  await check('no-script', {javaScriptEnabled: false}, async page => {
    await goto(page); await staticReading(page); assert.deepEqual(await heavy(page), []);
    assert.equal(await page.locator('.frozen-grid').isVisible(), false);
    const matrixSummary = page.locator('[data-static-matrix] > summary');
    await matrixSummary.focus(); await page.keyboard.press('Enter');
    assert.equal(await page.locator('.frozen-grid').isVisible(), true);
    const historySummary = page.locator('[data-static-history] > summary');
    await historySummary.focus(); await page.keyboard.press('Space');
    assert.equal(await page.locator('[data-sandbox-history-revision]').first().isVisible(), true);
    await matrixSummary.focus(); await page.keyboard.press('Space');
    assert.equal(await page.locator('.frozen-grid').isVisible(), false);
  });
  await check('mobile', {viewport: {width: 375, height: 812}, isMobile: true, hasTouch: true}, async page => {
    await goto(page); await staticReading(page);
    await page.locator('.graph-sandbox-frozen-scene').tap();
    assert.deepEqual(await heavy(page), [], 'touching the static map does not load the live viewer');
    await page.locator('[data-static-matrix] > summary').click();
    assert.equal(await page.locator('.frozen-grid').isVisible(), true);
    assert.deepEqual(await heavy(page), []);
    assert.equal(await page.locator('[data-sandbox-open]').isVisible(), true);
    await page.screenshot({path: path.join(receiptRoot, 'viewer-mobile-static.png'), fullPage: true});
    results.screenshots.push('viewer-mobile-static.png');
  });
  await check('tablet', {viewport: {width: 1024, height: 768}, isMobile: true, hasTouch: true}, async page => {
    await goto(page); await staticReading(page); await page.locator('[data-static-matrix] > summary').click();
    assert.deepEqual(await heavy(page), []);
    assert.equal(await page.locator('[data-sandbox-open]').isVisible(), true);
  });
  await check('touch-live', {viewport: {width: 390, height: 844}, isMobile: true, hasTouch: true}, async page => {
    await page.addInitScript(() => {
      window.__touchDiagnostics = [];
      const record = (phase, event) => {
        if (window.__touchDiagnostics.length >= 80) return;
        const root = document.querySelector('graphshell-tree');
        const canvas = root?.querySelector('canvas');
        const rect = canvas?.getBoundingClientRect();
        window.__touchDiagnostics.push({phase, type: event.type, x: event.clientX, y: event.clientY,
          pointerType: event.pointerType, button: event.button, target: event.target?.tagName,
          scrollY, prevented: event.defaultPrevented, canvas: rect?.toJSON(),
          overlay: root?.getAttribute('data-graphshell-painted-overlay'),
          beforeEvent: root?.getAttribute('data-graphshell-overlay-before-event')});
      };
      for (const type of ['pointerdown', 'pointerup', 'pointercancel']) {
        document.addEventListener(type, event => record('capture', event), true);
        document.addEventListener(type, event => record('bubble', event));
      }
    });
    await page.goto(`${baseUrl}/repos/?overlay_observe=1`, {waitUntil: 'networkidle'});
    await staticReading(page);
    await page.getByRole('button', {name: 'Open the live graph', exact: true}).tap();
    await state(page, 'ready');
    const viewer = page.locator('graphshell-tree');
    await viewer.scrollIntoViewIfNeeded();
    results.touch_routes = await checkBrowsingRoutes(page, viewer, {touch: true, pauseForCapture: true});
    await staticReading(page);
    await page.screenshot({path: path.join(receiptRoot, 'viewer-touch-live.png'), timeout: 90000});
    results.screenshots.push('viewer-touch-live.png');
  });
  await check('no-webgpu', {}, async page => {
    await page.addInitScript(() => Object.defineProperty(navigator, 'gpu', {value: undefined}));
    await goto(page); await staticReading(page); await page.locator('[data-static-matrix] > summary').click();
    assert.deepEqual(await heavy(page), []);
  });
  await check('no-adapter', {}, async page => {
    await page.addInitScript(() => Object.defineProperty(navigator, 'gpu', {value: {requestAdapter: async () => null}}));
    await goto(page); await activate(page); await state(page, 'unavailable');
    await staticReading(page); assert.deepEqual(await heavy(page), []);
  });
  await check('legacy-share', {}, async page => {
    await page.goto(`${baseUrl}/repos/#graphshell-scene=legacy`, {waitUntil: 'networkidle'});
    await staticReading(page);
    assert.match(await page.locator('[data-sandbox-status]').textContent(), /Cannot import/);
    assert.equal(await page.locator('[data-sandbox-open]').isVisible(), true);
    assert.deepEqual(await heavy(page), []);
    await activate(page); await state(page, 'ready');
    assert.equal(await page.locator('graphshell-tree').getAttribute('data-ready'), 'true');
  });
  await check('startup-timeout', {}, async page => {
    const diagnostics = [];
    page.on('console', msg => { if (msg.type() === 'error') diagnostics.push(msg.text()); });
    await page.addInitScript(() => {
      window.__rafRequests = 0;
      const raf = window.requestAnimationFrame.bind(window);
      window.requestAnimationFrame = callback => { window.__rafRequests++; return raf(callback); };
      const request = GPUAdapter.prototype.requestDevice;
      GPUAdapter.prototype.requestDevice = function(...args) {
        window.__heldAdapter = {vendor: this.info.vendor, architecture: this.info.architecture, device: this.info.device, description: this.info.description};
        return request.apply(this, args).then(device => new Promise(resolve => {
          window.__heldDevice = true;
          window.__cleanupDevice = () => device.destroy();
          device.lost.then(info => { window.__deviceLost = {reason: info.reason}; });
          window.__releaseDevice = () => { window.__releaseDevice = null; resolve(device); };
        }));
      };
    });
    try {
      await goto(page); await activate(page);
      await page.waitForFunction(() => window.__heldDevice === true, null, {timeout: 40000, polling: 100});
      const before = await page.evaluate(() => {
        window.__removedRoot = document.querySelector('graphshell-tree');
        return {connected: window.__removedRoot.isConnected, rafRequests: window.__rafRequests};
      });
      assert.equal(before.connected, true);
      // Exercise the real production 45-second deadline, without advancing clocks.
      await state(page, 'unavailable');
      await staticReading(page);
      assert.equal(await page.evaluate(() => window.__removedRoot.isConnected), false);
      await page.evaluate(() => window.__releaseDevice());
      await page.waitForFunction(() => window.__removedRoot.dataset.mountState === 'cancelled' && window.__deviceLost?.reason === 'destroyed', null, {timeout: 60000, polling: 100});
      const after = await page.evaluate(async () => {
        const entry = document.querySelector('[data-graph-sandbox]').dataset.sandboxRuntime;
        const module = await import(new URL('./pkg/graphshell_web.js', new URL(entry, location.href)).href);
        let refusal = null;
        try { module.run_scenario('settle 1\n'); } catch (error) { refusal = String(error); }
        const root = window.__removedRoot;
        return {ready: root.dataset.ready ?? null, mountState: root.dataset.mountState, mirror: root.querySelectorAll('[data-cambium-mirror]').length, fileInputs: root.querySelectorAll('#cambium-file-input').length, rafRequests: window.__rafRequests, deviceLost: window.__deviceLost, adapter: window.__heldAdapter, refusal};
      });
      if (process.env.MER3LY_REQUIRE_HARDWARE === 'true') assert.ok(after.adapter?.vendor === 'amd' && !/swiftshader/i.test(JSON.stringify(after.adapter)), 'timeout control requires actual AMD device');
      assert.notEqual(after.ready, 'true');
      assert.equal(after.mirror, 0); assert.equal(after.fileInputs, 0);
      assert.equal(after.rafRequests, before.rafRequests);
      assert.match(after.refusal, /tree has not booted/);
      assert.deepEqual(diagnostics, []);
      results.startup_cancellation = {before, after, deadline_ms: 45000};
    } finally {
      await page.evaluate(() => window.__cleanupDevice?.()).catch(() => {});
    }
  });
  for (const failure of ['forced', 'history', 'wasm', 'stylesheet']) {
    await check(`failure-${failure}`, {}, async page => {
      if (failure === 'history') await page.route('**/repository-host-history.json?*', route => route.fulfill({status: 503, body: 'unavailable'}));
      if (failure === 'wasm') await page.route('**/graphshell_web_bg.wasm', route => route.abort());
      if (failure === 'stylesheet') await page.route('**/graphshell/*/styles.css', route => route.abort());
      await page.goto(`${baseUrl}/repos/${failure === 'forced' ? '?graph-sandbox=no-wasm' : ''}`, {waitUntil: 'networkidle'});
      await activate(page); await state(page, 'unavailable'); await staticReading(page);
      assert.equal(await page.locator('[data-sandbox-interface]').isHidden(), true);
      if (failure === 'forced' || failure === 'history') assert.deepEqual(await heavy(page), []);
    });
  }
  return results;
}
