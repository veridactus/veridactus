// 微信登录后绑定手机号 — 提升账户安全性
import { useState, useEffect } from 'react';
import { useNavigate, useLocation } from 'react-router-dom';
import { motion } from 'framer-motion';
import { Smartphone, Shield, AlertTriangle, ArrowRight } from 'lucide-react';
import { getToken, setToken } from './AuthGuard';

const API = (import.meta as any)?.env?.VITE_API_URL || '';

export default function PhoneBind() {
  const navigate = useNavigate(); const location = useLocation();
  const [phone, setPhone] = useState(''); const [code, setCode] = useState('');
  const [sent, setSent] = useState(false); const [countdown, setCountdown] = useState(0);
  const [error, setError] = useState(''); const [loading, setLoading] = useState(false);

  useEffect(() => {
    const token = getToken(); if (!token) { const urlToken = new URLSearchParams(location.search).get('token');
      if (urlToken) { setToken(urlToken); window.history.replaceState({}, '', '/bind-phone'); } else { navigate('/login'); } }
  }, []);

  const handleSendCode = async () => {
    if (!phone || !/^\+?\d{10,15}$/.test(phone)) { setError('请输入正确的手机号'); return; }
    setLoading(true); setError('');
    try {
      const res = await fetch(`${API}/api/v1/auth/phone/send`, { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ phone }) });
      if (res.ok) { setSent(true); setCountdown(60); const t = setInterval(() => setCountdown(c => { if (c <= 1) { clearInterval(t); return 0; } return c - 1; }), 1000); }
      else { setError('发送验证码失败'); }
    } catch { setError('服务连接失败'); } finally { setLoading(false); }
  };

  const handleBind = async () => {
    if (!phone || !code) { setError('请填写手机号和验证码'); return; }
    setLoading(true); setError('');
    try {
      const res = await fetch(`${API}/api/v1/auth/bind-phone`, { method: 'POST', headers: { 'Content-Type': 'application/json', Authorization: `Bearer ${getToken()}` }, body: JSON.stringify({ phone, code }) });
      if (res.ok) { navigate('/chat', { replace: true }); } else { const d = await res.json(); setError(d.message || d.error || '绑定失败'); }
    } catch { setError('服务连接失败'); } finally { setLoading(false); }
  };

  return (
    <div className="min-h-screen flex items-center justify-center font-sans" style={{ background: '#0B0F19' }}>
      <motion.div initial={{ opacity: 0, y: 20 }} animate={{ opacity: 1, y: 0 }}
        className="rounded-[20px] py-10 px-8 max-w-[400px] w-full" style={{ background: 'rgba(19,22,51,0.95)', border: '1px solid rgba(108,92,231,0.2)', boxShadow: '0 0 60px rgba(108,92,231,0.1)' }}>
        <div className="text-center mb-7">
          <motion.div animate={{ scale: [1, 1.1, 1] }} transition={{ duration: 2, repeat: Infinity }}><Shield size={40} color="#6c5ce7" /></motion.div>
          <h2 className="text-xl font-bold text-white mt-3">绑定手机号</h2>
          <p className="text-[13px] text-[#8892b0] mt-1.5">绑定手机号后可找回账户、接收安全通知</p>
        </div>

        {error && <div className="py-2.5 px-3.5 rounded-btn mb-4 flex items-center gap-2 text-sm text-[#ff7675]" style={{ background: 'rgba(255,118,117,0.1)', border: '1px solid rgba(255,118,117,0.3)' }}><AlertTriangle size={16} /> {error}</div>}

        <div className="flex flex-col gap-3.5">
          <div className="relative">
            <Smartphone size={15} className="absolute left-3.5 top-1/2 -translate-y-1/2 text-[#8892b0]" />
            <input type="tel" placeholder="手机号 (如 +8613800138000)" value={phone} onChange={e => setPhone(e.target.value)}
              className="w-full py-3 pl-10 pr-3.5 rounded-btn text-sm text-[#e2e8f0] outline-none box-border" style={{ background: 'rgba(255,255,255,0.04)', border: '1px solid rgba(255,255,255,0.08)' }} />
          </div>

          {sent ? (
            <div className="flex gap-2.5">
              <input type="text" placeholder="验证码" value={code} onChange={e => setCode(e.target.value)}
                className="flex-1 py-3 px-3.5 rounded-btn text-sm text-[#e2e8f0] outline-none" style={{ background: 'rgba(255,255,255,0.04)', border: '1px solid rgba(255,255,255,0.08)' }} />
              <button type="button" disabled={countdown > 0} onClick={() => { setSent(true); handleSendCode(); }}
                className="px-4 rounded-btn border text-xs cursor-pointer" style={{ background: countdown > 0 ? 'rgba(108,92,231,0.1)' : 'rgba(108,92,231,0.2)', borderColor: 'rgba(108,92,231,0.3)', color: countdown > 0 ? '#8892b0' : '#6c5ce7' }}>
                {countdown > 0 ? `${countdown}s` : '重发'}
              </button>
            </div>
          ) : (
            <motion.button whileHover={{ scale: 1.01 }} whileTap={{ scale: 0.98 }} onClick={handleSendCode}
              className="py-3 rounded-btn border text-sm font-semibold text-white cursor-pointer" style={{ borderColor: 'rgba(7,193,96,0.3)', background: 'linear-gradient(135deg, rgba(7,193,96,0.15), rgba(7,193,96,0.05))' }}>
              <Smartphone size={14} className="inline align-sub mr-1.5" />发送验证码
            </motion.button>
          )}

          {sent && (
            <motion.button whileHover={{ scale: 1.01 }} whileTap={{ scale: 0.98 }} onClick={handleBind} disabled={loading}
              className="py-3 rounded-btn border-none text-sm font-bold cursor-pointer text-white" style={{ background: 'linear-gradient(135deg, #6c5ce7, #00d4aa)', opacity: loading ? 0.7 : 1 }}>
              {loading ? '绑定中...' : '确认绑定'} <ArrowRight size={14} className="inline align-sub ml-1" />
            </motion.button>
          )}
        </div>

        <button onClick={() => navigate('/chat', { replace: true })}
          className="w-full mt-4 py-2.5 rounded-btn bg-transparent border text-[13px] text-[#8892b0] cursor-pointer" style={{ borderColor: 'rgba(255,255,255,0.06)' }}>
          跳过，稍后绑定
        </button>
      </motion.div>
    </div>
  );
}