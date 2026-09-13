import assert from "node:assert/strict";
import test from "node:test";
import { readFile } from "node:fs/promises";

const source = await readFile(new URL("../src/contract.ts", import.meta.url), "utf8");
const ipc = await readFile(new URL("../src/ipc.ts", import.meta.url), "utf8");
const app = await readFile(new URL("../src/app.ts", import.meta.url), "utf8");
const resources = await readFile(new URL("../src/resources.ts", import.meta.url), "utf8");

test("frontend keeps the approved ten-command/five-topic contract", () => {
  const registry = source.match(/export const invokeRegistry = \[([\s\S]*?)\] as const/)?.[1].match(/"[a-z_]+"/g) ?? [];
  const topics = source.match(/job\.[a-z_]+/g) ?? [];
  assert.equal(registry.length, 10);
  assert.equal(new Set(topics).size, 5);
});

test("resource failures retain embedded language, theme, and system-font fallbacks", () => {
  assert.match(resources, /string \| null/);
  assert.match(resources, /if \(!path\) return fallback/);
  assert.match(resources, /setProperty\("--font", "system-ui, sans-serif"\)/);
  assert.match(resources, /catch \{ \/\* Keep the system-default font\. \*\/ \}/);
});

test("selection and lifecycle safeguards remain explicit", () => {
  assert.match(app, /pickerSelect\("managed"\)/);
  assert.match(app, /pickerSelect\("source"\)/);
  assert.match(app, /window\.confirm/);
  assert.match(app, /confirmed: true/);
  assert.match(ipc, /terminalSeen/);
  assert.match(ipc, /if \(released\) return/);
});
