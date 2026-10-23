import { motion } from 'motion/react';
import type { ReactNode } from 'react';
import { resolveTheme } from '../lib/theme';
import { useApp, type View } from '../stores/app';
import { ClockIcon, MoonIcon, PenIcon, SettingsIcon, SunIcon, TrendIcon } from './icons';
import styles from './Rail.module.css';

const ITEMS: { view: View; label: string; icon: ReactNode; key: string }[] = [
  { view: 'write', label: 'Write', icon: <PenIcon />, key: '1' },
  { view: 'history', label: 'History', icon: <ClockIcon />, key: '2' },
  { view: 'progress', label: 'Progress', icon: <TrendIcon />, key: '3' },
];

export function Rail() {
  const view = useApp((s) => s.view);
  const setView = useApp((s) => s.setView);
  const settings = useApp((s) => s.settings);
  const saveSettings = useApp((s) => s.saveSettings);
  const isDark = resolveTheme(settings.theme) === 'dark';

  const item = (v: View, label: string, icon: ReactNode) => (
    <button
      key={v}
      type="button"
      className={styles.button}
      aria-label={label}
      aria-current={view === v ? 'page' : undefined}
      title={label}
      onClick={() => setView(v)}
    >
      {view === v && <motion.span layoutId="rail-active" className={styles.active} />}
      <span className={styles.icon}>{icon}</span>
    </button>
  );

  return (
    <nav className={styles.rail} aria-label="Main" data-tauri-drag-region>
      <div className={styles.top} data-tauri-drag-region>
        <span className={styles.wordmark}>refine</span>
        <div className={styles.group}>{ITEMS.map((i) => item(i.view, i.label, i.icon))}</div>
      </div>
      <div className={styles.group}>
        <button
          type="button"
          className={styles.button}
          aria-label={isDark ? 'Switch to light theme' : 'Switch to dark theme'}
          title={isDark ? 'Light theme' : 'Dark theme'}
          onClick={() => void saveSettings({ ...settings, theme: isDark ? 'light' : 'dark' })}
        >
          <span className={styles.icon}>{isDark ? <SunIcon /> : <MoonIcon />}</span>
        </button>
        {item('settings', 'Settings', <SettingsIcon />)}
      </div>
    </nav>
  );
}

export const VIEW_SHORTCUTS = ITEMS.map((i) => ({ view: i.view, key: i.key }));
