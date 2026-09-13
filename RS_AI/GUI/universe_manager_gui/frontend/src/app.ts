import type { AppEntry, DetectionReport, IpcError, JobEvent, ManagerConfig, Preferences, SearchReport } from "./contract";
import { api, ipcError, JobClient, pickerSelect } from "./ipc";
import { loadResources, translate, type Messages } from "./resources";

type View = "dashboard" | "config" | "scan" | "manager" | "search" | "settings";
const defaultPreferences: Preferences = { language: "vi", theme: "system", font_id: "system-default" };

export class UniverseApp {
  private view: View = "dashboard";
  private config: ManagerConfig | null = null;
  private preferences = defaultPreferences;
  private messages: Messages = {};
  private busy = false;
  private progress = "";
  private error = "";
  private detection: DetectionReport | null = null;
  private search: SearchReport | null = null;
  private configSaved = false;
  private readonly jobs: JobClient;
  private readonly services: {
    api: typeof api;
    loadResources: typeof loadResources;
    pickerSelect: typeof pickerSelect;
    confirm: (message: string) => boolean;
  };

  constructor(private readonly root: HTMLElement, services: {
    api?: typeof api;
    loadResources?: typeof loadResources;
    pickerSelect?: typeof pickerSelect;
    confirm?: (message: string) => boolean;
    jobs?: JobClient;
  } = {}) {
    this.jobs = services.jobs ?? new JobClient();
    this.services = {
      api: services.api ?? api,
      loadResources: services.loadResources ?? loadResources,
      pickerSelect: services.pickerSelect ?? pickerSelect,
      confirm: services.confirm ?? ((message) => window.confirm(message)),
    };
  }

  async start(): Promise<void> {
    try { this.preferences = await this.services.api.getPreferences(); } catch { this.preferences = defaultPreferences; }
    this.messages = await this.services.loadResources(this.preferences);
    try { this.config = await this.services.api.loadConfig(); this.configSaved = true; } catch (error) { this.error = ipcError(error).message; }
    this.render();
  }

  dispose(): void { this.jobs.dispose(); }

  private t(key: string): string { return translate(this.messages, key); }
  private setError(error: unknown): void { this.error = ipcError(error).message; this.busy = false; this.render(); }
  private apps(): AppEntry[] { return this.config?.apps ?? []; }

  private async job(command: "scan_apps" | "detect_app" | "start_app" | "stop_app" | "search_apps", payload: object): Promise<unknown> {
    this.busy = true; this.error = ""; this.progress = "Starting…"; this.render();
    try {
      const response = await this.jobs.run(command, payload as never, (event: JobEvent<unknown>) => {
        this.progress = event.payload.message ?? event.state;
        const result = event.payload.result;
        if (result?.status === "failed" || result?.status === "cancelled") this.error = result.error.message;
        this.render();
      });
      this.busy = false; this.render(); return response.data;
    } catch (error) { this.setError(error); return null; }
  }

  private render(): void {
    const nav: View[] = ["dashboard", "config", "scan", "manager", "search", "settings"];
    this.root.innerHTML = `<div class="shell"><aside><h1>${this.t("app.title")}</h1>${nav.map((item) =>
      `<button class="nav ${this.view === item ? "active" : ""}" data-view="${item}">${this.t(`nav.${item}`)}</button>`).join("")}</aside>
      <main><header><h2>${this.t(`nav.${this.view}`)}</h2><span>${this.busy ? escapeHtml(this.progress) : ""}</span></header>
      ${this.error ? `<div class="error" role="alert">${escapeHtml(this.error)}</div>` : ""}${this.content()}</main></div>`;
    this.bind();
  }

  private content(): string {
    if (this.view === "dashboard") return `<section class="cards"><article><strong>${this.apps().length}</strong><span>Managed apps</span></article><article><strong>${this.apps().filter((app) => app.is_custom).length}</strong><span>Custom apps</span></article><article><strong>${escapeHtml(this.config?.settings.managed_dir ?? "—")}</strong><span>Managed directory</span></article></section>`;
    if (this.view === "config") return `<section><label>Managed directory</label><div class="row"><input id="managed" readonly value="${escapeAttr(this.config?.settings.managed_dir ?? "")}"><button id="pick-managed">Select…</button></div><button id="save-config" ${!this.config || this.busy ? "disabled" : ""}>Save config</button></section>`;
    if (this.view === "scan") return `<section><p>Scan the managed directory or inspect an app selected from a source directory.</p><div class="row"><button id="scan" ${this.busy ? "disabled" : ""}>Scan managed apps</button><button id="detect" class="secondary" ${this.busy ? "disabled" : ""}>Select & detect source…</button></div>${this.detection ? this.detectionView() : this.appTable(this.apps())}</section>`;
    if (this.view === "manager") return `<section>${this.appTable(this.apps(), true)}</section>`;
    if (this.view === "search") return `<section><form id="search-form" class="row"><input id="query" required placeholder="App name"><button ${this.busy ? "disabled" : ""}>Search</button></form>${this.searchTable()}</section>`;
    return `<section class="form"><label>${this.t("settings.language")}<select id="language"><option value="en">English</option><option value="vi">Tiếng Việt</option></select></label><label>${this.t("settings.theme")}<select id="theme"><option value="system">${this.t("settings.system")}</option><option value="light">${this.t("settings.light")}</option><option value="dark">${this.t("settings.dark")}</option></select></label><label>${this.t("settings.font")}<select id="font"><option value="system-default">${this.t("settings.system_default")}</option><option value="dejavusans">DejaVu Sans</option></select></label><button id="save-preferences">Apply & save</button></section>`;
  }

  private appTable(apps: AppEntry[], actions = false): string {
    if (!apps.length) return `<p class="empty">No applications.</p>`;
    const disabled = this.busy || !this.configSaved ? " disabled" : "";
    return `<div class="table"><div class="tr heading"><span>Name</span><span>Version</span><span>Type</span><span>Actions</span></div>${apps.map((app) => `<div class="tr"><span>${escapeHtml(app.name)}</span><span>${escapeHtml(app.version ?? "—")}</span><span>${app.install_type}</span><span>${actions ? `<button data-action="start" data-id="${escapeAttr(app.id)}"${disabled}>Start</button><button class="secondary" data-action="stop" data-id="${escapeAttr(app.id)}"${disabled}>Stop</button>` : ""}</span></div>`).join("")}</div>`;
  }

  private searchTable(): string {
    const results = this.search?.results ?? [];
    if (!this.search) return "";
    if (!results.length) return `<p class="empty">No results.</p>`;
    return `<div class="table"><div class="tr heading"><span>Name</span><span>ID</span><span>Version</span><span>Source</span></div>${results.map((item) => `<div class="tr"><span>${escapeHtml(item.name)}</span><span>${escapeHtml(item.id)}</span><span>${escapeHtml(item.version)}</span><span>${escapeHtml(item.source)}</span></div>`).join("")}</div>`;
  }

  private detectionView(): string {
    const report = this.detection;
    if (!report) return "";
    const paths = (label: string, values: { path: string }[]): string => `<p><strong>${label}:</strong> ${values.length ? values.map((item) => escapeHtml(item.path)).join(", ") : "—"}</p>`;
    return `<article class="report"><h3>${escapeHtml(report.suggested_name)}</h3><p>AppImage: ${report.is_appimage ? "Yes" : "No"}</p>${paths("Executables", report.executables)}${paths("Icons", report.icons)}${paths("Desktop templates", report.desktop_templates)}</article>`;
  }

  private bind(): void {
    this.root.querySelectorAll<HTMLElement>("[data-view]").forEach((button) => button.addEventListener("click", () => { this.view = button.dataset.view as View; this.error = ""; this.render(); }));
    this.root.querySelector("#pick-managed")?.addEventListener("click", () => void this.pickManaged());
    this.root.querySelector("#save-config")?.addEventListener("click", () => void this.saveConfig());
    this.root.querySelector("#scan")?.addEventListener("click", () => void this.scan());
    this.root.querySelector("#detect")?.addEventListener("click", () => void this.detect());
    this.root.querySelector("#search-form")?.addEventListener("submit", (event) => void this.runSearch(event));
    this.root.querySelector("#save-preferences")?.addEventListener("click", () => void this.savePreferences());
    this.root.querySelectorAll<HTMLElement>("[data-action]").forEach((button) => button.addEventListener("click", () => void this.action(button.dataset.action as "start" | "stop", button.dataset.id ?? "")));
    const values: [string, string][] = [["language", this.preferences.language], ["theme", this.preferences.theme], ["font", this.preferences.font_id]];
    for (const [id, value] of values) { const input = this.root.querySelector<HTMLSelectElement>(`#${id}`); if (input) input.value = value; }
  }

  private async pickManaged(): Promise<void> {
    try {
      const selected = await this.services.pickerSelect("managed");
      this.config = this.config ?? { settings: { managed_dir: selected.path.path }, apps: [] };
      this.config.settings.managed_dir = selected.path.path; this.configSaved = false; this.render();
    } catch (error) { this.setError(error); }
  }

  private async saveConfig(): Promise<void> {
    if (!this.config) return;
    this.busy = true; this.error = ""; this.render();
    try {
      this.config = await this.services.api.saveConfig(this.config);
      this.configSaved = true; this.busy = false; this.progress = "Saved"; this.render();
    } catch (error) { this.configSaved = false; this.setError(error); }
  }

  private async scan(): Promise<void> {
    const result = await this.job("scan_apps", {});
    if (Array.isArray(result) && this.config) {
      this.config.apps = result as AppEntry[];
      this.configSaved = false;
      this.render();
      await this.saveConfig();
    }
  }

  private async detect(): Promise<void> {
    try {
      const selected = await this.services.pickerSelect("source");
      const result = await this.job("detect_app", { path: selected.path });
      if (result) { this.detection = result as DetectionReport; this.render(); }
    } catch (error) { this.setError(error); }
  }

  private async action(operation: "start" | "stop", appId: string): Promise<void> {
    if (this.busy || !this.configSaved) return;
    const app = this.apps().find((item) => item.id === appId);
    if (!app || !this.services.confirm(`${operation === "start" ? "Start" : "Stop"} ${app.name}?`)) return;
    await this.job(operation === "start" ? "start_app" : "stop_app", { app_id: appId, confirmed: true });
  }

  private async runSearch(event: Event): Promise<void> {
    event.preventDefault(); const query = this.root.querySelector<HTMLInputElement>("#query")?.value.trim() ?? "";
    if (!query) return;
    const result = await this.job("search_apps", { query });
    if (result) { this.search = result as SearchReport; this.render(); }
  }

  private async savePreferences(): Promise<void> {
    const value = (id: string): string => this.root.querySelector<HTMLSelectElement>(`#${id}`)?.value ?? "";
    const next: Preferences = { language: value("language") as Preferences["language"], theme: value("theme") as Preferences["theme"], font_id: value("font") as Preferences["font_id"] };
    try { this.preferences = await this.services.api.setPreferences(next); this.messages = await this.services.loadResources(this.preferences); this.render(); } catch (error) { this.setError(error); }
  }
}

function escapeHtml(value: string): string { const node = document.createElement("span"); node.textContent = value; return node.innerHTML; }
function escapeAttr(value: string): string { return escapeHtml(value).replaceAll('"', "&quot;"); }

export function terminalError(event: JobEvent<unknown>): IpcError | null {
  const result = event.payload.result;
  return result?.status === "failed" || result?.status === "cancelled" ? result.error : null;
}
