import type {
  CapabilityReport, DistributionMode, DistributionReport, FontId, ImageScanReport, ImageSettings,
  JobCommand, JobEvent, JobRequest, JobResponse, JobState, Language, PathRef, Preferences,
  ProcessReport, Theme,
} from "./contract";
import { JobClient, call, ipcError, pickerSelect } from "./ipc";
import {
  applyFont, applyTheme, FALLBACK_FONT, FALLBACK_THEME, ResourceLoader, translation,
  type Dictionary, type LanguageId, type ResourcePaths, type ThemeId,
} from "./assets";

type View = "dashboard" | "settings" | "scan" | "environment" | "process" | "distribute";
type ResultValue = CapabilityReport | ImageScanReport | ProcessReport | DistributionReport | ImageSettings;
const languages = ["en", "vi"] as const satisfies readonly Language[];
const themes = ["system", "light", "dark"] as const satisfies readonly Theme[];
const fonts = ["system-default", "dejavusans"] as const satisfies readonly FontId[];
const modes = ["balanced", "greedy", "fixed"] as const satisfies readonly DistributionMode[];

function literal<T extends string>(value: string, values: readonly T[], field: string): T {
  if (values.includes(value as T)) return value as T;
  throw new Error(`invalid ${field}: ${value}`);
}

export async function applyAndPersistPreferences(
  next: Preferences,
  apply: (preferences: Preferences) => Promise<Preferences>,
  persist: (preferences: Preferences) => Promise<Preferences>,
): Promise<Preferences> { return persist(await apply(next)); }

export async function normalizeStartupPreferences(
  loaded: Preferences,
  apply: (preferences: Preferences) => Promise<Preferences>,
  persist: (preferences: Preferences) => Promise<Preferences>,
): Promise<Preferences> {
  const normalized = await apply(loaded);
  return loaded.language === normalized.language && loaded.theme === normalized.theme && loaded.font_id === normalized.font_id
    ? normalized : persist(normalized);
}

export class App {
  private readonly jobs = new JobClient();
  private readonly resources: ResourceLoader;
  private view: View = "dashboard";
  private busy = false;
  private status = "Ready";
  private error: string | null = null;
  private logs: string[] = [];
  private result: ResultValue | null = null;
  private inputDirectory = "";
  private outputDirectory = "";
  private files: PathRef[] = [];
  private capabilities: CapabilityReport | null = null;
  private settings: ImageSettings = {
    default_distribution_mode: "balanced", max_files_per_folder: 80, fixed_folder_count: 5,
    max_retries: 5, min_upscale_width: 600, target_upscale_width: 1280,
  };
  private preferences: Preferences = { language: "en", theme: "dark", font_id: "system-default" };
  private dictionary: Dictionary = {};
  private preferenceChanges: Promise<void> = Promise.resolve();

  constructor(private readonly root: HTMLElement, resourcePaths?: ResourcePaths) {
    resourcePaths ??= window.__IMG_SPLT_RESOURCES__;
    if (!resourcePaths) throw new Error("app-local resources were not initialized");
    this.resources = new ResourceLoader(resourcePaths);
  }

  async start(): Promise<void> {
    window.addEventListener("beforeunload", () => this.dispose(), { once: true });
    await applyFont(this.resources, FALLBACK_FONT);
    try {
      const [preferences, settings] = await Promise.all([
        call<Preferences>("preferences_get", {}), call<ImageSettings>("settings_load", {}),
      ]);
      this.settings = settings;
      this.preferences = await normalizeStartupPreferences(
        preferences,
        async (next) => { this.preferences = next; await this.applyPreferences(); return this.preferences; },
        (next) => call<Preferences>("preferences_set", next),
      );
    } catch (error) { this.recordError(error); }
    this.render();
  }

  dispose(): void { this.jobs.dispose(); }

  private async applyPreferences(): Promise<void> {
    const language = await this.resources.loadLanguage(this.preferences.language as LanguageId);
    const theme = await this.resources.loadTheme(this.preferences.theme as ThemeId);
    const font = await applyFont(this.resources, this.preferences.font_id);
    this.dictionary = language.dictionary;
    document.documentElement.lang = language.id;
    const themeId = applyTheme(theme);
    if (language.id !== this.preferences.language || themeId !== this.preferences.theme || font !== this.preferences.font_id) {
      this.preferences = { language: language.id, theme: themeId ?? FALLBACK_THEME, font_id: font };
    }
  }

  private tr(key: string, values?: Record<string, string | number>): string { return translation(this.dictionary, key, values); }
  private log(message: string): void { this.logs.push(`${new Date().toLocaleTimeString()} ${message}`); }
  private recordError(error: unknown): void {
    const parsed = ipcError(error);
    this.error = `${parsed.code}: ${parsed.message}`;
    this.status = this.tr("app.failed");
    this.log(this.error);
  }

  private jobState(state: JobState): string { return this.tr(`job_states.${state}`); }
  private event<T extends ResultValue>(event: JobEvent<T>): void {
    this.status = event.payload.message ?? this.jobState(event.state);
    if (event.payload.completed !== null && event.payload.total !== null) this.status += ` (${event.payload.completed}/${event.payload.total})`;
    const terminal = event.payload.result;
    if (terminal?.status === "completed") this.result = terminal.value;
    else if (terminal) this.recordError(terminal.error);
    this.log(this.status);
    this.render();
  }

  private async run<K extends JobCommand>(command: K, payload: JobRequest<K>): Promise<void> {
    this.busy = true; this.error = null; this.result = null; this.status = this.tr("app.starting"); this.render();
    try {
      const response = await this.jobs.run(command, payload, (event) => this.event(event));
      this.result = response.data as JobResponse<K>;
      if (command === "capabilities_check") this.capabilities = response.data as CapabilityReport;
      if (command === "scan_images") this.files = (response.data as ImageScanReport).images;
      this.status = this.tr("app.completed");
    } catch (error) { this.recordError(error); }
    finally { this.busy = false; this.render(); }
  }

  private async pick(kind: "input" | "output"): Promise<void> {
    try {
      const selected = await pickerSelect(kind);
      if (kind === "input") { this.inputDirectory = selected.path.path; this.files = []; }
      else this.outputDirectory = selected.path.path;
      this.error = null;
    } catch (error) { this.recordError(error); }
    this.render();
  }

  private nav(): string {
    const views: [View, string][] = [
      ["dashboard", "nav.dashboard"], ["settings", "nav.settings"], ["scan", "nav.scan"],
      ["environment", "nav.environment"], ["process", "nav.process"], ["distribute", "nav.distribute"],
    ];
    return views.map(([id, key]) => `<button data-view="${id}" class="${id === this.view ? "active" : ""}">${this.tr(key)}</button>`).join("");
  }

  private paths(): string {
    return `<div class="path-grid"><label>${this.tr("common.input")}<span><input readonly value="${escapeHtml(this.inputDirectory)}"><button id="pick-input">${this.tr("common.browse")}</button></span></label><label>${this.tr("common.output")}<span><input readonly value="${escapeHtml(this.outputDirectory)}"><button id="pick-output">${this.tr("common.browse")}</button></span></label></div>`;
  }

  private settingsFields(): string {
    return `<div class="form-grid"><label>${this.tr("settings.mode")}<select id="mode">${modes.map((mode) => `<option value="${mode}">${this.tr(`modes.${mode}`)}</option>`).join("")}</select></label><label>${this.tr("settings.max_files")}<input id="max-files" type="number" min="1" value="${this.settings.max_files_per_folder}"></label><label>${this.tr("settings.fixed_folders")}<input id="fixed-folders" type="number" min="1" value="${this.settings.fixed_folder_count}"></label><label>${this.tr("settings.retries")}<input id="retries" type="number" min="0" value="${this.settings.max_retries}"></label><label>${this.tr("settings.min_width")}<input id="min-width" type="number" min="1" value="${this.settings.min_upscale_width}"></label><label>${this.tr("settings.target_width")}<input id="target-width" type="number" min="1" value="${this.settings.target_upscale_width}"></label></div>`;
  }

  private panel(): string {
    const disabled = this.busy ? "disabled" : "";
    if (this.view === "dashboard") return `<section><h2>${this.tr("nav.dashboard")}</h2><p>${this.tr("dashboard.description")}</p><div class="cards"><article><strong>${this.tr("dashboard.images")}</strong><b>${this.files.length}</b></article><article><strong>FFmpeg</strong><b>${this.toolStatus(this.capabilities?.ffmpeg)}</b></article><article><strong>FFprobe</strong><b>${this.toolStatus(this.capabilities?.ffprobe)}</b></article></div><p class="status">${escapeHtml(this.status)}</p></section>`;
    if (this.view === "settings") return `<section><h2>${this.tr("nav.settings")}</h2><div class="form-grid"><label>${this.tr("settings.language")}<select id="language"><option value="en">English</option><option value="vi">Tiếng Việt</option></select></label><label>${this.tr("settings.theme")}<select id="theme"><option value="system">${this.tr("settings.system")}</option><option value="light">${this.tr("settings.light")}</option><option value="dark">${this.tr("settings.dark")}</option></select></label><label>${this.tr("settings.font")}<select id="font"><option value="system-default">${this.tr("settings.system_default")}</option><option value="dejavusans">DejaVu Sans</option></select></label></div><h3>${this.tr("settings.processing")}</h3>${this.settingsFields()}<button id="save-settings" ${disabled}>${this.tr("settings.save")}</button></section>`;
    if (this.view === "scan") return `<section><h2>${this.tr("nav.scan")}</h2>${this.paths()}<button id="scan" ${disabled}>${this.tr("scan.run")}</button><p>${this.tr("scan.found", { count: this.files.length })}</p><pre class="files">${escapeHtml(this.files.map((file) => file.path).join("\n"))}</pre></section>`;
    if (this.view === "environment") return `<section><h2>${this.tr("nav.environment")}</h2><button id="capabilities" ${disabled}>${this.tr("environment.check")}</button>${this.capabilities ? `<div class="cards"><article><strong>FFmpeg</strong><b>${this.toolStatus(this.capabilities.ffmpeg)}</b><small>${escapeHtml(this.capabilities.ffmpeg.version ?? "")}</small></article><article><strong>FFprobe</strong><b>${this.toolStatus(this.capabilities.ffprobe)}</b><small>${escapeHtml(this.capabilities.ffprobe.version ?? "")}</small></article></div>` : ""}</section>`;
    if (this.view === "process") return `<section><h2>${this.tr("nav.process")}</h2>${this.paths()}<p>${this.tr("common.selected", { count: this.files.length })}</p><div class="row"><label>${this.tr("process.format")}<input id="format" placeholder="png"></label><label><input id="upscale" type="checkbox"> ${this.tr("process.upscale")}</label></div><p class="warning">${this.tr("common.output_warning")}</p><button id="process" ${disabled}>${this.tr("process.run")}</button></section>`;
    return `<section><h2>${this.tr("nav.distribute")}</h2>${this.paths()}<p>${this.tr("common.selected", { count: this.files.length })}</p><div class="row"><label>${this.tr("settings.mode")}<select id="dist-mode">${modes.map((mode) => `<option value="${mode}">${this.tr(`modes.${mode}`)}</option>`).join("")}</select></label><label>${this.tr("distribute.chapter")}<input id="chapter" type="number" min="0"></label></div><p class="warning">${this.tr("common.output_warning")}</p><button id="distribute" ${disabled}>${this.tr("distribute.run")}</button></section>`;
  }

  private toolStatus(tool: CapabilityReport["ffmpeg"] | undefined): string {
    if (!tool) return this.tr("environment.unknown");
    return tool.available ? this.tr("environment.available") : this.tr("environment.missing");
  }

  private render(): void {
    this.root.innerHTML = `<header><h1>${this.tr("app.title")}</h1><span>${this.busy ? this.tr("app.working") : escapeHtml(this.status)}</span></header><div class="layout"><nav>${this.nav()}</nav><main>${this.error ? `<div class="error" role="alert">${escapeHtml(this.error)}</div>` : ""}${this.panel()}${this.result ? `<details><summary>${this.tr("common.result")}</summary><pre>${escapeHtml(JSON.stringify(this.result, null, 2))}</pre></details>` : ""}</main></div><aside><div><strong>${this.tr("log.title")}</strong><button id="clear-log">${this.tr("log.clear")}</button></div><pre>${escapeHtml(this.logs.join("\n"))}</pre></aside>`;
    this.bind();
  }

  private bind(): void {
    this.root.querySelectorAll<HTMLButtonElement>("[data-view]").forEach((button) => button.addEventListener("click", () => { this.view = button.dataset.view as View; this.render(); }));
    this.on("clear-log", () => { this.logs = []; this.render(); });
    this.on("pick-input", () => void this.pick("input")); this.on("pick-output", () => void this.pick("output"));
    this.on("capabilities", () => void this.run("capabilities_check", {}));
    this.on("scan", () => void this.run("scan_images", { directory: { path: this.inputDirectory } }));
    this.on("process", () => {
      if (window.confirm(this.tr("process.confirm"))) void this.run("process_images", {
        input_directory: { path: this.inputDirectory }, files: this.files,
        output_directory: { path: this.outputDirectory }, output_format: this.value("format") || null,
        upscale: this.checked("upscale"), settings: this.settings,
      });
    });
    this.on("distribute", () => {
      if (window.confirm(this.tr("distribute.confirm"))) void this.run("distribute", {
        input_directory: { path: this.inputDirectory }, files: this.files,
        output_directory: { path: this.outputDirectory }, chapter: this.optionalNumber("chapter"),
        mode: literal(this.value("dist-mode"), modes, "mode"),
        max_files_per_folder: this.settings.max_files_per_folder, fixed_folder_count: this.settings.fixed_folder_count,
      });
    });
    this.on("save-settings", () => void this.saveAllSettings());
    this.setValue("language", this.preferences.language); this.setValue("theme", this.preferences.theme);
    this.setValue("font", this.preferences.font_id); this.setValue("mode", this.settings.default_distribution_mode);
    this.setValue("dist-mode", this.settings.default_distribution_mode);
    for (const id of ["language", "theme", "font"]) this.root.querySelector(`#${id}`)?.addEventListener("change", () => this.queuePreferences());
  }

  private readSettings(): ImageSettings {
    return {
      default_distribution_mode: literal(this.value("mode"), modes, "mode"),
      max_files_per_folder: this.number("max-files"), fixed_folder_count: this.number("fixed-folders"),
      max_retries: this.number("retries"), min_upscale_width: this.number("min-width"),
      target_upscale_width: this.number("target-width"),
    };
  }

  private async saveAllSettings(): Promise<void> {
    try {
      this.settings = await call<ImageSettings>("settings_save", this.readSettings());
      await this.savePreferences();
      this.status = this.tr("app.settings_saved"); this.log(this.status);
    } catch (error) { this.recordError(error); }
    this.render();
  }

  private queuePreferences(): void { this.preferenceChanges = this.preferenceChanges.then(() => this.savePreferences()); }
  private async savePreferences(): Promise<void> {
    const next: Preferences = {
      language: literal(this.value("language"), languages, "language"),
      theme: literal(this.value("theme"), themes, "theme"),
      font_id: literal(this.value("font"), fonts, "font"),
    };
    this.preferences = await applyAndPersistPreferences(
      next,
      async (preferences) => { this.preferences = preferences; await this.applyPreferences(); this.render(); return this.preferences; },
      (preferences) => call<Preferences>("preferences_set", preferences),
    );
  }

  private on(id: string, callback: () => void): void { this.root.querySelector(`#${id}`)?.addEventListener("click", callback); }
  private value(id: string): string { return (this.root.querySelector<HTMLInputElement | HTMLSelectElement>(`#${id}`)?.value ?? "").trim(); }
  private checked(id: string): boolean { return this.root.querySelector<HTMLInputElement>(`#${id}`)?.checked ?? false; }
  private number(id: string): number { return Number(this.value(id)); }
  private optionalNumber(id: string): number | null { const value = this.value(id); return value ? Number(value) : null; }
  private setValue(id: string, value: string): void { const element = this.root.querySelector<HTMLInputElement | HTMLSelectElement>(`#${id}`); if (element) element.value = value; }
}

export function escapeHtml(value: string): string {
  return value.replace(/[&<>'"]/g, (character) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", "'": "&#39;", '"': "&quot;" })[character] ?? character);
}
