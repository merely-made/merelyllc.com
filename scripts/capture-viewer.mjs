import assert from 'node:assert/strict';

// Capture the measured document rectangle without locator.screenshot's extra
// scroll/actionability cycle. The full-page capture can leave a software GPU
// waiting for animation frames even though layout and presented pixels are ready.
export async function captureViewer(page, viewer, destination) {
  const bounds = () => viewer.evaluate(element => {
    const r = element.getBoundingClientRect();
    return {x: r.x + scrollX, y: r.y + scrollY, width: r.width, height: r.height};
  });
  const before = await bounds();
  assert.ok(before.width > 0 && before.height > 0, 'viewer capture has positive extent');
  await page.waitForTimeout(100);
  const clip = await bounds();
  assert.deepEqual(clip, before, 'viewer capture bounds remain stable');
  await page.screenshot({path: destination, fullPage: true, clip});
  assert.deepEqual(await bounds(), clip, 'viewer capture preserves layout');
  return clip;
}
