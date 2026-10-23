import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { AnimatePresence, motion } from 'motion/react';
import { useState } from 'react';
import { Segmented, Spinner } from '../../components/controls';
import { ArrowDownRightIcon, ArrowUpRightIcon, CheckIcon, PinIcon, RefreshIcon, SparkleIcon } from '../../components/icons';
import { toastError } from '../../components/Toaster';
import ui from '../../components/ui.module.css';
import type { AreaStats } from '../../bindings/AreaStats';
import type { LanguageStats } from '../../bindings/LanguageStats';
import type { PatternState } from '../../bindings/PatternState';
import type { PatternStats } from '../../bindings/PatternStats';
import type { RateTrend } from '../../bindings/RateTrend';
import type { Stats } from '../../bindings/Stats';
import { api, errorMessage } from '../../lib/api';
import { useApp } from '../../stores/app';
import { Practice } from './Practice';
import styles from './ProgressPage.module.css';

const AREA_LABELS: Record<string, string> = {
  grammar: 'Grammar',
  lexis: 'Vocabulary',
  orthography: 'Spelling & punctuation',
  register: 'Tone & register',
  discourse: 'Linking ideas',
  other: 'Other',
};

const STATE_LABELS: Record<PatternState, string> = {
  new: 'New',
  recurring: 'Recurring',
  improving: 'Improving',
  mastered: 'Mastered',
  relapsed: 'Came back',
  dormant: 'Quiet lately',
  archived: 'Archived',
};

const fmt = (n: number | null | undefined) => (n == null ? '–' : n.toLocaleString(undefined, { maximumFractionDigits: 2 }));
const pct = (n: number) => `${Math.round(n * 100)}%`;

/** For error rates, going down is the good direction. */
function TrendChip({ rate, minWords }: { rate: RateTrend; minWords: number }) {
  if (rate.trend === 'insufficientData') {
    const have = rate.recentWords + rate.priorWords;
    const need = Math.max(0, minWords * 2 - have);
    return (
      <span className={styles.trendMuted} title="Trends need enough writing to be more than noise">
        {need > 0 ? `~${need.toLocaleString()} more words for a trend` : 'Not enough data yet'}
      </span>
    );
  }
  if (rate.trend === 'stable') {
    return (
      <span className={styles.trendMuted} title={`No clear change (p = ${fmt(rate.pValue)})`}>
        Steady
      </span>
    );
  }
  const good = rate.trend === 'improving';
  return (
    <span className={good ? styles.trendGood : styles.trendBad} title={`p = ${fmt(rate.pValue)}`}>
      {good ? <ArrowDownRightIcon size={14} /> : <ArrowUpRightIcon size={14} />}
      {good ? 'Improving' : 'Rising'}
    </span>
  );
}

function Tile({ label, value, sub, children }: { label: string; value: string; sub?: string; children?: React.ReactNode }) {
  return (
    <div className={styles.tile}>
      <span className={styles.tileLabel}>{label}</span>
      <span className={styles.tileValue}>{value}</span>
      <span className={styles.tileSub}>
        {children}
        {sub && <span>{sub}</span>}
      </span>
    </div>
  );
}

function InsightCard({ language }: { language: string }) {
  const qc = useQueryClient();
  const { data: insights } = useQuery({ queryKey: ['insights'], queryFn: () => api().getInsights() });
  const refresh = useMutation({
    mutationFn: () => api().refreshInsights(),
    onSuccess: (i) => {
      qc.setQueryData(['insights'], i);
      void qc.invalidateQueries({ queryKey: ['stats'] });
    },
    onError: (e) => toastError(errorMessage(e)),
  });
  const mine = insights?.languages.find((l) => l.language === language);
  return (
    <motion.section className={styles.insight} layout>
      <div className={styles.insightHead}>
        <SparkleIcon size={16} className={styles.sparkle} />
        <span>Summary</span>
        <button
          type="button"
          className={`${ui.ghost} ${ui.small} ${styles.insightRefresh}`}
          onClick={() => refresh.mutate()}
          disabled={refresh.isPending}
        >
          {refresh.isPending ? <Spinner /> : <RefreshIcon size={14} />}
          {mine ? 'Rewrite' : 'Write it now'}
        </button>
      </div>
      {mine ? (
        <>
          <p className={styles.insightText}>{mine.summary}</p>
          {mine.tip && <p className={styles.tip}>{mine.tip}</p>}
          <span className={styles.insightFoot}>
            Written from your stats ·{' '}
            {new Date(insights!.generatedAt).toLocaleDateString(undefined, { day: 'numeric', month: 'short' })}
          </span>
        </>
      ) : (
        <p className={styles.insightEmpty}>
          A short note on what's improving and what to focus on. It's written in the background once you've checked a
          few texts.
        </p>
      )}
    </motion.section>
  );
}

function PatternCard({ p, onPractice }: { p: PatternStats; onPractice: () => void }) {
  const qc = useQueryClient();
  const category = useApp((s) => s.categories[p.category]);
  const pref = useMutation({
    mutationFn: (pinned: boolean | null) => api().setPatternPref(p.key, pinned),
    onSuccess: (stats) => qc.setQueryData(['stats'], stats),
  });
  const example = p.examples[0];
  return (
    <motion.li layout className={styles.pattern} initial={{ opacity: 0, y: 8 }} animate={{ opacity: 1, y: 0 }}>
      <div className={styles.patternTop}>
        <span className={styles.state} data-state={p.state}>
          {STATE_LABELS[p.state]}
        </span>
        {p.reminder?.due && <span className={styles.due}>Review due</span>}
        <span className={styles.patternActions}>
          <button
            type="button"
            className={ui.iconButton}
            aria-label={p.pinned ? 'Unpin' : 'Pin to watch list'}
            title={p.pinned ? 'Pinned (click to unpin)' : 'Pin'}
            data-on={p.pinned || undefined}
            onClick={() => pref.mutate(p.pinned ? null : true)}
          >
            <PinIcon size={16} />
          </button>
        </span>
      </div>
      <h3 className={styles.patternLabel}>{p.label}</h3>
      <span className={styles.patternCat}>
        {category?.label ?? p.category}
        {category?.kind === 'item' ? ' · a phrase to remember' : category?.kind === 'rule' ? ' · a rule' : ''}
      </span>
      {example && (
        <p className={styles.example} data-selectable>
          “{example.sentence}”
        </p>
      )}
      <div className={styles.patternStats}>
        <span>
          {p.daysWithError30d} {p.daysWithError30d === 1 ? 'day' : 'days'} with this in the last 30
        </span>
        {p.recentAccuracy != null && (
          <span className={styles.accuracy}>
            <span className={styles.bar}>
              <motion.span
                className={styles.barFill}
                initial={{ width: 0 }}
                animate={{ width: pct(p.recentAccuracy) }}
                transition={{ type: 'spring', stiffness: 120, damping: 20 }}
              />
            </span>
            {pct(p.recentAccuracy)} right lately
          </span>
        )}
      </div>
      <div className={styles.patternButtons}>
        {example && (
          <button type="button" className={`${ui.secondary} ${ui.small}`} onClick={onPractice}>
            Practice
          </button>
        )}
        <button type="button" className={`${ui.ghost} ${ui.small}`} onClick={() => pref.mutate(false)}>
          Not now
        </button>
      </div>
    </motion.li>
  );
}

/** A pattern outside the watch list: pin it, or undo a "Not now". */
function OtherPattern({ p }: { p: PatternStats }) {
  const qc = useQueryClient();
  const pref = useMutation({
    mutationFn: (pinned: boolean | null) => api().setPatternPref(p.key, pinned),
    onSuccess: (stats) => qc.setQueryData(['stats'], stats),
  });
  return (
    <li>
      <span>{p.label}</span>
      <span className={styles.otherSide}>
        <span className={styles.state} data-state={p.state}>
          {STATE_LABELS[p.state]}
        </span>
        {p.dismissed ? (
          <button type="button" className={`${ui.ghost} ${ui.small}`} onClick={() => pref.mutate(null)}>
            Watch again
          </button>
        ) : (
          <button type="button" className={ui.iconButton} aria-label={`Pin ${p.label}`} title="Pin to watch list" onClick={() => pref.mutate(true)}>
            <PinIcon size={14} />
          </button>
        )}
      </span>
    </li>
  );
}

function AreaRow({ a, maxShare }: { a: AreaStats; maxShare: number }) {
  const categories = useApp((s) => s.categories);
  const [open, setOpen] = useState(false);
  return (
    <li className={styles.area}>
      <button type="button" className={styles.areaMain} onClick={() => setOpen((o) => !o)} aria-expanded={open}>
        <span className={styles.areaName}>{AREA_LABELS[a.area] ?? a.area}</span>
        <span className={styles.areaBar}>
          <motion.span
            className={styles.areaFill}
            initial={{ width: 0 }}
            animate={{ width: `${(a.share / maxShare) * 100}%` }}
            transition={{ type: 'spring', stiffness: 120, damping: 22 }}
          />
        </span>
        <span className={styles.areaShare}>{pct(a.share)}</span>
      </button>
      <AnimatePresence initial={false}>
        {open && (
          <motion.ul
            className={styles.cats}
            initial={{ height: 0, opacity: 0 }}
            animate={{ height: 'auto', opacity: 1 }}
            exit={{ height: 0, opacity: 0 }}
          >
            {a.categories.map((c) => (
              <li key={c.category} className={styles.cat}>
                <span>{categories[c.category]?.label ?? c.category}</span>
                <span className={styles.catShare}>{pct(c.share)}</span>
                <TrendChip rate={c.rate} minWords={500} />
              </li>
            ))}
          </motion.ul>
        )}
      </AnimatePresence>
    </li>
  );
}

function LanguageView({ l, stats }: { l: LanguageStats; stats: Stats }) {
  const [practicing, setPracticing] = useState<PatternStats | null>(null);
  const t = stats.thresholds;
  const watched = l.patterns.filter((p) => p.onWatchList);
  const others = l.patterns.filter((p) => !p.onWatchList);
  const maxShare = Math.max(...l.areas.map((a) => a.share), 0.01);

  return (
    <div className={styles.language}>
      <InsightCard language={l.language} />

      <div className={styles.tiles}>
        <Tile
          label="Mistakes per 100 words"
          value={fmt(l.errorRate.recent)}
          sub={l.errorRate.prior != null ? `was ${fmt(l.errorRate.prior)}` : undefined}
        >
          <TrendChip rate={l.errorRate} minWords={t.minWindowWords} />
        </Tile>
        <Tile
          label="Hard-to-understand spots"
          value={fmt(l.globalErrorRate.recent)}
          sub="per 100 words"
        >
          <TrendChip rate={l.globalErrorRate} minWords={t.minWindowWords} />
        </Tile>
        <Tile label="Weeks with writing" value={`${l.totals.activeWeeks12} of 12`} sub={`${l.totals.words.toLocaleString()} words in ${l.totals.texts} texts`} />
        <Tile
          label="Suggestions you now use"
          value={l.suggestions.offered ? `${l.suggestions.reused} of ${l.suggestions.offered}` : '–'}
          sub={`last ${t.uptakeDays} days`}
        />
      </div>

      <section className={styles.section}>
        <h2 className={styles.sectionTitle}>Watching</h2>
        <p className={styles.sectionHint}>
          Mistakes that keep coming back, at most {t.watchListSize} at a time. They clear once you get them right
          consistently.
        </p>
        <AnimatePresence mode="wait">
          {practicing ? (
            <Practice key="practice" pattern={practicing} onDone={() => setPracticing(null)} />
          ) : watched.length ? (
            <motion.ul key="list" className={styles.patterns} initial={{ opacity: 0 }} animate={{ opacity: 1 }}>
              {watched.map((p) => (
                <PatternCard key={p.key} p={p} onPractice={() => setPracticing(p)} />
              ))}
            </motion.ul>
          ) : (
            <p className={styles.empty}>
              Nothing yet. A mistake lands here once it shows up in separate texts on different days. One-off slips
              don't count.
            </p>
          )}
        </AnimatePresence>
      </section>

      {l.areas.length > 0 && (
        <section className={styles.section}>
          <h2 className={styles.sectionTitle}>Where your mistakes are</h2>
          <ul className={styles.areas}>
            {l.areas.map((a) => (
              <AreaRow key={a.area} a={a} maxShare={maxShare} />
            ))}
          </ul>
        </section>
      )}

      {(l.masteredRecently.length > 0 || l.suggestions.recentUptakes.length > 0) && (
        <section className={styles.wins}>
          {l.masteredRecently.length > 0 && (
            <div>
              <h2 className={styles.sectionTitle}>Mastered lately</h2>
              <ul className={styles.winList}>
                {l.masteredRecently.map((m) => (
                  <li key={m.key}>
                    <CheckIcon size={14} className={styles.winIcon} /> {m.label}
                  </li>
                ))}
              </ul>
            </div>
          )}
          {l.suggestions.recentUptakes.length > 0 && (
            <div>
              <h2 className={styles.sectionTitle}>Phrases you picked up</h2>
              <ul className={styles.winList}>
                {l.suggestions.recentUptakes.map((u) => (
                  <li key={u.phrase}>
                    <SparkleIcon size={14} className={styles.sparkle} /> {u.phrase}
                  </li>
                ))}
              </ul>
            </div>
          )}
        </section>
      )}

      {others.length > 0 && (
        <details className={styles.more}>
          <summary>All patterns ({others.length})</summary>
          <ul className={styles.allPatterns}>
            {others.map((p) => (
              <OtherPattern key={p.key} p={p} />
            ))}
          </ul>
        </details>
      )}

      <details className={styles.more}>
        <summary>How these numbers work</summary>
        <ul className={styles.method}>
          <li>Rates are mistakes per 100 words, over at least {t.minWindowWords} words. Texts under {t.minTextWords} words don't count towards rates.</li>
          <li>
            "Improving" or "Rising" appears only when the change between your last {t.windowWords.toLocaleString()} words and
            the {t.windowWords.toLocaleString()} before is bigger than chance (p &lt; {t.significance}). Otherwise it says
            "Steady".
          </li>
          <li>
            A mistake is watched once it appears in {t.recurringTexts}+ texts on {t.recurringDays}+ different days within{' '}
            {t.recurringWindowDays} days. It's mastered after {t.masteredTexts} texts in a row at {pct(t.masteredAccuracy)}+
            correct and {t.masteredGapDays} days without the mistake.
          </li>
          <li>Suggestions never count as mistakes. Reusing a suggested phrase on your own counts as picking it up.</li>
          <li>Reviews come back after {t.ladderDays.join(', ')} days. A repeat of the mistake starts the schedule again.</li>
        </ul>
      </details>
    </div>
  );
}

export function ProgressPage() {
  const { data: stats, isLoading } = useQuery({ queryKey: ['stats'], queryFn: () => api().getStats() });
  const languages = stats?.languages.filter((l) => l.totals.texts > 0) ?? [];
  const [lang, setLang] = useState<string | null>(null);
  const current = languages.find((l) => l.language === lang) ?? languages[0];

  return (
    <div className={styles.page}>
      <header className={styles.header}>
        <h1 className={styles.title}>Progress</h1>
        {languages.length > 1 && current && (
          <Segmented
            label="Language"
            value={current.language}
            onChange={setLang}
            options={languages.map((l) => ({ value: l.language, label: l.languageName }))}
          />
        )}
      </header>
      {isLoading ? null : current && stats ? (
        <AnimatePresence mode="wait">
          <motion.div
            key={current.language}
            initial={{ opacity: 0, y: 6 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, transition: { duration: 0.1 } }}
          >
            <LanguageView l={current} stats={stats} />
          </motion.div>
        </AnimatePresence>
      ) : (
        <p className={styles.empty}>Check a few texts and your progress will build up here.</p>
      )}
    </div>
  );
}
