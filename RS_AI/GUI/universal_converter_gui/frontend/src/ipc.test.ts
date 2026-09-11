import { describe, expect, it, vi } from "vitest";
import type { Event } from "@tauri-apps/api/event";
import type { JobEvent } from "./contract";
import { invokeRegistry, jobTopics, request } from "./contract";
import { JobClient, pickerSelect, type BridgeAdapter } from "./ipc";
import { applyAndPersistPreferenceChange, JobErrorReporter, normalizeAndPersistStartupPreferences, pickerActions } from "./app";
import { applyFontAsset, recursiveKeys, ResourceLoader, translation, type Dictionary, type ResourcePaths } from "./assets";
import en from "../../langs/en.json";
import viLanguage from "../../langs/vi.json";

function event(state: JobEvent<unknown>["state"], requestId: string): Event<JobEvent<unknown>> {
  return { id: 1, event: "job.dependencies_check", payload: { schema_version: 1, job_id: requestId, seq: 0, state, payload: { request_id: requestId, completed: null, total: null, message: null, result: null } } };
}

describe("bridge contract", () => {
  it("builds exact nullable Req envelope", () => {
    expect(request({}, null)).toEqual({ schema_version: 1, request_id: null, payload: {} });
  });

  it("locks the exact nine-command invoke registry and six job topics", () => {
    expect(invokeRegistry).toEqual([
      "dependencies_check", "classify_file", "scan_directory", "batch_convert",
      "native_install", "native_uninstall", "preferences_get", "preferences_set",
      "picker_select",
    ]);
    expect(jobTopics).toEqual({
      dependencies_check: "job.dependencies_check",
      classify_file: "job.classify_file",
      scan_directory: "job.scan_directory",
      batch_convert: "job.batch_convert",
      native_install: "job.native_install",
      native_uninstall: "job.native_uninstall",
    });
  });

  it("sends picker intent with exact kind and selection", async () => {
    const invoke = vi.fn().mockResolvedValue({ schema_version: 1, request_id: null, data: { kind: "output", paths: [] } });
    const bridge = { invoke, listen: vi.fn() } as unknown as BridgeAdapter;
    await pickerSelect("output", "directory", bridge);
    expect(invoke).toHaveBeenCalledWith("picker_select", { request: { schema_version: 1, request_id: null, payload: { kind: "output", selection: "directory" } } });
  });

  it("keeps classify and native file/directory picker workflows explicit", async () => {
    expect(pickerActions.classifyFile).toEqual({ label: "Browse file", kind: "input", selection: "file" });
    expect(pickerActions.classifyDirectory).toEqual({ label: "Browse directory", kind: "input", selection: "directory" });
    expect(pickerActions.artifactFile).toEqual({ label: "Browse file", kind: "artifact", selection: "file" });
    expect(pickerActions.artifactDirectory).toEqual({ label: "Browse directory", kind: "artifact", selection: "directory" });
  });

  it("normalizes the A.1 error code union at runtime", async () => {
    const { ipcError } = await import("./ipc");
    expect(ipcError({ code: "forbidden", message: "denied" }).code).toBe("forbidden");
    expect(ipcError({ code: "not_in_a1", message: "bad code" }).code).toBe("internal");
  });
});

describe("terminal error reporting", () => {
  it("records an event error once and suppresses the duplicate invoke rejection", () => {
    const report = vi.fn();
    const reporter = new JobErrorReporter(report);
    const error = { code: "io", message: "failed", retryable: false, details: null } as const;
    reporter.terminal(error);
    reporter.invocation(error);
    expect(report).toHaveBeenCalledTimes(1);
    expect(report).toHaveBeenCalledWith(error);
  });

  it("records an invoke error when no terminal error event arrived", () => {
    const report = vi.fn();
    const reporter = new JobErrorReporter(report);
    reporter.invocation(new Error("listener failed"));
    expect(report).toHaveBeenCalledTimes(1);
  });
});

describe("JobClient lifecycle", () => {
  it("listens before invoke and unlistens exactly once on terminal and dispose", async () => {
    const order: string[] = [];
    const unlisten = vi.fn(() => order.push("unlisten"));
    let handler: ((value: Event<JobEvent<unknown>>) => void) | undefined;
    const bridge = {
      listen: vi.fn(async (_topic, callback) => { order.push("listen"); handler = callback as typeof handler; return unlisten; }),
      invoke: vi.fn(async (_command, args) => {
        order.push("invoke");
        const id = (args.request as { request_id: string }).request_id;
        handler?.(event("completed", id));
        return { schema_version: 1, request_id: id, data: { is_ok: true, missing: [] } };
      }),
    } as unknown as BridgeAdapter;
    const client = new JobClient(bridge);
    await client.run("dependencies_check", {}, vi.fn());
    client.dispose();
    expect(order).toEqual(["listen", "invoke", "unlisten"]);
    expect(unlisten).toHaveBeenCalledTimes(1);
  });

  it("has one listener for one active job and disposes it once", async () => {
    const unlisten = vi.fn();
    let resolveInvoke: ((value: unknown) => void) | undefined;
    const bridge = {
      listen: vi.fn(async () => unlisten),
      invoke: vi.fn(() => new Promise((resolve) => { resolveInvoke = resolve; })),
    } as unknown as BridgeAdapter;
    const client = new JobClient(bridge);
    const running = client.run("scan_directory", { directory: "/fixture", allowed_types: ["video"] }, vi.fn());
    await vi.waitFor(() => expect(bridge.invoke).toHaveBeenCalled());
    await expect(client.run("classify_file", { path: "/fixture" }, vi.fn())).rejects.toThrow("another job is active");
    client.dispose();
    client.dispose();
    expect(unlisten).toHaveBeenCalledTimes(1);
    resolveInvoke?.({ schema_version: 1, request_id: "ignored", data: {} });
    await running;
  });

  it.each(["failed", "cancelled"] as const)("keeps progress subscribed and releases once on %s", async (terminal) => {
    const unlisten = vi.fn();
    let handler: ((value: Event<JobEvent<unknown>>) => void) | undefined;
    const bridge = {
      listen: vi.fn(async (_topic, callback) => { handler = callback as typeof handler; return unlisten; }),
      invoke: vi.fn(async (_command, args) => {
        const id = (args.request as { request_id: string }).request_id;
        handler?.(event("progress", id));
        expect(unlisten).not.toHaveBeenCalled();
        handler?.(event(terminal, id));
        handler?.(event(terminal, id));
        return { schema_version: 1, request_id: id, data: {} };
      }),
    } as unknown as BridgeAdapter;
    const client = new JobClient(bridge);
    await client.run("scan_directory", { directory: "/fixture", allowed_types: ["video"] }, vi.fn());
    client.dispose();
    expect(unlisten).toHaveBeenCalledTimes(1);
  });
});

describe("app-local assets", () => {
  const paths: ResourcePaths = {
    languages: { en: "/bundle/langs/en.json", vi: "/bundle/langs/vi.json" },
    themes: { system: "/bundle/themes/system.json", light: "/bundle/themes/light.json", dark: "/bundle/themes/dark.json" },
    fonts: { primary: "/bundle/fonts/DejaVuSans.ttf" },
  };
  const viDictionary: Dictionary = { nav: { dashboard: "Tổng quan" }, settings: { save: "Lưu" } };

  it("has full recursive EN/VI key parity and translates recursively", () => {
    expect(recursiveKeys(en)).toEqual(recursiveKeys(viLanguage));
    expect(translation(viLanguage, "scan.types.archive")).toBe("Tệp nén");
    expect(translation(en, "job_states.cancelled")).toBe("Cancelled");
  });

  it("falls back for language parity mismatch and corrupt theme assets", async () => {
    const responses = new Map<string, unknown>([
      ["asset:///bundle/langs/en.json", { nav: { dashboard: "Dashboard" } }],
      ["asset:///bundle/langs/vi.json", viDictionary],
      ["asset:///bundle/themes/dark.json", { id: "dark", tokens: {} }],
      ["asset:///bundle/themes/system.json", { id: "system", tokens: { canvas: "a", foreground: "b", border: "c", panel: "d", panel_foreground: "e", accent: "f", error: "g" }, light_tokens: { canvas: "a", foreground: "b", border: "c", panel: "d", panel_foreground: "e", accent: "f", error: "g" } }],
    ]);
    vi.stubGlobal("window", { __TAURI_INTERNALS__: { convertFileSrc: (path: string) => `asset://${path}` } });
    vi.stubGlobal("fetch", vi.fn(async (url: string) => ({ ok: responses.has(url), status: 200, json: async () => responses.get(url) })));
    const loader = new ResourceLoader(paths);
    await expect(loader.loadLanguage("en")).resolves.toEqual({ id: "vi", dictionary: viDictionary });
    await expect(loader.loadTheme("dark")).resolves.toMatchObject({ id: "system" });
    vi.unstubAllGlobals();
  });

  it("rejects traversal before creating an asset URL", async () => {
    const convertFileSrc = vi.fn((path: string) => `asset://${path}`);
    vi.stubGlobal("window", { __TAURI_INTERNALS__: { convertFileSrc } });
    vi.stubGlobal("fetch", vi.fn().mockRejectedValue(new Error("missing")));
    const loader = new ResourceLoader({ ...paths, languages: { ...paths.languages, en: "/bundle/langs/../secret.json" } });
    await loader.loadLanguage("en");
    expect(convertFileSrc).not.toHaveBeenCalledWith("/bundle/langs/../secret.json", "asset");
    vi.unstubAllGlobals();
  });

  it("maps only dejavusans to the verified app-local font", () => {
    vi.stubGlobal("window", { __TAURI_INTERNALS__: { convertFileSrc: (path: string) => `asset://${path}` } });
    const loader = new ResourceLoader(paths);
    expect(loader.fontSource("dejavusans")).toEqual({
      id: "dejavusans", kind: "bundled", url: "asset:///bundle/fonts/DejaVuSans.ttf",
    });
    const missingPrimary = new ResourceLoader({ ...paths, fonts: { ...paths.fonts, primary: null } });
    expect(missingPrimary.fontSource("dejavusans")).toBeNull();
    expect(loader.fontSource("system-default")).toBeNull();
    vi.unstubAllGlobals();
  });

  it("uses the browser system font when the bundled font is corrupt", async () => {
    const add = vi.fn();
    const load = vi.fn(async function (this: { source: string }) {
      throw new Error("corrupt primary");
    });
    const FontFaceMock = vi.fn(function (this: { family: string; source: string; load: typeof load }, family: string, source: string) {
      this.family = family;
      this.source = source;
      this.load = load;
    });
    const style = { setProperty: vi.fn() };
    const documentElement = { dataset: {} as Record<string, string>, style };
    vi.stubGlobal("window", { __TAURI_INTERNALS__: { convertFileSrc: (path: string) => `asset://${path}` } });
    vi.stubGlobal("FontFace", FontFaceMock);
    vi.stubGlobal("document", { fonts: { add }, documentElement });
    await expect(applyFontAsset(new ResourceLoader(paths), "dejavusans")).resolves.toBe("system-default");
    expect(FontFaceMock).toHaveBeenCalledTimes(1);
    expect(documentElement.dataset).toMatchObject({ font: "system-default", fontSource: "system" });
    expect(style.setProperty).toHaveBeenCalledWith("--app-font", "system-ui, sans-serif");
    vi.unstubAllGlobals();
  });

  it("applies a preference change before waiting for persistence", async () => {
    const next = { language: "en", theme: "dark", font_id: "dejavusans" } as const;
    let applied = false;
    let releasePersistence: (() => void) | undefined;
    const persistence = new Promise<void>((resolve) => { releasePersistence = resolve; });
    const changing = applyAndPersistPreferenceChange(
      next,
      async (preferences) => { applied = true; return preferences; },
      async (preferences) => { expect(applied).toBe(true); await persistence; return preferences; },
    );
    await vi.waitFor(() => expect(applied).toBe(true));
    releasePersistence?.();
    await expect(changing).resolves.toEqual(next);
  });

  it("persists normalized asset fallback during startup through preferences_set", async () => {
    const loaded = { language: "en", theme: "dark", font_id: "dejavusans" } as const;
    const fallback = { language: "vi", theme: "system", font_id: "system-default" } as const;
    const persist = vi.fn(async (preferences) => preferences);

    await expect(normalizeAndPersistStartupPreferences(loaded, async () => fallback, persist)).resolves.toEqual(fallback);
    expect(persist).toHaveBeenCalledOnce();
    expect(persist).toHaveBeenCalledWith(fallback);
  });

  it("does not rewrite preferences when startup assets need no fallback", async () => {
    const loaded = { language: "vi", theme: "system", font_id: "system-default" } as const;
    const persist = vi.fn(async (preferences) => preferences);

    await expect(normalizeAndPersistStartupPreferences(loaded, async (preferences) => preferences, persist)).resolves.toEqual(loaded);
    expect(persist).not.toHaveBeenCalled();
  });
});
