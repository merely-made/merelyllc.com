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
    await page.touchscreen.tap(box.x + box.width / 2, box.y + box.height / 2);
  }
  assert.equal(await viewer.locator('.tree-controls').count(), 0, 'browse mode has no button row');
  assert.equal(await viewer.getByRole('region', {name: 'History', exact: true}).count(), 0, 'history starts closed');
  if (touch) {
    await tap(trigger); await waitOpen();
    await tap(menu.getByRole('option', {name: /^Fit visible graph/})); await closed();
    results.push('touch-menu-fit-focus');
    await tap(trigger); await waitOpen();
    await tap(menu.getByRole('option', {name: 'Graph tools', exact: true})); await closed();
    await viewer.getByRole('region', {name: 'Graph tools', exact: true}).waitFor();
    await tap(trigger); await waitOpen();
    await tap(menu.getByRole('option', {name: 'Graph tools', exact: true})); await closed();
    await viewer.getByRole('region', {name: 'Graph tools', exact: true}).waitFor({state: 'detached'});
    results.push('touch-tools');
    await tap(trigger); await waitOpen();
    await tap(menu.getByRole('button', {name: 'Close commands', exact: true})); await closed();
    results.push('touch-dismiss-focus');
  } else {
    for (const shortcut of ['Control+k', 'Meta+k', 'Shift+F10']) {
      await graph.focus(); await page.keyboard.press(shortcut); await waitOpen();
      await search.press('Escape'); await closed(); results.push(shortcut);
    }
    const box = await graph.boundingBox();
    assert.ok(box);
    await page.mouse.click(box.x + box.width / 2, box.y + box.height / 2, {button: 'right'});
    await waitOpen(); await run('Fit visible graph'); results.push('context-menu-fit-focus');
    await trigger.focus(); await page.keyboard.press('Enter'); await waitOpen();
    await run('Graph tools');
    await viewer.getByRole('region', {name: 'Graph tools', exact: true}).waitFor();
    await graph.focus(); await page.keyboard.press('Control+k'); await waitOpen(); await run('Graph tools');
    await viewer.getByRole('region', {name: 'Graph tools', exact: true}).waitFor({state: 'detached'});
    results.push('keyboard-tools');
    await trigger.press('Enter'); await waitOpen(); await run('Show history');
    await viewer.getByRole('region', {name: 'History', exact: true}).waitFor();
    assert.doesNotMatch(await viewer.locator('.history-value').textContent(), /\{|"commit"/);
    await viewer.getByRole('button', {name: 'Show checkpoint source', exact: true}).press('Enter');
    await viewer.getByRole('button', {name: 'Hide checkpoint source', exact: true}).waitFor();
    assert.match(await viewer.getByRole('region', {name: 'History', exact: true}).textContent(), /Checkpoint/);
    await trigger.press('Enter'); await waitOpen(); await run('Hide history');
    await viewer.getByRole('region', {name: 'History', exact: true}).waitFor({state: 'detached'});
    results.push('history-disclosure');
  }
  return results;
}
