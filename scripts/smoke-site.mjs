import assert from "node:assert/strict";
import { createServer } from "node:http";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";

import { chromium } from "playwright";

const siteRoot = path.resolve(process.env.MER3LY_SITE_DIR ?? "html");
const receiptRoot = path.resolve(
  process.env.MER3LY_RECEIPT_DIR ?? ".tmp/browser-smoke",
);
const headless = process.env.MER3LY_HEADLESS !== "false";
const browserChannel = process.env.MER3LY_BROWSER_CHANNEL ?? "chromium";
const mimeTypes = {
  ".css": "text/css; charset=utf-8",
  ".html": "text/html; charset=utf-8",
  ".jpg": "image/jpeg",
  ".js": "text/javascript; charset=utf-8",
  ".json": "application/json; charset=utf-8",
  ".png": "image/png",
  ".svg": "image/svg+xml",
  ".txt": "text/plain; charset=utf-8",
  ".wasm": "application/wasm",
  ".xml": "application/xml; charset=utf-8",
};

await mkdir(receiptRoot, { recursive: true });

const server = createServer(async (request, response) => {
  try {
    const url = new URL(request.url, "http://127.0.0.1");
    let pathname = decodeURIComponent(url.pathname);
    if (pathname === "/") pathname = "/index.html";
    if (pathname === "/favicon.ico") {
      response.writeHead(204);
      response.end();
      return;
    }
    if (pathname === "/radio" || pathname === "/radio/") {
      pathname = "/radio.html";
    }
    if (pathname.endsWith("/")) pathname += "index.html";
    const candidate = path.resolve(siteRoot, `.${pathname}`);
    const rootPrefix = `${siteRoot}${path.sep}`;
    if (candidate !== siteRoot && !candidate.startsWith(rootPrefix)) {
      response.writeHead(403);
      response.end("forbidden");
      return;
    }
    const bytes = await readFile(candidate);
    response.writeHead(200, {
      "Cache-Control": "no-store",
      "Content-Type":
        mimeTypes[path.extname(candidate)] ?? "application/octet-stream",
    });
    response.end(bytes);
  } catch {
    response.writeHead(404);
    response.end("not found");
  }
});

await new Promise((resolve, reject) => {
  server.once("error", reject);
  server.listen(0, "127.0.0.1", resolve);
});

const port = server.address().port;
const baseUrl = `http://127.0.0.1:${port}`;
const browser = await chromium.launch({
  channel: browserChannel,
  headless,
  args: headless
    ? ["--enable-unsafe-webgpu", "--use-angle=swiftshader"]
    : ["--enable-unsafe-webgpu"],
});

const receipt = {
  schema: "mer3ly.browser-smoke-receipt/v3",
  source_sha: process.env.GITHUB_SHA ?? "local",
  browser: `Chromium ${browser.version()}`,
  browser_channel: browserChannel,
  mode: headless ? "headless" : "headed",
  routes: {},
  desktop: {},
  mobile: {},
  reduced_motion: {},
  fallback: {},
  showcase: {},
  message_path_lab: {},
  radio_bench: {},
  projects: {},
  discovery: {},
  graph_sandbox: {},
};

try {
  const sitemapResponse = await fetch(`${baseUrl}/sitemap.xml`);
  assert.equal(sitemapResponse.status, 200);
  assert.match(
    sitemapResponse.headers.get("content-type") ?? "",
    /^application\/xml/,
  );
  const sitemapText = await sitemapResponse.text();
  const sitemapUrls = [...sitemapText.matchAll(/<loc>([^<]+)<\/loc>/g)].map(
    (match) => match[1],
  );
  assert.ok(sitemapUrls.length > 6);
  assert.equal(new Set(sitemapUrls).size, sitemapUrls.length);
  assert.equal(
    sitemapUrls.every((url) => url.startsWith("https://merelyllc.com/")),
    true,
  );
  for (const unsupported of ["lastmod", "changefreq", "priority"]) {
    assert.equal(sitemapText.includes(unsupported), false);
  }

  const robotsResponse = await fetch(`${baseUrl}/robots.txt`);
  assert.equal(robotsResponse.status, 200);
  assert.match(
    robotsResponse.headers.get("content-type") ?? "",
    /^text\/plain/,
  );
  assert.equal(
    await robotsResponse.text(),
    "User-agent: *\nAllow: /\nSitemap: https://merelyllc.com/sitemap.xml\n",
  );

  const faviconResponse = await fetch(`${baseUrl}/favicon.svg`);
  assert.equal(faviconResponse.status, 200);
  assert.match(
    faviconResponse.headers.get("content-type") ?? "",
    /^image\/svg\+xml/,
  );
  assert.ok((await faviconResponse.arrayBuffer()).byteLength > 0);
  receipt.discovery = {
    sitemap_urls: sitemapUrls.length,
    robots_policy: "allow-public",
    favicon: "favicon.svg",
  };

  for (const route of [
    "/",
    "/radio.html",
    "/devices/",
    "/devices/v4-desktop-radio/",
    "/devices/t114-field-radio/",
    "/projects/mere/",
    "/projects/retinue/",
  ]) {
    const page = await browser.newPage({ viewport: { width: 900, height: 900 } });
    const diagnostics = collectDiagnostics(page);
    const response = await page.goto(`${baseUrl}${route}`, {
      waitUntil: "networkidle",
    });
    assert.equal(response?.status(), 200, `${route} did not return 200`);
    assert.equal(await page.locator("h1").count(), 1, `${route} needs one h1`);
    assert.equal(await horizontalOverflow(page), 0, `${route} overflowed`);
    assert.deepEqual(diagnostics, [], `${route} emitted browser errors`);
    receipt.routes[route] = { status: 200, horizontal_overflow: 0 };
    await page.close();
  }

  const showcaseDesktop = await browser.newPage({
    viewport: { width: 1440, height: 1000 },
  });
  const showcaseDesktopDiagnostics = collectDiagnostics(showcaseDesktop);
  await showcaseDesktop.goto(`${baseUrl}/`, { waitUntil: "networkidle" });
  assert.equal(
    await showcaseDesktop.locator(".home-showcase-card").count(),
    5,
  );
  const showcaseImages = showcaseDesktop.locator(".home-showcase-figure img");
  for (const image of await showcaseImages.all()) {
    await image.scrollIntoViewIfNeeded();
  }
  await showcaseDesktop.waitForFunction(() =>
    [...document.querySelectorAll(".home-showcase-figure img")].every(
      (image) => image.complete,
    ),
  );
  assert.equal(
    await showcaseImages.evaluateAll((images) =>
      images.every(
        (image) =>
          image.complete && image.naturalWidth > 0 && image.naturalHeight > 0,
      ),
    ),
    true,
    "desktop showcase images did not decode",
  );
  assert.equal(await horizontalOverflow(showcaseDesktop), 0);
  assert.deepEqual(
    showcaseDesktopDiagnostics,
    [],
    "desktop showcase emitted browser errors",
  );
  await showcaseDesktop.screenshot({
    path: path.join(receiptRoot, "home-showcase-desktop.png"),
    fullPage: true,
  });
  receipt.showcase.desktop = {
    cards: 5,
    images: 5,
    horizontal_overflow: 0,
  };
  await showcaseDesktop.close();

  const showcaseMobile = await browser.newPage({
    viewport: { width: 375, height: 812 },
  });
  const showcaseMobileDiagnostics = collectDiagnostics(showcaseMobile);
  await showcaseMobile.goto(`${baseUrl}/`, { waitUntil: "networkidle" });
  const mobileShowcaseImages = showcaseMobile.locator(
    ".home-showcase-figure img",
  );
  for (const image of await mobileShowcaseImages.all()) {
    await image.scrollIntoViewIfNeeded();
  }
  await showcaseMobile.waitForFunction(() =>
    [...document.querySelectorAll(".home-showcase-figure img")].every(
      (image) => image.complete,
    ),
  );
  assert.equal(
    await mobileShowcaseImages.evaluateAll((images) =>
      images.every(
        (image) =>
          image.complete && image.naturalWidth > 0 && image.naturalHeight > 0,
      ),
    ),
    true,
    "mobile showcase images did not decode",
  );
  assert.equal(await horizontalOverflow(showcaseMobile), 0);
  assert.deepEqual(
    showcaseMobileDiagnostics,
    [],
    "mobile showcase emitted browser errors",
  );
  await showcaseMobile.screenshot({
    path: path.join(receiptRoot, "home-showcase-mobile.png"),
    fullPage: true,
  });
  receipt.showcase.mobile = {
    cards: await showcaseMobile.locator(".home-showcase-card").count(),
    horizontal_overflow: 0,
  };
  await showcaseMobile.close();

  const messagePathDesktop = await browser.newPage({
    viewport: { width: 1440, height: 1000 },
  });
  const messagePathDesktopDiagnostics = collectDiagnostics(messagePathDesktop);
  await messagePathDesktop.goto(`${baseUrl}/radio.html`, {
    waitUntil: "networkidle",
  });
  await messagePathDesktop.waitForFunction(
    () =>
      document.querySelector("[data-message-path-lab]")?.dataset.ready ===
      "true",
  );
  // Every expectation below comes from the committed traces, through the
  // page's own inline manifest, not from authored copy.
  const pathManifest = JSON.parse(
    await messagePathDesktop.locator("#message-path-traces").textContent(),
  );
  const coldPath = pathManifest.scenarios.find((scenario) => scenario.id === "cold");
  const warmPath = pathManifest.scenarios.find((scenario) => scenario.id === "warm");
  const pathLab = messagePathDesktop.locator("[data-message-path-lab]");
  const pathStep = pathLab.locator("[data-path-step]");
  const pathStatus = pathLab.locator("[data-path-status]");
  const pathScreenText = async () =>
    (await pathLab.locator("[data-path-screen-text] li").allTextContents()).join(" | ");
  assert.equal(await pathLab.locator("[data-lab-node]").count(), 5);
  assert.equal(await pathLab.locator("[data-lab-edge]").count(), 5);
  assert.equal(
    await pathLab.locator("[data-path-ledger] li").count(),
    coldPath.milestones.length,
  );
  assert.equal(await pathLab.getAttribute("data-scenario"), "cold");
  assert.equal(await pathLab.getAttribute("data-step"), String(coldPath.events - 1));
  assert.equal(await pathLab.getAttribute("data-node"), "garage");
  assert.match(
    await pathLab.locator("[data-path-route]").textContent(),
    /fire station → church steeple → water tower → county garage, 2 relays/,
  );
  assert.equal(
    await pathLab.locator('[data-lab-edge="fire-water"]').getAttribute("class"),
    "message-path-edge is-cut",
  );
  assert.equal(await pathLab.locator("[data-path-screen]").getAttribute("data-screen-name"), "traffic");
  assert.match(await pathScreenText(), /TRAFFIC/);
  assert.equal(await pathLab.locator("[data-path-canvas]").isVisible(), true);

  // Warm cut: the stale route's sends expire, then the reroute after the announce.
  await pathLab.locator("[data-path-scenario]").selectOption("warm");
  await messagePathDesktop.waitForFunction(
    () => document.querySelector("[data-message-path-lab]").dataset.scenario === "warm",
  );
  assert.equal(
    await pathLab.locator("[data-path-ledger] li").count(),
    warmPath.milestones.length,
  );
  await pathStep.focus();
  await pathStep.press("End");
  assert.equal(await pathLab.getAttribute("data-step"), String(warmPath.events - 1));
  assert.match(await pathStatus.textContent(), /Message 5 reaches the county garage/);
  const firstExpiry = warmPath.milestones.find((row) => row.kind === "expired");
  await pathLab.locator(`[data-path-ledger] li[data-lab-event="${firstExpiry.event}"] button`).click();
  assert.equal(await pathLab.getAttribute("data-step"), String(firstExpiry.event));
  // The expiry's time comes from the trace; only its shape is pinned here.
  assert.match(await pathStatus.textContent(), /^\d+\.\d+ s · /);
  assert.ok((await pathStatus.textContent()).endsWith(` · ${firstExpiry.text}`));
  await pathLab.locator('[data-lab-node="fire"]').click();
  assert.equal(await pathLab.getAttribute("data-node"), "fire");
  assert.match(await pathScreenText(), /link unanswered/);
  await pathLab.locator('[data-path-action="previous-milestone"]').click();
  assert.equal(await pathLab.getAttribute("data-event-kind"), "transmit");
  assert.match(
    await pathLab.locator('[data-lab-edge="fire-water"]').getAttribute("class"),
    /is-cut.*is-refused|is-refused.*is-cut/,
  );
  const beforeExpiry = warmPath.milestones.filter((row) => row.event < firstExpiry.event).at(-1);
  assert.ok((await pathStatus.textContent()).endsWith(beforeExpiry.text));
  assert.match(await pathStatus.textContent(), /Behind the cut, the \S.* does not hear it/);

  await pathLab.locator('[data-path-action="play"]').click();
  assert.equal(await pathLab.getAttribute("data-playing"), "true");
  await messagePathDesktop.waitForFunction(
    () => Number(document.querySelector("[data-message-path-lab]").dataset.step) >= 1,
  );
  await pathStep.press("End");
  assert.equal(await pathLab.getAttribute("data-playing"), "false");

  const sharedStep = String(firstExpiry.event);
  await pathLab.locator(`[data-path-ledger] li[data-lab-event="${sharedStep}"] button`).click();
  await pathLab.locator('[data-path-action="share"]').click();
  const sharedMessagePathUrl = new URL(messagePathDesktop.url());
  const sharedMessagePathParams = new URLSearchParams(
    sharedMessagePathUrl.hash.slice(1),
  );
  assert.equal(sharedMessagePathParams.get("message-path"), "v2");
  assert.equal(sharedMessagePathParams.get("trace"), warmPath.trace_sha256.slice(0, 12));
  assert.equal(sharedMessagePathParams.get("scenario"), "warm");
  assert.equal(sharedMessagePathParams.get("step"), sharedStep);
  assert.equal(sharedMessagePathParams.get("node"), "fire");

  const sharedMessagePath = await browser.newPage({
    viewport: { width: 1000, height: 900 },
  });
  const sharedMessagePathDiagnostics = collectDiagnostics(sharedMessagePath);
  await sharedMessagePath.goto(sharedMessagePathUrl.toString(), {
    waitUntil: "networkidle",
  });
  await sharedMessagePath.waitForFunction(
    () =>
      document.querySelector("[data-message-path-lab]")?.dataset.ready ===
      "true",
  );
  const sharedLab = sharedMessagePath.locator("[data-message-path-lab]");
  assert.equal(await sharedLab.getAttribute("data-scenario"), "warm");
  assert.equal(await sharedLab.getAttribute("data-step"), sharedStep);
  assert.equal(await sharedLab.getAttribute("data-node"), "fire");
  assert.equal(await sharedLab.getAttribute("data-link-state"), "none");
  assert.equal(await horizontalOverflow(sharedMessagePath), 0);

  // A link from the authored lab is explained, not misread.
  await sharedMessagePath.goto(
    `${baseUrl}/radio.html?legacy#message-path=v1&blocked=1&step=5&positions=church,40.0,30.0`,
    { waitUntil: "networkidle" },
  );
  await sharedMessagePath.waitForFunction(
    () =>
      document.querySelector("[data-message-path-lab]")?.dataset.ready ===
      "true",
  );
  assert.equal(await sharedLab.getAttribute("data-link-state"), "retired");
  assert.equal(await sharedLab.getAttribute("data-scenario"), "cold");
  assert.equal(await sharedLab.locator("[data-path-notice]").isVisible(), true);
  assert.match(
    await sharedLab.locator("[data-path-notice]").textContent(),
    /older version of the lab/,
  );
  assert.deepEqual(
    sharedMessagePathDiagnostics,
    [],
    "shared message path scene emitted browser errors",
  );
  await sharedMessagePath.close();

  assert.equal(await horizontalOverflow(messagePathDesktop), 0);
  assert.deepEqual(
    messagePathDesktopDiagnostics,
    [],
    "desktop message path lab emitted browser errors",
  );
  await pathLab.screenshot({
    path: path.join(receiptRoot, "message-path-lab-desktop.png"),
  });
  receipt.message_path_lab.desktop = {
    nodes: 5,
    edges: 5,
    traces: pathManifest.scenarios.map((scenario) => ({
      id: scenario.id,
      events: scenario.events,
      ledger_rows: scenario.milestones.length,
    })),
    screen: "traffic",
    shared_step: true,
    retired_link_notice: true,
    horizontal_overflow: 0,
  };
  await messagePathDesktop.close();

  const messagePathMobile = await browser.newPage({
    viewport: { width: 375, height: 812 },
  });
  const messagePathMobileDiagnostics = collectDiagnostics(messagePathMobile);
  await messagePathMobile.goto(`${baseUrl}/radio.html`, {
    waitUntil: "networkidle",
  });
  await messagePathMobile.waitForFunction(
    () =>
      document.querySelector("[data-message-path-lab]")?.dataset.ready ===
      "true",
  );
  const mobilePathLab = messagePathMobile.locator("[data-message-path-lab]");
  const mobileNodeBoxes = await mobilePathLab.locator("[data-lab-node]").evaluateAll(
    (nodes) =>
      nodes.map((node) => {
        const rect = node.getBoundingClientRect();
        return {
          width: rect.width,
          height: rect.height,
          x: rect.left + rect.width / 2,
          y: rect.top + rect.height / 2,
        };
      }),
  );
  assert.equal(
    mobileNodeBoxes.every(({ width, height }) => width >= 44 && height >= 44),
    true,
    "mobile radios need 44px targets",
  );
  const mobileXSpan =
    Math.max(...mobileNodeBoxes.map(({ x }) => x)) -
    Math.min(...mobileNodeBoxes.map(({ x }) => x));
  const mobileYSpan =
    Math.max(...mobileNodeBoxes.map(({ y }) => y)) -
    Math.min(...mobileNodeBoxes.map(({ y }) => y));
  assert.ok(mobileXSpan > 140, "mobile topology collapsed into a vertical line");
  assert.ok(mobileYSpan > 160, "mobile topology collapsed into a horizontal line");
  assert.equal(
    await mobilePathLab
      .locator(".message-path-stage")
      .evaluate((stage) => getComputedStyle(stage).touchAction),
    "pan-y",
    "message path stage must yield the page scroll",
  );
  assert.equal(await horizontalOverflow(messagePathMobile), 0);
  assert.deepEqual(
    messagePathMobileDiagnostics,
    [],
    "mobile message path lab emitted browser errors",
  );
  await mobilePathLab.screenshot({
    path: path.join(receiptRoot, "message-path-lab-mobile.png"),
  });
  receipt.message_path_lab.mobile = {
    width: 375,
    minimum_node_target: 44,
    x_span: Math.round(mobileXSpan),
    y_span: Math.round(mobileYSpan),
    horizontal_overflow: 0,
  };
  await messagePathMobile.close();

  const messagePathReduced = await browser.newPage({
    viewport: { width: 900, height: 900 },
  });
  const messagePathReducedDiagnostics = collectDiagnostics(messagePathReduced);
  await messagePathReduced.emulateMedia({ reducedMotion: "reduce" });
  await messagePathReduced.goto(`${baseUrl}/radio.html`, {
    waitUntil: "networkidle",
  });
  await messagePathReduced.waitForFunction(
    () =>
      document.querySelector("[data-message-path-lab]")?.dataset.ready ===
      "true",
  );
  const reducedPathLab = messagePathReduced.locator("[data-message-path-lab]");
  await reducedPathLab.locator('[data-path-action="play"]').click();
  assert.equal(await reducedPathLab.getAttribute("data-step"), String(coldPath.events - 1));
  assert.equal(await reducedPathLab.getAttribute("data-playing"), "false");
  assert.deepEqual(
    messagePathReducedDiagnostics,
    [],
    "reduced-motion message path lab emitted browser errors",
  );
  receipt.message_path_lab.reduced_motion = "jumps-to-trace-end";
  await messagePathReduced.close();

  const radioBenchDesktop = await browser.newPage({
    viewport: { width: 1440, height: 1000 },
  });
  const radioBenchDesktopDiagnostics = collectDiagnostics(radioBenchDesktop);
  await radioBenchDesktop.goto(`${baseUrl}/devices/v4-desktop-radio/`, {
    waitUntil: "networkidle",
  });
  const radioBench = radioBenchDesktop.locator("[data-radio-simulator]");
  await radioBenchDesktop.waitForFunction(
    () => document.querySelector("[data-radio-simulator]")?.dataset.ready === "true",
  );
  const radioPress = async (action) => {
    await radioBench.locator(`[data-radio-action="${action}"]`).click();
    return radioBenchState(radioBench);
  };

  // Screen names are radio-mirror's screen(); lines are radio-face's text projection.
  let radioState = await radioBenchState(radioBench);
  assert.equal(radioState.screen, "status");
  assert.equal(radioState.lines[0], "STATUS, RAD OK");
  assert.ok(radioState.lines.includes("BOARD: HELTEC V4"));
  assert.equal(radioState.staticHidden, true);
  assert.equal(radioState.canvasHidden, false);
  assert.ok(radioState.litPixels > 0, "radio-mirror drew nothing onto the canvas");
  assert.equal(radioState.led, "off");

  radioState = await radioPress("a-short");
  assert.equal(radioState.screen, "power");
  assert.match(radioState.lines[0], /^POWER, /);

  // The V4's one button: hold for the menu, tap to move, hold to select.
  radioState = await radioPress("a-long");
  assert.equal(radioState.screen, "menu:brightness:0");
  assert.ok(radioState.lines.includes("BRIGHTNESS (SELECTED)"));
  await radioPress("a-short");
  radioState = await radioPress("a-short");
  assert.equal(radioState.screen, "menu:verify:2");
  radioState = await radioPress("a-long");
  assert.equal(radioState.screen, "verify");
  assert.equal(radioState.lines[0], "VERIFY, HOST");

  await radioBench.locator("[data-radio-input]").selectOption("two");
  assert.equal(await radioBench.locator('[data-radio-action="chord"]').isVisible(), true);
  radioState = await radioPress("a-short");
  assert.equal(radioState.screen, "power", "any press leaves VERIFY");
  radioState = await radioPress("chord");
  assert.equal(radioState.screen, "menu:brightness:0");
  radioState = await radioPress("b-long");
  assert.equal(radioState.screen, "power");
  radioState = await radioPress("b-long");
  assert.equal(radioState.screen, "display-off");
  assert.equal(radioState.lastAction, "display-turned-off");
  assert.equal(radioState.panelLit, "false");

  // Raw key edges, classified by the firmware's own press classifier.
  await radioBench.locator("[data-radio-screen]").focus();
  await radioBenchDesktop.keyboard.down("a");
  await radioBenchDesktop.keyboard.up("a");
  radioState = await radioBenchState(radioBench);
  assert.equal(radioState.lastEvent, "a-short");
  assert.equal(radioState.lastAction, "display-woke");
  assert.equal(radioState.screen, "power");
  await radioBenchDesktop.keyboard.down("a");
  await radioBenchDesktop.waitForTimeout(800);
  await radioBenchDesktop.keyboard.up("a");
  radioState = await radioBenchState(radioBench);
  assert.equal(radioState.lastEvent, "a-long");
  assert.equal(radioState.screen, "verify");

  // Upstream images are site HTML, marked as the site's own boundary note.
  await radioBench.locator("[data-radio-firmware]").selectOption("meshtastic");
  radioState = await radioBenchState(radioBench);
  assert.equal(radioState.screen, "handoff:meshtastic");
  assert.equal(radioState.canvasHidden, true);
  assert.match(radioState.lines[0], /^Site note · not a firmware screen/);
  assert.match(radioState.lines[0], /does not counterfeit/);
  assert.equal(
    await radioBench.locator('[data-radio-handoff="meshtastic"]').isVisible(),
    true,
  );
  assert.equal(await radioBench.locator('[data-radio-action="a-short"]').isDisabled(), true);

  await radioBench.locator("[data-radio-firmware]").selectOption("retinue");
  await radioBench.locator("[data-radio-scenario]").selectOption("fault");
  radioState = await radioBenchState(radioBench);
  assert.equal(radioState.screen, "fault");
  assert.equal(radioState.lines[0], "FAULT, E01");
  assert.ok(radioState.lines.includes("SX1262 INIT"));
  assert.equal(radioState.led, "fault-triple");
  radioState = await radioPress("a-short");
  assert.equal(radioState.screen, "fault", "the fault preempts every page");

  // Without a host the controller offers the four local pages.
  await radioBench.locator("[data-radio-scenario]").selectOption("local");
  const localScreens = [(await radioBenchState(radioBench)).screen];
  for (let step = 0; step < 4; step += 1) {
    localScreens.push((await radioPress("a-short")).screen);
  }
  assert.deepEqual(localScreens, ["status", "power", "radio", "traffic", "status"]);
  assert.equal(await horizontalOverflow(radioBenchDesktop), 0);
  assert.deepEqual(
    radioBenchDesktopDiagnostics,
    [],
    "desktop radio bench emitted browser errors",
  );
  await radioBench.screenshot({
    path: path.join(receiptRoot, "radio-bench-desktop.png"),
  });
  receipt.radio_bench.desktop = {
    runtime: "radio-mirror",
    scenarios: 3,
    firmware_images: 4,
    input_faces: 2,
    screens_reached: [
      "status",
      "power",
      "menu:brightness:0",
      "menu:verify:2",
      "verify",
      "display-off",
      "handoff:meshtastic",
      "fault",
      "radio",
      "traffic",
    ],
    key_edges: "a-short, a-long",
    a_plus_b: "operable-on-two-button-face",
    horizontal_overflow: 0,
  };
  await radioBenchDesktop.close();

  const radioBenchMobile = await browser.newPage({
    viewport: { width: 375, height: 812 },
  });
  const radioBenchMobileDiagnostics = collectDiagnostics(radioBenchMobile);
  await radioBenchMobile.goto(`${baseUrl}/devices/v4-desktop-radio/`, {
    waitUntil: "networkidle",
  });
  const mobileBench = radioBenchMobile.locator("[data-radio-simulator]");
  await radioBenchMobile.waitForFunction(
    () => document.querySelector("[data-radio-simulator]")?.dataset.ready === "true",
  );
  await mobileBench.locator("[data-radio-input]").selectOption("two");
  await mobileBench.locator('[data-radio-action="chord"]').click();
  const mobileState = await radioBenchState(mobileBench);
  assert.equal(mobileState.screen, "menu:brightness:0");
  assert.equal(mobileState.lines[0], "MENU, LOCAL");
  assert.equal(await horizontalOverflow(radioBenchMobile), 0);
  assert.deepEqual(
    radioBenchMobileDiagnostics,
    [],
    "mobile radio bench emitted browser errors",
  );
  await mobileBench.screenshot({
    path: path.join(receiptRoot, "radio-bench-mobile.png"),
  });
  receipt.radio_bench.mobile = {
    width: 375,
    a_plus_b: "operable",
    horizontal_overflow: 0,
  };
  await radioBenchMobile.close();

  // Without script: the pinned build-time screens, each with its text projection.
  const radioBenchStatic = await browser.newPage({
    viewport: { width: 900, height: 900 },
    javaScriptEnabled: false,
  });
  await radioBenchStatic.goto(`${baseUrl}/devices/v4-desktop-radio/`, {
    waitUntil: "networkidle",
  });
  const staticBench = await radioBenchStatic.evaluate(async () => {
    const root = document.querySelector("[data-radio-simulator]");
    const images = [...root.querySelectorAll("img")];
    await Promise.all(images.map((image) => image.decode().catch(() => undefined)));
    return {
      ready: root.dataset.ready,
      fallback: root.querySelector("[data-radio-fallback]").textContent,
      screen: root.querySelector("[data-radio-screen]").dataset.screenName,
      reading: [...root.querySelectorAll("[data-radio-text] li")].map((li) => li.textContent),
      figures: [...root.querySelectorAll("[data-radio-gallery] figure")].map((figure) => ({
        screen: figure.dataset.screenName,
        alt: figure.querySelector("img").alt,
        width: figure.querySelector("img").naturalWidth,
        height: figure.querySelector("img").naturalHeight,
      })),
    };
  });
  assert.equal(staticBench.ready, "false");
  assert.match(staticBench.fallback, /rendered by radio-mirror from Retinue's firmware UI at retinue revision [0-9a-f]{7}/);
  assert.equal(staticBench.screen, "status");
  assert.equal(staticBench.reading[0], "STATUS, RAD OK");
  assert.deepEqual(
    staticBench.figures.map((figure) => figure.screen),
    [
      "status",
      "power",
      "radio",
      "traffic",
      "identity",
      "links",
      "peers",
      "menu:brightness:0",
      "verify",
      "display-off",
      "fault",
    ],
  );
  for (const figure of staticBench.figures) {
    assert.equal(figure.width, 128, `${figure.screen} image did not load`);
    assert.equal(figure.height, 64, `${figure.screen} image did not load`);
    assert.ok(figure.alt.length > 0, `${figure.screen} has no text projection`);
  }
  assert.match(staticBench.figures[0].alt, /^STATUS, RAD OK; BOARD: HELTEC V4/);
  assert.match(staticBench.figures.at(-1).alt, /^FAULT, E01; SX1262 INIT/);
  receipt.radio_bench.no_script = {
    screens: staticBench.figures.length,
    text_projection: "alt",
  };
  await radioBenchStatic.close();

  const visualProject = await browser.newPage({
    viewport: { width: 1200, height: 900 },
  });
  const visualProjectDiagnostics = collectDiagnostics(visualProject);
  await visualProject.goto(`${baseUrl}/projects/mere/`, {
    waitUntil: "networkidle",
  });
  assert.equal(
    await visualProject.locator("[data-project-id]").getAttribute(
      "data-project-id",
    ),
    "mere",
  );
  assert.equal(
    await visualProject.locator(".project-showcase-figure img").count(),
    1,
  );
  const visualMetadata = await projectMetadata(visualProject);
  assert.equal(
    visualMetadata.social_image,
    "https://merelyllc.com/showcase/mere.png",
  );
  assert.equal(visualMetadata.social_image_type, "image/png");
  assert.equal(visualMetadata.twitter_image, visualMetadata.social_image);
  assert.equal(
    visualMetadata.twitter_image_alt,
    visualMetadata.social_image_alt,
  );
  assert.ok(visualMetadata.social_image_alt.length > 0);
  assert.equal(visualMetadata.structured_type, "SoftwareSourceCode");
  assert.equal(
    visualMetadata.code_repository,
    "https://github.com/merely-made/mere",
  );
  assert.equal(await horizontalOverflow(visualProject), 0);
  assert.deepEqual(
    visualProjectDiagnostics,
    [],
    "visual project profile emitted browser errors",
  );
  await visualProject.screenshot({
    path: path.join(receiptRoot, "project-mere-desktop.png"),
    fullPage: true,
  });
  receipt.projects.visual = {
    repository: "mere",
    showcase_images: 1,
    social_image: visualMetadata.social_image,
    structured_type: visualMetadata.structured_type,
    horizontal_overflow: 0,
  };
  await visualProject.close();

  // The proof's four sibling artifacts (site canvas plan, Rulings 132-136).
  // Expectations below are derived from them, not pinned.
  const projectionMereHtml = await (await fetch(`${baseUrl}/projects/mere/`)).text();
  const projectionSrc = (attribute) => {
    const match = projectionMereHtml.match(new RegExp(`${attribute}="([^"]+)"`));
    assert.ok(match, `Mere profile is missing ${attribute}`);
    return match[1].replaceAll("&amp;", "&");
  };
  const fetchProjectionText = async (attribute) => {
    const response = await fetch(`${baseUrl}${projectionSrc(attribute)}`);
    assert.equal(response.status, 200, `${attribute} did not load`);
    return response.text();
  };
  const projectionCaptureText = await fetchProjectionText("data-capture-src");
  const projectionCapture = JSON.parse(projectionCaptureText);
  const projectionTraceText = await fetchProjectionText("data-trace-src");
  const projectionTrace = JSON.parse(projectionTraceText);
  const projectionShelfmark = JSON.parse(await fetchProjectionText("data-shelfmark-src"));
  assert.equal(projectionSrc("data-dataset-src"), "/repository-host-dataset.json");
  assert.equal(projectionCapture.version, 2);
  assert.equal(projectionCapture.authority.adapter, "mer3ly.repository-graph/v1");
  assert.equal(projectionTrace.version, 1);
  assert.equal(projectionShelfmark.schema, "mere.shelfmark/1");
  const projectionGeneration =
    projectionShelfmark.inputs.authority.expects_generation;
  // The epoch is above 2^53: the capture's bytes carry its exact digits.
  assert.ok(BigInt(projectionGeneration) > 2n ** 53n);
  assert.ok(projectionCaptureText.includes(`"epoch":${projectionGeneration}`));
  const projectionNodeCount = projectionCapture.scene.tables.items.filter(Boolean).length;
  const projectionEdgeCount = projectionCapture.scene.tables.relations.length;
  const projectionSteps = projectionTrace.steps.length;
  const projectionFinalRevision = projectionTrace.steps.reduce(
    (revision, step) => step.diff?.revision ?? revision,
    projectionCapture.scene.revision,
  );
  assert.equal(projectionCapture.score.items.length, projectionNodeCount);
  assert.ok(projectionSteps > 0 && projectionSteps <= 16);
  // The supplied trace folds with the S5 fold fact (Rulings 149-155). The
  // smoke's fold expectations come from it and the host dataset, not names.
  const projectionSourceOf = (instance) =>
    projectionCapture.scene.tables.sources[
      projectionCapture.scene.tables.items[instance].source
    ].id;
  const projectionFold = projectionTrace.steps
    .flatMap((step) => step.diff?.operations ?? [])
    .find((operation) => operation.AddFold)?.AddFold.value;
  assert.ok(projectionFold, "the supplied trace folds with the fold fact");
  assert.ok(
    !JSON.stringify(projectionTrace).includes('"fold"'),
    "the retired fold channel still travels",
  );
  const projectionFoldRoot = projectionSourceOf(projectionFold.stand_in.Member);
  const projectionFoldHidden = projectionFold.members
    .filter((member) => member !== projectionFold.stand_in.Member)
    .map(projectionSourceOf);
  const projectionUnfolded = projectionCapture.scene.tables.items
    .map((_, instance) => projectionSourceOf(instance))
    .find((id) => !projectionFold.members.some((member) => projectionSourceOf(member) === id));
  assert.ok(projectionFoldHidden.length > 0 && projectionUnfolded);
  const projectionDataset = JSON.parse(await fetchProjectionText("data-dataset-src"));
  const projectionFoldEdge = projectionDataset.relationships.find(
    (relationship) =>
      relationship.from_occurrence === projectionFoldRoot &&
      relationship.to_occurrence === projectionFoldHidden[0],
  )?.id;
  assert.ok(projectionFoldEdge, "a relationship leads into the fold");
  for (const retired of ["projection-scene.json"]) {
    assert.equal((await fetch(`${baseUrl}/${retired}`)).status, 404, `${retired} still ships`);
  }

  const projectionDesktop = await browser.newPage({
    viewport: { width: 1440, height: 1000 },
  });
  const projectionDesktopDiagnostics = collectDiagnostics(projectionDesktop);
  await projectionDesktop.goto(`${baseUrl}/projects/mere/`, {
    waitUntil: "networkidle",
  });
  try {
    await projectionDesktop.waitForFunction(
      () =>
        document.querySelector("[data-projection-proof]")?.dataset.ready ===
        "true",
    );
  } catch (error) {
    const state = await projectionDesktop
      .locator("[data-projection-proof]")
      .evaluate((element) => ({ ...element.dataset }));
    throw new Error(
      `portable projection did not initialize: ${JSON.stringify({ state, diagnostics: projectionDesktopDiagnostics })}`,
      { cause: error },
    );
  }
  const projectionProof = projectionDesktop.locator("[data-projection-proof]");
  // Ruling 140: the first load draws the captured scene without the graph
  // Wasm, which loads on first interaction under the repositories page's
  // own URLs, so both pages share one cached copy.
  assert.equal(await projectionProof.getAttribute("data-replay"), "static");
  const graphRuntimeFetched = () =>
    projectionDesktop.evaluate(() =>
      performance
        .getEntriesByType("resource")
        .some((entry) => entry.name.includes("mer3ly_repo_graph")),
    );
  assert.equal(await graphRuntimeFetched(), false, "the graph Wasm loaded before interaction");
  // With the script running, the reading opens with the scripted line, not
  // the no-script one.
  const readingLead = projectionProof.locator("[data-projection-reading-lead]");
  assert.equal(
    await readingLead.textContent(),
    await readingLead.getAttribute("data-scripted-lead"),
  );
  const sandboxRuntimeVersion = (await (await fetch(`${baseUrl}/repos/`)).text()).match(
    /\/graph-sandbox\.js(\?v=[0-9a-f]+)/,
  )?.[1];
  assert.ok(sandboxRuntimeVersion, "the repositories page names its graph runtime");
  assert.equal(
    await projectionProof.getAttribute("data-graph-runtime"),
    `/mer3ly_repo_graph.js${sandboxRuntimeVersion}`,
  );
  const canvasProjection = projectionProof.locator(
    '[data-projection-view="canvas"]',
  );
  const swatchProjection = projectionProof.locator(
    '[data-projection-view="swatch"]',
  );
  assert.equal(
    await canvasProjection.locator("[data-projection-node]").count(),
    projectionNodeCount,
  );
  assert.equal(
    await swatchProjection.locator("[data-projection-node]").count(),
    projectionNodeCount,
  );
  assert.equal(
    await canvasProjection.locator("[data-projection-edge]").count(),
    projectionEdgeCount,
  );
  assert.equal(
    await swatchProjection.locator("[data-projection-edge]").count(),
    projectionEdgeCount,
  );
  assert.equal(await projectionProof.getAttribute("data-scene-epoch"), projectionGeneration);
  assert.equal(
    await projectionProof.getAttribute("data-capture-address"),
    projectionShelfmark.projection,
  );
  assert.equal(
    await canvasProjection
      .locator('[data-projection-node="mere"]')
      .getAttribute("data-x"),
    await swatchProjection
      .locator('[data-projection-node="mere"]')
      .getAttribute("data-x"),
  );

  await projectionProof.locator('[data-projection-action="replay"]').click();
  await projectionDesktop.waitForFunction(
    () => {
      const root = document.querySelector("[data-projection-proof]");
      return (
        root.dataset.replay === "ready" &&
        root.dataset.cursor === root.dataset.actionCount
      );
    },
  );
  assert.equal(await graphRuntimeFetched(), true);
  assert.equal(
    await projectionProof.getAttribute("data-cursor"),
    String(projectionSteps),
  );
  assert.equal(
    await projectionProof.getAttribute("data-scene-revision"),
    String(projectionFinalRevision),
  );
  await projectionProof.locator('[data-projection-action="reset"]').click();
  assert.equal(await projectionProof.getAttribute("data-cursor"), "0");
  assert.equal(
    await projectionProof.getAttribute("data-scene-revision"),
    String(projectionCapture.scene.revision),
  );

  const canvasTurnstone = canvasProjection.locator(
    '[data-projection-node="turnstone"]',
  );
  const turnstoneBefore = Number(await canvasTurnstone.getAttribute("data-x"));
  const turnstoneBox = await canvasTurnstone.boundingBox();
  assert.ok(turnstoneBox, "canvas Turnstone node needs a draggable box");
  await projectionDesktop.mouse.move(
    turnstoneBox.x + turnstoneBox.width / 2,
    turnstoneBox.y + turnstoneBox.height / 2,
  );
  await projectionDesktop.mouse.down();
  await projectionDesktop.mouse.move(
    turnstoneBox.x + turnstoneBox.width / 2 - 46,
    turnstoneBox.y + turnstoneBox.height / 2 + 24,
    { steps: 5 },
  );
  await projectionDesktop.mouse.up();
  const canvasTurnstoneX = Number(await canvasTurnstone.getAttribute("data-x"));
  const swatchTurnstoneX = Number(
    await swatchProjection
      .locator('[data-projection-node="turnstone"]')
      .getAttribute("data-x"),
  );
  assert.ok(
    Math.abs(canvasTurnstoneX - turnstoneBefore) > 0.03,
    "dragging did not move Turnstone",
  );
  assert.equal(canvasTurnstoneX, swatchTurnstoneX);

  const swatchHostEdge = swatchProjection.locator(
    '[data-projection-edge-control="turnstone-hosts-mere"]',
  );
  await swatchHostEdge.click();
  assert.equal(await projectionProof.getAttribute("data-selected-kind"), "edge");
  assert.equal(
    await projectionProof.getAttribute("data-selected-id"),
    "turnstone-hosts-mere",
  );
  await projectionProof.locator('[data-projection-action="edge"]').click();
  assert.equal(
    await canvasProjection
      .locator('[data-projection-edge="turnstone-hosts-mere"]')
      .evaluate((edge) => edge.classList.contains("is-curated-out")),
    true,
  );
  assert.equal(
    await swatchProjection
      .locator('[data-projection-edge="turnstone-hosts-mere"]')
      .evaluate((edge) => edge.classList.contains("is-curated-out")),
    true,
  );

  await canvasProjection
    .locator(`[data-projection-node="${projectionFoldRoot}"]`)
    .click();
  await projectionProof.locator('[data-projection-action="fold"]').click();
  assert.equal(await projectionProof.getAttribute("data-folded"), projectionFoldRoot);
  assert.equal(
    await projectionProof.getAttribute("data-fold-labels"),
    projectionFold.label,
  );
  for (const hidden of projectionFoldHidden) {
    for (const view of [canvasProjection, swatchProjection]) {
      assert.equal(
        await view.locator(`[data-projection-node="${hidden}"]`).isHidden(),
        true,
        `${hidden} is not hidden by the fold`,
      );
    }
  }
  assert.equal(
    await canvasProjection
      .locator(`[data-projection-edge="${projectionFoldEdge}"]`)
      .isHidden(),
    true,
    "folded dependency edge remains painted",
  );
  const projectionBadge = canvasProjection.locator(
    `[data-projection-node="${projectionFoldRoot}"] .projection-proof-node-fold`,
  );
  // The "+N" is the fact's hidden count, and the badge names the fold's label.
  assert.equal(await projectionBadge.textContent(), `+${projectionFoldHidden.length}`);
  assert.ok((await projectionBadge.getAttribute("title")).startsWith(projectionFold.label));
  assert.equal(
    await canvasProjection
      .locator(`[data-projection-node="${projectionUnfolded}"] .projection-proof-node-fold`)
      .isHidden(),
    true,
    "unfolded nodes display an empty fold badge",
  );

  await projectionProof.locator('[data-projection-action="share"]').click();
  const sharedProjectionUrl = new URL(projectionDesktop.url());
  const sharedProjectionParams = new URLSearchParams(
    sharedProjectionUrl.hash.slice(1),
  );
  assert.equal(sharedProjectionParams.get("projection-scene"), "v3");
  assert.equal(
    sharedProjectionParams.get("projection"),
    projectionShelfmark.projection,
  );
  assert.equal(
    sharedProjectionParams.get("expects-generation"),
    projectionGeneration,
  );
  assert.equal(
    sharedProjectionParams.get("position"),
    await projectionProof.getAttribute("data-cursor"),
  );
  assert.ok((sharedProjectionParams.get("trace") ?? "").length > 20);
  const sharedProjectionActions = Number(
    await projectionProof.getAttribute("data-action-count"),
  );
  const sharedProjectionCursor = Number(
    await projectionProof.getAttribute("data-cursor"),
  );

  const projectionReceiver = await browser.newPage({
    viewport: { width: 1000, height: 900 },
  });
  const projectionReceiverDiagnostics = collectDiagnostics(projectionReceiver);
  await projectionReceiver.goto(sharedProjectionUrl.toString(), {
    waitUntil: "networkidle",
  });
  await projectionReceiver.waitForFunction(
    () =>
      document.querySelector("[data-projection-proof]")?.dataset.ready ===
      "true",
  );
  const receivedProof = projectionReceiver.locator("[data-projection-proof]");
  assert.equal(
    Number(await receivedProof.getAttribute("data-action-count")),
    sharedProjectionActions,
  );
  assert.equal(
    Number(await receivedProof.getAttribute("data-cursor")),
    sharedProjectionCursor,
  );
  assert.equal(await receivedProof.getAttribute("data-folded"), projectionFoldRoot);
  assert.equal(await receivedProof.getAttribute("data-link-state"), "restored");
  const receivedCursor = receivedProof.locator("[data-projection-cursor]");
  await receivedCursor.press("Home");
  assert.equal(await receivedProof.getAttribute("data-cursor"), "0");
  assert.equal(
    await receivedProof
      .locator(`[data-projection-node="${projectionFoldHidden[0]}"]`)
      .first()
      .isVisible(),
    true,
  );
  await receivedCursor.press("End");
  assert.equal(
    Number(await receivedProof.getAttribute("data-cursor")),
    sharedProjectionActions,
  );
  assert.equal(await receivedProof.getAttribute("data-folded"), projectionFoldRoot);
  assert.equal(await horizontalOverflow(projectionReceiver), 0);
  assert.deepEqual(
    projectionReceiverDiagnostics,
    [],
    "shared portable scene emitted browser errors",
  );
  await projectionReceiver.close();

  assert.equal(await horizontalOverflow(projectionDesktop), 0);
  assert.deepEqual(
    projectionDesktopDiagnostics,
    [],
    "desktop projection proof emitted browser errors",
  );
  await projectionProof.screenshot({
    path: path.join(receiptRoot, "mere-projection-proof-desktop.png"),
  });
  receipt.projects.projection_proof = {
    nodes: projectionNodeCount,
    edges: projectionEdgeCount,
    projections: 2,
    contract: "chirograph-capture-v2-scenotime-trace-incipit-shelfmark",
    capture_address: projectionShelfmark.projection,
    initial_revision: projectionCapture.scene.revision,
    supplied_trace_steps: projectionSteps,
    shared_state: true,
    shared_trace: true,
    horizontal_overflow: 0,
  };
  await projectionDesktop.close();

  const projectionMobile = await browser.newPage({
    viewport: { width: 375, height: 812 },
  });
  const projectionMobileDiagnostics = collectDiagnostics(projectionMobile);
  await projectionMobile.goto(`${baseUrl}/projects/mere/`, {
    waitUntil: "networkidle",
  });
  await projectionMobile.waitForFunction(
    () =>
      document.querySelector("[data-projection-proof]")?.dataset.ready ===
      "true",
  );
  const mobileProof = projectionMobile.locator("[data-projection-proof]");
  const mobileCanvas = mobileProof.locator('[data-projection-view="canvas"]');
  const mobileSwatch = mobileProof.locator('[data-projection-view="swatch"]');
  const mobileTargets = await mobileProof
    .locator("[data-projection-node]")
    .evaluateAll((nodes) =>
      nodes.map((node) => {
        const rect = node.getBoundingClientRect();
        return { width: rect.width, height: rect.height };
      }),
    );
  assert.equal(
    mobileTargets.every(({ width, height }) => width >= 44 && height >= 44),
    true,
    "portable scene nodes need 44px mobile targets",
  );
  const mobileSwatchMere = mobileSwatch.locator('[data-projection-node="mere"]');
  const mobileMereBefore = await mobileSwatchMere.getAttribute("data-x");
  await mobileSwatchMere.press("ArrowRight");
  await projectionMobile.waitForFunction(
    () => {
      const root = document.querySelector("[data-projection-proof]");
      return root.dataset.replay === "ready" && root.dataset.cursor === "1";
    },
  );
  assert.notEqual(await mobileSwatchMere.getAttribute("data-x"), mobileMereBefore);
  assert.equal(
    await mobileSwatchMere.getAttribute("data-x"),
    await mobileCanvas
      .locator('[data-projection-node="mere"]')
      .getAttribute("data-x"),
  );
  assert.equal(await horizontalOverflow(projectionMobile), 0);
  assert.deepEqual(
    projectionMobileDiagnostics,
    [],
    "mobile projection proof emitted browser errors",
  );
  await projectionMobile.evaluate(() => document.activeElement?.blur());
  await mobileProof.screenshot({
    path: path.join(receiptRoot, "mere-projection-proof-mobile.png"),
  });
  receipt.projects.projection_proof.mobile = {
    width: 375,
    minimum_node_target: 44,
    swatch_controls_canvas: true,
    horizontal_overflow: 0,
  };
  await assertStageScrollPolicy(projectionMobile, ".projection-proof-stage", ".projection-proof-node", "projection proof");
  await projectionMobile.close();

  const projectionReduced = await browser.newPage({
    viewport: { width: 900, height: 900 },
  });
  const projectionReducedDiagnostics = collectDiagnostics(projectionReduced);
  await projectionReduced.emulateMedia({ reducedMotion: "reduce" });
  await projectionReduced.goto(`${baseUrl}/projects/mere/`, {
    waitUntil: "networkidle",
  });
  await projectionReduced.waitForFunction(
    () =>
      document.querySelector("[data-projection-proof]")?.dataset.ready ===
      "true",
  );
  const reducedProof = projectionReduced.locator("[data-projection-proof]");
  await reducedProof.locator('[data-projection-action="replay"]').click();
  await projectionReduced.waitForFunction(
    () => document.querySelector("[data-projection-proof]")?.dataset.replay === "ready",
  );
  assert.equal(
    await reducedProof.getAttribute("data-cursor"),
    await reducedProof.getAttribute("data-action-count"),
  );
  assert.deepEqual(
    projectionReducedDiagnostics,
    [],
    "reduced-motion projection proof emitted browser errors",
  );
  receipt.projects.projection_proof.reduced_motion = "jumps-to-final-state";
  await projectionReduced.close();

  const projectionFallback = await browser.newPage({
    viewport: { width: 900, height: 900 },
  });
  const projectionFallbackDiagnostics = collectDiagnostics(projectionFallback);
  await projectionFallback.goto(`${baseUrl}/projects/mere/?projection=no-scene`, {
    waitUntil: "networkidle",
  });
  await projectionFallback.waitForFunction(
    () =>
      document.querySelector("[data-projection-proof]")?.dataset.state ===
      "unavailable",
  );
  assert.equal(
    await projectionFallback.locator("[data-projection-fallback]").isVisible(),
    true,
  );
  assert.equal(
    await projectionFallback.locator("[data-projection-interface]").isHidden(),
    true,
  );
  assert.deepEqual(
    projectionFallbackDiagnostics,
    [],
    "portable scene fallback emitted browser errors",
  );
  assert.equal(
    await projectionFallback.locator("[data-projection-reading-step]").count(),
    projectionSteps,
  );
  receipt.projects.projection_proof.fallback = "trace-reading-and-semantic-relations-remain";
  await projectionFallback.close();

  // Ruling 140: if the replay cannot load, a polite notice says so and the
  // build-time reading stays.
  const projectionNoReplay = await browser.newPage({
    viewport: { width: 900, height: 900 },
  });
  const projectionNoReplayDiagnostics = collectDiagnostics(projectionNoReplay);
  await projectionNoReplay.goto(`${baseUrl}/projects/mere/?projection=no-replay`, {
    waitUntil: "networkidle",
  });
  await projectionNoReplay.waitForFunction(
    () => document.querySelector("[data-projection-proof]")?.dataset.ready === "true",
  );
  await projectionNoReplay
    .locator('[data-projection-proof] [data-projection-action="replay"]')
    .click();
  await projectionNoReplay.waitForFunction(
    () => document.querySelector("[data-projection-proof]")?.dataset.replay === "failed",
  );
  assert.equal(
    await projectionNoReplay.locator("[data-projection-notice]").isVisible(),
    true,
  );
  assert.equal(
    await projectionNoReplay.locator("[data-projection-fallback]").isVisible(),
    true,
  );
  assert.equal(
    await projectionNoReplay.locator("[data-projection-reading-step]").count(),
    projectionSteps,
  );
  assert.deepEqual(
    projectionNoReplayDiagnostics,
    [],
    "projection replay failure emitted browser errors",
  );
  receipt.projects.projection_proof.replay_failure = "notice-and-reading-remain";
  await projectionNoReplay.close();

  // Ruling 21: an old link names a retired format, says so politely, and shows
  // the default trace.
  const projectionRetired = await browser.newPage({
    viewport: { width: 900, height: 900 },
  });
  const projectionRetiredDiagnostics = collectDiagnostics(projectionRetired);
  await projectionRetired.goto(
    `${baseUrl}/projects/mere/#projection-scene=v2&authority=retired&trace=e30&cursor=1`,
    { waitUntil: "networkidle" },
  );
  await projectionRetired.waitForFunction(
    () =>
      document.querySelector("[data-projection-proof]")?.dataset.ready === "true",
  );
  const retiredProof = projectionRetired.locator("[data-projection-proof]");
  assert.equal(await retiredProof.getAttribute("data-link-state"), "retired");
  assert.equal(await retiredProof.getAttribute("data-cursor"), "0");
  assert.equal(
    await retiredProof.getAttribute("data-action-count"),
    String(projectionSteps),
  );
  assert.equal(
    await projectionRetired.locator("[data-projection-notice]").isVisible(),
    true,
  );
  assert.deepEqual(
    projectionRetiredDiagnostics,
    [],
    "retired projection link emitted browser errors",
  );
  receipt.projects.projection_proof.retired_link = "notice-and-default-trace";
  await projectionRetired.close();

  // S5: a v3 link whose steps fold the retired way (a `fold` channel and
  // visibility diffs) gets the same retired notice. Its steps are the supplied
  // ones as raw text, so the epoch's digits survive, with the channel added.
  const retiredFoldSteps = projectionTraceText
    .slice(projectionTraceText.indexOf('"steps":') + '"steps":'.length, -1)
    .replace('"channels":[]', '"channels":[["fold",1.0]]');
  assert.ok(retiredFoldSteps.includes('["fold",1.0]'));
  const retiredFoldParams = new URLSearchParams({
    "projection-scene": "v3",
    projection: projectionShelfmark.projection,
    "expects-generation": projectionGeneration,
    position: "2",
    trace: Buffer.from(retiredFoldSteps)
      .toString("base64")
      .replaceAll("+", "-")
      .replaceAll("/", "_")
      .replace(/=+$/, ""),
  });
  const projectionRetiredFold = await browser.newPage({
    viewport: { width: 900, height: 900 },
  });
  const projectionRetiredFoldDiagnostics = collectDiagnostics(projectionRetiredFold);
  await projectionRetiredFold.goto(
    `${baseUrl}/projects/mere/#${retiredFoldParams.toString()}`,
    { waitUntil: "networkidle" },
  );
  await projectionRetiredFold.waitForFunction(
    () =>
      document.querySelector("[data-projection-proof]")?.dataset.ready === "true",
  );
  const retiredFoldProof = projectionRetiredFold.locator("[data-projection-proof]");
  assert.equal(await retiredFoldProof.getAttribute("data-link-state"), "retired");
  assert.equal(await retiredFoldProof.getAttribute("data-cursor"), "0");
  assert.deepEqual(
    projectionRetiredFoldDiagnostics,
    [],
    "retired fold link emitted browser errors",
  );
  receipt.projects.projection_proof.retired_fold_link = "notice-and-default-trace";
  await projectionRetiredFold.close();

  const textProject = await browser.newPage({
    viewport: { width: 375, height: 812 },
  });
  const textProjectDiagnostics = collectDiagnostics(textProject);
  await textProject.goto(`${baseUrl}/projects/retinue/`, {
    waitUntil: "networkidle",
  });
  assert.equal(
    await textProject.locator("[data-project-id]").getAttribute(
      "data-project-id",
    ),
    "retinue",
  );
  assert.equal(
    await textProject.locator(".project-showcase-figure").count(),
    0,
  );
  assert.equal(
    await textProject
      .locator(".project-no-image-copy")
      .getByText("intentionally text-first")
      .count(),
    1,
  );
  const textMetadata = await projectMetadata(textProject);
  assert.equal(textMetadata.social_image, "https://merelyllc.com/og.jpg");
  assert.equal(textMetadata.social_image_type, "image/jpeg");
  assert.equal(textMetadata.twitter_image, textMetadata.social_image);
  assert.equal(textMetadata.twitter_image_alt, textMetadata.social_image_alt);
  assert.ok(textMetadata.social_image_alt.length > 0);
  assert.equal(textMetadata.structured_type, "SoftwareSourceCode");
  assert.equal(
    textMetadata.code_repository,
    "https://github.com/merely-made/retinue",
  );
  assert.equal(await horizontalOverflow(textProject), 0);
  assert.deepEqual(
    textProjectDiagnostics,
    [],
    "text-only project profile emitted browser errors",
  );
  await textProject.screenshot({
    path: path.join(receiptRoot, "project-retinue-mobile.png"),
    fullPage: true,
  });
  receipt.projects.text_only = {
    repository: "retinue",
    showcase_images: 0,
    social_image: textMetadata.social_image,
    structured_type: textMetadata.structured_type,
    horizontal_overflow: 0,
  };
  await textProject.close();

  const cycleActor = async (root, name, expected, limit = 18) => {
    const actor = root.locator(`[data-sandbox-cycle="${name}"]`);
    for (let step = 0; step < limit; step += 1) {
      if ((await actor.getAttribute("data-sandbox-control-value")) === expected) return;
      await actor.click();
      await new Promise((resolve) => setTimeout(resolve, 80));
    }
    assert.fail(`could not cycle ${name} to ${expected}`);
  };

  const sandboxDesktop = await browser.newPage({
    viewport: { width: 1440, height: 900 },
  });
  const sandboxDesktopDiagnostics = collectDiagnostics(sandboxDesktop);
  const sandboxDesktopResponse = await sandboxDesktop.goto(`${baseUrl}/repos/`, {
    waitUntil: "networkidle",
  });
  assert.equal(sandboxDesktopResponse?.status(), 200);
  const expectedRepositoryCountNow = await sandboxDesktop
    .locator("[data-repository-id]")
    .count();
  const expectedRelationProjectionCountNow = await sandboxDesktop
    .locator("[data-relation-id]")
    .count();
  assert.ok(expectedRepositoryCountNow > 0);
  assert.equal(expectedRelationProjectionCountNow % 2, 0);
  assert.equal(
    sitemapUrls.filter((url) => url.includes("/projects/")).length,
    expectedRepositoryCountNow,
  );
  assert.equal(await sandboxDesktop.locator("[data-repository-graph]").count(), 0);

  await sandboxDesktop.waitForFunction(
    () => document.querySelector("[data-graph-sandbox]")?.dataset.sandboxState === "ready",
  );
  const liveSandbox = sandboxDesktop.locator("[data-graph-sandbox]");
  assert.equal(
    await liveSandbox.getAttribute("data-sandbox-scene-schema"),
    "mere.shelfmark/1",
  );
  assert.equal(await liveSandbox.getAttribute("data-sandbox-dataset"), "live");
  assert.equal(await liveSandbox.locator("[data-sandbox-cycle]").count(), 5);
  assert.equal(await liveSandbox.locator("[data-sandbox-node]").count(), expectedRepositoryCountNow);
  assert.equal(await liveSandbox.locator('[data-sandbox-node][data-face="identity"]').count(), expectedRepositoryCountNow);
  assert.equal(
    await liveSandbox
      .locator('[data-sandbox-cycle="arrangement"]')
      .getAttribute("data-sandbox-control-value"),
    "graph_layout:stack",
  );

  await cycleActor(liveSandbox, "reading", "changes");
  assert.ok(
    (await liveSandbox.locator('[data-sandbox-node]:not([data-change="stable"])').count()) > 0,
    "live Changes must derive adjacent-checkpoint changes",
  );
  const liveChangeNodeCount = await liveSandbox.locator("[data-sandbox-node]").count();
  assert.ok(
    liveChangeNodeCount >= expectedRepositoryCountNow,
    "Changes may retain repositories removed since the prior checkpoint",
  );
  assert.equal(
    await liveSandbox.locator('[data-sandbox-node][data-face="delta"]').count(),
    liveChangeNodeCount,
  );
  assert.equal(await liveSandbox.locator("[data-sandbox-history-control]").isVisible(), true);

  await cycleActor(liveSandbox, "dataset", "specimen");
  assert.equal(await liveSandbox.getAttribute("data-sandbox-dataset"), "specimen");
  assert.equal(await liveSandbox.locator("[data-sandbox-node]").count(), 12);
  await cycleActor(liveSandbox, "reading", "activity");
  assert.equal(await liveSandbox.locator('[data-sandbox-node][data-face="signal"]').count(), 12);
  assert.equal(
    await liveSandbox
      .locator('[data-sandbox-cycle="arrangement"]')
      .getAttribute("data-sandbox-control-value"),
    "graph_layout:timeline",
  );

  await cycleActor(liveSandbox, "reading", "neighbors");
  assert.equal(await liveSandbox.locator('[data-sandbox-node][data-face="orbit"]').count(), 4);
  assert.match(
    await liveSandbox.locator('[data-sandbox-node="merecat"]').getAttribute("class"),
    /is-reading-focus/,
  );
  await liveSandbox.locator('[data-sandbox-node="mere"]').click();
  await sandboxDesktop.waitForFunction(
    () => document.querySelectorAll("[data-sandbox-node]").length === 5,
  );
  assert.match(
    await liveSandbox.locator('[data-sandbox-node="mere"]').getAttribute("class"),
    /is-reading-focus/,
  );
  await cycleActor(liveSandbox, "arrangement", "graph_layout:grid");
  assert.equal(await liveSandbox.locator("[data-sandbox-node]").count(), 5);

  await cycleActor(liveSandbox, "reading", "matrix");
  const matrixRows = await liveSandbox.locator(".graph-sandbox-matrix tbody tr").count();
  const matrixColumns =
    (await liveSandbox.locator('.graph-sandbox-matrix thead th[scope="col"]').count()) - 1;
  assert.equal(await liveSandbox.locator(".graph-sandbox-matrix-cell").count(), matrixRows * matrixColumns);
  assert.ok((await liveSandbox.locator(".graph-sandbox-matrix-cell.has-relation").count()) > 0);
  await cycleActor(liveSandbox, "reading", "graph");
  assert.equal(await liveSandbox.locator('[data-sandbox-node][data-face="identity"]').count(), 12);
  const ashlandFace = liveSandbox.locator('[data-sandbox-node="ashland"]');
  await ashlandFace.focus();
  await ashlandFace.press("Enter");
  assert.equal(await ashlandFace.locator(".graph-sandbox-node-detail").isVisible(), true);
  await cycleActor(liveSandbox, "mobility", "free");
  await cycleActor(liveSandbox, "environment", "props-tangible");
  await ashlandFace.press("p");
  await sandboxDesktop.waitForFunction(
    () => document.querySelector('[data-sandbox-node="ashland"]')?.classList.contains("is-pinned"),
  );
  await cycleActor(liveSandbox, "arrangement", "graph_layout:grid");
  assert.match(await ashlandFace.getAttribute("class"), /is-pinned/);

  await cycleActor(liveSandbox, "reading", "matrix");
  const matrixCell = liveSandbox.locator(".graph-sandbox-matrix-cell.has-relation").first();
  await matrixCell.click();
  assert.match(await matrixCell.getAttribute("class"), /is-facet-selected/);
  assert.equal(await liveSandbox.locator("[data-sandbox-clear-matrix]").isEnabled(), true);
  await liveSandbox.locator("[data-sandbox-clear-matrix]").click();
  assert.equal(await liveSandbox.locator("[data-sandbox-clear-matrix]").isDisabled(), true);
  await liveSandbox.locator("[data-sandbox-clear-facets]").click();
  assert.equal(await liveSandbox.locator("[data-sandbox-clear-facets]").isDisabled(), true);
  await matrixCell.click();
  assert.equal(await liveSandbox.locator("[data-sandbox-clear-matrix]").isEnabled(), true);
  await liveSandbox.locator("[data-sandbox-clear-facets]").click();
  assert.equal(await liveSandbox.locator("[data-sandbox-clear-facets]").isDisabled(), true);
  await cycleActor(liveSandbox, "reading", "graph");
  const deckFacet = liveSandbox.locator('.graph-sandbox-deck-card[data-source-id="ashland"] .graph-sandbox-deck-title');
  await deckFacet.click();
  assert.equal(await deckFacet.getAttribute("aria-pressed"), "true");
  assert.match(
    await liveSandbox.locator('.graph-sandbox-deck-card[data-source-id="ashland"]').getAttribute("class"),
    /is-facet-selected/,
  );
  await cycleActor(liveSandbox, "arrangement", "graph_layout:grid");
  await liveSandbox.locator('[data-sandbox-camera="pan-right"]').click();
  await liveSandbox.locator('[data-sandbox-camera="zoom-in"]').click();
  assert.match(
    await liveSandbox.locator("[data-sandbox-stage]").getAttribute("data-sandbox-camera-state"),
    /80,0,1\.20/,
  );
  const dismissedDeck = liveSandbox.locator('.graph-sandbox-deck-card[data-source-id="ashland"]');
  await dismissedDeck.locator(".graph-sandbox-deck-dismiss").click();
  assert.equal(await dismissedDeck.isHidden(), true);

  await liveSandbox.locator("[data-sandbox-share]").click();
  assert.match(sandboxDesktop.url(), /#graphshell-scene=[A-Za-z0-9_-]+$/);
  const portableGraphshellUrl = sandboxDesktop.url();
  const sandboxReceiver = await browser.newPage({ viewport: { width: 1100, height: 900 } });
  const sandboxReceiverDiagnostics = collectDiagnostics(sandboxReceiver);
  await sandboxReceiver.goto(portableGraphshellUrl, { waitUntil: "networkidle" });
  await sandboxReceiver.waitForFunction(
    () => document.querySelector("[data-graph-sandbox]")?.dataset.sandboxState === "ready",
  );
  const receivedGraphshell = sandboxReceiver.locator("[data-graph-sandbox]");
  assert.equal(await receivedGraphshell.getAttribute("data-sandbox-dataset"), "specimen");
  assert.equal(
    await receivedGraphshell
      .locator('[data-sandbox-cycle="arrangement"]')
      .getAttribute("data-sandbox-control-value"),
    "graph_layout:grid",
  );
  assert.equal(
    await receivedGraphshell
      .locator('[data-sandbox-cycle="mobility"]')
      .getAttribute("data-sandbox-control-value"),
    "free",
  );
  assert.equal(
    await receivedGraphshell
      .locator('[data-sandbox-cycle="environment"]')
      .getAttribute("data-sandbox-control-value"),
    "props-tangible",
  );
  assert.equal(
    await receivedGraphshell.locator("[data-sandbox-stage]").getAttribute("data-sandbox-camera-state"),
    "80,0,1.20",
  );
  assert.equal(
    await receivedGraphshell.locator('.graph-sandbox-deck-card[data-source-id="ashland"]').isHidden(),
    true,
  );
  assert.ok(
    (await receivedGraphshell.locator(".graph-sandbox-deck-card.is-facet-selected").count()) > 0,
    "selected Deck facet reopens",
  );
  assert.equal(
    await receivedGraphshell
      .locator('.graph-sandbox-deck-card[data-source-id="ashland"] .graph-sandbox-deck-title')
      .getAttribute("aria-pressed"),
    "true",
  );
  assert.match(
    await receivedGraphshell.locator('[data-sandbox-node="ashland"]').getAttribute("class"),
    /is-pinned/,
  );
  assert.deepEqual(sandboxReceiverDiagnostics, [], "portable Graphshell scene emitted browser errors");
  await sandboxReceiver.close();

  assert.equal(await horizontalOverflow(sandboxDesktop), 0);
  assert.deepEqual(sandboxDesktopDiagnostics, [], "desktop Graphshell emitted browser errors");
  await liveSandbox.screenshot({
    path: path.join(receiptRoot, "graphshell-sandbox-desktop.png"),
  });
  receipt.desktop = {
    repositories: expectedRepositoryCountNow,
    graph_edges: expectedRelationProjectionCountNow / 2,
    horizontal_overflow: 0,
    live_canvas: "graphshell-only",
  };
  receipt.graph_sandbox = {
    state: "ready",
    datasets: { live: expectedRepositoryCountNow, specimen: 12 },
    readings: ["graph", "changes", "activity", "neighbors", "matrix"],
    faces: ["identity", "delta", "signal", "orbit", "table"],
    controls: "in-graph-cycle-actors",
    motion: ["anchored", "free"],
    frozen: "static-renderer-policy",
    portable_scene: "reopened-from-url",
  };
  await sandboxDesktop.close();

  const sandboxMobile = await browser.newPage({ viewport: { width: 420, height: 900 } });
  const sandboxMobileDiagnostics = collectDiagnostics(sandboxMobile);
  await sandboxMobile.goto(`${baseUrl}/repos/`, { waitUntil: "networkidle" });
  await sandboxMobile.waitForFunction(
    () => document.querySelector("[data-graph-sandbox]")?.dataset.sandboxState === "ready",
  );
  const mobileSandbox = sandboxMobile.locator("[data-graph-sandbox]");
  const mobileControlTargets = await mobileSandbox
    .locator("[data-sandbox-cycle]")
    .evaluateAll((controls) => controls.map((control) => control.getBoundingClientRect().height));
  assert.equal(mobileControlTargets.every((height) => height >= 44), true);
  assert.equal(await horizontalOverflow(sandboxMobile), 0);
  assert.deepEqual(sandboxMobileDiagnostics, [], "mobile Graphshell emitted browser errors");
  await mobileSandbox.screenshot({
    path: path.join(receiptRoot, "graphshell-sandbox-mobile.png"),
  });
  receipt.mobile = {
    repositories: expectedRepositoryCountNow,
    graph_edges: expectedRelationProjectionCountNow / 2,
    minimum_control_target: 44,
    horizontal_overflow: 0,
  };

  // Stage gestures, in the shape an embedded map uses. The invariant that
  // matters most is the negative one: a single finger must stay with the page,
  // or the sandbox becomes a region a phone cannot scroll past.
  const stageGesture = (page, moves) =>
    page.evaluate(([moves]) => {
      const stage = document.querySelector("[data-sandbox-stage]");
      const rect = stage.getBoundingClientRect();
      const cx = rect.left + rect.width / 2;
      const cy = rect.top + rect.height / 2;
      const touch = (id, x, y) =>
        new Touch({ identifier: id, target: stage, clientX: cx + x, clientY: cy + y });
      const fire = (type, points) => {
        const touches = points.map(([x, y], index) => touch(index + 1, x, y));
        const event = new TouchEvent(type, {
          touches: type === "touchend" ? [] : touches,
          targetTouches: type === "touchend" ? [] : touches,
          changedTouches: touches,
          bubbles: true,
          cancelable: true,
        });
        stage.dispatchEvent(event);
        return event.defaultPrevented;
      };
      const prevented = [fire("touchstart", moves[0]), fire("touchmove", moves[1])];
      const state = stage.dataset.sandboxCameraState;
      fire("touchend", moves[1]);
      const [x, y, zoom] = state.split(",").map(Number);
      return { x, y, zoom, prevented };
    }, [moves]);

  await assertStageScrollPolicy(sandboxMobile, ".graph-sandbox-stage", ".graph-sandbox-node", "graph sandbox");
  const restingCamera = await sandboxMobile
    .locator("[data-sandbox-stage]")
    .getAttribute("data-sandbox-camera-state");
  const oneFinger = await stageGesture(sandboxMobile, [[[0, 0]], [[0, -120]]]);
  assert.deepEqual(oneFinger.prevented, [false, false], "one finger must not capture the page scroll");
  assert.equal(
    await sandboxMobile.locator("[data-sandbox-stage]").getAttribute("data-sandbox-camera-state"),
    restingCamera,
    "one finger must leave the camera alone",
  );
  const panned = await stageGesture(sandboxMobile, [[[-50, 0], [50, 0]], [[50, 0], [150, 0]]]);
  assert.deepEqual(panned.prevented, [true, true], "two fingers must claim the gesture");
  assert.ok(panned.x > 50, `two-finger pan moved the camera to ${panned.x}`);
  assert.equal(panned.zoom, 1, "a pan with no span change must not zoom");
  const pinched = await stageGesture(sandboxMobile, [[[-50, 0], [50, 0]], [[-100, 0], [100, 0]]]);
  assert.ok(pinched.zoom > 1.8 && pinched.zoom <= 2.1, `pinch reached ${pinched.zoom}x`);
  assert.ok(
    Math.abs(pinched.x - panned.x) < 2 && Math.abs(pinched.y - panned.y) < 2,
    "pinch must hold its anchor rather than drifting the view",
  );
  receipt.gestures = {
    one_finger: "page-scroll",
    two_finger: "pan-and-pinch",
    wheel: "modifier-only",
    anchored_zoom: true,
  };
  await sandboxMobile.close();

  const sandboxFallback = await browser.newPage({ viewport: { width: 375, height: 812 } });
  const sandboxFallbackDiagnostics = collectDiagnostics(sandboxFallback);
  await sandboxFallback.goto(`${baseUrl}/repos/?graph-sandbox=no-wasm`, {
    waitUntil: "networkidle",
  });
  await sandboxFallback.waitForFunction(
    () => document.querySelector("[data-graph-sandbox]")?.dataset.sandboxState === "unavailable",
  );
  assert.equal(await sandboxFallback.locator("[data-sandbox-fallback]").isVisible(), true);
  assert.equal(await sandboxFallback.locator("[data-sandbox-interface]").isHidden(), true);
  assert.equal(await sandboxFallback.locator("[data-repository-id]").count(), expectedRepositoryCountNow);
  assert.equal(await horizontalOverflow(sandboxFallback), 0);
  assert.deepEqual(sandboxFallbackDiagnostics, [], "forced sandbox fallback emitted browser errors");
  receipt.fallback = { state: "unavailable", semantic_index: expectedRepositoryCountNow };
  await sandboxFallback.close();

  await writeFile(
    path.join(receiptRoot, "receipt.json"),
    `${JSON.stringify(receipt, null, 2)}\n`,
    "utf8",
  );
  process.stdout.write(
    `${headless ? "browser" : "headed"} smoke accepted: ${receipt.desktop.repositories} repositories, ${receipt.desktop.graph_edges} graph edges\n`,
  );
} finally {
  await browser.close();
  await new Promise((resolve) => server.close(resolve));
}


// Every draggable stage follows one policy: the stage yields the vertical
// scroll so a phone can always get past it, and its nodes narrow that back to
// none so one-finger dragging still works. Asserted per stage rather than
// once, because each canvas carries its own rule.
async function assertStageScrollPolicy(page, stageSelector, nodeSelector, label) {
  const policy = await page.evaluate(
    ([stageSelector, nodeSelector]) => {
      const stage = document.querySelector(stageSelector);
      const node = document.querySelector(nodeSelector);
      return {
        stage: stage ? getComputedStyle(stage).touchAction : null,
        node: node ? getComputedStyle(node).touchAction : null,
      };
    },
    [stageSelector, nodeSelector],
  );
  assert.equal(policy.stage, "pan-y", `${label} stage must yield the page scroll`);
  assert.equal(policy.node, "none", `${label} nodes must keep their own drag`);
}

function collectDiagnostics(page) {
  const diagnostics = [];
  page.on("pageerror", (error) => diagnostics.push(`pageerror: ${error.message}`));
  page.on("console", (message) => {
    if (message.type() === "error") {
      const location = message.location();
      if (
        message.text().startsWith("Failed to load resource:") &&
        location.url.startsWith("https://fonts.gstatic.com/")
      ) {
        return;
      }
      const diagnostic = `console-error: ${message.text()}${location.url ? ` @ ${location.url}` : ""}`;
      if (process.env.MER3LY_DEBUG_DIAGNOSTICS === "1") {
        process.stderr.write(`${diagnostic}\n`);
      }
      diagnostics.push(diagnostic);
    }
  });
  return diagnostics;
}

async function radioBenchState(bench) {
  return bench.evaluate((root) => {
    const screen = root.querySelector("[data-radio-screen]");
    const canvas = root.querySelector("[data-radio-canvas]");
    const pixels = canvas
      .getContext("2d")
      .getImageData(0, 0, canvas.width, canvas.height).data;
    let litPixels = 0;
    for (let index = 0; index < pixels.length; index += 4) {
      if (pixels[index] || pixels[index + 1] || pixels[index + 2]) litPixels += 1;
    }
    return {
      screen: screen.dataset.screenName,
      lines: [...root.querySelectorAll("[data-radio-text] li")].map((li) => li.textContent),
      led: root.querySelector("[data-radio-led]").dataset.ledState,
      panelLit: screen.dataset.panelLit,
      lastAction: root.dataset.lastAction,
      lastEvent: root.dataset.lastEvent ?? null,
      staticHidden: root.querySelector("[data-radio-static]").hidden,
      canvasHidden: canvas.hidden,
      litPixels,
    };
  });
}

async function horizontalOverflow(page) {
  return page.evaluate(
    () => document.documentElement.scrollWidth - document.documentElement.clientWidth,
  );
}

async function projectMetadata(page) {
  return page.evaluate(() => {
    const canonical = document.querySelector('link[rel="canonical"]').href;
    const payload = JSON.parse(
      document.querySelector('script[type="application/ld+json"]').textContent,
    );
    const entity = payload["@graph"].find(
      (node) => node["@id"] === `${canonical}#repository`,
    );
    return {
      social_image: document
        .querySelector('meta[property="og:image"]')
        .getAttribute("content"),
      social_image_type: document
        .querySelector('meta[property="og:image:type"]')
        .getAttribute("content"),
      social_image_alt: document
        .querySelector('meta[property="og:image:alt"]')
        .getAttribute("content"),
      twitter_image: document
        .querySelector('meta[name="twitter:image"]')
        .getAttribute("content"),
      twitter_image_alt: document
        .querySelector('meta[name="twitter:image:alt"]')
        .getAttribute("content"),
      structured_type: entity["@type"],
      code_repository: entity.codeRepository,
    };
  });
}
