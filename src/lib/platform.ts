import type { Platform } from './api';

let current: Platform = 'web';

export function setPlatform(p: Platform) {
  current = p;
  document.documentElement.dataset.platform = p;
}

export function platform(): Platform {
  return current;
}

const isAppleWeb = () => typeof navigator !== 'undefined' && /Mac|iPhone|iPad/.test(navigator.platform);

export function isMac(): boolean {
  return current === 'macos' || (current === 'web' && isAppleWeb());
}

/** ⌘ on macOS, Ctrl elsewhere. */
export function modKey(): string {
  return isMac() ? '⌘' : 'Ctrl';
}

/** Label for a shortcut like `mod+Enter` → "⌘ ↵" / "Ctrl ↵". */
export function shortcutLabel(keys: string): string {
  const map: Record<string, string> = {
    mod: modKey(),
    shift: isMac() ? '⇧' : 'Shift',
    alt: isMac() ? '⌥' : 'Alt',
    enter: '↵',
    esc: 'Esc',
  };
  return keys
    .split('+')
    .map((k) => map[k.toLowerCase()] ?? k.toUpperCase())
    .join(isMac() ? '' : '+');
}

/** True when the event has the platform's primary modifier pressed. */
export function hasMod(e: { metaKey: boolean; ctrlKey: boolean }): boolean {
  return isMac() ? e.metaKey : e.ctrlKey;
}
