import { describe, expect, it, vi } from "vitest";
import { applyAndPersistPreferences, normalizeStartupPreferences } from "./app";
import { ResourceLoader, type ResourcePaths } from "./assets";
import { invokeRegistry, request, type JobEvent, type Preferences } from "./contract";
import { JobClient, pickerSelect, type BridgeAdapter } from "./ipc";

function event(requestId: string, state: JobEvent<unknown>["state"]): JobEvent<unknown> {
  return {
    schema_version: 1, job_id: requestId, seq: 0, state,
    payload: { request_id: requestId, completed: null, total: null, message: null, result: null },
  };
}

describe("bridge contract", () => {
  it("has exact nine invokes and four job topics", () => {
    expect(invokeRegistry).toEqual([
      "settings_load", "settings_save", "capabilities_check", "scan_images", "process_images",
      "distribute", "preferences_get", "preferences_set", "picker_select",
    ]);
    expect(request({}, null)).toEqual({ schema_version: 1, request_id: null, payload: {} });
  });

  it("uses the narrow directory picker payload", async () => {
    const invoke = vi.fn().mockResolvedValue({ schema_version: 1, request_id: null, data: { kind: "input", path: { path: "/tmp/in" } } });
    const bridge = { invoke, listen: vi.fn() } as unknown as BridgeAdapter;
    await expect(pickerSelect("input", bridge)).resolves.toEqual({ kind: "input", path: { path: "/tmp/in" } });
    expect(invoke).toHaveBeenCalledWith("picker_select", { request: { schema_version: 1, request_id: null, payload: { kind: "input" } } });
  });

  it("unlistens exactly once at terminal event", async () => {
    const unlisten = vi.fn();
    let handler: ((value: { payload: JobEvent<unknown> }) => void) | undefined;
    const bridge: BridgeAdapter = {
      listen: vi.fn(async (_topic, callback) => { handler = callback as typeof handler; return unlisten; }),
      invoke: vi.fn(async (_command, args) => {
        const id = (args.request as { request_id: string }).request_id;
        handler?.({ payload: event(id, "completed") });
        handler?.({ payload: event(id, "completed") });
        return { schema_version: 1, request_id: id, data: { ffmpeg: { available: true, version: null }, ffprobe: { available: true, version: null } } } as never;
      }),
    };
    await new JobClient(bridge).run("capabilities_check", {}, vi.fn());
    expect(unlisten).toHaveBeenCalledTimes(1);
  });
});

describe("restart persistence", () => {
  const selected: Preferences = { language: "vi", theme: "system", font_id: "dejavusans" };

  it("applies before persisting a preference change", async () => {
    const order: string[] = [];
    await applyAndPersistPreferences(selected, async (value) => { order.push("apply"); return value; }, async (value) => { order.push("persist"); return value; });
    expect(order).toEqual(["apply", "persist"]);
  });

  it("persists normalized startup fallback once", async () => {
    const fallback: Preferences = { language: "en", theme: "dark", font_id: "system-default" };
    const persist = vi.fn(async (value: Preferences) => value);
    await expect(normalizeStartupPreferences(selected, async () => fallback, persist)).resolves.toEqual(fallback);
    expect(persist).toHaveBeenCalledOnce();
  });

  it("keeps valid selected assets returned by a status-zero asset response", async () => {
    const lightSelected: Preferences = { ...selected, theme: "light" };
    const paths: ResourcePaths = {
      languages: { en: "/bundle/langs/en.json", vi: "/bundle/langs/vi.json" },
      themes: { system: "/bundle/themes/system.json", light: "/bundle/themes/light.json", dark: "/bundle/themes/dark.json" },
      fonts: { primary: "/bundle/fonts/DejaVuSans.ttf" },
    };
    const keys = { app: { ready: "Ready" } };
    const responses = new Map<string, unknown>([
      ["asset:///bundle/langs/en.json", keys],
      ["asset:///bundle/langs/vi.json", { app: { ready: "Sẵn sàng" } }],
      ["asset:///bundle/themes/light.json", {
        id: "light",
        tokens: { canvas: "a", foreground: "b", border: "c", panel: "d", panel_foreground: "e", accent: "f", error: "g" },
      }],
    ]);
    vi.stubGlobal("window", { __TAURI_INTERNALS__: { convertFileSrc: (path: string) => `asset://${path}` } });
    vi.stubGlobal("fetch", vi.fn(async (url: string) => ({
      ok: false, status: 0, json: async () => responses.get(url),
    })));
    const loader = new ResourceLoader(paths);
    const persist = vi.fn(async (value: Preferences) => value);

    const normalized = await normalizeStartupPreferences(lightSelected, async (loaded) => ({
      language: (await loader.loadLanguage(loaded.language)).id,
      theme: (await loader.loadTheme(loaded.theme))?.id ?? "dark",
      font_id: loaded.font_id === "dejavusans" && loader.fontUrl("dejavusans") ? "dejavusans" : "system-default",
    }), persist);

    expect(normalized).toEqual(lightSelected);
    expect(persist).not.toHaveBeenCalled();
  });

  it("persists safe fallbacks when selected assets are corrupt or missing", async () => {
    const lightSelected: Preferences = { ...selected, theme: "light" };
    const paths: ResourcePaths = {
      languages: { en: "/bundle/langs/en.json", vi: "/bundle/langs/vi.json" },
      themes: { system: "/bundle/themes/system.json", light: "/bundle/themes/light.json", dark: "/bundle/themes/dark.json" },
      fonts: { primary: null },
    };
    const responses = new Map<string, unknown>([
      ["asset:///bundle/langs/en.json", { app: { ready: "Ready" } }],
      ["asset:///bundle/langs/vi.json", []],
      ["asset:///bundle/themes/dark.json", {
        id: "dark",
        tokens: { canvas: "a", foreground: "b", border: "c", panel: "d", panel_foreground: "e", accent: "f", error: "g" },
      }],
    ]);
    vi.stubGlobal("window", { __TAURI_INTERNALS__: { convertFileSrc: (path: string) => `asset://${path}` } });
    vi.stubGlobal("fetch", vi.fn(async (url: string) => ({
      ok: responses.has(url), status: responses.has(url) ? 200 : 404,
      json: async () => responses.get(url),
    })));
    const loader = new ResourceLoader(paths);
    const persist = vi.fn(async (value: Preferences) => value);

    const normalized = await normalizeStartupPreferences(lightSelected, async (loaded) => ({
      language: (await loader.loadLanguage(loaded.language)).id,
      theme: (await loader.loadTheme(loaded.theme))?.id ?? "dark",
      font_id: loaded.font_id === "dejavusans" && loader.fontUrl("dejavusans") ? "dejavusans" : "system-default",
    }), persist);

    expect(normalized).toEqual({ language: "en", theme: "dark", font_id: "system-default" });
    expect(persist).toHaveBeenCalledOnce();
  });
});
