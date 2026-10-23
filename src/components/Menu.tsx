import { AnimatePresence, motion } from 'motion/react';
import { useEffect, useRef, useState, type ReactNode } from 'react';
import { CheckIcon, ChevronDownIcon } from './icons';
import styles from './Menu.module.css';

export interface MenuOption<T extends string> {
  value: T;
  label: string;
  hint?: string;
}

/** A pill that opens a small menu. Keyboard: arrows, Enter, Escape. */
export function PillSelect<T extends string>({
  value,
  options,
  onChange,
  label,
  icon,
  footer,
}: {
  value: T;
  options: MenuOption<T>[];
  onChange: (v: T) => void;
  label: string;
  icon?: ReactNode;
  footer?: ReactNode;
}) {
  const [open, setOpen] = useState(false);
  const [active, setActive] = useState(0);
  const root = useRef<HTMLDivElement>(null);
  const current = options.find((o) => o.value === value);

  useEffect(() => {
    if (!open) return;
    setActive(Math.max(0, options.findIndex((o) => o.value === value)));
    const onDown = (e: MouseEvent) => {
      if (!root.current?.contains(e.target as Node)) setOpen(false);
    };
    document.addEventListener('mousedown', onDown);
    return () => document.removeEventListener('mousedown', onDown);
  }, [open, options, value]);

  const choose = (v: T) => {
    onChange(v);
    setOpen(false);
  };

  const onKeyDown = (e: React.KeyboardEvent) => {
    if (!open && (e.key === 'ArrowDown' || e.key === 'Enter' || e.key === ' ')) {
      e.preventDefault();
      setOpen(true);
      return;
    }
    if (!open) return;
    if (e.key === 'Escape') {
      e.preventDefault();
      e.stopPropagation();
      setOpen(false);
    } else if (e.key === 'ArrowDown') {
      e.preventDefault();
      setActive((i) => (i + 1) % options.length);
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      setActive((i) => (i - 1 + options.length) % options.length);
    } else if (e.key === 'Enter') {
      e.preventDefault();
      const o = options[active];
      if (o) choose(o.value);
    }
  };

  return (
    <div className={styles.root} ref={root} onKeyDown={onKeyDown}>
      <button
        type="button"
        className={styles.pill}
        aria-haspopup="listbox"
        aria-expanded={open}
        aria-label={`${label}: ${current?.label ?? ''}`}
        onClick={() => setOpen((o) => !o)}
      >
        {icon}
        <span>{current?.label ?? label}</span>
        <ChevronDownIcon size={14} className={styles.chevron} />
      </button>
      <AnimatePresence>
        {open && (
          <motion.div
            className={styles.menu}
            role="listbox"
            aria-label={label}
            initial={{ opacity: 0, y: -4, scale: 0.98 }}
            animate={{ opacity: 1, y: 0, scale: 1 }}
            exit={{ opacity: 0, y: -4, scale: 0.98, transition: { duration: 0.12 } }}
            transition={{ type: 'spring', stiffness: 600, damping: 38 }}
          >
            {options.map((o, i) => (
              <button
                key={o.value}
                type="button"
                role="option"
                aria-selected={o.value === value}
                className={styles.option}
                data-active={i === active || undefined}
                onMouseEnter={() => setActive(i)}
                onClick={() => choose(o.value)}
              >
                <span className={styles.optionText}>
                  <span>{o.label}</span>
                  {o.hint && <span className={styles.hint}>{o.hint}</span>}
                </span>
                {o.value === value && <CheckIcon size={16} />}
              </button>
            ))}
            {footer && <div className={styles.footer}>{footer}</div>}
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}
