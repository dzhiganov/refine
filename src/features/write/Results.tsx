import { motion, type Variants } from 'motion/react';
import { useCallback, useEffect, useMemo, useState } from 'react';
import { Kbd, Segmented } from '../../components/controls';
import { CheckIcon, CopyIcon, SparkleIcon } from '../../components/icons';
import { toast } from '../../components/Toaster';
import ui from '../../components/ui.module.css';
import type { DiffPart } from '../../bindings/DiffPart';
import type { Entry } from '../../bindings/Entry';
import type { Mistake } from '../../bindings/Mistake';
import type { Suggestion } from '../../bindings/Suggestion';
import { hasMod } from '../../lib/platform';
import { useApp } from '../../stores/app';
import styles from './Results.module.css';

const block: Variants = {
  hidden: { opacity: 0, y: 14, filter: 'blur(4px)' },
  shown: (i: number) => ({
    opacity: 1,
    y: 0,
    filter: 'blur(0px)',
    transition: { type: 'spring', stiffness: 300, damping: 30, delay: 0.06 + i * 0.08 },
  }),
};

const item: Variants = {
  hidden: { opacity: 0, y: 8 },
  shown: (i: number) => ({ opacity: 1, y: 0, transition: { delay: 0.22 + i * 0.05 } }),
};

/** Does a diff run belong to this mistake? Good enough for hover highlighting. */
function partMatches(part: DiffPart, m: Mistake) {
  const t = part.text.trim();
  if (!t) return false;
  if (part.op === 'delete') return m.original.includes(t);
  if (part.op === 'insert') return m.correction.includes(t);
  return false;
}

type ViewMode = 'changes' | 'clean';
const VIEW_KEY = 'refine-fixed-view';

function savedView(): ViewMode {
  try {
    return localStorage.getItem(VIEW_KEY) === 'changes' ? 'changes' : 'clean';
  } catch {
    return 'clean';
  }
}

function FixedBlock({ entry, hovered, context }: { entry: Entry; hovered: Mistake | null; context: string }) {
  // Clean by default (the text you'll copy); the choice sticks.
  const [mode, setModeState] = useState<ViewMode>(savedView);
  const setMode = (m: ViewMode) => {
    setModeState(m);
    try {
      localStorage.setItem(VIEW_KEY, m);
    } catch {
      /* private mode */
    }
  };
  // Hovering a mistake shows where it is, even in the clean view.
  const showChanges = mode === 'changes' || hovered !== null;
  const [copied, setCopied] = useState(false);
  const clean = entry.mistakes.length === 0;

  const copy = useCallback(async () => {
    try {
      await navigator.clipboard.writeText(entry.fixedText);
      setCopied(true);
      setTimeout(() => setCopied(false), 1600);
    } catch {
      toast("Couldn't copy. Select the text and copy it manually.");
    }
  }, [entry.fixedText]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (hasMod(e) && e.shiftKey && e.key.toLowerCase() === 'c') {
        e.preventDefault();
        void copy();
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [copy]);

  return (
    <motion.section className={`${styles.block} ${styles.fixed}`} variants={block} custom={0} aria-label="Fixed text">
      <header className={styles.blockHeader}>
        <div className={styles.blockTitle}>
          <h2>Fixed text</h2>
          {clean ? (
            <span className={styles.allGood}>
              <CheckIcon size={14} /> Nothing to fix
            </span>
          ) : (
            <span className={styles.count}>
              {entry.mistakes.length} {entry.mistakes.length === 1 ? 'change' : 'changes'}
            </span>
          )}
          <span className={styles.context}>{context}</span>
        </div>
        <div className={styles.blockActions}>
          {!clean && (
            <Segmented
              label="View"
              value={mode}
              onChange={setMode}
              options={[
                { value: 'changes', label: 'Changes' },
                { value: 'clean', label: 'Clean' },
              ]}
            />
          )}
          <button type="button" className={ui.primary} onClick={() => void copy()}>
            {copied ? <CheckIcon size={16} /> : <CopyIcon size={16} />}
            {copied ? 'Copied' : 'Copy'}
            <Kbd keys="mod+shift+c" />
          </button>
        </div>
      </header>
      <div className={styles.text} data-selectable>
        {!showChanges || clean
          ? entry.fixedText
          : entry.diff.map((part, i) => {
              if (part.op === 'equal') return <span key={i}>{part.text}</span>;
              const lit = hovered ? partMatches(part, hovered) : false;
              return part.op === 'delete' ? (
                <del key={i} className={styles.del} data-lit={lit || undefined}>
                  {part.text}
                </del>
              ) : (
                <ins key={i} className={styles.ins} data-lit={lit || undefined}>
                  {part.text}
                </ins>
              );
            })}
      </div>
    </motion.section>
  );
}

function MistakeCard({ m, i, onHover }: { m: Mistake; i: number; onHover: (m: Mistake | null) => void }) {
  const category = useApp((s) => s.categories[m.category]);
  return (
    <motion.li
      className={styles.card}
      variants={item}
      custom={i}
      onMouseEnter={() => onHover(m)}
      onMouseLeave={() => onHover(null)}
      onFocus={() => onHover(m)}
      onBlur={() => onHover(null)}
      tabIndex={0}
    >
      <div className={styles.change} data-selectable>
        <span className={styles.from}>{m.original}</span>
        <span className={styles.arrow} aria-hidden>
          →
        </span>
        <span className={styles.to}>{m.correction || '(remove)'}</span>
      </div>
      <p className={styles.explanation} data-selectable>
        {m.explanation}
      </p>
      <div className={styles.meta}>
        <span className={ui.chip}>{category?.label ?? m.category}</span>
        {m.impedesMeaning && <span className={`${ui.chip} ${styles.meaning}`}>Affects meaning</span>}
      </div>
    </motion.li>
  );
}

function SuggestionCard({ s, i }: { s: Suggestion; i: number }) {
  const label = s.kind === 'synonym' ? 'For variety' : 'More natural';
  return (
    <motion.li className={`${styles.card} ${styles.idea}`} variants={item} custom={i}>
      <div className={styles.change} data-selectable>
        <span className={styles.ideaFrom}>{s.original}</span>
        <span className={styles.arrow} aria-hidden>
          →
        </span>
        <span className={styles.ideaTo}>{s.replacement}</span>
      </div>
      <p className={styles.explanation} data-selectable>
        {s.explanation}
      </p>
      <div className={styles.meta}>
        <span className={`${ui.chip} ${styles.ideaChip}`}>{label}</span>
      </div>
    </motion.li>
  );
}

export function Results({ entry }: { entry: Entry }) {
  const [hovered, setHovered] = useState<Mistake | null>(null);
  const language = useApp((s) => s.settings.languages.find((l) => l.code === entry.language)?.name ?? entry.language);
  const when = useMemo(
    () => new Date(entry.createdAt).toLocaleString(undefined, { dateStyle: 'medium', timeStyle: 'short' }),
    [entry.createdAt],
  );

  return (
    <motion.div className={styles.results} initial="hidden" animate="shown">
      <FixedBlock entry={entry} hovered={hovered} context={`${language} · ${entry.tone.name} · ${when}`} />

      <div className={styles.columns}>
        <motion.section className={styles.block} variants={block} custom={1} aria-label="Mistakes">
          <header className={styles.blockHeader}>
            <div className={styles.blockTitle}>
              <h2>Mistakes</h2>
              <span className={styles.count}>{entry.mistakes.length}</span>
            </div>
          </header>
          {entry.mistakes.length ? (
            <ul className={styles.list}>
              {entry.mistakes.map((m, i) => (
                <MistakeCard key={m.id} m={m} i={i} onHover={setHovered} />
              ))}
            </ul>
          ) : (
            <p className={styles.empty}>No mistakes. If it's understandable and correct, it stays as you wrote it.</p>
          )}
        </motion.section>

        <motion.section className={styles.block} variants={block} custom={2} aria-label="Suggestions">
          <header className={styles.blockHeader}>
            <div className={styles.blockTitle}>
              <h2>
                <SparkleIcon size={16} className={styles.sparkle} /> Suggestions
              </h2>
              <span className={styles.count}>{entry.suggestions.length}</span>
            </div>
          </header>
          {entry.suggestions.length ? (
            <ul className={styles.list}>
              {entry.suggestions.map((s, i) => (
                <SuggestionCard key={s.id} s={s} i={i} />
              ))}
            </ul>
          ) : (
            <p className={styles.empty}>Nothing to add this time. It already sounds natural.</p>
          )}
        </motion.section>
      </div>
    </motion.div>
  );
}

export function ResultsSkeleton() {
  return (
    <motion.div
      className={styles.skeleton}
      initial={{ opacity: 0, y: 10 }}
      animate={{ opacity: 1, y: 0 }}
      exit={{ opacity: 0, transition: { duration: 0.1 } }}
      aria-hidden
    >
      <div className={styles.shimmer} style={{ height: 120 }} />
      <div className={styles.columns}>
        <div className={styles.shimmer} style={{ height: 160 }} />
        <div className={styles.shimmer} style={{ height: 160 }} />
      </div>
    </motion.div>
  );
}
