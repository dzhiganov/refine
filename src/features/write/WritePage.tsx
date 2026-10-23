import { useQuery } from '@tanstack/react-query';
import { AnimatePresence, motion } from 'motion/react';
import { useEffect, useLayoutEffect, useRef } from 'react';
import { Kbd, Spinner } from '../../components/controls';
import { PillSelect } from '../../components/Menu';
import ui from '../../components/ui.module.css';
import { api } from '../../lib/api';
import { hasMod } from '../../lib/platform';
import { useApp } from '../../stores/app';
import { Results, ResultsSkeleton } from './Results';
import styles from './WritePage.module.css';

function wordCount(text: string) {
  return text.split(/\s+/).filter((w) => /[\p{L}\p{N}]/u.test(w)).length;
}

function Toolbar() {
  const settings = useApp((s) => s.settings);
  const tones = useApp((s) => s.tones);
  const saveSettings = useApp((s) => s.saveSettings);
  const setView = useApp((s) => s.setView);

  const languages = [
    { value: 'auto', label: 'Auto-detect', hint: settings.languages.map((l) => l.name).join(' · ') },
    ...settings.languages.map((l) => ({ value: l.code, label: l.name, hint: `Level ${l.level}` })),
  ];
  const toneOptions = tones.map((t) => ({ value: t.id, label: t.name, hint: t.description }));

  return (
    <div className={styles.toolbar}>
      <PillSelect
        label="Language"
        value={settings.languageMode}
        options={languages}
        onChange={(languageMode) => void saveSettings({ ...settings, languageMode })}
      />
      <PillSelect
        label="Tone"
        value={settings.toneId}
        options={toneOptions}
        onChange={(toneId) => void saveSettings({ ...settings, toneId })}
        footer={
          <button type="button" className={styles.menuLink} onClick={() => setView('settings')}>
            Edit tones…
          </button>
        }
      />
      <WatchingChips />
    </div>
  );
}

/** The patterns being worked on, as a quiet reminder while writing (noticing, §3.3). */
function WatchingChips() {
  const languageMode = useApp((s) => s.settings.languageMode);
  const { data } = useQuery({ queryKey: ['stats'], queryFn: () => api().getStats() });
  const watched = (data?.languages ?? [])
    .filter((l) => languageMode === 'auto' || l.language === languageMode)
    .flatMap((l) => l.patterns.filter((p) => p.onWatchList).map((p) => ({ ...p, lang: l.language })))
    .slice(0, 3);
  if (watched.length === 0) return null;
  return (
    <div className={styles.watching} aria-label="Patterns you're watching">
      <span className={styles.watchingLabel}>Watching</span>
      {watched.map((p) => (
        <span key={p.key} className={styles.watchChip} title={p.examples[0]?.explanation ?? p.label}>
          {p.label}
        </span>
      ))}
    </div>
  );
}

function Composer() {
  const draft = useApp((s) => s.draft);
  const setDraft = useApp((s) => s.setDraft);
  const check = useApp((s) => s.check);
  const checking = useApp((s) => s.checking);
  const checkError = useApp((s) => s.checkError);
  const setView = useApp((s) => s.setView);
  const ref = useRef<HTMLTextAreaElement>(null);

  // Grow with the text.
  useLayoutEffect(() => {
    const el = ref.current;
    if (!el) return;
    el.style.height = 'auto';
    el.style.height = `${Math.max(el.scrollHeight, 240)}px`;
  }, [draft]);

  useEffect(() => {
    ref.current?.focus();
  }, []);

  const words = wordCount(draft);
  return (
    <motion.div layout className={styles.composer} data-busy={checking || undefined}>
      <textarea
        ref={ref}
        className={styles.textarea}
        value={draft}
        placeholder="Paste or type your text…"
        spellCheck={false}
        aria-label="Your text"
        disabled={checking}
        onChange={(e) => setDraft(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === 'Enter' && hasMod(e)) {
            e.preventDefault();
            void check();
          }
        }}
      />
      <div className={styles.composerFooter}>
        <span className={styles.count}>{words > 0 ? `${words} ${words === 1 ? 'word' : 'words'}` : ''}</span>
        <button
          type="button"
          className={ui.primary}
          disabled={checking || !draft.trim()}
          onClick={() => void check()}
        >
          {checking ? (
            <>
              <Spinner /> Checking
            </>
          ) : (
            <>
              Check <Kbd keys="mod+enter" />
            </>
          )}
        </button>
      </div>
      <AnimatePresence>
        {checkError && (
          <motion.div
            className={styles.error}
            role="alert"
            initial={{ opacity: 0, height: 0 }}
            animate={{ opacity: 1, height: 'auto' }}
            exit={{ opacity: 0, height: 0 }}
          >
            <span>{checkError.message}</span>
            {checkError.fixInSettings && (
              <button type="button" className={`${ui.secondary} ${ui.small}`} onClick={() => setView('settings')}>
                Open Settings
              </button>
            )}
          </motion.div>
        )}
      </AnimatePresence>
    </motion.div>
  );
}

export function WritePage() {
  const entry = useApp((s) => s.entry);
  const checking = useApp((s) => s.checking);

  return (
    <div className={styles.page}>
      <header className={styles.header}>
        <h1 className={styles.title}>{entry ? 'Refined' : 'Write'}</h1>
        {!entry && <Toolbar />}
      </header>
      <AnimatePresence mode="popLayout" initial={false}>
        {entry ? (
          <motion.div
            key={`result-${entry.id}`}
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0, transition: { duration: 0.12 } }}
          >
            <Results entry={entry} />
          </motion.div>
        ) : (
          <motion.div
            key="compose"
            initial={{ opacity: 0, y: 8 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: -8, transition: { duration: 0.12 } }}
          >
            <Composer />
            <AnimatePresence>{checking && <ResultsSkeleton />}</AnimatePresence>
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}
