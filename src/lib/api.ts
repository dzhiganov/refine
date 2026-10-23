/**
 * Typed bridge to the Rust core. Inside Tauri it calls commands; in a plain browser (vite dev,
 * tests, design reviews) it falls back to an in-memory mock with the same contract.
 * All types come from src/bindings (generated from Rust by `pnpm bindings`).
 */
import type { AppError } from '../bindings/AppError';
import type { CategoryInfo } from '../bindings/CategoryInfo';
import type { CheckRequest } from '../bindings/CheckRequest';
import type { Entry } from '../bindings/Entry';
import type { HistoryPage } from '../bindings/HistoryPage';
import type { Insights } from '../bindings/Insights';
import type { ModelInfo } from '../bindings/ModelInfo';
import type { ProviderConfig } from '../bindings/ProviderConfig';
import type { ProviderStatus } from '../bindings/ProviderStatus';
import type { Settings } from '../bindings/Settings';
import type { Stats } from '../bindings/Stats';
import type { Tone } from '../bindings/Tone';

export type Platform = 'macos' | 'windows' | 'linux' | 'web';

export interface Bootstrap {
  appVersion: string;
  platform: Platform;
  settings: Settings;
  tones: Tone[];
  providers: ProviderStatus[];
  categories: CategoryInfo[];
}

export interface Api {
  bootstrap(): Promise<Bootstrap>;
  saveSettings(settings: Settings): Promise<Settings>;
  setApiKey(providerId: string, key: string): Promise<ProviderStatus[]>;
  listModels(provider: ProviderConfig): Promise<ModelInfo[]>;
  saveTone(tone: Tone): Promise<Tone>;
  deleteTone(id: string): Promise<Tone[]>;
  checkText(request: CheckRequest): Promise<Entry>;
  history(args: { before?: string | null; limit?: number; query?: string }): Promise<HistoryPage>;
  getEntry(id: string): Promise<Entry>;
  deleteEntry(id: string): Promise<void>;
  getStats(): Promise<Stats>;
  getInsights(): Promise<Insights | null>;
  refreshInsights(): Promise<Insights>;
  /** Pin (true), dismiss (false) or reset (null) a watch-list pattern. */
  setPatternPref(key: string, pinned: boolean | null): Promise<Stats>;
  recordReview(key: string, passed: boolean): Promise<Stats>;
  /** Writes the export file and returns its path. */
  exportHistory(): Promise<string>;
  revealPath(path: string): Promise<void>;
  touch(): Promise<void>;
  onEvent(name: 'stats-updated' | 'insights-updated', cb: () => void): () => void;
}

export const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

export function errorMessage(e: unknown): string {
  if (e && typeof e === 'object' && 'message' in e) return String((e as AppError).message);
  return String(e);
}

export function errorKind(e: unknown): AppError['kind'] | null {
  return e && typeof e === 'object' && 'kind' in e ? (e as AppError).kind : null;
}

async function tauriApi(): Promise<Api> {
  const { invoke } = await import('@tauri-apps/api/core');
  const { listen } = await import('@tauri-apps/api/event');
  const { revealItemInDir } = await import('@tauri-apps/plugin-opener');
  return {
    bootstrap: () => invoke('bootstrap'),
    saveSettings: (settings) => invoke('save_settings', { settings }),
    setApiKey: (providerId, key) => invoke('set_api_key', { providerId, key }),
    listModels: (provider) => invoke('list_models', { provider }),
    saveTone: (tone) => invoke('save_tone', { tone }),
    deleteTone: (id) => invoke('delete_tone', { id }),
    checkText: (request) => invoke('check_text', { request }),
    history: ({ before, limit, query }) => invoke('history', { before: before ?? null, limit, query }),
    getEntry: (id) => invoke('get_entry', { id }),
    deleteEntry: (id) => invoke('delete_entry', { id }),
    getStats: () => invoke('get_stats'),
    getInsights: () => invoke('get_insights'),
    refreshInsights: () => invoke('refresh_insights'),
    setPatternPref: (key, pinned) => invoke('set_pattern_pref', { key, pinned }),
    recordReview: (key, passed) => invoke('record_review', { key, passed }),
    exportHistory: () => invoke('export_history'),
    revealPath: (path) => revealItemInDir(path),
    touch: () => invoke('touch'),
    onEvent: (name, cb) => {
      const un = listen(name, () => cb());
      return () => void un.then((f) => f());
    },
  };
}

let instance: Api | null = null;

export async function initApi(): Promise<Api> {
  if (!instance) instance = isTauri ? await tauriApi() : (await import('./mock')).createMockApi();
  return instance;
}

/** The API after `initApi()` has resolved (the app awaits it before first render). */
export function api(): Api {
  if (!instance) throw new Error('initApi() has not run');
  return instance;
}

/** For tests. */
export function setApi(next: Api) {
  instance = next;
}
