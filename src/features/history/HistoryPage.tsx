import { useInfiniteQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { AnimatePresence, motion } from 'motion/react';
import { useEffect, useMemo, useState } from 'react';
import { Spinner } from '../../components/controls';
import { DownloadIcon, SearchIcon, TrashIcon } from '../../components/icons';
import { toast, toastError } from '../../components/Toaster';
import ui from '../../components/ui.module.css';
import type { Entry } from '../../bindings/Entry';
import { api, errorMessage } from '../../lib/api';
import { useApp } from '../../stores/app';
import styles from './HistoryPage.module.css';

function dayLabel(d: Date) {
  const today = new Date();
  const startOf = (x: Date) => new Date(x.getFullYear(), x.getMonth(), x.getDate()).getTime();
  const diff = Math.round((startOf(today) - startOf(d)) / 86_400_000);
  if (diff === 0) return 'Today';
  if (diff === 1) return 'Yesterday';
  return d.toLocaleDateString(undefined, {
    weekday: diff < 7 ? 'long' : undefined,
    day: 'numeric',
    month: 'long',
    year: d.getFullYear() === today.getFullYear() ? undefined : 'numeric',
  });
}

export function useExport() {
  return useMutation({
    mutationFn: () => api().exportHistory(),
    onSuccess: (path) => {
      const name = path.split(/[\\/]/).pop();
      toast(`Saved ${name} to Downloads`, { label: 'Show', run: () => void api().revealPath(path) });
    },
    onError: (e) => toastError(errorMessage(e)),
  });
}

function Row({ entry }: { entry: Entry }) {
  const openEntry = useApp((s) => s.openEntry);
  const qc = useQueryClient();
  const remove = useMutation({
    mutationFn: () => api().deleteEntry(entry.id),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ['history'] });
      void qc.invalidateQueries({ queryKey: ['stats'] });
    },
  });
  const time = new Date(entry.createdAt).toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit' });
  const n = entry.mistakes.length;
  return (
    <motion.li
      layout
      initial={{ opacity: 0, y: 6 }}
      animate={{ opacity: 1, y: 0 }}
      exit={{ opacity: 0, height: 0, transition: { duration: 0.15 } }}
      className={styles.row}
    >
      <button type="button" className={styles.rowMain} onClick={() => openEntry(entry)}>
        <span className={styles.snippet}>{entry.input}</span>
        <span className={styles.rowMeta}>
          <span className={ui.chip}>{entry.language.toUpperCase()}</span>
          <span className={ui.chip}>{entry.tone.name}</span>
          <span className={n ? styles.fixes : styles.clean}>{n ? `${n} ${n === 1 ? 'fix' : 'fixes'}` : 'No fixes'}</span>
          {entry.suggestions.length > 0 && (
            <span className={styles.ideas}>
              {entry.suggestions.length} {entry.suggestions.length === 1 ? 'idea' : 'ideas'}
            </span>
          )}
          <span className={styles.time}>{time}</span>
        </span>
      </button>
      <button
        type="button"
        className={`${ui.iconButton} ${styles.delete}`}
        aria-label="Delete from history"
        title="Delete"
        onClick={() => remove.mutate()}
      >
        <TrashIcon size={16} />
      </button>
    </motion.li>
  );
}

export function HistoryPage() {
  const [query, setQuery] = useState('');
  const [q, setQ] = useState('');
  // Search scans the whole history; wait until typing pauses.
  useEffect(() => {
    const t = setTimeout(() => setQ(query.trim()), 250);
    return () => clearTimeout(t);
  }, [query]);
  const exporter = useExport();
  const history = useInfiniteQuery({
    queryKey: ['history', q],
    queryFn: ({ pageParam }) => api().history({ before: pageParam, limit: 40, query: q || undefined }),
    initialPageParam: null as string | null,
    getNextPageParam: (last) => last.nextBefore,
  });

  const groups = useMemo(() => {
    const out: { label: string; entries: Entry[] }[] = [];
    for (const page of history.data?.pages ?? []) {
      for (const e of page.entries) {
        const label = dayLabel(new Date(e.createdAt));
        const last = out[out.length - 1];
        if (last?.label === label) last.entries.push(e);
        else out.push({ label, entries: [e] });
      }
    }
    return out;
  }, [history.data]);

  const empty = !history.isLoading && groups.length === 0;

  return (
    <div className={styles.page}>
      <header className={styles.header}>
        <h1 className={styles.title}>History</h1>
        <button
          type="button"
          className={ui.secondary}
          onClick={() => exporter.mutate()}
          disabled={exporter.isPending}
          title="Everything, as one JSON file in Downloads"
        >
          {exporter.isPending ? <Spinner /> : <DownloadIcon size={16} />} Export all
        </button>
      </header>

      <label className={styles.search}>
        <SearchIcon size={16} />
        <input
          className={styles.searchInput}
          placeholder="Search your texts"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          aria-label="Search history"
        />
      </label>

      {empty && (
        <p className={styles.empty}>
          {q ? 'Nothing matches that.' : 'Texts you check show up here, so you can come back to them.'}
        </p>
      )}

      <AnimatePresence initial={false}>
        {groups.map((g) => (
          <motion.section key={g.label} layout className={styles.group}>
            <h2 className={styles.groupLabel}>{g.label}</h2>
            <ul className={styles.list}>
              <AnimatePresence initial={false}>
                {g.entries.map((e) => (
                  <Row key={e.id} entry={e} />
                ))}
              </AnimatePresence>
            </ul>
          </motion.section>
        ))}
      </AnimatePresence>

      {history.hasNextPage && (
        <button
          type="button"
          className={`${ui.ghost} ${styles.more}`}
          onClick={() => void history.fetchNextPage()}
          disabled={history.isFetchingNextPage}
        >
          {history.isFetchingNextPage ? <Spinner /> : 'Show older'}
        </button>
      )}
    </div>
  );
}
