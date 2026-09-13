import assert from "node:assert/strict";
import test, { after } from "node:test";
import { createServer } from "vite";

const server = await createServer({ server: { middlewareMode: true, hmr: false }, appType: "custom" });
const { loadBundledJson } = await server.ssrLoadModule("/src/resources.ts");
after(async () => server.close());

test("bundled JSON uses Tauri's converted asset URL", async () => {
  let fetched = "";
  const value = await loadBundledJson(
    "/bundle/langs/en.json",
    { fallback: true },
    (path) => `asset://localhost/${path}`,
    async (url) => { fetched = url; return { ok: true, async json() { return { loaded: true }; } }; },
  );
  assert.equal(fetched, "asset://localhost//bundle/langs/en.json");
  assert.deepEqual(value, { loaded: true });
});

test("asset conversion and fetch failures safely retain the fallback", async () => {
  const fallback = { safe: true };
  const value = await loadBundledJson("/missing.json", fallback, () => { throw new Error("not in Tauri"); });
  assert.equal(value, fallback);
  const failedFetch = await loadBundledJson("/missing.json", fallback, (path) => path, async () => { throw new Error("unavailable"); });
  assert.equal(failedFetch, fallback);
});
