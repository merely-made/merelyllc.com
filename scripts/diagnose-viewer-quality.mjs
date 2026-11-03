import {browserArguments} from './browser-launch.mjs';
import {chromium} from 'playwright';
import {createServer} from 'node:http';
import {readFile, writeFile} from 'node:fs/promises';
import path from 'node:path';

const root = path.resolve('.tmp/viewer-linux-site');
const out = path.resolve('.tmp/viewer-linux');
const server = createServer(async (req, res) => {
  try {
    let pathname = decodeURIComponent(new URL(req.url, 'http://localhost').pathname);
    if (pathname.endsWith('/')) pathname += 'index.html';
    const file = path.resolve(root, '.' + pathname);
    if (!file.startsWith(root + path.sep)) throw new Error('outside root');
    const bytes = await readFile(file);
    res.setHeader('Content-Type', ({'.html': 'text/html', '.js': 'text/javascript', '.css': 'text/css', '.json': 'application/json', '.wasm': 'application/wasm', '.svg': 'image/svg+xml'})[path.extname(file)] || 'application/octet-stream');
    res.end(bytes);
  } catch { res.writeHead(404); res.end(); }
});
await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
const browser = await chromium.launch({headless: false, args: browserArguments({headless: false, softwareGpu: true})});
const page = await browser.newPage({viewport: {width: 1440, height: 1000}});
const errors = [];
page.on('pageerror', error => errors.push(String(error)));
page.on('console', message => { if (message.type() === 'error') errors.push(message.text()); });
try {
  await page.addInitScript(() => {
    const probe = window.__qualityProbe = {started: performance.now(), submissions: 0, maps: 0, resolved: 0, rejected: 0, unmaps: 0, drained: 0, pending: 0, peak: 0, first: [], last: [], failures: [], devices: [], samples: []};
    const requestDevice = GPUAdapter.prototype.requestDevice;
    GPUAdapter.prototype.requestDevice = async function (...args) {
      const device = await requestDevice.apply(this, args);
      device.addEventListener("uncapturederror", event => probe.failures.push(String(event.error)));
      device.lost.then(info => probe.devices.push({reason: info.reason, message: info.message}));
      return device;
    };
    const submit = GPUQueue.prototype.submit;
    GPUQueue.prototype.submit = function (...args) { probe.submissions++; return submit.apply(this, args); };
    const map = GPUBuffer.prototype.mapAsync;
    GPUBuffer.prototype.mapAsync = function (...args) {
      const row = {n: ++probe.maps, label: this.label, size: this.size, at: performance.now() - probe.started};
      probe.pending++; probe.peak = Math.max(probe.peak, probe.pending);
      if (probe.first.length < 12) probe.first.push(row);
      probe.last.push(row); if (probe.last.length > 12) probe.last.shift();
      // Delay delivery to the Rust owner even on fast GPUs. The former host
      // exhausted all 256 slots while the first cold shader was still running.
      const original = map.apply(this, args);
      const promise = this.label === 'download'
        ? original.then(() => new Promise(resolve => setTimeout(resolve, 5000)))
        : original;
      promise.then(() => { probe.resolved++; probe.pending--; row.done = performance.now() - probe.started; }, error => { probe.rejected++; probe.pending--; if (probe.failures.length < 12) probe.failures.push(String(error)); });
      return promise;
    };
    const unmap = GPUBuffer.prototype.unmap;
    GPUBuffer.prototype.unmap = function (...args) { probe.unmaps++; if (this.label === 'download') probe.drained++; return unmap.apply(this, args); };
  });
  await page.goto(`http://127.0.0.1:${server.address().port}/repos/`, {waitUntil: 'networkidle'});
  await page.evaluate(async () => {
    const device = await (await navigator.gpu.requestAdapter()).requestDevice();
    const buffer = device.createBuffer({size: 32, usage: GPUBufferUsage.MAP_READ | GPUBufferUsage.COPY_DST});
    const encoder = device.createCommandEncoder(); encoder.clearBuffer(buffer); device.queue.submit([encoder.finish()]);
    await Promise.race([buffer.mapAsync(GPUMapMode.READ), new Promise((_, reject) => setTimeout(() => reject(new Error('Independent GPU readback timed out')), 20000))]); buffer.unmap(); buffer.destroy(); device.destroy();
  });
  await page.getByRole('button', {name: 'Open the live graph', exact: true}).click();
  await page.waitForFunction(() => ['ready', 'unavailable'].includes(document.querySelector('[data-graph-sandbox]')?.dataset.sandboxState), null, {timeout: 60000, polling: 100}).catch(error => errors.push(String(error)));
  const initial = await page.evaluate(() => ({state: document.querySelector('[data-graph-sandbox]')?.dataset.sandboxState, now: performance.now(), probe: window.__qualityProbe}));
  for (let n = 0; n < 4; n++) {
    await page.waitForTimeout(10000);
    await page.evaluate(() => { const p = window.__qualityProbe; p.samples.push({at: performance.now() - p.started, maps: p.maps, resolved: p.resolved, pending: p.pending}); });
  }
  const settled = await page.evaluate(() => ({state: document.querySelector('[data-graph-sandbox]')?.dataset.sandboxState, now: performance.now(), probe: window.__qualityProbe}));
  await writeFile(path.join(out, 'quality-readback-diagnosis.json'), JSON.stringify({browser: browser.version(), errors, initial, settled}, null, 2));
  console.log(JSON.stringify({errors, initial, settled}));
  if (errors.length || settled.state !== 'ready' || settled.probe.peak > 4
      || settled.probe.resolved < 4 || settled.probe.drained < 2 || settled.probe.failures.length) {
    throw new Error('Viewer must remain ready and make bounded readback progress with delayed completion');
  }
} finally { await browser.close(); await new Promise(resolve => server.close(resolve)); }
