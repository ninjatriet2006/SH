import type {
  BatchConvertReport,
  Classification,
  DependencyReport,
  FileType,
  FontId,
  JobCommand,
  JobEvent,
  JobRequest,
  JobResponse,
  Language,
  NativeOperationResult,
  PathRef,
  Preferences,
  ScanReport,
  Theme,
  JobState,
} from "./contract";
import { JobClient, call, ipcError, pickerSelect } from "./ipc";
import type { IpcError, PickerKind, PickerSelection } from "./contract";
import {
  applyFontAsset,
  applyThemeAsset,
  FALLBACK_FONT,
  FALLBACK_THEME,
  ResourceLoader,
  translation,
  type Dictionary,
  type FontId as AssetFontId,
  type LanguageId,
  type ResourcePaths,
  type ThemeId,
} from "./assets";

type View = "dashboard" | "dependencies" | "classify" | "scan" | "batch" | "native" | "settings";
type ResultValue = DependencyReport | Classification | ScanReport | BatchConvertReport | NativeOperationResult;

const fileTypes = ["archive", "video", "image", "audio", "document", "directory", "unknown"] as const satisfies readonly FileType[];
const languages = ["vi", "en"] as const satisfies readonly Language[];
const themes = ["system", "light", "dark"] as const satisfies readonly Theme[];
const fontIds = ["system-default", "dejavusans"] as const satisfies readonly FontId[];

function exactLiteral<T extends string>(value: string, values: readonly T[], field: string): T {
  if (values.includes(value as T)) return value as T;
  throw new Error(`invalid ${field}: ${value}`);
}

interface PickerAction { label: string; kind: PickerKind; selection: PickerSelection }
export const pickerActions = {
  classifyFile: { label: "Browse file", kind: "input", selection: "file" },
  classifyDirectory: { label: "Browse directory", kind: "input", selection: "directory" },
  artifactFile: { label: "Browse file", kind: "artifact", selection: "file" },
  artifactDirectory: { label: "Browse directory", kind: "artifact", selection: "directory" },
} as const satisfies Record<string, PickerAction>;

export async function applyAndPersistPreferenceChange(
  next: Preferences,
  apply: (preferences: Preferences) => Promise<Preferences>,
  persist: (preferences: Preferences) => Promise<Preferences>,
): Promise<Preferences> {
  return persist(await apply(next));
}

export async function normalizeAndPersistStartupPreferences(
  loaded: Preferences,
  apply: (preferences: Preferences) => Promise<Preferences>,
  persist: (preferences: Preferences) => Promise<Preferences>,
): Promise<Preferences> {
  const normalized = await apply(loaded);
  return preferencesEqual(loaded, normalized) ? normalized : persist(normalized);
}

function preferencesEqual(left: Preferences, right: Preferences): boolean {
  return left.language === right.language && left.theme === right.theme && left.font_id === right.font_id;
}

export class JobErrorReporter {
  private terminalReported = false;

  constructor(private readonly report: (error: unknown) => void) {}

  terminal(error: IpcError): void {
    this.terminalReported = true;
    this.report(error);
  }

  invocation(error: unknown): void {
    if (!this.terminalReported) this.report(error);
  }
}

export class App {
  private readonly jobs = new JobClient();
  private view: View = "dashboard";
  private busy = false;
  private status = "Ready";
  private error: string | null = null;
  private logs: string[] = [];
  private result: ResultValue | null = null;
  private classifyPath = "";
  private scanPath = "";
  private batchFiles: PathRef[] = [];
  private outputDirectory = "";
  private artifactPath = "";
  private preferences: Preferences = { language: "vi", theme: "system", font_id: "system-default" };
  private dictionary: Dictionary = {};
  private readonly resources: ResourceLoader;
  private preferenceChanges: Promise<void> = Promise.resolve();

  constructor(private readonly root: HTMLElement, resourcePaths?: ResourcePaths) {
    resourcePaths ??= window.__UNIVERSAL_CONVERTER_RESOURCES__;
    if (!resourcePaths) throw new Error("app-local resources were not initialized");
    this.resources = new ResourceLoader(resourcePaths);
  }

  async start(): Promise<void> {
    window.addEventListener("beforeunload", () => this.dispose(), { once: true });
    await applyFontAsset(this.resources, FALLBACK_FONT);
    try {
      const loaded = await call<Preferences>("preferences_get", {});
      this.preferences = await normalizeAndPersistStartupPreferences(
        loaded,
        async (preferences) => {
          this.preferences = preferences;
          await this.applyPreferences();
          return this.preferences;
        },
        (preferences) => call<Preferences>("preferences_set", preferences),
      );
    } catch (error) {
      this.recordError(error);
    }
    this.render();
  }

  dispose(): void { this.jobs.dispose(); }

  private async applyPreferences(): Promise<void> {
    const language = await this.resources.loadLanguage(this.preferences.language as LanguageId);
    const theme = await this.resources.loadTheme(this.preferences.theme as ThemeId);
    const font = await applyFontAsset(this.resources, this.preferences.font_id as AssetFontId);
    this.dictionary = language.dictionary;
    document.documentElement.lang = language.id;
    applyThemeAsset(theme);
    if (this.status === "Ready") this.status = this.tr("app.ready");
    if (language.id !== this.preferences.language || theme?.id !== this.preferences.theme || font !== this.preferences.font_id) {
      this.preferences = { language: language.id, theme: theme?.id ?? FALLBACK_THEME, font_id: font };
    }
  }

  private tr(key: string, values?: Record<string, string | number>): string { return translation(this.dictionary, key, values); }

  private log(message: string): void {
    this.logs.push(`${new Date().toLocaleTimeString()} ${message}`);
  }

  private recordError(error: unknown): void {
    const parsed = ipcError(error);
    this.error = `${parsed.code}: ${parsed.message}`;
    this.status = this.tr("app.failed");
    this.log(this.error);
  }

  private handleJobEvent<T extends ResultValue>(event: JobEvent<T>, errors: JobErrorReporter): void {
    this.status = event.payload.message ?? this.jobStateLabel(event.state);
    if (event.payload.completed !== null && event.payload.total !== null) {
      this.status += ` (${event.payload.completed}/${event.payload.total})`;
    }
    const terminal = event.payload.result;
    if (terminal?.status === "completed") this.result = terminal.value;
    else if (terminal) errors.terminal(terminal.error);
    this.log(this.status);
    this.render();
  }

  private jobStateLabel(state: JobState): string { return this.tr(`job_states.${state}`); }

  private async run<K extends JobCommand>(command: K, payload: JobRequest<K>): Promise<void> {
    this.busy = true;
    this.error = null;
    this.result = null;
    this.status = this.tr("app.starting");
    this.render();
    const errors = new JobErrorReporter((error) => this.recordError(error));
    try {
      const response = await this.jobs.run(command, payload, (event) => this.handleJobEvent(event, errors));
      this.result = response.data as JobResponse<K>;
      this.status = this.tr("app.completed");
    } catch (error) {
      errors.invocation(error);
    } finally {
      this.busy = false;
      this.render();
    }
  }

  private async pick(kind: "input" | "output" | "artifact", selection: "file" | "files" | "directory"): Promise<void> {
    try {
      const selected = await pickerSelect(kind, selection);
      if (kind === "output") this.outputDirectory = selected.paths[0]?.path ?? "";
      else if (kind === "artifact") this.artifactPath = selected.paths[0]?.path ?? "";
      else if (this.view === "classify") this.classifyPath = selected.paths[0]?.path ?? "";
      else if (this.view === "scan") this.scanPath = selected.paths[0]?.path ?? "";
      else this.batchFiles = selected.paths;
      this.error = null;
    } catch (error) { this.recordError(error); }
    this.render();
  }

  private pickAction(action: PickerAction): void {
    void this.pick(action.kind, action.selection);
  }

  private navigation(): string {
    const views: [View, string][] = [["dashboard", "nav.dashboard"], ["dependencies", "nav.dependencies"], ["classify", "nav.classify"], ["scan", "nav.scan"], ["batch", "nav.batch"], ["native", "nav.native"], ["settings", "nav.settings"]];
    return views.map(([id, key]) => `<button data-view="${id}" class="${id === this.view ? "active" : ""}>${this.tr(key)}</button>`).join("");
  }

  private panel(): string {
    const disabled = this.busy ? "disabled" : "";
    if (this.view === "dashboard") return `<section><h2>${this.tr("nav.dashboard")}</h2><p class="status">${this.status}</p><p>${this.tr("dashboard.description")}</p></section>`;
    if (this.view === "dependencies") return `<section><h2>${this.tr("nav.dependencies")}</h2><button id="deps" ${disabled}>${this.tr("dependencies.check")}</button></section>`;
    if (this.view === "classify") return `<section><h2>${this.tr("classify.title")}</h2><div class="row"><input readonly value="${escapeHtml(this.classifyPath)}" placeholder="${this.tr("classify.placeholder")}"><button id="classify-file-pick" ${disabled}>${this.tr("classify.browse_file")}</button><button id="classify-directory-pick" ${disabled}>${this.tr("classify.browse_directory")}</button><button id="classify-run" ${disabled}>${this.tr("classify.run")}</button></div></section>`;
    if (this.view === "scan") return `<section><h2>${this.tr("scan.title")}</h2><div class="row"><input readonly value="${escapeHtml(this.scanPath)}" placeholder="${this.tr("scan.placeholder")}"><button id="scan-pick" ${disabled}>${this.tr("scan.browse")}</button><button id="scan-run" ${disabled}>${this.tr("scan.run")}</button></div><fieldset>${["archive", "video", "image", "audio", "document"].map((type) => `<label><input type="checkbox" name="type" value="${type}" checked>${this.tr(`scan.types.${type}`)}</label>`).join("")}</fieldset></section>`;
    if (this.view === "batch") return `<section><h2>${this.tr("batch.title")}</h2><div class="row"><button id="batch-input" ${disabled}>${this.tr("batch.select_files")}</button><span>${this.tr("batch.files_selected", { count: this.batchFiles.length })}</span></div><div class="row"><input readonly value="${escapeHtml(this.outputDirectory)}" placeholder="${this.tr("batch.output_placeholder")}"><button id="batch-output" ${disabled}>${this.tr("batch.browse")}</button></div><div class="row"><label>${this.tr("batch.format")} <input id="format" value="mp4"></label><label><input id="overwrite" type="checkbox"> ${this.tr("batch.overwrite")}</label><button id="batch-run" ${disabled}>${this.tr("batch.convert")}</button></div></section>`;
    if (this.view === "native") return `<section><h2>${this.tr("native.title")}</h2><div class="row"><input readonly value="${escapeHtml(this.artifactPath)}" placeholder="${this.tr("native.artifact_placeholder")}"><button id="artifact-file-pick" ${disabled}>${this.tr("native.browse_file")}</button><button id="artifact-directory-pick" ${disabled}>${this.tr("native.browse_directory")}</button></div><div class="row"><label>${this.tr("native.target_directory")} <input id="target" placeholder="${this.tr("native.target_placeholder")}"></label><button id="install" ${disabled}>${this.tr("native.install")}</button></div><div class="row"><label>${this.tr("native.installation_id")} <input id="installation-id"></label><button id="uninstall" ${disabled}>${this.tr("native.uninstall")}</button></div><p>${this.tr("native.notice")}</p></section>`;
    return `<section><h2>${this.tr("nav.settings")}</h2><label>${this.tr("settings.language")} <select id="language"><option value="vi">Tiếng Việt</option><option value="en">English</option></select></label><label>${this.tr("settings.theme")} <select id="theme"><option value="system">${this.tr("settings.system")}</option><option value="light">${this.tr("settings.light")}</option><option value="dark">${this.tr("settings.dark")}</option></select></label><label>${this.tr("settings.font")} <select id="font"><option value="system-default">${this.tr("settings.system_default")}</option><option value="dejavusans">${this.tr("settings.dejavu_sans")}</option></select></label><button id="save-settings">${this.tr("settings.save")}</button></section>`;
  }

  private render(): void {
    this.root.innerHTML = `<header><h1>${this.tr("app.title")}</h1><span>${this.busy ? this.tr("app.working") : this.status}</span></header><div class="layout"><nav>${this.navigation()}</nav><main>${this.error ? `<div class="error" role="alert">${escapeHtml(this.error)}</div>` : ""}${this.panel()}${this.result ? `<pre>${escapeHtml(JSON.stringify(this.result, null, 2))}</pre>` : ""}</main></div><aside><div><strong>${this.tr("log.title")}</strong><button id="clear-log">${this.tr("log.clear")}</button></div><pre>${escapeHtml(this.logs.join("\n"))}</pre></aside>`;
    this.bind();
  }

  private bind(): void {
    this.root.querySelectorAll<HTMLButtonElement>("[data-view]").forEach((button) => button.addEventListener("click", () => { this.view = button.dataset.view as View; this.render(); }));
    this.on("clear-log", () => { this.logs = []; this.render(); });
    this.on("deps", () => void this.run("dependencies_check", {}));
    this.on("classify-file-pick", () => this.pickAction(pickerActions.classifyFile));
    this.on("classify-directory-pick", () => this.pickAction(pickerActions.classifyDirectory));
    this.on("classify-run", () => void this.run("classify_file", { path: this.classifyPath }));
    this.on("scan-pick", () => void this.pick("input", "directory"));
    this.on("scan-run", () => void this.run("scan_directory", { directory: this.scanPath, allowed_types: [...this.root.querySelectorAll<HTMLInputElement>('input[name="type"]:checked')].map((input) => exactLiteral(input.value, fileTypes, "file type")) }));
    this.on("batch-input", () => void this.pick("input", "files"));
    this.on("batch-output", () => void this.pick("output", "directory"));
    this.on("batch-run", () => void this.run("batch_convert", { files: this.batchFiles, output_directory: { path: this.outputDirectory }, output_format: this.value("format"), overwrite: this.checked("overwrite") }));
    this.on("artifact-file-pick", () => this.pickAction(pickerActions.artifactFile));
    this.on("artifact-directory-pick", () => this.pickAction(pickerActions.artifactDirectory));
    this.on("install", () => { if (window.confirm(this.tr("native.confirm_install"))) void this.run("native_install", { artifact: { path: this.artifactPath }, target_dir: { path: this.value("target") }, confirmed: true }); });
    this.on("uninstall", () => { if (window.confirm(this.tr("native.confirm_uninstall"))) void this.run("native_uninstall", { installation_id: this.value("installation-id"), confirmed: true }); });
    this.on("save-settings", () => this.queuePreferenceChange(this.selectedPreferences(), true));
    this.setValue("language", this.preferences.language); this.setValue("theme", this.preferences.theme); this.setValue("font", this.preferences.font_id);
    for (const id of ["language", "theme", "font"]) {
      this.root.querySelector<HTMLSelectElement>(`#${id}`)?.addEventListener("change", () => {
        this.queuePreferenceChange(this.selectedPreferences(), false);
      });
    }
  }

  private selectedPreferences(): Preferences {
    return {
      language: exactLiteral(this.value("language"), languages, "language"),
      theme: exactLiteral(this.value("theme"), themes, "theme"),
      font_id: exactLiteral(this.value("font"), fontIds, "font"),
    };
  }

  private queuePreferenceChange(next: Preferences, announce: boolean): void {
    this.preferenceChanges = this.preferenceChanges.then(() => this.savePreferences(next, announce));
  }

  private async savePreferences(next: Preferences, announce: boolean): Promise<void> {
    try {
      this.preferences = await applyAndPersistPreferenceChange(
        next,
        async (preferences) => {
          this.preferences = preferences;
          await this.applyPreferences();
          this.render();
          return this.preferences;
        },
        (preferences) => call<Preferences>("preferences_set", preferences),
      );
      if (announce) { this.status = this.tr("app.settings_saved"); this.log(this.status); }
    }
    catch (error) { this.recordError(error); }
    this.render();
  }

  private on(id: string, callback: () => void): void { this.root.querySelector(`#${id}`)?.addEventListener("click", callback); }
  private value(id: string): string { return (this.root.querySelector<HTMLInputElement | HTMLSelectElement>(`#${id}`)?.value ?? "").trim(); }
  private checked(id: string): boolean { return this.root.querySelector<HTMLInputElement>(`#${id}`)?.checked ?? false; }
  private setValue(id: string, value: string): void { const element = this.root.querySelector<HTMLSelectElement>(`#${id}`); if (element) element.value = value; }
}

export function escapeHtml(value: string): string {
  return value.replace(/[&<>'"]/g, (character) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", "'": "&#39;", '"': "&quot;" })[character] ?? character);
}
