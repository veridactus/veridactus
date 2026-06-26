// VERIDACTUS Toast — 生产级轻量通知系统
// 替换所有浏览器原生 alert() 作为用户反馈
import { create } from 'zustand';
import { motion, AnimatePresence } from 'framer-motion';
import { CheckCircle, XCircle, AlertTriangle, Info, X } from 'lucide-react';
import { useEffect } from 'react';

// ==================== Toast Store ====================

export type ToastType = 'success' | 'error' | 'warning' | 'info';

export interface ToastItem {
  id: string;
  type: ToastType;
  message: string;
  duration?: number; // ms, 默认 3000
}

interface ToastState {
  toasts: ToastItem[];
  addToast: (t: Omit<ToastItem, 'id'>) => void;
  removeToast: (id: string) => void;
}

let toastId = 0;
export const useToastStore = create<ToastState>((set) => ({
  toasts: [],
  addToast: (t) => {
    const id = `toast-${++toastId}`;
    set((s) => ({ toasts: [...s.toasts, { ...t, id }] }));
    const dur = t.duration ?? 3000;
    if (dur > 0) setTimeout(() => { useToastStore.getState().removeToast(id); }, dur);
  },
  removeToast: (id) => set((s) => ({ toasts: s.toasts.filter((t) => t.id !== id) })),
}));

// ==================== 便捷 API ====================

export const toast = {
  success: (message: string, duration?: number) =>
    useToastStore.getState().addToast({ type: 'success', message, duration }),
  error: (message: string, duration?: number) =>
    useToastStore.getState().addToast({ type: 'error', message, duration }),
  warning: (message: string, duration?: number) =>
    useToastStore.getState().addToast({ type: 'warning', message, duration }),
  info: (message: string, duration?: number) =>
    useToastStore.getState().addToast({ type: 'info', message, duration }),
};

// ==================== 渲染容器 ====================

const iconMap: Record<ToastType, typeof CheckCircle> = {
  success: CheckCircle,
  error: XCircle,
  warning: AlertTriangle,
  info: Info,
};

const colorMap: Record<ToastType, string> = {
  success: '#00d4aa',
  error: '#ff7675',
  warning: '#fdcb6e',
  info: '#6c5ce7',
};

export function ToastContainer() {
  const { toasts, removeToast } = useToastStore();

  return (
    <div
      className="fixed top-4 right-4 z-[2000] flex flex-col gap-2 pointer-events-none"
      style={{ maxWidth: '380px' }}
    >
      <AnimatePresence>
        {toasts.map((t) => {
          const Icon = iconMap[t.type];
          return (
            <motion.div
              key={t.id}
              initial={{ opacity: 0, x: 60, scale: 0.95 }}
              animate={{ opacity: 1, x: 0, scale: 1 }}
              exit={{ opacity: 0, x: 30, scale: 0.95 }}
              transition={{ duration: 0.22, ease: [0.16, 1, 0.3, 1] }}
              className="pointer-events-auto flex items-start gap-3 p-4 rounded-card border border-[var(--border-default)] bg-[var(--bg-secondary)] shadow-card"
              style={{ borderLeftColor: colorMap[t.type], borderLeftWidth: 3 }}
            >
              <Icon size={18} color={colorMap[t.type]} className="mt-0.5 flex-shrink-0" />
              <span className="text-sm text-[var(--text-primary)] flex-1">{t.message}</span>
              <button
                onClick={() => removeToast(t.id)}
                className="p-0.5 rounded text-[var(--text-tertiary)] hover:text-[var(--text-primary)] flex-shrink-0"
              >
                <X size={14} />
              </button>
            </motion.div>
          );
        })}
      </AnimatePresence>
    </div>
  );
}

// ==================== 自动挂载 Hook ====================

export function useToastMount() {
  useEffect(() => {
    let container = document.getElementById('veridactus-toast-root');
    if (!container) {
      container = document.createElement('div');
      container.id = 'veridactus-toast-root';
      document.body.appendChild(container);
    }
  }, []);
  return null;
}
