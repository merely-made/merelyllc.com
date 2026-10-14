import assert from 'node:assert/strict';

// Exercise browser input against the retained command surface, not DOM clicks
// on the pointer-blind accessibility mirror.
export async function checkBrowsingRoutes(page, viewer, {touch = false} = {}) {
  const menu = viewer.getByRole('dialog', {name: 'Graph commands', exact: true});
  const trigger = viewer.getByRole('button', {name: 'Graph menu', exact: true});
  const search = viewer.getByRole('textbox', {name: 'Search graph commands', exact: true});
  const graph = viewer.getByRole('group', {name: /^\d+ of \d+ shown$/});
  const results = [];
  async function waitOpen() {
    await menu.waitFor(); await search.waitFor();
    assert.equal(await search.evaluate(e => e === document.activeElement), true, 'menu search receives focus');
  }
  async function closed() {
    await menu.waitFor({state: 'detached'});
    await page.waitForFunction(() => document.activeElement?.getAttribute('role') === 'group', null, {polling: 100});
    assert.equal(await graph.evaluate(e => e === document.activeElement), true, 'focus returns to this graph');
  }
  async function run(query) {
    await search.pressSequentially(query);
    await search.press('Enter');
    await closed();
  }
  async function tap(locator) {
    const box = await locator.boundingBox();
    const viewport = page.viewportSize();
    assert.ok(box && box.width > 0 && box.height >= 44, 'touch target is at least 44px high');
    assert.ok(box.x >= 0 && box.y >= 0 && box.x + box.width <= viewport.width + 1 && box.y + box.height <= viewport.height + 1, 'touch target is visible inside viewport');
    if (await menu.count() && !await locator.evaluate(e => e.textContent === 'Graph menu')) {
      const clip = await menu.boundingBox();
      assert.ok(clip && box.x >= clip.x && box.y >= clip.y &&
        box.x + box.width <= clip.x + clip.width + 1 && box.y + box.height <= clip.y + clip.height + 1,
        'touch target is fully inside the painted menu clip');
    }
    await page.evaluate(({box, label}) => {
      window.__touchDiagnostics ??= [];
      if (window.__touchDiagnostics.length < 80) window.__touchDiagnostics.push({phase: 'intended-tap', label, box, scrollY});
    }, {box, label: await locator.textContent()});
    await page.touchscreen.tap(box.x + box.width / 2, box.y + box.height / 2);
  }
  async function touchCommand(label) {
    for (let pageNumber = 0; pageNumber < 40; pageNumber++) {
      const option = menu.getByRole('option', {name: label, exact: true});
      if (await option.count()) { await tap(option); await closed(); return; }
      const next = menu.getByRole('button', {name: 'Next commands', exact: true});
      assert.equal(await next.count(), 1, `touch can reach ${label}`);
      await tap(next);
      await page.waitForTimeout(80);
    }
    assert.fail(`touch command paging did not reach ${label}`);
  }
  let resumePhysics = false;
  async function openMenu() {
    if (touch) await tap(trigger); else await trigger.press('Enter');
    await waitOpen();
  }
  async function command(query) { await openMenu(); await run(query); }
  async function geometry() {
    // These are the shared producer's painted-node rectangles exposed by its
    // accessibility mirror, measured relative to the graph's viewport.
    await page.waitForTimeout(120);
    const rows = await graph.evaluate(root => {
      const origin = root.getBoundingClientRect();
      return [...root.querySelectorAll('[role="group"]')].map(node => {
        const box = node.getBoundingClientRect();
        return {name: node.getAttribute('aria-label'), x: box.x - origin.x,
          y: box.y - origin.y, width: box.width, height: box.height};
      }).sort((a, b) => String(a.name).localeCompare(String(b.name)));
    });
    assert.ok(rows.length > 1, 'shared producer exposes painted project rectangles');
    return rows;
  }
  function difference(a, b) {
    if (a.length !== b.length || a.some((row, i) => row.name !== b[i].name)) return Infinity;
    return Math.max(...a.flatMap((row, i) => ['x', 'y', 'width', 'height'].map(key => Math.abs(row[key] - b[i][key]))));
  }
  async function perturbForFit() {
    await openMenu();
    await search.pressSequentially('physics');
    if (await menu.getByRole('option', {name: 'Pause physics', exact: true}).count()) {
      await run(''); resumePhysics = true;
    } else { await search.press('Escape'); await closed(); }
    await command('Fit visible graph');
    const fitted = await geometry();
    await command('Zoom in'); await command('Zoom in'); await command('Pan right');
    const changed = await geometry();
    assert.ok(difference(fitted, changed) > 8, 'controlled zoom/pan changes the painted viewport');
    return {fitted, changed};
  }
  async function verifyFit(prepared, route) {
    const restored = await geometry();
    assert.ok(difference(prepared.changed, restored) > 8, `${route}: Fit changes the displaced viewport`);
    assert.ok(difference(prepared.fitted, restored) <= 2, `${route}: Fit restores the fitted project rectangles`);
    results.push({route, fit_geometry: {...prepared, restored}});
    await page.evaluate(rows => { window.__browsingRouteEvidence = rows; }, results);
  }
  // The mirror exposes semantic nodes, not the retained stylesheet classes.
  for (const label of ['Graph commands', 'Fit visible graph', 'Graph tools']) {
    assert.equal(await viewer.getByRole('button', {name: label, exact: true}).count(), 0, `browse mode omits the old ${label} toolbar button`);
  }
  assert.equal(await trigger.count(), 1, 'one shared Graph menu entry');
  assert.equal(await viewer.getByRole('region', {name: 'History', exact: true}).count(), 0, 'history starts closed');
  if (touch) {
    const prepared = await perturbForFit();
    await tap(trigger); await waitOpen();
    await tap(menu.getByRole('option', {name: /^Fit visible graph/})); await closed();
    await verifyFit(prepared, 'touch-menu-fit-focus');
    await tap(trigger); await waitOpen();
    await tap(menu.getByRole('button', {name: 'Next commands', exact: true}));
    await menu.getByRole('button', {name: 'Previous commands', exact: true}).waitFor();
    await tap(menu.getByRole('button', {name: 'Previous commands', exact: true}));
    await menu.getByRole('option', {name: 'Fit visible graph', exact: true}).waitFor();
    await tap(menu.getByRole('button', {name: 'Close commands', exact: true})); await closed();
    results.push('touch-next-previous-close');
    await tap(trigger); await waitOpen();
    await touchCommand('Graph tools');
    await viewer.getByRole('button', {name: 'Apply arrangement', exact: true}).waitFor();
    await tap(trigger); await waitOpen();
    await touchCommand('Graph tools');
    await viewer.getByRole('button', {name: 'Apply arrangement', exact: true}).waitFor({state: 'detached'});
    results.push('touch-tools');
    await tap(trigger); await waitOpen();
    await tap(menu.getByRole('button', {name: 'Close commands', exact: true})); await closed();
    results.push('touch-dismiss-focus');
  } else {
    for (const shortcut of ['Control+k', 'Meta+k', 'Shift+F10']) {
      await graph.focus(); await page.keyboard.press(shortcut); await waitOpen();
      await search.press('Escape'); await closed(); results.push(shortcut);
    }
    await openMenu();
    await search.press('ArrowUp');
    const selected = menu.locator('[role="option"][aria-selected="true"]');
    await selected.waitFor();
    assert.ok(await menu.getByRole('option').count() <= 2, 'keyboard selection reveals a bounded command page');
    assert.ok(await menu.getByRole('button', {name: 'Previous commands', exact: true}).count(), 'ArrowUp wraps to the final command page');
    await search.press('ArrowDown');
    await menu.getByRole('option', {name: 'Fit visible graph', exact: true}).waitFor();
    await search.press('Escape'); await closed(); results.push('keyboard-command-pages');
    const prepared = await perturbForFit();
    const box = await graph.boundingBox();
    assert.ok(box);
    await page.mouse.click(box.x + box.width / 2, box.y + box.height / 2, {button: 'right'});
    await waitOpen(); await run('Fit visible graph'); await verifyFit(prepared, 'context-menu-fit-focus');
    await trigger.focus(); await page.keyboard.press('Enter'); await waitOpen();
    await run('Graph tools');
    await viewer.getByRole('button', {name: 'Apply arrangement', exact: true}).waitFor();
    await graph.focus(); await page.keyboard.press('Control+k'); await waitOpen(); await run('Graph tools');
    await viewer.getByRole('button', {name: 'Apply arrangement', exact: true}).waitFor({state: 'detached'});
    results.push('keyboard-tools');
    await trigger.press('Enter'); await waitOpen(); await run('Show history');
    await viewer.getByRole('region', {name: 'History', exact: true}).waitFor();
    const caption = await viewer.getByRole('region', {name: 'History', exact: true}).getByRole('status').textContent();
    assert.match(caption, /Current checkpoint \d+ of \d+/);
    assert.doesNotMatch(caption, /\{|"commit"/);
    await viewer.getByRole('button', {name: 'Show checkpoint source', exact: true}).press('Enter');
    await viewer.getByRole('button', {name: 'Hide checkpoint source', exact: true}).waitFor();
    assert.match(await viewer.getByRole('region', {name: 'History', exact: true}).textContent(), /Checkpoint/);
    await trigger.press('Enter'); await waitOpen(); await run('Hide history');
    await viewer.getByRole('region', {name: 'History', exact: true}).waitFor({state: 'detached'});
    results.push('history-disclosure');
  }
  if (resumePhysics) await command('Play physics');
  return results;
}
