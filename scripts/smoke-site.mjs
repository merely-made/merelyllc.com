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

  const desktop = await browser.newPage({
    viewport: { width: 1440, height: 900 },
  });
  const desktopDiagnostics = collectDiagnostics(desktop);
  const desktopResponse = await desktop.goto(`${baseUrl}/repos/`, {
    waitUntil: "networkidle",
  });
  assert.equal(desktopResponse?.status(), 200);
  await waitForGraphState(desktop);
  let desktopState = await graphState(desktop);
  const expectedRepositoryCount = await desktop
    .locator("[data-repository-id]")
    .count();
  const expectedRelationProjectionCount = await desktop
    .locator("[data-relation-id]")
    .count();
  assert.ok(expectedRepositoryCount > 0);
  assert.equal(
    sitemapUrls.filter((url) => url.includes("/projects/")).length,
    expectedRepositoryCount,
  );
  assert.equal(expectedRelationProjectionCount % 2, 0);
  assert.equal(desktopState.repositories, expectedRepositoryCount);
  assert.equal(
    desktopState.relation_text_projections,
    expectedRelationProjectionCount,
  );
  assert.equal(desktopState.graph_nodes, expectedRepositoryCount);
  assert.equal(desktopState.graph_edges, expectedRelationProjectionCount / 2);
  assert.equal(desktopState.horizontal_overflow, 0);
  assert.ok(
    desktopState.state === "ready" || desktopState.state === "unavailable",
    "graph did not settle into ready or fallback state",
  );

  await desktop.waitForFunction(() =>
    ["ready", "unavailable"].includes(
      document.querySelector("[data-graph-sandbox]")?.dataset.sandboxState,
    ),
  );
  const sandboxRoot = desktop.locator("[data-graph-sandbox]");
  const sandboxState = await sandboxRoot.getAttribute("data-sandbox-state");
  assert.equal(sandboxState, "ready", "the real Wasm physics sandbox must initialize");
  assert.equal(
    await sandboxRoot.getAttribute("data-sandbox-scene-schema"),
    "mer3ly.graphshell-scene-state/v1",
  );
  assert.equal(await sandboxRoot.getAttribute("data-sandbox-dataset"), "live");
  assert.equal(
    await sandboxRoot.locator("[data-sandbox-node]").count(),
    expectedRepositoryCount,
  );
  const sandboxArrangement = sandboxRoot.locator('[data-sandbox-control="arrangement"]');
  const sandboxReading = sandboxRoot.locator('[data-sandbox-control="scene"]');
  assert.equal(await sandboxReading.locator("option").count(), 5);
  assert.equal(await sandboxArrangement.locator("option").count(), 8);
  assert.equal(await sandboxArrangement.inputValue(), "graph_layout:stack");

  await sandboxRoot.locator('[data-sandbox-control="scene"]').selectOption("changes");
  assert.ok(
    (await sandboxRoot.locator('[data-sandbox-node]:not([data-change="stable"])').count()) > 0,
    "live Changes must derive at least one change from adjacent public checkpoints",
  );
  assert.equal(await sandboxRoot.locator("[data-sandbox-history-control]").isVisible(), true);
  const sourceTime = sandboxRoot.locator("[data-sandbox-history]");
  assert.ok(Number(await sourceTime.getAttribute("max")) > 0);

  await sandboxRoot.locator('[data-sandbox-control="dataset"]').selectOption("specimen");
  assert.equal(await sandboxRoot.getAttribute("data-sandbox-dataset"), "specimen");
  assert.equal(await sandboxRoot.locator("[data-sandbox-node]").count(), 12);
  assert.equal(
    await sandboxRoot.locator('[data-sandbox-node][data-change="added"]').count(),
    3,
  );
  await sandboxRoot.locator('[data-sandbox-control="scene"]').selectOption("activity");
  assert.equal(await sandboxArrangement.inputValue(), "graph_layout:timeline");
  await sandboxRoot.locator('[data-sandbox-control="scene"]').selectOption("matrix");
  assert.equal(await sandboxRoot.locator(".graph-sandbox-matrix-cell").count(), 144);
  assert.ok(
    (await sandboxRoot.locator(".graph-sandbox-matrix-cell.has-relation").count()) > 10,
  );
  await sandboxReading.selectOption("neighbors");
  assert.equal(await sandboxArrangement.inputValue(), "graph_layout:radial");
  assert.equal(await sandboxArrangement.isEnabled(), true);
  assert.equal(await sandboxRoot.locator("[data-sandbox-node]").count(), 4);
  assert.equal(
    await sandboxRoot.locator('[data-sandbox-node="merecat"]').getAttribute("class").then(
      (value) => value.includes("is-reading-focus"),
    ),
    true,
  );
  await sandboxRoot.locator('[data-sandbox-node="mere"]').click();
  assert.equal(await sandboxRoot.locator("[data-sandbox-node]").count(), 5);
  assert.equal(
    await sandboxRoot.locator('[data-sandbox-node="mere"]').getAttribute("class").then(
      (value) => value.includes("is-reading-focus"),
    ),
    true,
    "selecting a neighbor must recompose the actor set around the new focus",
  );
  await sandboxArrangement.selectOption("graph_layout:grid");
  assert.equal(await sandboxRoot.locator("[data-sandbox-node]").count(), 5);
  await sandboxRoot.locator('[data-sandbox-control="scene"]').selectOption("graph");
  await sandboxArrangement.selectOption("graph_layout:radial");
  await sandboxRoot.locator('[data-sandbox-node="ashland"]').click();
  assert.equal(
    await sandboxRoot.locator("[data-sandbox-inspector-title]").textContent(),
    "Ashland",
  );
  await sandboxRoot.locator('[data-sandbox-control="mobility"]').selectOption("free");
  await sandboxRoot.locator('[data-sandbox-control="backdrop"]').selectOption("props");
  await sandboxRoot.locator("[data-sandbox-tangible]").check();
  await sandboxRoot.locator("[data-sandbox-pin]").click();
  await desktop.waitForFunction(
    () =>
      document
        .querySelector('[data-sandbox-node="ashland"]')
        ?.classList.contains("is-pinned") === true,
  );
  await sandboxArrangement.selectOption("graph_layout:grid");
  assert.equal(
    await sandboxRoot.locator('[data-sandbox-node="ashland"]').getAttribute("class").then(
      (value) => value.includes("is-pinned"),
    ),
    true,
    "pins must survive arrangement changes",
  );
  await sandboxArrangement.selectOption("graph_layout:radial");
  await sandboxRoot.locator("[data-sandbox-share]").click();
  assert.match(desktop.url(), /#graphshell-scene=[A-Za-z0-9_-]+$/);
  const portableSceneUrl = desktop.url();

  const portableScene = await browser.newPage({ viewport: { width: 1100, height: 900 } });
  const portableSceneDiagnostics = collectDiagnostics(portableScene);
  await portableScene.goto(portableSceneUrl, { waitUntil: "networkidle" });
  await portableScene.waitForFunction(
    () => document.querySelector("[data-graph-sandbox]")?.dataset.sandboxState === "ready",
  );
  const receivedSandbox = portableScene.locator("[data-graph-sandbox]");
  assert.equal(await receivedSandbox.getAttribute("data-sandbox-dataset"), "specimen");
  assert.equal(
    await receivedSandbox.locator('[data-sandbox-control="arrangement"]').inputValue(),
    "graph_layout:radial",
  );
  assert.equal(
    await receivedSandbox.locator('[data-sandbox-control="mobility"]').inputValue(),
    "free",
  );
  assert.equal(
    await receivedSandbox.locator('[data-sandbox-control="backdrop"]').inputValue(),
    "props",
  );
  assert.equal(await receivedSandbox.locator("[data-sandbox-tangible]").isChecked(), true);
  await portableScene.waitForFunction(
    () =>
      document
        .querySelector('[data-sandbox-node="ashland"]')
        ?.classList.contains("is-pinned") === true,
  );
  assert.deepEqual(
    portableSceneDiagnostics,
    [],
    "reopened portable Graphshell scene emitted browser errors",
  );
  await portableScene.close();
  await sandboxRoot.screenshot({
    path: path.join(receiptRoot, "graphshell-sandbox-desktop.png"),
  });
  receipt.graph_sandbox = {
    state: sandboxState,
    datasets: { live: expectedRepositoryCount, specimen: 12 },
    scenes: ["graph", "changes", "activity", "neighbors", "matrix"],
    arrangement: "graph_layout:radial",
    motion: "free",
    backdrop: "props",
    collidable: true,
    pin: "ashland",
    pin_survives_arrangement: true,
    portable_scene: "reopened-from-url",
    changes: "adjacent-public-checkpoint-diff",
    reading_registry: "mere.graph-reading-registry/v1",
    representation_registry: "mere.graph-representation-registry/v1",
  };

  let selectedProfile = "fallback-not-applicable";
  let sharedSceneUrl = null;
  let sharedSceneExpectation = null;
  if (desktopState.state === "ready") {
    const mere = desktop.locator('[data-graph-node-id="mere"]');
    assert.equal(await mere.getAttribute("aria-label"), "Mere, platform, active");
    try {
      const arrangementPicker = desktop.locator("select[data-graph-arrangement]");
      assert.equal(await arrangementPicker.locator("option").count(), 9);
      assert.equal(
        await arrangementPicker.locator("option:not(:disabled)").count(),
        8,
      );
      await desktop.waitForFunction(() =>
        [...document.querySelectorAll("[data-graph-node-id]")].every(
          (node) => node.style.left && node.style.top,
        ),
      );
      const beforeArrangement = await graphNodePositions(desktop);
      await arrangementPicker.selectOption("graph_layout:grid");
      await desktop.waitForFunction(
        () =>
          document.querySelector("[data-repository-graph]").dataset
            .graphMorphing === "false" &&
          document.querySelector("[data-repository-graph]").dataset
            .graphArrangement === "graph_layout:grid",
      );
      const afterArrangement = await graphNodePositions(desktop);
      assert.equal(
        beforeArrangement.some(
          (before, index) =>
            Math.hypot(
              before.x - afterArrangement[index].x,
              before.y - afterArrangement[index].y,
            ) > 8,
        ),
        true,
        "arrangement selection did not move repository nodes",
      );
      assert.equal(
        await desktop
          .locator("[data-repository-graph]")
          .getAttribute("data-graph-node-form"),
        "tile",
      );
      assert.match(
        await desktop.locator("[data-graph-scene-caption]").textContent(),
        /Index tiles/,
      );
      assert.equal(await mere.getAttribute("aria-pressed"), "true");
      const historyControls = desktop.locator("[data-graph-history-controls]");
      const historyVisible = await historyControls.isVisible();
      if (historyVisible) {
        const historyRange = historyControls.locator("[data-graph-history]");
        const liveValue = Number(await historyRange.getAttribute("max"));
        assert.ok(liveValue > 0, "history needs a committed checkpoint before live");
        const historySnapshots = await desktop.evaluate(() =>
          JSON.parse(document.querySelector("#repository-graph-data").textContent)
            .history.checkpoints.filter(
              ({ availability }) => availability === "available",
            ),
        );
        const historicalMereValue = historySnapshots.findIndex(({ graph }) =>
          graph.nodes.some(({ id }) => id === "mere"),
        );
        const graphshellDisappearsAt = historySnapshots.findIndex(
          ({ graph }, index) =>
            index > 0 && !graph.nodes.some(({ id }) => id === "graphshell"),
        );
        assert.ok(historicalMereValue > 0, "a historical Mere checkpoint is available");
        assert.ok(
          graphshellDisappearsAt > historicalMereValue,
          "Graphshell appears and later closes in public history",
        );
        await historyRange.press("Home");
        await desktop.waitForFunction(
          () =>
            document.querySelector("[data-repository-graph]").dataset
              .graphMorphing === "false" &&
            document.querySelector('[data-graph-node-id="graphshell"]'),
        );
        assert.match(
          await historyControls.locator("[data-graph-history-status]").textContent(),
          /^Committed .*merely-made\/mere/,
        );
        assert.equal(await historyRange.inputValue(), "0");
        assert.equal(await desktop.locator('[data-graph-node-id="mere"]').count(), 0);
        assert.equal(await desktop.locator('[data-graph-node-id="graphshell"]').count(), 1);
        await historyRange.press("ArrowRight");
        await desktop.waitForFunction(
          () =>
            document.querySelector('[data-graph-node-id="webrender-wgpu"]') &&
            document.querySelector("[data-repository-graph]").dataset
              .graphMorphing === "false",
        );
        assert.equal(await historyRange.inputValue(), "1");
        for (let value = 1; value < historicalMereValue; value += 1) {
          await historyRange.press("ArrowRight");
        }
        await desktop.waitForFunction(
          () =>
            document.querySelector('[data-graph-node-id="mere"]') &&
            document.querySelector("[data-repository-graph]").dataset
              .graphMorphing === "false",
        );
        assert.equal(
          await desktop
            .locator("[data-repository-graph]")
            .getAttribute("data-graph-arrangement"),
          "graph_layout:grid",
          "source time preserves the current arrangement",
        );
        assert.equal(
          await desktop
            .locator('[data-graph-node-id="graphshell"]')
            .getAttribute("aria-pressed"),
          "true",
          "the surviving historical selection stays selected",
        );
        await mere.click();
        await expectSelectedNode(desktop, "mere");
        assert.equal(await mere.getAttribute("aria-pressed"), "true");
        const historicalArrangementIds = [
          "graph_layout:radial",
          "graph_layout:stack",
          "graph_layout:grid",
          "graph_layout:phyllotaxis",
          "graph_layout:timeline",
          "graph_layout:kanban",
          "graph_layout:penrose",
          "graph_layout:lsystem",
        ];
        for (const arrangementId of historicalArrangementIds) {
          await arrangementPicker.selectOption(arrangementId);
          await desktop.waitForFunction(
            (expected) => {
              const root = document.querySelector("[data-repository-graph]");
              return (
                root.dataset.graphArrangement === expected &&
                root.dataset.graphMorphing === "false"
              );
            },
            arrangementId,
          );
          assert.equal(
            await historyRange.inputValue(),
            String(historicalMereValue),
            `${arrangementId} keeps the committed source cursor`,
          );
          assert.match(
            await historyControls
              .locator("[data-graph-history-status]")
              .textContent(),
            /^Committed /,
            `${arrangementId} remains on committed source truth`,
          );
          assert.equal(
            await mere.getAttribute("aria-pressed"),
            "true",
            `${arrangementId} preserves a selected repository present at the cursor`,
          );
        }
        await desktop.locator('button[data-graph-action="share"]').click();
        sharedSceneUrl = desktop.url();
        const sharedScene = new URL(sharedSceneUrl);
        const sharedParams = new URLSearchParams(sharedScene.hash.slice(1));
        const expectedCursor = historySnapshots[historicalMereValue].cursor;
        assert.equal(sharedParams.get("repository-scene"), "v1");
        assert.equal(sharedParams.get("arrangement"), "graph_layout:lsystem");
        assert.equal(sharedParams.get("selected"), "mere");
        assert.equal(sharedParams.get("source"), expectedCursor.source);
        assert.equal(sharedParams.get("commit"), expectedCursor.commit);
        assert.doesNotMatch(
          sharedScene.hash,
          /C%3A|C:|mark_|private|127\.0\.0\.1/i,
          "shared public scene contains a private reference",
        );
        sharedSceneExpectation = {
          arrangement: sharedParams.get("arrangement"),
          cursor: historicalMereValue,
          selected: sharedParams.get("selected"),
        };
        const shared = await browser.newPage({
          viewport: { width: 1440, height: 900 },
        });
        const sharedDiagnostics = collectDiagnostics(shared);
        await shared.goto(sharedSceneUrl, { waitUntil: "networkidle" });
        await waitForGraphState(shared);
        assert.equal(
          (await graphState(shared)).state,
          "ready",
          "shared public scene needs an interactive receiver",
        );
        await shared.waitForFunction(
          () =>
            document.querySelector("[data-repository-graph]").dataset
              .graphMorphing === "false",
        );
        assert.equal(
          await shared
            .locator("[data-repository-graph]")
            .getAttribute("data-graph-arrangement"),
          sharedSceneExpectation.arrangement,
        );
        assert.equal(
          await shared.locator("[data-graph-history]").inputValue(),
          String(sharedSceneExpectation.cursor),
        );
        await expectSelectedNode(shared, sharedSceneExpectation.selected);
        assert.deepEqual(sharedDiagnostics, [], "shared desktop scene emitted browser errors");
        await shared.close();

        const refused = new URL(sharedSceneUrl);
        const refusedParams = new URLSearchParams(refused.hash.slice(1));
        refusedParams.set("source", "public-source-not-present");
        refusedParams.set("commit", "unavailable-commit");
        refused.hash = refusedParams.toString();
        const receiver = await browser.newPage({
          viewport: { width: 1440, height: 900 },
        });
        const receiverDiagnostics = collectDiagnostics(receiver);
        await receiver.goto(refused.toString(), { waitUntil: "networkidle" });
        await waitForGraphState(receiver);
        assert.equal((await graphState(receiver)).state, "ready");
        assert.match(
          await receiver.locator("[data-graph-status]").textContent(),
          /source cursor is unavailable/,
        );
        assert.equal(
          await receiver.locator("[data-graph-node-id]").count(),
          expectedRepositoryCount,
        );
        assert.deepEqual(
          receiverDiagnostics,
          [],
          "refused shared scene emitted browser errors",
        );
        await receiver.close();
        for (
          let value = historicalMereValue;
          value < graphshellDisappearsAt;
          value += 1
        ) {
          await historyRange.press("ArrowRight");
        }
        await desktop.waitForFunction(
          () =>
            !document.querySelector('[data-graph-node-id="graphshell"]') &&
            document.querySelector("[data-repository-graph]").dataset
              .graphMorphing === "false",
        );
        assert.equal(await historyRange.inputValue(), String(graphshellDisappearsAt));
        assert.equal(await desktop.locator('[data-graph-node-id="graphshell"]').count(), 0);
        await historyControls
          .locator('button[data-graph-action="return-live"]')
          .click();
        await desktop.waitForFunction(
          () =>
            document.querySelector("[data-repository-graph]").dataset
              .graphMorphing === "false",
        );
        assert.equal(await historyRange.inputValue(), String(liveValue));
        assert.match(
          await historyControls.locator("[data-graph-history-status]").textContent(),
          /^Live authority/,
        );
        assert.equal(
          await desktop.locator("[data-graph-node-id]").count(),
          expectedRepositoryCount,
        );
        desktopState.source_time = "public-lineage-and-live";
        desktopState.history_checkpoints = historySnapshots.length;
        desktopState.source_time_arrangement_matrix = historicalArrangementIds;
        desktopState.shared_scene = sharedSceneExpectation;
      } else {
        desktopState.source_time = "no-committed-checkpoints";
      }
      desktopState.arrangements = 8;
      desktopState.morphed_to = "graph_layout:grid";
      await mere.click({ timeout: 2000 });
      await mere.press("ArrowRight", { timeout: 2000 });
      const selectedNode = desktop.locator(
        ".repository-graph-node.is-selected",
      );
      const selectedId = await selectedNode.getAttribute("data-graph-node-id", {
        timeout: 2000,
      });
      assert.ok(selectedId);
      assert.notEqual(selectedId, "mere");
      await desktop.locator("[data-repository-graph]").screenshot({
        path: path.join(receiptRoot, "desktop-repository-graph.png"),
      });
      await selectedNode.press("Enter", { timeout: 2000 });
      await desktop.waitForURL(`**/projects/${selectedId}/`);
      assert.equal(
        await desktop
          .locator("[data-project-id]")
          .getAttribute("data-project-id"),
        selectedId,
      );
      selectedProfile = selectedId;
    } catch (error) {
      desktopState = await graphState(desktop);
      if (desktopState.state !== "unavailable") {
        throw error;
      }
      await desktop.locator("[data-repository-graph]").screenshot({
        path: path.join(receiptRoot, "desktop-repository-graph.png"),
      });
    }
  } else {
    await desktop.locator("[data-repository-graph]").screenshot({
      path: path.join(receiptRoot, "desktop-repository-graph.png"),
    });
  }
  assert.deepEqual(desktopDiagnostics, [], "desktop emitted browser errors");
  receipt.desktop = { ...desktopState, selected_profile: selectedProfile };
  await desktop.close();

  const mobile = await browser.newPage({
    viewport: { width: 420, height: 900 },
  });
  const mobileDiagnostics = collectDiagnostics(mobile);
  await mobile.goto(`${baseUrl}/repos/`, { waitUntil: "networkidle" });
  await waitForGraphState(mobile);
  const mobileState = await graphState(mobile);
  assert.equal(mobileState.repositories, expectedRepositoryCount);
  assert.equal(mobileState.horizontal_overflow, 0);
  if (mobileState.state === "ready") {
    const arrangementPicker = mobile.locator("select[data-graph-arrangement]");
    const arrangementScenes = [
      ["graph_layout:radial", "medallion", "orbits"],
      ["graph_layout:stack", "tile", "index"],
      ["graph_layout:grid", "tile", "index"],
      ["graph_layout:phyllotaxis", "seed", "field"],
      ["graph_layout:timeline", "flag", "timeline"],
      ["graph_layout:kanban", "card", "lanes"],
      ["graph_layout:penrose", "facet", "tessellation"],
      ["graph_layout:lsystem", "leaf", "branches"],
    ];
    const sceneReceipts = [];
    for (const [arrangementId, nodeForm, scaffold] of arrangementScenes) {
      await arrangementPicker.selectOption(arrangementId);
      await mobile.waitForFunction(
        (expected) => {
          const root = document.querySelector("[data-repository-graph]");
          return (
            root.dataset.graphArrangement === expected &&
            root.dataset.graphMorphing === "false"
          );
        },
        arrangementId,
      );
      const scene = await graphSceneState(mobile);
      assert.equal(scene.nodes, expectedRepositoryCount);
      assert.equal(scene.outside_stage, 0);
      assert.equal(scene.outside_node_bounds, 0);
      assert.equal(scene.selected, "mere");
      assert.equal(scene.node_form, nodeForm);
      assert.equal(scene.scaffold, scaffold);
      if (["orbits", "timeline", "lanes", "tessellation", "branches"].includes(scaffold)) {
        assert.ok(scene.scaffold_items > 0, `${arrangementId} has no scene scaffold`);
      }
      assert.ok(
        scene.minimum_distance >= 28,
        `${arrangementId} crowded repository nodes on mobile`,
      );
      if (arrangementId === "graph_layout:timeline") {
        assert.equal(scene.scaffold_items, 39, "timeline scaffold count changed");
        assert.ok(scene.minimum_hit_width >= 44, "timeline targets are too narrow");
        assert.ok(scene.minimum_hit_height >= 44, "timeline targets are too short");
        assert.equal(scene.overlapping_nodes, 0, "timeline targets overlap");

        const genet = mobile.locator('[data-graph-node-id="genet"]');
        await genet.click({ position: { x: 2, y: 2 } });
        await expectSelectedNode(mobile, "genet");

        const mere = mobile.locator('[data-graph-node-id="mere"]');
        await mere.click({ position: { x: 2, y: 2 } });
        await expectSelectedNode(mobile, "mere");
      }
      sceneReceipts.push(scene);
    }
    mobileState.arrangements = sceneReceipts;
    const historyControls = mobile.locator("[data-graph-history-controls]");
    if (await historyControls.isVisible()) {
      const historyRange = historyControls.locator("[data-graph-history]");
      const historyTarget = await historyRange.evaluate((input) => {
        const rect = input.getBoundingClientRect();
        return { width: rect.width, height: rect.height };
      });
      assert.ok(historyTarget.height >= 44, "history range is a 44px mobile target");
      await historyRange.press("Home");
      await mobile.waitForFunction(
        () =>
          document.querySelector('[data-graph-node-id="graphshell"]') &&
          document.querySelector("[data-repository-graph]").dataset
            .graphMorphing === "false",
      );
      assert.equal(await historyRange.inputValue(), "0");
      assert.equal(await mobile.locator('[data-graph-node-id="graphshell"]').count(), 1);
      await historyRange.press("ArrowRight");
      await mobile.waitForFunction(
        () =>
          document.querySelector('[data-graph-node-id="webrender-wgpu"]') &&
          document.querySelector("[data-repository-graph]").dataset
            .graphMorphing === "false",
      );
      await historyControls
        .locator('button[data-graph-action="return-live"]')
        .click();
      await mobile.waitForFunction(
        () =>
          document.querySelector("[data-repository-graph]").dataset
            .graphMorphing === "false",
      );
      assert.match(
        await historyControls.locator("[data-graph-history-status]").textContent(),
        /^Live authority/,
      );
      mobileState.source_time = "public-lineage-and-live";
    }
    if (sharedSceneUrl && sharedSceneExpectation) {
      const shared = await browser.newPage({
        viewport: { width: 420, height: 900 },
      });
      const sharedDiagnostics = collectDiagnostics(shared);
      await shared.goto(sharedSceneUrl, { waitUntil: "networkidle" });
      await waitForGraphState(shared);
      assert.equal((await graphState(shared)).state, "ready");
      await shared.waitForFunction(
        () =>
          document.querySelector("[data-repository-graph]").dataset
            .graphMorphing === "false",
      );
      assert.equal(
        await shared
          .locator("[data-repository-graph]")
          .getAttribute("data-graph-arrangement"),
        sharedSceneExpectation.arrangement,
      );
      assert.equal(
        await shared.locator("[data-graph-history]").inputValue(),
        String(sharedSceneExpectation.cursor),
      );
      await expectSelectedNode(shared, sharedSceneExpectation.selected);
      assert.equal(await horizontalOverflow(shared), 0);
      assert.deepEqual(sharedDiagnostics, [], "shared mobile scene emitted browser errors");
      await shared.close();
      mobileState.shared_scene = sharedSceneExpectation;
    }
  }
  assert.deepEqual(mobileDiagnostics, [], "mobile emitted browser errors");
  await mobile.locator("[data-repository-graph]").screenshot({
    path: path.join(receiptRoot, "mobile-repository-graph.png"),
  });
  receipt.mobile = mobileState;
  await mobile.close();

  const reduced = await browser.newPage({
    viewport: { width: 420, height: 900 },
  });
  await reduced.emulateMedia({ reducedMotion: "reduce" });
  const reducedDiagnostics = collectDiagnostics(reduced);
  await reduced.goto(`${baseUrl}/repos/`, { waitUntil: "networkidle" });
  await waitForGraphState(reduced);
  const reducedState = await graphState(reduced);
  const reducedMediaMatches = await reduced.evaluate(() =>
    window.matchMedia("(prefers-reduced-motion: reduce)").matches,
  );
  assert.equal(reducedMediaMatches, true);
  if (reducedState.state === "ready") {
    assert.equal(
      await reduced
        .locator("[data-repository-graph]")
        .getAttribute("data-reduced-motion"),
      "true",
    );
    await reduced
      .locator("select[data-graph-arrangement]")
      .selectOption("graph_layout:penrose");
    assert.equal(
      await reduced
        .locator("[data-repository-graph]")
        .getAttribute("data-graph-morphing"),
      "false",
    );
    assert.equal(
      await reduced
        .locator("[data-repository-graph]")
        .getAttribute("data-graph-arrangement"),
      "graph_layout:penrose",
    );
  }
  assert.equal(reducedState.horizontal_overflow, 0);
  assert.deepEqual(
    reducedDiagnostics,
    [],
    "reduced-motion path emitted browser errors",
  );
  receipt.reduced_motion = {
    graph_state: reducedState.state,
    horizontal_overflow: reducedState.horizontal_overflow,
    media_query_matches: reducedMediaMatches,
    graph_client_acknowledged:
      reducedState.state === "ready" ? true : "fallback-not-applicable",
  };
  await reduced.close();

  const fallback = await browser.newPage({
    viewport: { width: 375, height: 812 },
  });
  const fallbackDiagnostics = collectDiagnostics(fallback);
  await fallback.goto(`${baseUrl}/repos/?graph=no-webgpu`, {
    waitUntil: "networkidle",
  });
  await waitForGraphState(fallback);
  const fallbackState = await graphState(fallback);
  assert.equal(fallbackState.state, "unavailable");
  assert.equal(fallbackState.repositories, expectedRepositoryCount);
  assert.equal(fallbackState.horizontal_overflow, 0);
  assert.equal(
    await fallback
      .locator("[data-graph-interface]")
      .evaluate((element) => element.hidden),
    true,
  );
  assert.deepEqual(
    fallbackDiagnostics,
    [],
    "forced fallback emitted browser errors",
  );
  await fallback.screenshot({
    path: path.join(receiptRoot, "webgpu-fallback.png"),
    fullPage: true,
  });
  receipt.fallback = fallbackState;
  await fallback.close();

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

async function waitForGraphState(page) {
  await page.waitForFunction(() => {
    const state = document.querySelector("[data-repository-graph]")?.dataset
      .graphState;
    return state === "ready" || state === "unavailable";
  });
}

async function horizontalOverflow(page) {
  return page.evaluate(
    () => document.documentElement.scrollWidth - document.documentElement.clientWidth,
  );
}

async function graphState(page) {
  return page.evaluate(() => {
    const payload = JSON.parse(
      document.querySelector("#repository-graph-data").textContent,
    );
    return {
      state: document.querySelector("[data-repository-graph]").dataset
        .graphState,
      repositories: document.querySelectorAll("[data-repository-id]").length,
      relation_text_projections: document.querySelectorAll("[data-relation-id]")
        .length,
      graph_nodes: payload.nodes.length,
      graph_edges: payload.edges.length,
      horizontal_overflow:
        document.documentElement.scrollWidth -
        document.documentElement.clientWidth,
    };
  });
}

async function expectSelectedNode(page, expectedId) {
  await page.waitForFunction(
    (id) =>
      document.querySelector(".repository-graph-node.is-selected")?.dataset
        .graphNodeId === id,
    expectedId,
  );
}

async function graphNodePositions(page) {
  return page.locator("[data-graph-node-id]").evaluateAll((nodes) =>
    nodes.map((node) => ({
      x: Number.parseFloat(node.style.left),
      y: Number.parseFloat(node.style.top),
    })),
  );
}

async function graphSceneState(page) {
  return page.evaluate(() => {
    const root = document.querySelector("[data-repository-graph]");
    const stage = document.querySelector("[data-graph-stage]");
    const stageRect = stage.getBoundingClientRect();
    const points = [...document.querySelectorAll("[data-graph-node-id]")].map(
      (node) => ({
        x: Number.parseFloat(node.style.left),
        y: Number.parseFloat(node.style.top),
      }),
    );
    const nodeRects = [...document.querySelectorAll("[data-graph-node-id]")].map(
      (node) => node.getBoundingClientRect(),
    );
    let minimumDistance = Number.POSITIVE_INFINITY;
    let overlappingNodes = 0;
    for (let index = 0; index < points.length; index += 1) {
      for (let other = index + 1; other < points.length; other += 1) {
        minimumDistance = Math.min(
          minimumDistance,
          Math.hypot(
            points[index].x - points[other].x,
            points[index].y - points[other].y,
          ),
        );
        const a = nodeRects[index];
        const b = nodeRects[other];
        if (
          Math.min(a.right, b.right) - Math.max(a.left, b.left) > 1 &&
          Math.min(a.bottom, b.bottom) - Math.max(a.top, b.top) > 1
        ) {
          overlappingNodes += 1;
        }
      }
    }
    return {
      arrangement: root.dataset.graphArrangement,
      node_form: root.dataset.graphNodeForm,
      scaffold: root.dataset.graphScaffold,
      scaffold_items: document.querySelector("[data-graph-scene]").children.length,
      nodes: points.length,
      minimum_distance: Math.round(minimumDistance),
      minimum_hit_width: Math.round(
        Math.min(...nodeRects.map((rect) => rect.width)),
      ),
      minimum_hit_height: Math.round(
        Math.min(...nodeRects.map((rect) => rect.height)),
      ),
      overlapping_nodes: overlappingNodes,
      outside_stage: points.filter(
        (point) =>
          point.x < 0 ||
          point.y < 0 ||
          point.x > stageRect.width ||
          point.y > stageRect.height,
      ).length,
      outside_node_bounds: nodeRects.filter(
        (rect) =>
          rect.left < stageRect.left ||
          rect.top < stageRect.top ||
          rect.right > stageRect.right ||
          rect.bottom > stageRect.bottom,
      ).length,
      selected: document.querySelector(".repository-graph-node.is-selected")
        ?.dataset.graphNodeId,
    };
  });
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
