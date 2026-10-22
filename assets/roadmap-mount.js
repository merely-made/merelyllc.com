// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

// The roadmap's live map: Graphshell's qualified viewer, imported unchanged on
// request, drawing the roadmap dataset the site publishes. The frozen scene
// stays the reading for visitors without WebGPU.
const root = document.querySelector('[data-roadmap-map]');
if (root) {
  const open = root.querySelector('[data-roadmap-open]');
  const status = root.querySelector('[data-roadmap-status]');
  const surface = root.querySelector('[data-roadmap-surface]');
  const say = (text) => { status.textContent = text; };
  if (!navigator.gpu) {
    open.hidden = true;
    say('The live map needs WebGPU; the frozen scene below is the same graph.');
  }
  open.addEventListener('click', async () => {
    open.disabled = true;
    say('Loading the live map…');
    try {
      if (!await navigator.gpu.requestAdapter()) throw new Error('no WebGPU adapter');
      const style = document.createElement('link');
      style.rel = 'stylesheet';
      style.href = root.dataset.viewerStyle;
      const styled = new Promise((resolve, reject) => {
        style.onload = resolve;
        style.onerror = () => reject(new Error('viewer stylesheet unavailable'));
      });
      document.head.append(style);
      await Promise.all([styled, import(root.dataset.viewerRuntime)]);
      const viewer = document.createElement('graphshell-tree');
      viewer.className = 'site-graphshell-viewer';
      viewer.setAttribute('data-presenter', 'browse');
      viewer.setAttribute('data-project-link-field', 'detail_url');
      viewer.setAttribute('data-source-link-field', 'source_url');
      viewer.setAttribute('data-dataset-src', root.dataset.datasetSrc);
      surface.hidden = false;
      surface.append(viewer);
      open.hidden = true;
      say('');
    } catch (error) {
      open.disabled = false;
      say(`The live map could not load (${error.message ?? error}); the frozen scene below is the same graph.`);
    }
  });
}
