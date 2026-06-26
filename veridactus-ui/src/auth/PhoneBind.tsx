// 微信登录后绑定手机号 — 提升账户安全性
import { useState, useEffect, useRef } from 'react';
import { useNavigate, useLocation } from 'react-router-dom';
import { motion } from 'framer-motion';
import { Smartphone, Check, Shield, AlertTriangle, ArrowRight } from 'lucide-react';
import { getToken, setToken } from './AuthGuard';

const API = (import.meta as any)?.env?.VITE_API_URL || '';

export default function PhoneBind() {
  const navigate = useNavigate();
  const location = useLocation();
  const [phone, setPhone] = useState('');
  const [code, setCode] = useState('');
  const [sent, setSent] = useState(false);
  const [countdown, setCountdown] = useState(0);
  const [error, setError] = useState('');
  const [loading, setLoading] = useState(false);

  // 检查是否有 JWT token
  useEffect(() => {
    const token = getToken();
    if (!token) {
      // 从 URL 参数中提取 token (微信回调返回)
      const params = new URLSearchParams(location.search);
      const urlToken = params.get('token');
      if (urlToken) {
        setToken(urlToken);
        window.history.replaceState({}, '', '/bind-phone');
      } else {
        navigate('/login');
      }
    }
  }, []);

  const handleSendCode = async () => {
    if (!phone || !/^\+?\d{10,15}$/.test(phone)) { setError('请输入正确的手机号'); return; }
    setLoading(true); setError('');
    try {
      const res = await fetch(`${API}/api/v1/auth/phone/send`, {
        method: 'POST', headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ phone }),
      });
      if (res.ok) { setSent(true); setCountdown(60); const t = setInterval(() => setCountdown(c => { if (c <= 1) { clearInterval(t); return 0; } return c - 1; }), 1000); }
      else { setError('发送验证码失败'); }
    } catch { setError('服务连接失败'); }
    finally { setLoading(false); }
  };

  const handleBind = async () => {
    if (!phone || !code) { setError('请填写手机号和验证码'); return; }
    setLoading(true); setError('');
    try {
      const token = getToken();
      const res = await fetch(`${API}/api/v1/auth/bind-phone`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json', Authorization: `Bearer ${token}` },
        body: JSON.stringify({ phone, code }),
      });
      if (res.ok) {
        navigate('/chat', { replace: true });
      } else {
        const d = await res.json();
        setError(d.message || d.error || '绑定失败');
      }
    } catch { setError('服务连接失败'); }
    finally { setLoading(false); }
  };

  const handleSkip = () => navigate('/chat', { replace: true });

  return (
    <div style={{ minHeight: '100vh', display: 'flex', alignItems: 'center', justifyContent: 'center', background: '#0B0F19', fontFamily: 'system-ui, -apple-system, sans-serif' }}>
      <motion.div initial={{ opacity: 0, y: 20 }} animate={{ opacity: 1, y: 0 }} style={{ background: 'rgba(19,22,51,0.95)', borderRadius: 20, padding: '40px 32px', maxWidth: 400, width: '100%', border: '1px solid rgba(108,92,231,0.2)', boxShadow: '0 0 60px rgba(108,92,231,0.1)' }}>
        <div style={{ textAlign: 'center', marginBottom: 28 }}>
          <motion.div animate={{ scale: [1, 1.1, 1] }} transition={{ duration: 2, repeat: Infinity }}>
            <Shield size={40} color="#6c5ce7" />
          </motion.div>
          <h2 style={{ fontSize: 20, fontWeight: 700, color: '#fff', marginTop: 12 }}>绑定手机号</h2>
          <p style={{ fontSize: 13, color: '#8892b0', marginTop: 6 }}>绑定手机号后可找回账户、接收安全通知</p>
        </div>

        {error && (
          <div style={{ padding: '10px 14px', borderRadius: 10, marginBottom: 16, background: 'rgba(255,118,117,0.1)', border: '1px solid rgba(255,118,117,0.3)', display: 'flex', alignItems: 'center', gap: 8, fontSize: 13, color: '#ff7675' }}>
            <AlertTriangle size={16} /> {error}
          </div>
        )}

        <div style={{ display: 'flex', flexDirection: 'column', gap: 14 }}>
          <div style={{ position: 'relative' }}>
            <Smartphone size={15} style={{ position: 'absolute', left: 14, top: '50%', transform: 'translateY(-50%)', color: '#8892b0' }} />
            <input type="tel" placeholder="手机号 (如 +8613800138000)" value={phone} onChange={e => setPhone(e.target.value)}
              style={{ width: '100%', padding: '12px 14px 12px 40px', borderRadius: 10, background: 'rgba(255,255,255,0.04)', border: '1px solid rgba(255,255,255,0.08)', color: '#e2e8f0', fontSize: 14, outline: 'none', boxSizing: 'border-box' }} />
          </div>

          {sent ? (
            <div style={{ display: 'flex', gap: 10 }}>
              <input type="text" placeholder="验证码" value={code} onChange={e => setCode(e.target.value)}
                style={{ flex: 1, padding: '12px 14px', borderRadius: 10, background: 'rgba(255,255,255,0.04)', border: '1px solid rgba(255,255,255,0.08)', color: '#e2e8f0', fontSize: 14, outline: 'none' }} />
              <button type="button" disabled={countdown > 0}
                onClick={() => { setSent(true); handleSendCode(); }}
                style={{ padding: '0 16px', borderRadius: 10, background: countdown > 0 ? 'rgba(108,92,231,0.1)' : 'rgba(108,92,231,0.2)', border: '1px solid rgba(108,92,231,0.3)', color: countdown > 0 ? '#8892b0' : '#6c5ce7', fontSize: 12, cursor: 'pointer' }}>
                {countdown > 0 ? `${countdown}s` : '重发'}
              </button>
            </div>
          ) : (
            <motion.button whileHover={{ scale: 1.01 }} whileTap={{ scale: 0.98 }}
              onClick={handleSendCode}
              style={{ padding: '12px', borderRadius: 10, border: '1px solid rgba(7,193,96,0.3)', background: 'linear-gradient(135deg, rgba(7,193,96,0.15), rgba(7,193,96,0.05))', color: '#fff', fontSize: 14, fontWeight: 600, cursor: 'pointer' }}>
              <Smartphone size={14} style={{ verticalAlign: -2, marginRight: 6 }} />发送验证码
            </motion.button>
          )}

          {sent && (
            <motion.button whileHover={{ scale: 1.01 }} whileTap={{ scale: 0.98 }}
              onClick={handleBind}
              disabled={loading}
              style={{ padding: '12px', borderRadius: 10, border: 'none', background: 'linear-gradient(135deg, #6c5ce7, #00d4aa)', color: '#000', fontSize: 14, fontWeight: 700, cursor: 'pointer', opacity: loading ? 0.7 : 1 }}>
              {loading ? '绑定中...' : '确认绑定'} <ArrowRight size={14} style={{ verticalAlign: -2, marginLeft: 4 }} />
            </motion.button>
          )}
        </div>

        <button onClick={handleSkip}
          style={{ width: '100%', marginTop: 16, padding: '10px', borderRadius: 10, background: 'transparent', border: '1px solid rgba(255,255,255,0.06)', color: '#8892b0', fontSize: 13, cursor: 'pointer' }}>
          跳过，稍后绑定
        </button>
      </motion.div>
    </div>
  );
}
