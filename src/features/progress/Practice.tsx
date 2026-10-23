import { useMutation, useQueryClient } from '@tanstack/react-query';
import { motion } from 'motion/react';
import { useMemo, useState } from 'react';
import { Kbd } from '../../components/controls';
import ui from '../../components/ui.module.css';
import type { PatternStats } from '../../bindings/PatternStats';
import { api } from '../../lib/api';
import { hasMod } from '../../lib/platform';
import styles from './ProgressPage.module.css';

const norm = (s: string) => s.toLowerCase().replace(/[\s.,!?;:]+/g, ' ').trim();

/**
 * "Fix your own sentence": retrieval beats rereading (§3.5). The learner corrects one of their
 * own past sentences, sees the answer, and grades themselves; the reminder ladder moves.
 */
export function Practice({ pattern, onDone }: { pattern: PatternStats; onDone: () => void }) {
  const example = pattern.examples[0]!;
  const expected = useMemo(
    () => example.sentence.replace(example.original, example.correction),
    [example],
  );
  const [answer, setAnswer] = useState(example.sentence);
  const [revealed, setRevealed] = useState(false);
  const qc = useQueryClient();
  const record = useMutation({
    mutationFn: (passed: boolean) => api().recordReview(pattern.key, passed),
    onSuccess: (stats) => {
      qc.setQueryData(['stats'], stats);
      onDone();
    },
  });
  const matched = norm(answer) === norm(expected);

  return (
    <motion.div
      className={styles.practice}
      initial={{ opacity: 0, scale: 0.98 }}
      animate={{ opacity: 1, scale: 1 }}
      exit={{ opacity: 0, scale: 0.98, transition: { duration: 0.12 } }}
    >
      <span className={styles.practiceKicker}>Fix your own sentence · {pattern.label}</span>
      <textarea
        className={`${ui.input} ${styles.practiceInput}`}
        value={answer}
        onChange={(e) => setAnswer(e.target.value)}
        disabled={revealed}
        autoFocus
        aria-label="Your corrected sentence"
        onKeyDown={(e) => {
          if (e.key === 'Enter' && hasMod(e)) {
            e.preventDefault();
            setRevealed(true);
          }
        }}
      />
      {revealed ? (
        <motion.div initial={{ opacity: 0, y: 6 }} animate={{ opacity: 1, y: 0 }} className={styles.reveal}>
          <span className={matched ? styles.revealGood : styles.revealLabel}>
            {matched ? 'Exactly right.' : 'The fix:'}
          </span>
          <p className={styles.expected} data-selectable>
            {expected}
          </p>
          <p className={styles.revealWhy}>{example.explanation}</p>
          <div className={styles.patternButtons}>
            <button type="button" className={`${ui.primary} ${ui.small}`} onClick={() => record.mutate(true)}>
              I had it
            </button>
            <button type="button" className={`${ui.secondary} ${ui.small}`} onClick={() => record.mutate(false)}>
              Not yet
            </button>
          </div>
        </motion.div>
      ) : (
        <div className={styles.patternButtons}>
          <button type="button" className={`${ui.primary} ${ui.small}`} onClick={() => setRevealed(true)}>
            Show the fix <Kbd keys="mod+enter" />
          </button>
          <button type="button" className={`${ui.ghost} ${ui.small}`} onClick={onDone}>
            Cancel
          </button>
        </div>
      )}
    </motion.div>
  );
}
