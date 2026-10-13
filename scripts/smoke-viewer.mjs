import assert from 'node:assert/strict';
import path from 'node:path';

// Runs inside the caller's finite browser lease. Every case gets a fresh page
// and closes it; failure evidence belongs to the caller's receipt directory.
export async function checkQualifiedViewer(browser, baseUrl, receiptRoot) {
  const results = {cases: {}, screenshots: []};
  async function check(name, options, run) {
    const page = await browser.newPage({viewport: {width: 1440, height: 1000}, ...options});
    const errors = [];
    page.on('pageerror', error => errors.push(String(error)));
    try {
      await run(page);
      assert.deepEqual(errors, [], `${name}: uncaught page errors`);
      results.cases[name] = 'pass';
    } catch (error) {
      await page.screenshot({path: path.join(receiptRoot, `viewer-${name}-failure.png`), fullPage: true}).catch(() => {});
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
  const state = (page, expected) => page.waitForFunction(expected => document.querySelector('[data-graph-sandbox]').dataset.sandboxState === expected, expected, {timeout: 60000});

  await check('desktop', {}, async page => {
    const diagnostics = [];
    page.on('console', msg => { if (msg.type() === 'error') diagnostics.push(msg.text()); });
    await page.goto(`${baseUrl}/repos/?dataset=/untrusted-dataset.json`, {waitUntil: 'networkidle'});
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
    await viewer.getByRole('button', {name: 'Choose projects', exact: true}).click();
    const choices = viewer.getByRole('button', {name: /^Select /});
    assert.equal(await choices.count(), results.repositories);
    await choices.first().focus();
    await page.keyboard.press('Enter');
    await viewer.getByRole('button', {name: 'Clear project selection', exact: true}).waitFor();
    assert.ok(await viewer.getByRole('link', {name: 'Project page', exact: true}).count() > 0);
    const projectLink = await viewer.getByRole('link', {name: 'Project page', exact: true}).first().getAttribute('href');
    assert.match(projectLink, /^https:\/\/merelyllc\.com\/projects\/[a-z0-9-]+\/$/);
    assert.equal((await page.request.get(new URL(new URL(projectLink).pathname, baseUrl).href)).status(), 200);
    results.selected_project = projectLink;
    await viewer.scrollIntoViewIfNeeded();
    const panel = await viewer.getByRole('region', {name: 'Project inspector', exact: true}).boundingBox();
    const graph = viewer.getByRole('group', {name: /^\d+ of \d+ shown$/});
    const graphBox = await graph.boundingBox();
    assert.ok(panel && graphBox && panel.x >= graphBox.x + graphBox.width - 2, 'inspector beside graph');
    await viewer.getByRole('button', {name: 'Graph commands', exact: true}).press('Enter');
    await viewer.getByRole('dialog', {name: 'Graph commands', exact: true}).waitFor();
    await viewer.getByRole('button', {name: 'Close commands', exact: true}).press('Enter');
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
    await page.screenshot({path: path.join(receiptRoot, 'viewer-desktop.png'), fullPage: true});
    await viewer.screenshot({path: path.join(receiptRoot, 'viewer-desktop-component.png')});
    await page.screenshot({path: path.join(receiptRoot, 'viewer-desktop-viewport.png')});
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
    await viewer.getByRole('link', {name: 'Project page', exact: true}).first().click();
    await page.waitForURL(new URL(projectLink, baseUrl).href);
    assert.equal(await page.locator('h1').count(), 1);
  });
  await check('no-script', {javaScriptEnabled: false}, async page => {
    await goto(page); await staticReading(page); assert.deepEqual(await heavy(page), []);
  });
  await check('mobile', {viewport: {width: 375, height: 812}, isMobile: true, hasTouch: true}, async page => {
    await goto(page); await staticReading(page);
    await page.locator('.frozen-grid').click();
    assert.deepEqual(await heavy(page), []);
    assert.equal(await page.locator('[data-sandbox-open]').isHidden(), true);
    await page.screenshot({path: path.join(receiptRoot, 'viewer-mobile-static.png'), fullPage: true});
    results.screenshots.push('viewer-mobile-static.png');
  });
  await check('tablet', {viewport: {width: 1024, height: 768}, isMobile: true, hasTouch: true}, async page => {
    await goto(page); await staticReading(page); await page.locator('.frozen-grid').click();
    assert.deepEqual(await heavy(page), []);
    assert.equal(await page.locator('[data-sandbox-open]').isHidden(), true);
  });
  await check('no-webgpu', {}, async page => {
    await page.addInitScript(() => Object.defineProperty(navigator, 'gpu', {value: undefined}));
    await goto(page); await staticReading(page); await page.locator('.frozen-grid').click();
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
    assert.match(await page.locator('[data-sandbox-status]').textContent(), /cannot import/);
    assert.deepEqual(await heavy(page), []);
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
