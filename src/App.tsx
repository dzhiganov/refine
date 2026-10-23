import { useQueryClient } from '@tanstack/react-query';
import { AnimatePresence, motion } from 'motion/react';
import { useEffect } from 'react';
import { Rail, VIEW_SHORTCUTS } from './components/Rail';
import { Toaster } from './components/Toaster';
import { HistoryPage } from './features/history/HistoryPage';
import { ProgressPage } from './features/progress/ProgressPage';
import { SettingsPage } from './features/settings/SettingsPage';
import { WritePage } from './features/write/WritePage';
import { api } from './lib/api';
import { hasMod } from './lib/platform';
import { useApplyTheme } from './lib/theme';
import { useApp, type View } from './stores/app';
import styles from './App.module.css';

const PAGES: Record<View, () => React.JSX.Element> = {
  write: WritePage,
  history: HistoryPage,
  progress: ProgressPage,
  settings: SettingsPage,
};

/** Background stats work waits for idle time; tell the core when the user is active. */
function useActivityPing() {
  useEffect(() => {
    let last = 0;
    const ping = () => {
      const now = Date.now();
      if (now - last > 20_000) {
        last = now;
        void api().touch();
      }
    };
    window.addEventListener('keydown', ping);
    window.addEventListener('pointerdown', ping);
    return () => {
      window.removeEventListener('keydown', ping);
      window.removeEventListener('pointerdown', ping);
    };
  }, []);
}

/** Refetch stats when the idle worker has refreshed them. */
function useBackgroundUpdates() {
  const qc = useQueryClient();
  useEffect(() => {
    const offs = [
      api().onEvent('stats-updated', () => void qc.invalidateQueries({ queryKey: ['stats'] })),
      api().onEvent('insights-updated', () => void qc.invalidateQueries({ queryKey: ['insights'] })),
    ];
    return () => offs.forEach((off) => off());
  }, [qc]);
}

function useGlobalShortcuts() {
  const setView = useApp((s) => s.setView);
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (!hasMod(e) || e.altKey) return;
      const hit = VIEW_SHORTCUTS.find((s) => s.key === e.key);
      if (hit) {
        e.preventDefault();
        setView(hit.view);
      } else if (e.key === ',') {
        e.preventDefault();
        setView('settings');
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [setView]);
}

export function App() {
  const view = useApp((s) => s.view);
  const theme = useApp((s) => s.settings.theme);
  const entry = useApp((s) => s.entry);
  const qc = useQueryClient();
  useApplyTheme(theme);
  useActivityPing();
  useBackgroundUpdates();
  useGlobalShortcuts();

  // A new check changes history and stats.
  useEffect(() => {
    if (entry) {
      void qc.invalidateQueries({ queryKey: ['history'] });
      void qc.invalidateQueries({ queryKey: ['stats'] });
    }
  }, [entry, qc]);

  const Page = PAGES[view];
  return (
    <div className={styles.shell}>
      <div className={styles.dragStrip} data-tauri-drag-region />
      <Rail />
      <main className={styles.main}>
        <AnimatePresence mode="wait" initial={false}>
          <motion.div
            key={view}
            className={styles.page}
            initial={{ opacity: 0, y: 6, filter: 'blur(2px)' }}
            animate={{ opacity: 1, y: 0, filter: 'blur(0px)' }}
            exit={{ opacity: 0, y: -4, filter: 'blur(2px)', transition: { duration: 0.12 } }}
            transition={{ type: 'spring', stiffness: 420, damping: 36 }}
          >
            <Page />
          </motion.div>
        </AnimatePresence>
      </main>
      <Toaster />
    </div>
  );
}
