import { useEffect } from 'react';
import type { Theme } from '../bindings/Theme';
import { isTauri } from './api';

export type ResolvedTheme = 'dark' | 'light';

export function resolveTheme(theme: Theme): ResolvedTheme {
  if (theme !== 'system') return theme;
  return window.matchMedia?.('(prefers-color-scheme: light)').matches ? 'light' : 'dark';
}

async function syncWindowTheme(theme: Theme) {
  if (!isTauri) return;
  const { getCurrentWindow } = await import('@tauri-apps/api/window');
  // null = follow the OS; also recolors the native title bar on Windows.
  await getCurrentWindow()
    .setTheme(theme === 'system' ? null : theme)
    .catch(() => {});
}

/** Keeps <html data-theme> (and the native window) in sync with the setting and the OS. */
export function useApplyTheme(theme: Theme) {
  useEffect(() => {
    const apply = () => {
      document.documentElement.dataset.theme = resolveTheme(theme);
    };
    apply();
    try {
      localStorage.setItem('refine-theme', theme);
    } catch {
      /* private mode */
    }
    void syncWindowTheme(theme);
    if (theme !== 'system' || !window.matchMedia) return;
    const mq = window.matchMedia('(prefers-color-scheme: light)');
    mq.addEventListener('change', apply);
    return () => mq.removeEventListener('change', apply);
  }, [theme]);
}
