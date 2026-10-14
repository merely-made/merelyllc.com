import assert from 'node:assert/strict';

// Capture the measured document rectangle without locator.screenshot's extra
// scroll/actionability cycle. Stable DOM bounds are a layout check, not proof
// of completed GPU presentation. Software-GPU capture stalls remain under investigation.
export async function captureViewer(page, viewer, destination) {
  const bounds = () => viewer.evaluate(element => {
    const r = element.getBoundingClientRect();
    return {x: r.x + scrollX, y: r.y + scrollY, width: r.width, height: r.height};
  });
  const before = await bounds();
  // Preserve observations before capture so a timeout does not erase them.
  await page.evaluate(() => {
    window.__captureDiagnostics = {
      fonts: document.fonts.status,
      visibility: document.visibilityState,
      observedOverlay: document.querySelector('graphshell-tree')?.getAttribute('data-graphshell-painted-overlay'),
      capturedAt: performance.now(),
    };
  });
  assert.ok(before.width > 0 && before.height > 0, 'viewer capture has positive extent');
  await page.waitForTimeout(100);
  const clip = await bounds();
  assert.deepEqual(clip, before, 'viewer capture bounds remain stable');
  await page.screenshot({path: destination, fullPage: true, clip, timeout: 90000});
  assert.deepEqual(await bounds(), clip, 'viewer capture preserves layout');
  return clip;
}
