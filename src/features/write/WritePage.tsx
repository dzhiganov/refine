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
    ...settings.languages.map((l) => ({ value: l.code, label: l.name })),
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

function Composer({ stale }: { stale: string | null }) {
  const draft = useApp((s) => s.draft);
  const setDraft = useApp((s) => s.setDraft);
  const check = useApp((s) => s.check);
  const checking = useApp((s) => s.checking);
  const checkError = useApp((s) => s.checkError);
  const hasResult = useApp((s) => s.entry !== null);
  const newText = useApp((s) => s.newText);
  const setView = useApp((s) => s.setView);
  const ref = useRef<HTMLTextAreaElement>(null);
  // Roomy while writing; once there's a result below, it hugs the text so both fit.
  const minHeight = hasResult ? 120 : 240;

  useLayoutEffect(() => {
    const el = ref.current;
    if (!el) return;
    el.style.height = 'auto';
    el.style.height = `${Math.max(el.scrollHeight, minHeight)}px`;
  }, [draft, minHeight]);

  useEffect(() => {
    ref.current?.focus();
  }, []);

  const clear = () => {
    newText();
    ref.current?.focus();
  };

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
        readOnly={checking}
        onChange={(e) => setDraft(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === 'Enter' && hasMod(e)) {
            e.preventDefault();
            void check();
          }
        }}
      />
      <div className={styles.composerFooter}>
        <span className={styles.count}>
          {words > 0 ? `${words} ${words === 1 ? 'word' : 'words'}` : ''}
          <AnimatePresence>
            {stale && !checking && (
              <motion.span
                className={styles.stale}
                initial={{ opacity: 0, x: -4 }}
                animate={{ opacity: 1, x: 0 }}
                exit={{ opacity: 0 }}
              >
                {stale}
              </motion.span>
            )}
          </AnimatePresence>
        </span>
        <div className={styles.actions}>
          {(draft || hasResult) && !checking && (
            <button type="button" className={ui.ghost} onClick={clear} title="Clear the text and the result">
              Clear
            </button>
          )}
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
                {hasResult ? 'Check again' : 'Check'} <Kbd keys="mod+enter" />
              </>
            )}
          </button>
        </div>
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
  const draft = useApp((s) => s.draft);
  const checking = useApp((s) => s.checking);
  const newText = useApp((s) => s.newText);
  const toneId = useApp((s) => s.settings.toneId);
  const results = useRef<HTMLDivElement>(null);
  // Why the result below no longer matches what a check would give now.
  const stale =
    entry === null
      ? null
      : draft.trimEnd() !== entry.input
        ? 'Edited since the last check'
        : toneId !== entry.tone.id
          ? 'Tone changed since the last check'
          : null;

  // Ctrl/⌘+N: start over.
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (hasMod(e) && e.key.toLowerCase() === 'n') {
        e.preventDefault();
        newText();
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [newText]);

  // A fresh result below a long text: bring its top into view.
  useEffect(() => {
    const el = results.current;
    if (!entry || !el) return;
    const top = el.getBoundingClientRect().top;
    if (top > window.innerHeight - 160) el.scrollIntoView({ behavior: 'smooth', block: 'start' });
  }, [entry?.id]); // eslint-disable-line react-hooks/exhaustive-deps

  return (
    <div className={styles.page}>
      <header className={styles.header}>
        <h1 className={styles.title}>Write</h1>
        <Toolbar />
      </header>
      <Composer stale={stale} />
      <div ref={results} className={styles.results}>
        <AnimatePresence mode="popLayout" initial={false}>
          {checking ? (
            <ResultsSkeleton key="skeleton" />
          ) : entry ? (
            <motion.div
              key={`result-${entry.id}`}
              initial={{ opacity: 0 }}
              animate={{ opacity: stale ? 0.45 : 1 }}
              exit={{ opacity: 0, transition: { duration: 0.12 } }}
              transition={{ duration: 0.25 }}
            >
              <Results entry={entry} />
            </motion.div>
          ) : null}
        </AnimatePresence>
      </div>
    </div>
  );
}
