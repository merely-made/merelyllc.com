// The grammar-g9 lane's reading of Chrome's own computed accessibility tree
// (dynamics grammar plan, F68), the one receipt that carries the scenario
// runner's single stated exception to "no DevTools": it attaches to the
// receipt's Chrome over the DevTools protocol, read-only, and never sends
// input. It calls Accessibility.getFullAXTree on the page and checks that the
// graph canvas's slot (a group named "N of M shown") holds the expected
// number of item groups, each with Drag and Pin buttons that carry a
// description. Node 24's built-in WebSocket; nothing installed.
//
// Usage: node cdp-ax-check.mjs <devtools-port> <out.json> <expected-items>
// Exits 0 when the tree holds, 1 when a check fails, 2 when it cannot read.

import { writeFileSync } from "node:fs";

const [port, out, expectedText] = process.argv.slice(2);
const expected = Number(expectedText);
if (!port || !out || !Number.isFinite(expected)) {
  console.error("usage: node cdp-ax-check.mjs <devtools-port> <out.json> <expected-items>");
  process.exit(2);
}

const targets = await (await fetch(`http://127.0.0.1:${port}/json/list`)).json();
const page = targets.find((t) => t.type === "page" && t.url.includes("tree.html"));
if (!page) {
  console.error("no tree.html page among the targets");
  process.exit(2);
}

const socket = new WebSocket(page.webSocketDebuggerUrl);
let next = 1;
const pending = new Map();
socket.onmessage = (event) => {
  const message = JSON.parse(event.data);
  if (message.id && pending.has(message.id)) {
    pending.get(message.id)(message);
    pending.delete(message.id);
  }
};
await new Promise((resolve, reject) => {
  socket.onopen = resolve;
  socket.onerror = reject;
});
// Read-only calls only: the computed tree, nothing that acts on the page.
const call = (method, params = {}) =>
  new Promise((resolve) => {
    const id = next++;
    pending.set(id, resolve);
    socket.send(JSON.stringify({ id, method, params }));
  });

const reply = await call("Accessibility.getFullAXTree");
socket.close();
if (reply.error) {
  console.error(`getFullAXTree: ${JSON.stringify(reply.error)}`);
  process.exit(2);
}
const nodes = reply.result.nodes;
const byId = new Map(nodes.map((node) => [node.nodeId, node]));
const role = (node) => node.role?.value ?? "";
const name = (node) => node.name?.value ?? "";
const description = (node) => node.description?.value ?? "";
// Children as a reader meets them: ignored or unnamed generic wrappers are
// looked through.
const children = (node) =>
  (node.childIds ?? []).flatMap((id) => {
    const child = byId.get(id);
    if (!child) return [];
    if (child.ignored || (role(child) === "generic" && !name(child))) return children(child);
    return [child];
  });

const slot = nodes.find((node) => role(node) === "group" && name(node).endsWith(" shown"));
const report = { slot: slot ? name(slot) : null, items: [], failures: [] };
if (!slot) {
  report.failures.push("no canvas slot (a group named '... shown') in Chrome's tree");
} else {
  for (const item of children(slot).filter((node) => role(node) === "group")) {
    const buttons = children(item).filter((node) => role(node) === "button");
    report.items.push({
      name: name(item),
      buttons: buttons.map((button) => ({ name: name(button), described: description(button) !== "" })),
    });
  }
  if (report.items.length !== expected) {
    report.failures.push(`${report.items.length} item groups, expected ${expected}`);
  }
  for (const item of report.items) {
    for (const wanted of ["Drag", "Pin"]) {
      const button = item.buttons.find((b) => b.name === wanted);
      if (!button) report.failures.push(`${wanted} missing on ${JSON.stringify(item.name)}`);
      else if (!button.described) report.failures.push(`${wanted} on ${JSON.stringify(item.name)} has no description`);
    }
  }
}
writeFileSync(out, JSON.stringify(report, null, 2));
console.log(
  report.failures.length === 0
    ? `CDP OK: ${report.slot}, ${report.items.length} items, each with Drag and Pin`
    : `CDP FAIL: ${report.failures.slice(0, 3).join("; ")}`,
);
process.exit(report.failures.length === 0 ? 0 : 1);
