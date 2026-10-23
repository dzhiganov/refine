import { useMutation, useQuery } from '@tanstack/react-query';
import { AnimatePresence, motion } from 'motion/react';
import { useEffect, useMemo, useRef, useState, type ReactNode } from 'react';
import { Segmented, Spinner, Switch } from '../../components/controls';
import { CheckIcon, ChevronDownIcon, DownloadIcon, PlusIcon, SearchIcon, TrashIcon } from '../../components/icons';
import { toast, toastError } from '../../components/Toaster';
import ui from '../../components/ui.module.css';
import type { Cefr } from '../../bindings/Cefr';
import type { Formality } from '../../bindings/Formality';
import type { ModelInfo } from '../../bindings/ModelInfo';
import type { ProviderConfig } from '../../bindings/ProviderConfig';
import type { Settings } from '../../bindings/Settings';
import type { ShortForms } from '../../bindings/ShortForms';
import type { Theme } from '../../bindings/Theme';
import type { Tone } from '../../bindings/Tone';
import { api, errorMessage } from '../../lib/api';
import { platform } from '../../lib/platform';
import { useApp } from '../../stores/app';
import { useExport } from '../history/HistoryPage';
import styles from './SettingsPage.module.css';

const LEVELS: Cefr[] = ['A1', 'A2', 'B1', 'B2', 'C1', 'C2'];

function Section({ title, hint, children }: { title: string; hint?: string; children: ReactNode }) {
  return (
    <section className={styles.section}>
      <div className={styles.sectionHead}>
        <h2>{title}</h2>
        {hint && <p>{hint}</p>}
      </div>
      <div className={styles.sectionBody}>{children}</div>
    </section>
  );
}

function Row({ label, hint, children }: { label: string; hint?: string; children: ReactNode }) {
  return (
    <div className={styles.row}>
      <div className={styles.rowLabel}>
        <span>{label}</span>
        {hint && <small>{hint}</small>}
      </div>
      <div className={styles.rowControl}>{children}</div>
    </div>
  );
}

function keyStoreName() {
  const p = platform();
  if (p === 'macos') return 'macOS Keychain';
  if (p === 'windows') return 'Windows Credential Manager';
  return 'a local file (dev build)';
}

function price(m: ModelInfo) {
  if (m.promptPrice == null) return null;
  if (m.promptPrice === 0 && m.completionPrice === 0) return 'Free';
  return `$${m.promptPrice} / $${m.completionPrice} per 1M`;
}

function ModelPicker({ provider, onPick }: { provider: ProviderConfig; onPick: (id: string) => void }) {
  const [open, setOpen] = useState(false);
  const [q, setQ] = useState('');
  const models = useQuery({
    queryKey: ['models', provider.id, provider.baseUrl],
    queryFn: () => api().listModels(provider),
    enabled: open,
    staleTime: 10 * 60_000,
  });
  const list = useMemo(() => {
    const term = q.trim().toLowerCase();
    const all = models.data ?? [];
    return (term ? all.filter((m) => m.id.toLowerCase().includes(term) || m.name.toLowerCase().includes(term)) : all).slice(0, 200);
  }, [models.data, q]);

  return (
    <div className={styles.picker}>
      <button type="button" className={styles.pickerButton} onClick={() => setOpen((o) => !o)} aria-expanded={open}>
        <span className={provider.model ? styles.modelId : styles.placeholder}>{provider.model || 'Choose a model'}</span>
        <ChevronDownIcon size={16} />
      </button>
      <AnimatePresence>
        {open && (
          <motion.div
            className={styles.pickerPanel}
            initial={{ opacity: 0, height: 0 }}
            animate={{ opacity: 1, height: 'auto' }}
            exit={{ opacity: 0, height: 0 }}
          >
            <label className={styles.pickerSearch}>
              <SearchIcon size={14} />
              <input autoFocus placeholder="Search models" value={q} onChange={(e) => setQ(e.target.value)} />
            </label>
            <div className={styles.pickerList} role="listbox" aria-label="Models">
              {models.isLoading && (
                <div className={styles.pickerNote}>
                  <Spinner /> Loading models…
                </div>
              )}
              {models.error && <div className={styles.pickerNote}>{errorMessage(models.error)}</div>}
              {!models.isLoading && !models.error && list.length === 0 && (
                <div className={styles.pickerNote}>No models found{q ? ' for that search' : ''}.</div>
              )}
              {list.map((m) => (
                <button
                  key={m.id}
                  type="button"
                  role="option"
                  aria-selected={m.id === provider.model}
                  className={styles.modelRow}
                  onClick={() => {
                    onPick(m.id);
                    setOpen(false);
                  }}
                >
                  <span className={styles.modelMain}>
                    <span className={styles.modelName}>{m.name}</span>
                    <span className={styles.modelMeta}>
                      {m.id}
                      {m.contextLength ? ` · ${Math.round(m.contextLength / 1000)}k context` : ''}
                    </span>
                  </span>
                  <span className={styles.modelSide}>
                    {m.structuredOutputs && <span className={styles.badge} title="Supports strict JSON output">JSON</span>}
                    {price(m) && <span className={styles.modelPrice}>{price(m)}</span>}
                    {m.id === provider.model && <CheckIcon size={16} />}
                  </span>
                </button>
              ))}
            </div>
            {provider.kind === 'openRouter' && (
              <p className={styles.pickerHint}>
                Models marked JSON enforce the answer format; others work too, with a fallback.
              </p>
            )}
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}

function ProviderSection() {
  const settings = useApp((s) => s.settings);
  const providers = useApp((s) => s.providers);
  const setProviders = useApp((s) => s.setProviders);
  const saveSettings = useApp((s) => s.saveSettings);
  const active = settings.providers.find((p) => p.id === settings.activeProviderId) ?? settings.providers[0]!;
  const hasKey = providers.find((p) => p.id === active.id)?.hasKey ?? false;
  const [key, setKey] = useState('');
  const [baseUrl, setBaseUrl] = useState(active.baseUrl);
  useEffect(() => setBaseUrl(active.baseUrl), [active.id, active.baseUrl]);

  const update = (patch: Partial<ProviderConfig>) =>
    saveSettings({
      ...settings,
      providers: settings.providers.map((p) => (p.id === active.id ? { ...p, ...patch } : p)),
    }).catch((e) => toastError(errorMessage(e)));

  const saveKey = useMutation({
    mutationFn: (value: string) => api().setApiKey(active.id, value),
    onSuccess: (statuses, value) => {
      setProviders(statuses);
      setKey('');
      toast(value ? `Key saved to ${keyStoreName()}` : 'Key removed');
    },
    onError: (e) => toastError(errorMessage(e)),
  });

  return (
    <Section title="Model" hint="Use your own key, or a model running on this computer.">
      <Row label="Provider">
        <Segmented
          label="Provider"
          value={active.id}
          onChange={(id) => void saveSettings({ ...settings, activeProviderId: id })}
          options={settings.providers.map((p) => ({ value: p.id, label: p.label }))}
        />
      </Row>
      <Row label="Address" hint={active.needsKey ? undefined : 'The local server must be running.'}>
        <input
          className={ui.input}
          value={baseUrl}
          onChange={(e) => setBaseUrl(e.target.value)}
          onBlur={() => baseUrl !== active.baseUrl && void update({ baseUrl: baseUrl.trim() })}
          spellCheck={false}
        />
      </Row>
      {active.needsKey && (
        <Row label="API key" hint={hasKey ? `Stored in ${keyStoreName()}` : 'Stored in your system keychain, never in files.'}>
          <form
            className={styles.inline}
            onSubmit={(e) => {
              e.preventDefault();
              if (key.trim()) saveKey.mutate(key.trim());
            }}
          >
            <input
              className={ui.input}
              type="password"
              value={key}
              placeholder={hasKey ? '••••••••••••••••' : 'Paste your key'}
              onChange={(e) => setKey(e.target.value)}
              autoComplete="off"
            />
            <button type="submit" className={ui.secondary} disabled={!key.trim() || saveKey.isPending}>
              Save
            </button>
            {hasKey && (
              <button type="button" className={ui.ghost} onClick={() => saveKey.mutate('')}>
                Remove
              </button>
            )}
          </form>
        </Row>
      )}
      <Row label="Model">
        <ModelPicker provider={active} onPick={(model) => void update({ model })} />
      </Row>
    </Section>
  );
}

function LanguagesSection() {
  const settings = useApp((s) => s.settings);
  const saveSettings = useApp((s) => s.saveSettings);
  const [code, setCode] = useState('');
  const [name, setName] = useState('');
  const save = (next: Settings) => saveSettings(next).catch((e) => toastError(errorMessage(e)));

  const add = () => {
    const c = code.trim().toLowerCase();
    if (!/^[a-z]{2,3}$/.test(c) || !name.trim() || settings.languages.some((l) => l.code === c)) return;
    void save({ ...settings, languages: [...settings.languages, { code: c, name: name.trim(), level: 'B1' }] });
    setCode('');
    setName('');
  };

  return (
    <Section title="Languages" hint="Your level shapes how explanations are written and how ambitious suggestions get.">
      {settings.languages.map((l) => (
        <Row key={l.code} label={l.name} hint={l.code}>
          <div className={styles.inline}>
            <Segmented
              label={`${l.name} level`}
              value={l.level}
              onChange={(level) =>
                void save({ ...settings, languages: settings.languages.map((x) => (x.code === l.code ? { ...x, level } : x)) })
              }
              options={LEVELS.map((v) => ({ value: v, label: v }))}
            />
            {settings.languages.length > 1 && (
              <button
                type="button"
                className={ui.iconButton}
                aria-label={`Remove ${l.name}`}
                onClick={() =>
                  void save({
                    ...settings,
                    languages: settings.languages.filter((x) => x.code !== l.code),
                    languageMode: settings.languageMode === l.code ? 'auto' : settings.languageMode,
                  })
                }
              >
                <TrashIcon size={16} />
              </button>
            )}
          </div>
        </Row>
      ))}
      <Row label="Add a language" hint="ISO code and English name, e.g. fr · French">
        <div className={styles.inline}>
          <input className={`${ui.input} ${styles.code}`} placeholder="fr" value={code} onChange={(e) => setCode(e.target.value)} />
          <input className={ui.input} placeholder="French" value={name} onChange={(e) => setName(e.target.value)} onKeyDown={(e) => e.key === 'Enter' && add()} />
          <button type="button" className={ui.secondary} onClick={add}>
            <PlusIcon size={16} /> Add
          </button>
        </div>
      </Row>
      <Row label="Explanations in">
        <select
          className={`${ui.input} ${styles.select}`}
          value={settings.explanationLanguage}
          onChange={(e) => void save({ ...settings, explanationLanguage: e.target.value })}
        >
          {settings.languages.map((l) => (
            <option key={l.code} value={l.code}>
              {l.name}
            </option>
          ))}
        </select>
      </Row>
    </Section>
  );
}

function ToneEditor({ tone, onClose }: { tone: Tone; onClose: () => void }) {
  const setTones = useApp((s) => s.setTones);
  const tones = useApp((s) => s.tones);
  const [draft, setDraft] = useState(tone);
  const save = useMutation({
    mutationFn: () => api().saveTone(draft),
    onSuccess: (saved) => {
      setTones(tones.some((t) => t.id === saved.id) ? tones.map((t) => (t.id === saved.id ? saved : t)) : [...tones, saved]);
      onClose();
    },
    onError: (e) => toastError(errorMessage(e)),
  });
  const remove = useMutation({
    mutationFn: () => api().deleteTone(tone.id),
    onSuccess: (next) => {
      setTones(next);
      onClose();
    },
    onError: (e) => toastError(errorMessage(e)),
  });

  return (
    <motion.div className={styles.toneEditor} initial={{ opacity: 0, height: 0 }} animate={{ opacity: 1, height: 'auto' }} exit={{ opacity: 0, height: 0 }}>
      <label>
        <span className={ui.label}>Name</span>
        <input className={ui.input} value={draft.name} onChange={(e) => setDraft({ ...draft, name: e.target.value })} autoFocus={!tone.id} />
      </label>
      <label>
        <span className={ui.label}>Context for the model: who reads it, where, what's normal there</span>
        <textarea className={ui.input} rows={3} value={draft.description} onChange={(e) => setDraft({ ...draft, description: e.target.value })} />
      </label>
      <div className={styles.toneRow}>
        <div>
          <span className={ui.label}>Formality</span>
          <Segmented<Formality>
            label="Formality"
            value={draft.formality}
            onChange={(formality) => setDraft({ ...draft, formality })}
            options={[
              { value: 'casual', label: 'Casual' },
              { value: 'neutral', label: 'Neutral' },
              { value: 'formal', label: 'Formal' },
            ]}
          />
        </div>
        <div>
          <span className={ui.label}>Abbreviations & short forms</span>
          <Segmented<ShortForms>
            label="Short forms"
            value={draft.shortForms}
            onChange={(shortForms) => setDraft({ ...draft, shortForms })}
            options={[
              { value: 'welcome', label: 'Welcome' },
              { value: 'sparingly', label: 'Sparingly' },
              { value: 'avoid', label: 'Avoid' },
            ]}
          />
        </div>
      </div>
      <div className={styles.toneActions}>
        {tone.id && tones.length > 1 && (
          <button type="button" className={ui.danger} onClick={() => remove.mutate()}>
            Delete
          </button>
        )}
        <span className={styles.spacer} />
        <button type="button" className={ui.ghost} onClick={onClose}>
          Cancel
        </button>
        <button type="button" className={ui.primary} onClick={() => save.mutate()} disabled={!draft.name.trim() || save.isPending}>
          Save tone
        </button>
      </div>
    </motion.div>
  );
}

const NEW_TONE: Tone = { id: '', name: '', description: '', formality: 'neutral', shortForms: 'sparingly', builtin: false };

function TonesSection() {
  const tones = useApp((s) => s.tones);
  const [editing, setEditing] = useState<string | null>(null);
  return (
    <Section title="Tones" hint="Where your text will be posted. It tells the model what's normal there, like abbreviations in Slack but not in a formal email.">
      <ul className={styles.tones}>
        {tones.map((t) => (
          <li key={t.id} className={styles.tone}>
            <button type="button" className={styles.toneHead} onClick={() => setEditing(editing === t.id ? null : t.id)} aria-expanded={editing === t.id}>
              <span className={styles.toneName}>{t.name}</span>
              <span className={styles.toneDesc}>{t.description}</span>
            </button>
            <AnimatePresence>{editing === t.id && <ToneEditor tone={t} onClose={() => setEditing(null)} />}</AnimatePresence>
          </li>
        ))}
        <li className={styles.tone}>
          {editing === 'new' ? (
            <AnimatePresence>
              <ToneEditor tone={NEW_TONE} onClose={() => setEditing(null)} />
            </AnimatePresence>
          ) : (
            <button type="button" className={`${ui.ghost} ${styles.newTone}`} onClick={() => setEditing('new')}>
              <PlusIcon size={16} /> New tone
            </button>
          )}
        </li>
      </ul>
    </Section>
  );
}

function NumberInput({ value, onChange, min, max, suffix }: { value: number; onChange: (n: number) => void; min: number; max: number; suffix: string }) {
  const [text, setText] = useState(String(value));
  const ref = useRef(value);
  useEffect(() => {
    if (ref.current !== value) setText(String(value));
    ref.current = value;
  }, [value]);
  return (
    <span className={styles.number}>
      <input
        className={`${ui.input} ${styles.numberInput}`}
        inputMode="numeric"
        value={text}
        onChange={(e) => setText(e.target.value.replace(/\D/g, ''))}
        onBlur={() => {
          const n = Math.min(max, Math.max(min, Number(text) || min));
          setText(String(n));
          if (n !== value) onChange(n);
        }}
      />
      {suffix}
    </span>
  );
}

export function SettingsPage() {
  const settings = useApp((s) => s.settings);
  const saveSettings = useApp((s) => s.saveSettings);
  const appVersion = useApp((s) => s.appVersion);
  const exporter = useExport();
  const save = (next: Settings) => saveSettings(next).catch((e) => toastError(errorMessage(e)));
  const policy = settings.insights;

  return (
    <div className={styles.page}>
      <h1 className={styles.title}>Settings</h1>
      <ProviderSection />
      <LanguagesSection />
      <TonesSection />
      <Section title="Progress summary" hint="Numbers are always computed on your computer. Only the short written summary uses the model.">
        <Row label="Write it in the background" hint="While you're away from the app">
          <Switch label="Write summary automatically" checked={policy.auto} onChange={(auto) => void save({ ...settings, insights: { ...policy, auto } })} />
        </Row>
        <Row label="How often">
          <span className={styles.inline}>
            after <NumberInput value={policy.minNewEntries} min={1} max={100} suffix="new texts" onChange={(minNewEntries) => void save({ ...settings, insights: { ...policy, minNewEntries } })} />
            and <NumberInput value={policy.minHoursBetween} min={1} max={720} suffix="hours" onChange={(minHoursBetween) => void save({ ...settings, insights: { ...policy, minHoursBetween } })} />
          </span>
        </Row>
        <Row label="Idle after">
          <NumberInput value={settings.idleMinutes} min={1} max={120} suffix="minutes" onChange={(idleMinutes) => void save({ ...settings, idleMinutes })} />
        </Row>
      </Section>
      <Section title="Appearance">
        <Row label="Theme">
          <Segmented<Theme>
            label="Theme"
            value={settings.theme}
            onChange={(theme) => void save({ ...settings, theme })}
            options={[
              { value: 'system', label: 'System' },
              { value: 'dark', label: 'Dark' },
              { value: 'light', label: 'Light' },
            ]}
          />
        </Row>
      </Section>
      <Section title="Your data" hint="Everything stays on this computer. The export is one JSON file with every text, result and stat.">
        <Row label="Export">
          <button type="button" className={ui.secondary} onClick={() => exporter.mutate()} disabled={exporter.isPending}>
            {exporter.isPending ? <Spinner /> : <DownloadIcon size={16} />} Export all to Downloads
          </button>
        </Row>
      </Section>
      <p className={styles.version}>Refine {appVersion}</p>
    </div>
  );
}
