import { create } from 'zustand';
import type { CategoryInfo } from '../bindings/CategoryInfo';
import type { Entry } from '../bindings/Entry';
import type { ProviderStatus } from '../bindings/ProviderStatus';
import type { Settings } from '../bindings/Settings';
import type { Tone } from '../bindings/Tone';
import { api, errorKind, errorMessage, type Bootstrap } from '../lib/api';

export type View = 'write' | 'history' | 'progress' | 'settings';

export interface CheckError {
  message: string;
  /** Problems the user fixes in Settings (no key, no model) get a button there. */
  fixInSettings: boolean;
}

interface AppState {
  ready: boolean;
  appVersion: string;
  settings: Settings;
  tones: Tone[];
  providers: ProviderStatus[];
  categories: Record<string, CategoryInfo>;

  view: View;
  draft: string;
  /** The last result, shown under the editor (fresh or opened from History). It stays while the
   *  text is edited; the page marks it out of date until the next check. */
  entry: Entry | null;
  checking: boolean;
  checkError: CheckError | null;

  init(b: Bootstrap): void;
  setView(view: View): void;
  setDraft(text: string): void;
  check(): Promise<void>;
  openEntry(entry: Entry): void;
  /** Clears the editor and the result. */
  newText(): void;
  saveSettings(next: Settings): Promise<void>;
  setTones(tones: Tone[]): void;
  setProviders(p: ProviderStatus[]): void;
}

export const useApp = create<AppState>((set, get) => ({
  ready: false,
  appVersion: '',
  settings: null as unknown as Settings,
  tones: [],
  providers: [],
  categories: {},

  view: 'write',
  draft: '',
  entry: null,
  checking: false,
  checkError: null,

  init: (b) =>
    set({
      ready: true,
      appVersion: b.appVersion,
      settings: b.settings,
      tones: b.tones,
      providers: b.providers,
      categories: Object.fromEntries(b.categories.map((c) => [c.id, c])),
    }),

  setView: (view) => set({ view }),

  setDraft: (draft) => set({ draft, checkError: null }),

  check: async () => {
    const { draft, settings, checking } = get();
    if (checking || !draft.trim()) return;
    set({ checking: true, checkError: null });
    try {
      const entry = await api().checkText({
        text: draft,
        toneId: settings.toneId,
        language: settings.languageMode === 'auto' ? null : settings.languageMode,
      });
      set({ entry, checking: false });
    } catch (e) {
      const kind = errorKind(e);
      set({
        checking: false,
        checkError: { message: errorMessage(e), fixInSettings: kind === 'missingKey' || kind === 'noModel' },
      });
    }
  },

  openEntry: (entry) => set({ entry, draft: entry.input, view: 'write', checkError: null }),

  newText: () => set({ entry: null, draft: '', checkError: null }),

  saveSettings: async (next) => {
    const prev = get().settings;
    set({ settings: next });
    try {
      set({ settings: await api().saveSettings(next) });
    } catch (e) {
      set({ settings: prev });
      throw e;
    }
  },

  setTones: (tones) => set({ tones }),
  setProviders: (providers) => set({ providers }),
}));
