// 🛡️ 安全盾牌 — 纯视觉指示器，绝不阻止输入
import { useEffect, useState } from 'react';
import { motion } from 'framer-motion';
import { Shield, ShieldAlert, ShieldCheck } from 'lucide-react';

interface SafetyShieldProps { text: string; size?: number; }

const PII_PATTERNS = [
  { name: 'email', regex: /[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}/g },
  { name: 'phone', regex: /1[3-9]\d{9}/g },
  { name: 'id_card', regex: /[1-9]\d{5}(?:19|20)\d{2}(?:0[1-9]|1[0-2])(?:0[1-9]|[12]\d|3[01])\d{3}[\dXx]/g },
];

const INJECTION_PATTERNS = [
  /ignore\s+(?:all\s+)?(?:previous\s+)?instructions/i,
  /you\s+are\s+now\s+(?:a\s+)?DAN/i,
  /system:\s*you\s+are/i,
  /pretend\s+(?:you\s+are|to\s+be)/i,
  /forget\s+(?:all\s+)?(?:previous\s+)?instructions/i,
];

export default function SafetyShield({ text, size = 22 }: SafetyShieldProps) {
  const [status, setStatus] = useState<'idle'|'safe'|'warning'>('idle');

  useEffect(() => {
    const t = setTimeout(() => {
      if (!text.trim()) { setStatus('idle'); return; }
      if (INJECTION_PATTERNS.some(p => p.test(text))) { setStatus('warning'); return; }
      for (const p of PII_PATTERNS) { if (p.regex.test(text)) { setStatus('warning'); return; } }
      setStatus('safe');
    }, 200);
    return () => clearTimeout(t);
  }, [text]);

  const color = status === 'warning' ? '#fdcb6e' : status === 'safe' ? '#00d4aa' : 'var(--text-secondary)';
  const Icon = status === 'warning' ? ShieldAlert : status === 'safe' ? ShieldCheck : Shield;

  return (
    <motion.div
      animate={{ scale: status === 'warning' ? [1, 1.1, 1] : 1 }}
      transition={{ repeat: status === 'warning' ? Infinity : 0, duration: 1.5 }}
      style={{ display: 'flex', alignItems: 'center', flexShrink: 0 }}
      title={status === 'warning' ? '检测到敏感信息，发送时将自动处理' : status === 'safe' ? '输入安全' : ''}
    >
      <Icon size={size} color={color} />
    </motion.div>
  );
}
