import { motion } from 'motion/react';
import { useId } from 'react';
import { shortcutLabel } from '../lib/platform';
import ui from './ui.module.css';

export function Kbd({ keys, muted }: { keys: string; muted?: boolean }) {
  return <kbd className={muted ? ui.kbdMuted : ui.kbd}>{shortcutLabel(keys)}</kbd>;
}

export function Spinner() {
  return <span className={ui.spinner} role="status" aria-label="Working" />;
}

export function Switch({
  checked,
  onChange,
  label,
}: {
  checked: boolean;
  onChange: (v: boolean) => void;
  label: string;
}) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      aria-label={label}
      className={ui.switch}
      onClick={() => onChange(!checked)}
    />
  );
}

export function Segmented<T extends string>({
  value,
  options,
  onChange,
  label,
}: {
  value: T;
  options: { value: T; label: string }[];
  onChange: (v: T) => void;
  label: string;
}) {
  const id = useId();
  return (
    <div className={ui.segmented} role="group" aria-label={label}>
      {options.map((o) => (
        <button
          key={o.value}
          type="button"
          className={ui.segment}
          aria-pressed={o.value === value}
          onClick={() => onChange(o.value)}
        >
          {o.value === value && <motion.span layoutId={`seg-${id}`} className={ui.segmentThumb} />}
          <span className={ui.segmentLabel}>{o.label}</span>
        </button>
      ))}
    </div>
  );
}
