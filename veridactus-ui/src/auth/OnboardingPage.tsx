// Onboarding 引导页 — BYOK vs Platform 双卡片选择（生产级）
import { useState, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import { motion } from 'framer-motion';
import Lottie from 'lottie-react';
import { Shield, Key, Database, Zap, ArrowRight, Check, Loader2 } from 'lucide-react';
import { getStoredToken } from './useAuth';

// 内置简约盾牌/安全锁 JSON 动画（无需外部文件）
const shieldLottie = {
  v: "5.7.4", fr: 30, ip: 0, op: 60, w: 200, h: 200, nm: "Shield",
  layers: [{
    nm: "Shield", ty: 4, ip: 0, op: 60, sr: 1, st: 0,
    shapes: [{
      ty: "gr", it: [
        { ty: "rc", d: 1, s: { a: 0, k: [100, 120] }, p: { a: 0, k: [0, -10] }, r: { a: 1, k: [{ t: 0, s: [0] }, { t: 30, s: [5] }, { t: 60, s: [0] }] } },
        { ty: "fl", c: { a: 0, k: [0.42, 0.36, 0.91, 1] }, o: { a: 1, k: [{ t: 0, s: [60] }, { t: 30, s: [100] }, { t: 60, s: [60] }] } },
        { ty: "st", c: { a: 0, k: [0.42, 0.36, 0.91, 1] }, w: { a: 0, k: 3 }, o: { a: 0, k: 100 } }
      ]
    }]
  }]
};

const API_BASE = (import.meta as any)?.env?.VITE_API_URL || '';

export default function OnboardingPage() {
  const navigate = useNavigate();
  const [selected, setSelected] = useState<'byok' | 'platform' | null>(null);
  const [saving, setSaving] = useState(false);
  const token = getStoredToken();

  // 如果已登录检查是否已完成 onboarding
  useEffect(() => {
    if (!token) {
      navigate('/login', { replace: true });
      return;
    }
    fetch(`${API_BASE}/api/v1/settings`, {
      headers: { Authorization: `Bearer ${token}` },
    }).then(r => r.json()).then(d => {
      if (d?.settings?.onboarding_completed === 'true') {
        navigate('/chat', { replace: true });
      }
    }).catch(() => {});
  }, []);

  const handleContinue = async () => {
    if (!selected || saving) return;
    setSaving(true);

    try {
      // 保存用户的选择偏好到后端
      if (token) {
        await fetch(`${API_BASE}/api/v1/settings`, {
          method: 'PUT',
          headers: {
            'Content-Type': 'application/json',
            Authorization: `Bearer ${token}`,
          },
          body: JSON.stringify({
            onboarding_completed: 'true',
            key_preference: selected,
          }),
        });
      }
    } catch {
      // 设置保存失败不影响后续流程
      console.warn('Failed to save onboarding preference');
    }

    if (selected === 'byok') {
      navigate('/api-keys');
    } else {
      navigate('/chat');
    }
  };

  const steps = [
    { id: 1, label: '选择模式', active: true, done: false },
    { id: 2, label: selected ? '开始使用' : '等待选择', active: !!selected, done: false },
  ];

  return (
    <div style={{
      minHeight: '100vh', display: 'flex', alignItems: 'center', justifyContent: 'center',
      background: 'linear-gradient(135deg, #0B0F19 0%, #131633 50%, #1a1040 100%)',
      fontFamily: 'system-ui, sans-serif', padding: 24,
    }}>
      <motion.div
        initial={{ opacity: 0, y: 30 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.6 }}
        style={{ maxWidth: 700, width: '100%', textAlign: 'center' }}
      >
        {/* Step indicator */}
        <div style={{ display: 'flex', justifyContent: 'center', gap: 32, marginBottom: 24 }}>
          {steps.map((s, i) => (
            <div key={s.id} style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
              <div style={{
                width: 28, height: 28, borderRadius: '50%',
                background: s.active ? 'linear-gradient(135deg, #6c5ce7, #00d4aa)' : 'rgba(255,255,255,0.08)',
                border: s.active ? 'none' : '1px solid rgba(255,255,255,0.2)',
                display: 'flex', alignItems: 'center', justifyContent: 'center',
                fontSize: 12, fontWeight: 700, color: '#fff',
              }}>
                {s.id}
              </div>
              <span style={{ fontSize: 12, color: s.active ? '#fff' : '#5a6a8a', fontWeight: 600 }}>
                {s.label}
              </span>
              {i < steps.length - 1 && (
                <div style={{ width: 20, height: 1, background: 'rgba(255,255,255,0.1)' }} />
              )}
            </div>
          ))}
        </div>

        {/* Lottie 盾牌动画 — AI-1.md 指令 5.1 */}
        <div style={{ width: 120, height: 120, margin: '0 auto' }}>
          <Lottie animationData={shieldLottie} loop={true} />
        </div>
        <h1 style={{
          fontSize: 28, fontWeight: 800, color: '#fff', marginTop: 16,
          background: 'linear-gradient(135deg, #6c5ce7, #00d4aa)',
          WebkitBackgroundClip: 'text', WebkitTextFillColor: 'transparent',
        }}>
          欢迎来到 VERIDACTUS
        </h1>
        <p style={{ color: '#8892b0', fontSize: 14, marginTop: 8, marginBottom: 40 }}>
          选择你的 AI 治理方式 — 后续可在设置中随时更改
        </p>

        <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 20, marginBottom: 32 }}>
          {/* BYOK Card */}
          <motion.div
            whileHover={{ scale: 1.02, y: -4 }}
            whileTap={{ scale: 0.98 }}
            onClick={() => setSelected('byok')}
            style={{
              background: selected === 'byok'
                ? 'linear-gradient(135deg, rgba(108,92,231,0.15), rgba(108,92,231,0.08))'
                : 'rgba(255,255,255,0.03)',
              border: selected === 'byok'
                ? '2px solid rgba(108,92,231,0.5)'
                : '1px solid rgba(255,255,255,0.08)',
              borderRadius: 20, padding: '28px 24px', cursor: 'pointer',
              transition: 'all 0.2s', textAlign: 'left', position: 'relative',
              boxShadow: selected === 'byok' ? '0 0 40px rgba(108,92,231,0.2)' : 'none',
            }}
          >
            {selected === 'byok' && (
              <motion.div initial={{ scale: 0 }} animate={{ scale: 1 }}
                style={{
                  position: 'absolute', top: 12, right: 12,
                  width: 24, height: 24, borderRadius: '50%',
                  background: '#6c5ce7', display: 'flex', alignItems: 'center', justifyContent: 'center',
                }}>
                <Check size={14} color="#fff" />
              </motion.div>
            )}
            <div style={{
              width: 48, height: 48, borderRadius: 14, marginBottom: 16,
              background: 'rgba(108,92,231,0.2)', display: 'flex', alignItems: 'center', justifyContent: 'center',
            }}>
              <Key size={24} color="#6c5ce7" />
            </div>
            <h3 style={{ fontSize: 18, fontWeight: 700, color: '#fff', marginBottom: 8 }}>
              🔐 我有 API Key
            </h3>
            <p style={{ fontSize: 13, color: '#8892b0', lineHeight: 1.6 }}>
              自带 LLM Provider Key（智谱 / DeepSeek / 百度千帆）。
              加密存储在安全金库中，你拥有完全控制权。
            </p>
            <div style={{ marginTop: 16, display: 'flex', flexDirection: 'column', gap: 6 }}>
              {['AES-256-GCM 信封加密', '密钥永不落盘', '支持环境变量注入'].map(f => (
                <span key={f} style={{ fontSize: 11, color: '#6c5ce7', display: 'flex', alignItems: 'center', gap: 4 }}>
                  <Check size={12} /> {f}
                </span>
              ))}
            </div>
          </motion.div>

          {/* Platform Card */}
          <motion.div
            whileHover={{ scale: 1.02, y: -4 }}
            whileTap={{ scale: 0.98 }}
            onClick={() => setSelected('platform')}
            style={{
              background: selected === 'platform'
                ? 'linear-gradient(135deg, rgba(0,212,170,0.15), rgba(0,212,170,0.08))'
                : 'rgba(255,255,255,0.03)',
              border: selected === 'platform'
                ? '2px solid rgba(0,212,170,0.5)'
                : '1px solid rgba(255,255,255,0.08)',
              borderRadius: 20, padding: '28px 24px', cursor: 'pointer',
              transition: 'all 0.2s', textAlign: 'left', position: 'relative',
              boxShadow: selected === 'platform' ? '0 0 40px rgba(0,212,170,0.2)' : 'none',
            }}
          >
            {selected === 'platform' && (
              <motion.div initial={{ scale: 0 }} animate={{ scale: 1 }}
                style={{
                  position: 'absolute', top: 12, right: 12,
                  width: 24, height: 24, borderRadius: '50%',
                  background: '#00d4aa', display: 'flex', alignItems: 'center', justifyContent: 'center',
                }}>
                <Check size={14} color="#fff" />
              </motion.div>
            )}
            <div style={{
              width: 48, height: 48, borderRadius: 14, marginBottom: 16,
              background: 'rgba(0,212,170,0.2)', display: 'flex', alignItems: 'center', justifyContent: 'center',
            }}>
              <Zap size={24} color="#00d4aa" />
            </div>
            <h3 style={{ fontSize: 18, fontWeight: 700, color: '#fff', marginBottom: 8 }}>
              ⚡ 我需要 API Key
            </h3>
            <p style={{ fontSize: 13, color: '#8892b0', lineHeight: 1.6 }}>
              平台统一聚合主流模型（GLM / DeepSeek / 千帆）。
              按微美元计费，即用即付，无需自行管理 Key。
            </p>
            <div style={{ marginTop: 16, display: 'flex', flexDirection: 'column', gap: 6 }}>
              {['国内模型全聚合', '微美元计费', '¥10 免费额度'].map(f => (
                <span key={f} style={{ fontSize: 11, color: '#00d4aa', display: 'flex', alignItems: 'center', gap: 4 }}>
                  <Check size={12} /> {f}
                </span>
              ))}
            </div>
          </motion.div>
        </div>

        <motion.button
          whileHover={selected && !saving ? { scale: 1.02 } : {}}
          whileTap={selected && !saving ? { scale: 0.98 } : {}}
          onClick={handleContinue}
          disabled={!selected || saving}
          style={{
            padding: '16px 48px', borderRadius: 14,
            background: selected
              ? 'linear-gradient(135deg, #6c5ce7, #00d4aa)'
              : 'rgba(255,255,255,0.05)',
            border: 'none', color: '#fff', fontSize: 16, fontWeight: 700,
            cursor: selected && !saving ? 'pointer' : 'not-allowed',
            opacity: selected && !saving ? 1 : 0.4, transition: 'all 0.2s',
            display: 'inline-flex', alignItems: 'center', gap: 8,
          }}
        >
          {saving ? (
            <><Loader2 size={18} style={{ animation: 'spin 1s linear infinite' }} /> 保存中...</>
          ) : (
            <>开始使用 <ArrowRight size={18} /></>
          )}
        </motion.button>
      </motion.div>
    </div>
  );
}
