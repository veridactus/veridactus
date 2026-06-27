// Onboarding 引导页 — BYOK vs Platform 双卡片选择（生产级）
import { useState, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import { motion } from 'framer-motion';
import Lottie from 'lottie-react';
import { Shield, Key, Zap, ArrowRight, Check, Loader2 } from 'lucide-react';
import { getStoredToken } from './useAuth';

const shieldLottie = { v: "5.7.4", fr: 30, ip: 0, op: 60, w: 200, h: 200, nm: "Shield", layers: [{ nm: "Shield", ty: 4, ip: 0, op: 60, sr: 1, st: 0, shapes: [{ ty: "gr", it: [{ ty: "rc", d: 1, s: { a: 0, k: [100, 120] }, p: { a: 0, k: [0, -10] }, r: { a: 1, k: [{ t: 0, s: [0] }, { t: 30, s: [5] }, { t: 60, s: [0] }] } }, { ty: "fl", c: { a: 0, k: [0.42, 0.36, 0.91, 1] }, o: { a: 1, k: [{ t: 0, s: [60] }, { t: 30, s: [100] }, { t: 60, s: [60] }] } }, { ty: "st", c: { a: 0, k: [0.42, 0.36, 0.91, 1] }, w: { a: 0, k: 3 }, o: { a: 0, k: 100 } }] }] }] };
const API_BASE = (import.meta as any)?.env?.VITE_API_URL || '';

const cards = [
  { id: 'byok' as const, color: '#6c5ce7', icon: Key, title: '🔐 我有 API Key', desc: '自带 LLM Provider Key（智谱 / DeepSeek / 百度千帆）。加密存储在安全金库中，你拥有完全控制权。', features: ['AES-256-GCM 信封加密', '密钥永不落盘', '支持环境变量注入'], next: '/api-keys' },
  { id: 'platform' as const, color: '#00d4aa', icon: Zap, title: '⚡ 我需要 API Key', desc: '平台统一聚合主流模型（GLM / DeepSeek / 千帆）。按微美元计费，即用即付，无需自行管理 Key。', features: ['国内模型全聚合', '微美元计费', '¥10 免费额度'], next: '/chat' },
];

export default function OnboardingPage() {
  const navigate = useNavigate();
  const [selected, setSelected] = useState<'byok' | 'platform' | null>(null);
  const [saving, setSaving] = useState(false);
  const token = getStoredToken();

  useEffect(() => { if (!token) { navigate('/login', { replace: true }); return; }
    fetch(`${API_BASE}/api/v1/settings`, { headers: { Authorization: `Bearer ${token}` } })
      .then(r => r.json()).then(d => { if (d?.settings?.onboarding_completed === 'true') navigate('/chat', { replace: true }); }).catch(() => {});
  }, []);

  const handleContinue = async () => {
    if (!selected || saving) return; setSaving(true);
    try { if (token) await fetch(`${API_BASE}/api/v1/settings`, { method: 'PUT', headers: { 'Content-Type': 'application/json', Authorization: `Bearer ${token}` }, body: JSON.stringify({ onboarding_completed: 'true', key_preference: selected }) }); } catch {}
    navigate(cards.find(c => c.id === selected)?.next || '/chat');
  };

  const steps = [{ id: 1, label: '选择模式', active: true }, { id: 2, label: selected ? '开始使用' : '等待选择', active: !!selected }];

  return (
    <div className="min-h-screen flex items-center justify-center p-6 font-sans" style={{ background: 'linear-gradient(135deg, #0B0F19 0%, #131633 50%, #1a1040 100%)' }}>
      <motion.div initial={{ opacity: 0, y: 30 }} animate={{ opacity: 1, y: 0 }} transition={{ duration: 0.6 }} className="max-w-[700px] w-full text-center">
        {/* Steps */}
        <div className="flex justify-center gap-8 mb-6">
          {steps.map((s, i) => (<div key={s.id} className="flex items-center gap-2">
            <div className="w-7 h-7 rounded-full flex items-center justify-center text-xs font-bold text-white" style={{ background: s.active ? 'linear-gradient(135deg, #6c5ce7, #00d4aa)' : 'rgba(255,255,255,0.08)', border: s.active ? 'none' : '1px solid rgba(255,255,255,0.2)' }}>{s.id}</div>
            <span className="text-xs font-semibold" style={{ color: s.active ? '#fff' : '#5a6a8a' }}>{s.label}</span>
            {i < steps.length - 1 && <div className="w-5 h-px" style={{ background: 'rgba(255,255,255,0.1)' }} />}
          </div>))}
        </div>

        <div className="w-[120px] h-[120px] mx-auto"><Lottie animationData={shieldLottie} loop={true} /></div>
        <h1 className="text-3xl font-extrabold mt-4 bg-gradient-to-r from-[#6c5ce7] to-[#00d4aa] bg-clip-text text-transparent">欢迎来到 VERIDACTUS</h1>
        <p className="text-sm text-[#8892b0] mt-2 mb-10">选择你的 AI 治理方式 — 后续可在设置中随时更改</p>

        <div className="grid grid-cols-2 gap-5 mb-8">
          {cards.map(c => (
            <motion.div key={c.id} whileHover={{ scale: 1.02, y: -4 }} whileTap={{ scale: 0.98 }} onClick={() => setSelected(c.id)}
              className="rounded-[20px] py-7 px-6 cursor-pointer text-left relative transition-all" style={{
                background: selected === c.id ? `linear-gradient(135deg, ${c.color}26, ${c.color}14)` : 'rgba(255,255,255,0.03)',
                border: selected === c.id ? `2px solid ${c.color}80` : '1px solid rgba(255,255,255,0.08)',
                boxShadow: selected === c.id ? `0 0 40px ${c.color}33` : 'none',
              }}>
              {selected === c.id && <motion.div initial={{ scale: 0 }} animate={{ scale: 1 }} className="absolute top-3 right-3 w-6 h-6 rounded-full flex items-center justify-center" style={{ background: c.color }}><Check size={14} color="#fff" /></motion.div>}
              <div className="w-12 h-12 rounded-2xl mb-4 flex items-center justify-center" style={{ background: `${c.color}33` }}><c.icon size={24} color={c.color} /></div>
              <h3 className="text-lg font-bold text-white mb-2">{c.title}</h3>
              <p className="text-[13px] text-[#8892b0] leading-relaxed">{c.desc}</p>
              <div className="mt-4 flex flex-col gap-1.5">{c.features.map(f => <span key={f} className="text-[11px] flex items-center gap-1" style={{ color: c.color }}><Check size={12} /> {f}</span>)}</div>
            </motion.div>
          ))}
        </div>

        <motion.button whileHover={selected && !saving ? { scale: 1.02 } : {}} whileTap={selected && !saving ? { scale: 0.98 } : {}} onClick={handleContinue} disabled={!selected || saving}
          className="py-4 px-12 rounded-2xl border-none text-white text-base font-bold inline-flex items-center gap-2 transition-all" style={{
            background: selected ? 'linear-gradient(135deg, #6c5ce7, #00d4aa)' : 'rgba(255,255,255,0.05)',
            cursor: selected && !saving ? 'pointer' : 'not-allowed', opacity: selected && !saving ? 1 : 0.4,
          }}>
          {saving ? <><Loader2 size={18} className="animate-spin" /> 保存中...</> : <>开始使用 <ArrowRight size={18} /></>}
        </motion.button>
      </motion.div>
    </div>
  );
}