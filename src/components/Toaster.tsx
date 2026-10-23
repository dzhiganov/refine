import { AnimatePresence, motion } from 'motion/react';
import { create } from 'zustand';
import { CloseIcon } from './icons';
import styles from './Toaster.module.css';

interface Toast {
  id: number;
  message: string;
  tone: 'default' | 'error';
  action?: { label: string; run: () => void };
}

interface ToastState {
  toasts: Toast[];
  push(t: Omit<Toast, 'id'>): void;
  dismiss(id: number): void;
}

let next = 1;

export const useToasts = create<ToastState>((set, get) => ({
  toasts: [],
  push: (t) => {
    const id = next++;
    set({ toasts: [...get().toasts.slice(-2), { ...t, id }] });
    setTimeout(() => get().dismiss(id), t.tone === 'error' ? 7000 : 4000);
  },
  dismiss: (id) => set({ toasts: get().toasts.filter((t) => t.id !== id) }),
}));

export const toast = (message: string, action?: Toast['action']) =>
  useToasts.getState().push({ message, tone: 'default', action });
export const toastError = (message: string) => useToasts.getState().push({ message, tone: 'error' });

export function Toaster() {
  const { toasts, dismiss } = useToasts();
  return (
    <div className={styles.region} role="status" aria-live="polite">
      <AnimatePresence initial={false}>
        {toasts.map((t) => (
          <motion.div
            key={t.id}
            layout
            className={styles.toast}
            data-tone={t.tone}
            initial={{ opacity: 0, y: 16, scale: 0.96 }}
            animate={{ opacity: 1, y: 0, scale: 1 }}
            exit={{ opacity: 0, y: 8, scale: 0.96, transition: { duration: 0.15 } }}
          >
            <span className={styles.message}>{t.message}</span>
            {t.action && (
              <button
                type="button"
                className={styles.action}
                onClick={() => {
                  t.action?.run();
                  dismiss(t.id);
                }}
              >
                {t.action.label}
              </button>
            )}
            <button type="button" className={styles.close} aria-label="Dismiss" onClick={() => dismiss(t.id)}>
              <CloseIcon size={14} />
            </button>
          </motion.div>
        ))}
      </AnimatePresence>
    </div>
  );
}
