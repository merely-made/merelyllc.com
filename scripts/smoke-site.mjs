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
  channel: "chromium",
  headless,
  args: headless
    ? ["--enable-unsafe-webgpu", "--use-angle=swiftshader"]
    : ["--enable-unsafe-webgpu"],
});

const receipt = {
  schema: "mer3ly.browser-smoke-receipt/v3",
  source_sha: process.env.GITHUB_SHA ?? "local",
  browser: `Chromium ${browser.version()}`,
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
    sitemapUrls.every((url) => url.startsWith("https://mer3ly.net/")),
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
    "User-agent: *\nAllow: /\nSitemap: https://mer3ly.net/sitemap.xml\n",
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
    "/projects/mesocosm/",
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
  const pathLab = messagePathDesktop.locator("[data-message-path-lab]");
  const pathStep = pathLab.locator("[data-path-step]");
  assert.equal(await pathLab.locator("[data-lab-node]").count(), 5);
  assert.equal(await pathLab.locator("[data-lab-edge]").count(), 5);
  assert.equal(await pathLab.locator("[data-lab-event]").count(), 6);
  assert.equal(await pathLab.getAttribute("data-blocked"), "true");

  await pathLab.locator("[data-path-blocked]").uncheck();
  assert.equal(await pathLab.getAttribute("data-blocked"), "false");
  assert.match(await pathLab.locator("[data-path-route]").textContent(), /^Direct/);
  await pathStep.press("End");
  assert.equal(await pathLab.getAttribute("data-step"), "5");
  assert.match(
    await pathLab.locator("[data-path-status]").textContent(),
    /direct route/,
  );

  await pathLab.locator("[data-path-blocked]").check();
  await pathLab.locator('[data-path-action="send"]').click();
  assert.equal(await pathLab.getAttribute("data-playing"), "true");
  await messagePathDesktop.waitForFunction(
    () => Number(document.querySelector("[data-message-path-lab]").dataset.step) >= 1,
  );
  await pathStep.press("End");
  assert.equal(await pathLab.getAttribute("data-playing"), "false");
  assert.match(await pathLab.locator("[data-path-route]").textContent(), /^Reroute/);

  const church = pathLab.locator('[data-lab-node="church"]');
  const churchBefore = {
    x: Number(await church.getAttribute("data-x")),
    y: Number(await church.getAttribute("data-y")),
  };
  const churchBox = await church.boundingBox();
  assert.ok(churchBox, "church radio needs a draggable box");
  await messagePathDesktop.mouse.move(
    churchBox.x + churchBox.width / 2,
    churchBox.y + churchBox.height / 2,
  );
  await messagePathDesktop.mouse.down();
  await messagePathDesktop.mouse.move(
    churchBox.x + churchBox.width / 2 + 54,
    churchBox.y + churchBox.height / 2 + 34,
    { steps: 5 },
  );
  await messagePathDesktop.mouse.up();
  const churchAfter = {
    x: Number(await church.getAttribute("data-x")),
    y: Number(await church.getAttribute("data-y")),
  };
  assert.ok(
    Math.hypot(churchAfter.x - churchBefore.x, churchAfter.y - churchBefore.y) > 3,
    "dragging did not move the church radio",
  );

  await pathLab.locator('[data-path-action="share"]').click();
  const sharedMessagePathUrl = new URL(messagePathDesktop.url());
  const sharedMessagePathParams = new URLSearchParams(
    sharedMessagePathUrl.hash.slice(1),
  );
  assert.equal(sharedMessagePathParams.get("message-path"), "v1");
  assert.equal(sharedMessagePathParams.get("blocked"), "1");
  assert.equal(sharedMessagePathParams.get("step"), "5");
  assert.match(sharedMessagePathParams.get("positions") ?? "", /church,/);

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
  assert.equal(
    await sharedMessagePath
      .locator('[data-lab-node="church"]')
      .getAttribute("data-x"),
    churchAfter.x.toFixed(1),
  );
  assert.equal(await horizontalOverflow(sharedMessagePath), 0);
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
    steps: 6,
    draggable: true,
    shared_scene: true,
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
  await reducedPathLab.locator('[data-path-action="send"]').click();
  assert.equal(await reducedPathLab.getAttribute("data-step"), "5");
  assert.equal(await reducedPathLab.getAttribute("data-playing"), "false");
  assert.deepEqual(
    messagePathReducedDiagnostics,
    [],
    "reduced-motion message path lab emitted browser errors",
  );
  receipt.message_path_lab.reduced_motion = "jumps-to-complete-state";
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
  assert.equal(await radioBench.locator("[data-screen-header]").textContent(), "PHY · OK");
  await radioBench.locator('[data-radio-action="a-short"]').click();
  assert.equal(await radioBench.locator("[data-screen-header]").textContent(), "PHY · POWER");

  await radioBench.locator("[data-radio-scenario]").selectOption("host");
  await radioBench.locator('[data-radio-action="a-long"]').click();
  assert.equal(await radioBench.locator("[data-screen-header]").textContent(), "MENU");
  await radioBench.locator('[data-radio-action="a-short"]').click();
  await radioBench.locator('[data-radio-action="a-long"]').click();
  assert.equal(await radioBench.locator("[data-screen-header]").textContent(), "VERIFY · HOST");

  await radioBench.locator("[data-radio-input]").selectOption("two");
  await radioBench.locator('[data-radio-action="a-short"]').click();
  await radioBench.locator('[data-radio-action="chord"]').click();
  assert.equal(await radioBench.locator("[data-screen-header]").textContent(), "MENU");
  assert.equal(await radioBench.locator('[data-radio-action="chord"]').isVisible(), true);

  await radioBench.locator("[data-radio-firmware]").selectOption("meshtastic");
  assert.equal(await radioBench.locator("[data-screen-header]").textContent(), "MST · HANDOFF");
  assert.match(await radioBench.locator("[data-radio-boundary]").textContent(), /does not counterfeit/);
  assert.equal(await radioBench.locator('[data-radio-action="a-short"]').isDisabled(), true);

  await radioBench.locator("[data-radio-firmware]").selectOption("retinue");
  await radioBench.locator("[data-radio-scenario]").selectOption("fault");
  assert.equal(await radioBench.locator("[data-screen-header]").textContent(), "PHY · FAULT");
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
    scenarios: 3,
    firmware_images: 4,
    input_faces: 2,
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
  assert.equal(await mobileBench.locator("[data-screen-header]").textContent(), "MENU");
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
    "https://mer3ly.net/showcase/mere.png",
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

  const projectionArtifactResponse = await fetch(`${baseUrl}/projection-scene.json`);
  assert.equal(projectionArtifactResponse.status, 200);
  const projectionArtifact = await projectionArtifactResponse.json();
  assert.equal(projectionArtifact.schema, "mer3ly.portable-projection/v1");
  assert.equal(projectionArtifact.adapter, "mer3ly.repository-graph/v1");
  assert.equal(projectionArtifact.score.items.length, 8);
  assert.equal(projectionArtifact.snapshot.tables.items.length, 8);
  assert.equal(projectionArtifact.snapshot.tables.relations.length, 9);
  assert.equal(projectionArtifact.default_trace.length, 7);

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
  const canvasProjection = projectionProof.locator(
    '[data-projection-view="canvas"]',
  );
  const swatchProjection = projectionProof.locator(
    '[data-projection-view="swatch"]',
  );
  assert.equal(await canvasProjection.locator("[data-projection-node]").count(), 8);
  assert.equal(await swatchProjection.locator("[data-projection-node]").count(), 8);
  assert.equal(await canvasProjection.locator("[data-projection-edge]").count(), 9);
  assert.equal(await swatchProjection.locator("[data-projection-edge]").count(), 9);
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
      return root.dataset.cursor === root.dataset.actionCount;
    },
  );
  assert.equal(await projectionProof.getAttribute("data-cursor"), "7");
  assert.equal(await projectionProof.getAttribute("data-scene-revision"), "5");
  await projectionProof.locator('[data-projection-action="reset"]').click();
  assert.equal(await projectionProof.getAttribute("data-cursor"), "0");
  assert.equal(await projectionProof.getAttribute("data-scene-revision"), "1");

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

  await canvasProjection.locator('[data-projection-node="mere"]').click();
  await projectionProof.locator('[data-projection-action="fold"]').click();
  assert.equal(await projectionProof.getAttribute("data-folded"), "mere");
  assert.equal(
    await canvasProjection.locator('[data-projection-node="genet"]').isHidden(),
    true,
  );
  assert.equal(
    await swatchProjection.locator('[data-projection-node="genet"]').isHidden(),
    true,
  );
  assert.equal(
    await canvasProjection
      .locator('[data-projection-edge="mere-depends-on-genet"]')
      .isHidden(),
    true,
    "folded dependency edge remains painted",
  );
  assert.equal(
    await canvasProjection
      .locator('[data-projection-node="woodshed"] .projection-proof-node-fold')
      .isHidden(),
    true,
    "unfolded nodes display an empty fold badge",
  );

  await projectionProof.locator('[data-projection-action="share"]').click();
  const sharedProjectionUrl = new URL(projectionDesktop.url());
  const sharedProjectionParams = new URLSearchParams(
    sharedProjectionUrl.hash.slice(1),
  );
  assert.equal(sharedProjectionParams.get("projection-scene"), "v2");
  assert.equal(
    sharedProjectionParams.get("authority"),
    projectionArtifact.authority_sha256,
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
  assert.equal(await receivedProof.getAttribute("data-folded"), "mere");
  const receivedCursor = receivedProof.locator("[data-projection-cursor]");
  await receivedCursor.press("Home");
  assert.equal(await receivedProof.getAttribute("data-cursor"), "0");
  assert.equal(
    await receivedProof.locator('[data-projection-node="genet"]').first().isVisible(),
    true,
  );
  await receivedCursor.press("End");
  assert.equal(
    Number(await receivedProof.getAttribute("data-cursor")),
    sharedProjectionActions,
  );
  assert.equal(await receivedProof.getAttribute("data-folded"), "mere");
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
    nodes: 8,
    edges: 9,
    projections: 2,
    contract: "sceno-score-scene-scenotime-diff",
    initial_revision: projectionArtifact.snapshot.revision,
    supplied_trace_steps: projectionArtifact.default_trace.length,
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
  await mobileSwatchMere.press("ArrowRight");
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
  receipt.projects.projection_proof.fallback = "semantic-relations-remain";
  await projectionFallback.close();

  const textProject = await browser.newPage({
    viewport: { width: 375, height: 812 },
  });
  const textProjectDiagnostics = collectDiagnostics(textProject);
  await textProject.goto(`${baseUrl}/projects/mesocosm/`, {
    waitUntil: "networkidle",
  });
  assert.equal(
    await textProject.locator("[data-project-id]").getAttribute(
      "data-project-id",
    ),
    "mesocosm",
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
  assert.equal(textMetadata.social_image, "https://mer3ly.net/og.jpg");
  assert.equal(textMetadata.social_image_type, "image/jpeg");
  assert.equal(textMetadata.twitter_image, textMetadata.social_image);
  assert.equal(textMetadata.twitter_image_alt, textMetadata.social_image_alt);
  assert.ok(textMetadata.social_image_alt.length > 0);
  assert.equal(textMetadata.structured_type, "SoftwareSourceCode");
  assert.equal(
    textMetadata.code_repository,
    "https://github.com/merely-made/mesocosm",
  );
  assert.equal(await horizontalOverflow(textProject), 0);
  assert.deepEqual(
    textProjectDiagnostics,
    [],
    "text-only project profile emitted browser errors",
  );
  await textProject.screenshot({
    path: path.join(receiptRoot, "project-mesocosm-mobile.png"),
    fullPage: true,
  });
  receipt.projects.text_only = {
    repository: "mesocosm",
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
    "mer3ly.graphshell-scene-state/v1",
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
  assert.equal(await liveSandbox.locator(".graph-sandbox-matrix-cell").count(), 144);
  assert.ok((await liveSandbox.locator(".graph-sandbox-matrix-cell.has-relation").count()) > 10);
  await cycleActor(liveSandbox, "reading", "graph");
  assert.equal(await liveSandbox.locator('[data-sandbox-node][data-face="identity"]').count(), 12);
  const ashlandFace = liveSandbox.locator('[data-sandbox-node="ashland"]');
  await ashlandFace.click();
  assert.equal(await ashlandFace.locator(".graph-sandbox-node-detail").isVisible(), true);
  await cycleActor(liveSandbox, "mobility", "free");
  await cycleActor(liveSandbox, "environment", "props-tangible");
  await ashlandFace.press("p");
  await sandboxDesktop.waitForFunction(
    () => document.querySelector('[data-sandbox-node="ashland"]')?.classList.contains("is-pinned"),
  );
  await cycleActor(liveSandbox, "arrangement", "graph_layout:grid");
  assert.match(await ashlandFace.getAttribute("class"), /is-pinned/);

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
