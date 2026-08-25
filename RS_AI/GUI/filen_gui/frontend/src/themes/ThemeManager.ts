/**
 * filen_gui — ThemeManager (docs/themes-runtime.md §2, v3.0.0)
 *
 * Runtime custom themes: load/parse/validate/merge 3 tầng/apply lên `:root`.
 * Framework-agnostic (không phụ thuộc Tauri API) — phase 1 scaffold dùng
 * loader fetch từ `public/themes/`; phase sau thay bằng command Rust
 * (themes-runtime.md §2.3 lựa chọn B) mà không đổi API class này.
 *
 * Merge 3 tầng: `default ⊕ user ⊕ runtime` (i18n-and-themes.md §3.5).
 * Apply: `setProperty("--token-name", value)` trên `:root` — không rebuild DOM.
 */
import { DEFAULT_TOKENS } from "./defaultTokens";

export interface ThemeFile {
  name: string;
  version: string;
  tokens: Record<string, string>;
}

export interface ThemeEntry {
  slug: string; // tên file (không đuôi .json)
  file: string; // path/url tuyệt đối
  name: string;
  version: string;
  valid: boolean; // false nếu parse/validate fail cấp file
  errors: string[]; // warning/error log (key lỗi không làm hỏng theme)
  tokens: Record<string, string>; // tokens đã validate (chỉ key hợp lệ)
}

const COLOR_HEX_RE = /^#([0-9A-Fa-f]{6}|[0-9A-Fa-f]{8})$/;
const COLOR_ALPHA_RE = /^#[0-9A-Fa-f]{6}@0\.\d+$/;
const COLOR_RGBA_RE = /^rgba?\([^)]*\)$/;
const REF_RE = /^@[A-Za-z0-9.]+$/;

/** Map token `group.subgroup.name` → CSS var `--group-subgroup-name`. */
function tokenToCssVar(token: string): string {
  return `--${token.replace(/\./g, "-")}`;
}

/** Token thuộc nhóm màu (colors.*) — cần validate hex/rgba. */
function isColorToken(token: string): boolean {
  return token.startsWith("colors.");
}

/**
 * Resolve giá trị theme: nếu là reference `@token` thì duyệt chuỗi reference
 * (có cycle detection) tới literal. Trả về null + push warning nếu lỗi.
 */
function resolveRef(
  key: string,
  value: string,
  raw: Record<string, string>,
  defaults: Record<string, string>,
  errors: string[],
): string | null {
  if (!REF_RE.test(value)) {
    // literal — validate màu nếu token thuộc nhóm màu
    if (isColorToken(key) && !COLOR_HEX_RE.test(value) && !COLOR_ALPHA_RE.test(value) && !COLOR_RGBA_RE.test(value)) {
      errors.push(`[${key}] màu không hợp lệ — bỏ qua`);
      return null;
    }
    return value;
  }

  const seen = new Set<string>([key]);
  let cur: string | undefined = value;
  while (typeof cur === "string" && REF_RE.test(cur)) {
    const target = cur.slice(1);
    if (seen.has(target)) {
      errors.push(`[${key}] reference cycle (${target}) — bỏ qua`);
      return null;
    }
    if (!(target in defaults) && !(target in raw)) {
      errors.push(`[${key}] reference tới token không tồn tại — bỏ qua`);
      return null;
    }
    seen.add(target);
    cur = target in raw ? raw[target] : defaults[target];
  }
  if (typeof cur !== "string" || REF_RE.test(cur)) {
    errors.push(`[${key}] reference không resolve được — bỏ qua`);
    return null;
  }
  return cur;
}

/** Validate 1 theme file (i18n-and-themes.md §3.6). Key lỗi → warning, không fail. */
function validateThemeFile(
  raw: unknown,
  file: string,
  slug: string,
  defaults: Record<string, string>,
): ThemeEntry {
  const errors: string[] = [];
  const base: ThemeEntry = { slug, file, name: slug, version: "", valid: false, errors, tokens: {} };

  if (typeof raw !== "object" || raw === null) {
    return { ...base, errors: ["JSON root không phải object"] };
  }
  const obj = raw as Record<string, unknown>;
  if (typeof obj.name !== "string") return { ...base, errors: ["thiếu name (string)"] };
  if (typeof obj.version !== "string") return { ...base, errors: ["thiếu version (string)"] };
  if (typeof obj.tokens !== "object" || obj.tokens === null) {
    return { ...base, errors: ["thiếu tokens (object)"] };
  }

  const rawTokens = obj.tokens as Record<string, unknown>;
  const tokens: Record<string, string> = {};
  for (const [key, value] of Object.entries(rawTokens)) {
    if (typeof value !== "string") {
      errors.push(`[${key}] value không phải string — bỏ qua`);
      continue;
    }
    if (!(key in defaults)) {
      errors.push(`[${key}] unknown token — bỏ qua`);
      continue;
    }
    if (key.startsWith("on")) {
      errors.push(`[${key}] key "on*" bị chặn (security) — bỏ qua`);
      continue;
    }
    const resolved = resolveRef(key, value, rawTokens as Record<string, string>, defaults, errors);
    if (resolved !== null) tokens[key] = resolved;
  }

  return { slug, file, name: obj.name, version: obj.version, valid: true, errors, tokens };
}

export class ThemeManager {
  private readonly defaultTokens: Record<string, string>;
  private runtimeTweaks: Record<string, string> = {};
  private current: ThemeEntry | null = null;
  private hotReloadCbs: Array<(entry: ThemeEntry | null) => void> = [];

  constructor(defaultTokens: Record<string, string> = DEFAULT_TOKENS) {
    this.defaultTokens = defaultTokens;
  }

  /** Parse chuỗi JSON → ThemeEntry (validate). */
  parse(json: string, file: string): ThemeEntry {
    const slug = file.split("/").pop()?.replace(/\.json$/, "") ?? "theme";
    let raw: unknown;
    try {
      raw = JSON.parse(json);
    } catch {
      return { slug, file, name: slug, version: "", valid: false, errors: ["JSON sai cú pháp"], tokens: {} };
    }
    return validateThemeFile(raw, file, slug, this.defaultTokens);
  }

  /** Load + validate 1 file theme từ text. */
  loadOne(file: string, text: string): ThemeEntry {
    return this.parse(text, file);
  }

  /**
   * Load mọi theme từ thư mục (dev: fetch `public/themes/*.json`).
   * Production thay bằng command Rust `themes_list` (themes-runtime.md §2.3).
   */
  async loadAll(baseUrl = `${import.meta.env.BASE_URL ?? "/"}themes/`): Promise<ThemeEntry[]> {
    const entries: ThemeEntry[] = [];
    try {
      const res = await fetch(baseUrl);
      if (!res.ok) return entries;
      const files = (await res.json()) as string[];
      for (const file of files) {
        if (!file.endsWith(".json")) continue;
        const textRes = await fetch(`${baseUrl}${file}`);
        if (!textRes.ok) continue;
        entries.push(this.loadOne(`${baseUrl}${file}`, await textRes.text()));
      }
    } catch {
      // không có thư mục themes → trả về rỗng, dùng default
    }
    return entries;
  }

  /** Merge 3 tầng + apply lên `:root`. entry=null → default theme. */
  apply(entry: ThemeEntry | null): void {
    this.current = entry;
    this.reapply();
  }

  /** Runtime tweak (tầng 3) — không ghi file. */
  setRuntimeTweak(token: string, value: string): void {
    this.runtimeTweaks[token] = value;
    this.reapply();
  }

  clearRuntimeTweaks(): void {
    this.runtimeTweaks = {};
    this.reapply();
  }

  getActive(): string | null {
    return this.current?.slug ?? null;
  }

  /** Đăng ký callback khi hot-reload (Rust watcher emit `themes:changed`). */
  onHotReload(cb: (entry: ThemeEntry | null) => void): () => void {
    this.hotReloadCbs.push(cb);
    return () => {
      this.hotReloadCbs = this.hotReloadCbs.filter((c) => c !== cb);
    };
  }

  private reapply(): void {
    const finalTokens: Record<string, string> = { ...this.defaultTokens };
    if (this.current?.valid) {
      for (const [k, v] of Object.entries(this.current.tokens)) finalTokens[k] = v;
    }
    for (const [k, v] of Object.entries(this.runtimeTweaks)) finalTokens[k] = v;
    this.writeToRoot(finalTokens);
    const entry = this.current;
    this.hotReloadCbs.forEach((cb) => cb(entry));
  }

  private writeToRoot(tokens: Record<string, string>): void {
    const root = document.documentElement;
    for (const [token, value] of Object.entries(tokens)) {
      root.style.setProperty(tokenToCssVar(token), value);
    }
  }
}