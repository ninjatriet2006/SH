import assert from "node:assert/strict";
import test, { after } from "node:test";
import { createServer } from "vite";

const server = await createServer({ server: { middlewareMode: true, hmr: false }, appType: "custom" });
const { UniverseApp } = await server.ssrLoadModule("/src/app.ts");
after(async () => server.close());

globalThis.document = {
  createElement() {
    let text = "";
    return {
      set textContent(value) { text = String(value); },
      get innerHTML() { return text.replaceAll("&", "&amp;").replaceAll("<", "&lt;").replaceAll(">", "&gt;").replaceAll('"', "&quot;"); },
    };
  },
};

function root() {
  return { innerHTML: "", querySelector() { return null; }, querySelectorAll() { return []; } };
}

function appEntry() {
  return {
    id: "demo", name: "Demo", install_type: "InPlace", source_path: null,
    install_path: "/apps/demo", exec_path: "/apps/demo/demo", icon_path: null,
    desktop_file: "demo.desktop", symlink_file: null, added_at: "now", is_custom: false,
    start_cmd: null, stop_cmd: null, category: null, package_type: null,
    inventory_sources: [], registry_key: null, product_code: null, about_url: null,
    publisher: null, version: "1", uninstall_cmd: null,
  };
}

test("UniverseApp saves scanned apps before permitting confirmed actions", async () => {
  const calls = [];
  const entry = appEntry();
  let finishSave;
  const savePending = new Promise((resolve) => { finishSave = resolve; });
  const api = {
    async getPreferences() { return { language: "en", theme: "system", font_id: "system-default" }; },
    async loadConfig() { return { settings: { managed_dir: "/apps" }, apps: [] }; },
    async saveConfig(config) { calls.push("config_save"); await savePending; return config; },
    async setPreferences(value) { return value; },
  };
  const jobs = {
    dispose() {},
    async run(command) { calls.push(command); return { data: command === "scan_apps" ? [entry] : { completed: true } }; },
  };
  let confirmations = 0;
  const app = new UniverseApp(root(), { api, jobs, loadResources: async () => ({}), confirm: () => { confirmations += 1; return true; } });
  await app.start();
  const scanning = app.scan();
  await new Promise((resolve) => setImmediate(resolve));
  await app.action("start", entry.id);
  assert.deepEqual(calls, ["scan_apps", "config_save"]);
  assert.equal(confirmations, 0);
  finishSave({ settings: { managed_dir: "/apps" }, apps: [entry] });
  await scanning;
  await app.action("start", entry.id);
  assert.deepEqual(calls, ["scan_apps", "config_save", "start_app"]);
  assert.equal(confirmations, 1);
});

test("UniverseApp keeps actions locked when saving scan results fails", async () => {
  const calls = [];
  const entry = appEntry();
  const api = {
    async getPreferences() { return { language: "en", theme: "system", font_id: "system-default" }; },
    async loadConfig() { return { settings: { managed_dir: "/apps" }, apps: [] }; },
    async saveConfig() { calls.push("config_save"); throw new Error("disk full"); },
    async setPreferences(value) { return value; },
  };
  const jobs = {
    dispose() {},
    async run(command) { calls.push(command); return { data: command === "scan_apps" ? [entry] : {} }; },
  };
  const app = new UniverseApp(root(), { api, jobs, loadResources: async () => ({}), confirm: () => true });
  await app.start();
  await app.scan();
  await app.action("stop", entry.id);
  assert.deepEqual(calls, ["scan_apps", "config_save"]);
});
