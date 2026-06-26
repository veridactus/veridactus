// VERIDACTUS Dialog — 生产级模态对话框
// 替换所有浏览器原生 prompt()/confirm()/alert()
// 支持：标题/内容/操作按钮/输入框/键盘 Escape/点击遮罩关闭
import { ReactNode, useEffect, useRef, useCallback } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { X } from 'lucide-react';

export interface DialogProps {
  open: boolean;
  onClose: () => void;
  title?: string;
  children?: ReactNode;
  /** 底部操作按钮 */
  actions?: ReactNode;
  /** 最大宽度 */
  maxWidth?: string;
  /** 是否允许点击遮罩关闭 */
  closeOnBackdrop?: boolean;
}

export default function Dialog({
  open, onClose, title, children, actions, maxWidth = '420px', closeOnBackdrop = true,
}: DialogProps) {
  const overlayRef = useRef<HTMLDivElement>(null);

  const handleKeyDown = useCallback((e: KeyboardEvent) => {
    if (e.key === 'Escape') onClose();
  }, [onClose]);

  useEffect(() => {
    if (open) {
      document.addEventListener('keydown', handleKeyDown);
      document.body.style.overflow = 'hidden';
    }
    return () => {
      document.removeEventListener('keydown', handleKeyDown);
      document.body.style.overflow = '';
    };
  }, [open, handleKeyDown]);

  return (
    <AnimatePresence>
      {open && (
        <motion.div
          ref={overlayRef}
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          exit={{ opacity: 0 }}
          transition={{ duration: 0.18 }}
          onClick={(e) => { if (closeOnBackdrop && e.target === overlayRef.current) onClose(); }}
          className="fixed inset-0 z-[1000] flex items-center justify-center p-4"
          style={{ background: 'rgba(0,0,0,0.6)', backdropFilter: 'blur(4px)' }}
        >
          <motion.div
            initial={{ opacity: 0, scale: 0.95, y: 12 }}
            animate={{ opacity: 1, scale: 1, y: 0 }}
            exit={{ opacity: 0, scale: 0.95, y: 8 }}
            transition={{ duration: 0.2, ease: [0.16, 1, 0.3, 1] }}
            className="w-full rounded-card border border-[var(--border-default)] bg-[var(--bg-secondary)] p-6 shadow-card"
            style={{ maxWidth }}
          >
            {/* Header */}
            {title && (
              <div className="flex items-center justify-between mb-4">
                <h2 className="text-lg font-bold text-[var(--text-primary)]">{title}</h2>
                <button
                  onClick={onClose}
                  className="p-1 rounded-lg text-[var(--text-tertiary)] hover:text-[var(--text-primary)] hover:bg-[var(--bg-glass)] transition-colors"
                  aria-label="关闭"
                >
                  <X size={18} />
                </button>
              </div>
            )}

            {/* Content */}
            {children && <div className="text-sm text-[var(--text-secondary)] leading-relaxed">{children}</div>}

            {/* Actions */}
            {actions && <div className="flex justify-end gap-3 mt-6">{actions}</div>}
          </motion.div>
        </motion.div>
      )}
    </AnimatePresence>
  );
}

// ==================== 预设对话框类型 ====================

/** 确认对话框 (替代 window.confirm) */
export function ConfirmDialog({
  open, onClose, onConfirm, title = '确认操作', message, confirmText = '确认', cancelText = '取消',
  danger = false,
}: {
  open: boolean; onClose: () => void; onConfirm: () => void;
  title?: string; message: string; confirmText?: string; cancelText?: string; danger?: boolean;
}) {
  return (
    <Dialog open={open} onClose={onClose} title={title}>
      <p>{message}</p>
      {{
        actions: (
          <>
            <button onClick={onClose} className="btn-secondary text-sm px-4 py-2">
              {cancelText}
            </button>
            <button
              onClick={() => { onConfirm(); onClose(); }}
              className={danger ? 'text-sm px-4 py-2 rounded-btn bg-[var(--color-error)] text-white font-semibold hover:opacity-90 transition-opacity' : 'btn-primary text-sm px-4 py-2'}
            >
              {confirmText}
            </button>
          </>
        ),
      }.actions}
    </Dialog>
  );
}

/** 提示对话框 (替代 window.alert) */
export function AlertDialog({
  open, onClose, title = '提示', message,
}: { open: boolean; onClose: () => void; title?: string; message: string }) {
  return (
    <Dialog open={open} onClose={onClose} title={title}>
      <p>{message}</p>
      {{
        actions: (
          <button onClick={onClose} className="btn-primary text-sm px-4 py-2">
            确定
          </button>
        ),
      }.actions}
    </Dialog>
  );
}

/** 输入对话框 (替代 window.prompt) */
export function PromptDialog({
  open, onClose, onSubmit, title = '输入', placeholder, defaultValue = '', submitText = '确认',
}: {
  open: boolean; onClose: () => void; onSubmit: (value: string) => void;
  title?: string; placeholder?: string; defaultValue?: string; submitText?: string;
}) {
  const inputRef = useRef<HTMLInputElement>(null);
  useEffect(() => { if (open && inputRef.current) { inputRef.current.value = defaultValue; inputRef.current.focus(); } }, [open, defaultValue]);

  const handleSubmit = () => {
    const val = inputRef.current?.value?.trim();
    if (val) { onSubmit(val); onClose(); }
  };

  return (
    <Dialog open={open} onClose={onClose} title={title}>
      <input
        ref={inputRef}
        type="text"
        placeholder={placeholder}
        defaultValue={defaultValue}
        onKeyDown={(e) => { if (e.key === 'Enter') handleSubmit(); }}
        className="input-field w-full"
        autoFocus
      />
      {{
        actions: (
          <>
            <button onClick={onClose} className="btn-secondary text-sm px-4 py-2">取消</button>
            <button onClick={handleSubmit} className="btn-primary text-sm px-4 py-2">{submitText}</button>
          </>
        ),
      }.actions}
    </Dialog>
  );
}
