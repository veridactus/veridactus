// 微信扫码登录 Modal — 真实 QR 码 + 轮询 + 手动确认
// 使用 qrserver.com 生成真实可扫描的二维码
import { useState, useEffect, useRef } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { X, ExternalLink, RefreshCw, Check, Shield, Smartphone } from 'lucide-react';
import { setToken } from './AuthGuard';

const API = (import.meta as any)?.env?.VITE_API_URL || '';
const POLL_MS = 1500; // 轮询间隔

interface Props {
  isOpen: boolean;
  onClose: () => void;
  onSuccess: (needBindPhone: boolean) => void;
}

export default function WeChatQRModal({ isOpen, onClose, onSuccess }: Props) {
  const [state, setState] = useState('');
  const [qrDataUrl, setQrDataUrl] = useState('');    // QR 码包含的 URL（HTML 回调页）
  const [devCallbackUrl, setDevCallbackUrl] = useState(''); // Dev 模拟扫码的 JSON API URL
  const [status, setStatus] = useState<'loading'|'scanning'|'success'|'expired'|'error'>('loading');
  const [errorMsg, setErrorMsg] = useState('');
  const [isDev, setIsDev] = useState(true);
  const pollRef = useRef<NodeJS.Timeout | null>(null);
  const popupRef = useRef<Window | null>(null);
  const mountedRef = useRef(true);

  // 初始化
  useEffect(() => {
    if (!isOpen) return;
    mountedRef.current = true;
    setStatus('loading'); setErrorMsg('');

    fetch(`${API}/api/v1/auth/login/wechat`)
      .then(r => r.json())
      .then(d => {
        if (!mountedRef.current) return;
        setIsDev(d.dev_mode || false);
        setState(d.state || '');
        if (d.dev_mode) {
          // 开发模式：
          //   QR 指向 HTML 回调页 → 模拟微信扫码后跳转的页面
          //   Dev 按钮调用 JSON API → 直接获取 token
          const cbUrl = `${API}${d.dev_url}`;
          setQrDataUrl(cbUrl);
          // JSON API: 替换 callback-page → callback
          const apiUrl = cbUrl.replace('/wechat/callback-page?', '/callback/wechat?');
          setDevCallbackUrl(apiUrl);
        } else if (d.login_url) {
          setQrDataUrl(d.login_url);
        }
        if (d.state) startPolling(d.state);
        setStatus('scanning');
      })
      .catch(() => { if (mountedRef.current) { setErrorMsg('无法连接认证服务'); setStatus('error'); } });

    return () => { mountedRef.current = false; stopPolling(); if (popupRef.current) popupRef.current.close(); };
  }, [isOpen]);

  // 轮询状态
  const startPolling = (s: string) => {
    stopPolling();
    pollRef.current = setInterval(async () => {
      try {
        const res = await fetch(`${API}/api/v1/auth/wechat/status?state=${s}`);
        const d = await res.json();
        if (d.status === 'completed') {
          stopPolling();
          setToken(d.token);
          setStatus('success');
          setTimeout(() => { if (mountedRef.current) { onSuccess(d.need_bind_phone || false); onClose(); } }, 1000);
        } else if (d.status === 'expired') {
          setStatus('expired');
          stopPolling();
        }
      } catch { /* ignore poll errors */ }
    }, POLL_MS);
  };

  const stopPolling = () => {
    if (pollRef.current) { clearInterval(pollRef.current); pollRef.current = null; }
  };

  // 开发模式: 模拟扫码 → 调用 JSON API 获取 token
  const handleDevScan = async () => {
    setStatus('scanning');
    try {
      const res = await fetch(devCallbackUrl);
      const data = await res.json();
      if (data.access_token) {
        setToken(data.access_token);
        localStorage.setItem('veridactus_user', JSON.stringify(data.user || {}));
        setStatus('success');
        setTimeout(() => { if (mountedRef.current) { onSuccess(data.need_bind_phone || false); onClose(); } }, 1000);
      } else { setErrorMsg('登录失败'); setStatus('error'); }
    } catch { setErrorMsg('服务连接失败'); setStatus('error'); }
  };

  // 生产模式: 打开微信授权窗口
  const handleProdWeChat = () => {
    if (!qrDataUrl) return;
    // 将 redirect_uri 替换为我们的 HTML 回调页
    const prodUrl = qrDataUrl.replace(/redirect_uri=[^&]+/, 'redirect_uri=' + encodeURIComponent(window.location.origin + '/api/v1/auth/wechat/callback-page'));
    popupRef.current = window.open(prodUrl, 'wechat_login', 'width=800,height=650');
    if (!popupRef.current) setErrorMsg('请允许浏览器弹窗');
  };

  const handleRetry = () => { setStatus('loading'); setErrorMsg(''); setQrDataUrl(''); };

  if (!isOpen) return null;

  // QR code image URL (using qrserver.com free API)
  const qrImageUrl = qrDataUrl
    ? `https://api.qrserver.com/v1/create-qr-code/?size=200x200&data=${encodeURIComponent(qrDataUrl)}`
    : '';

  return (
    <AnimatePresence>
      <motion.div initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }}
        style={{ position: 'fixed', inset: 0, zIndex: 9999, display: 'flex', alignItems: 'center', justifyContent: 'center', background: 'rgba(0,0,0,0.75)', backdropFilter: 'blur(4px)' }}
        onClick={onClose}>
        <motion.div initial={{ scale: 0.92 }} animate={{ scale: 1 }} exit={{ scale: 0.92 }}
          onClick={e => e.stopPropagation()}
          style={{ background: '#131633', borderRadius: 24, padding: '36px 28px 28px', maxWidth: 380, width: '100%', textAlign: 'center', border: '1px solid rgba(7,193,96,0.2)', boxShadow: '0 0 80px rgba(7,193,96,0.12)', position: 'relative' }}>
          <button onClick={onClose} style={{ position: 'absolute', top: 14, right: 14, background: 'none', border: 'none', color: '#8892b0', cursor: 'pointer' }}><X size={18} /></button>

          {/* Header */}
          <div style={{ marginBottom: 20 }}>
            <div style={{ width: 48, height: 48, borderRadius: 12, margin: '0 auto', background: 'linear-gradient(135deg, #07c160, #06ad56)', display: 'flex', alignItems: 'center', justifyContent: 'center', boxShadow: '0 6px 20px rgba(7,193,96,0.35)' }}>
              <span style={{ fontSize: 24 }}>💬</span>
            </div>
            <h3 style={{ fontSize: 18, fontWeight: 700, color: '#fff', margin: '12px 0 4px' }}>微信扫码登录</h3>
            <p style={{ fontSize: 12, color: '#8892b0' }}>
              {status === 'loading' ? '正在生成二维码...' :
               status === 'scanning' ? '请使用微信扫描二维码' :
               status === 'success' ? '登录成功！' :
               status === 'expired' ? '二维码已过期' : '出错了'}
            </p>
          </div>

          {/* QR Code Image */}
          <div style={{
            width: 200, height: 200, margin: '0 auto 16px', borderRadius: 12, overflow: 'hidden',
            background: status === 'success' ? 'linear-gradient(135deg, rgba(0,212,170,0.15), rgba(0,212,170,0.05))' : '#fff',
            border: `2px solid ${status === 'success' ? 'rgba(0,212,170,0.5)' : 'rgba(7,193,96,0.25)'}`,
            display: 'flex', alignItems: 'center', justifyContent: 'center', position: 'relative',
          }}>
            {status === 'loading' && (
              <motion.div animate={{ rotate: 360 }} transition={{ duration: 1, repeat: Infinity, ease: 'linear' }}
                style={{ width: 28, height: 28, border: '3px solid rgba(7,193,96,0.15)', borderTopColor: '#07c160', borderRadius: '50%' }} />
            )}
            {(status === 'scanning' || status === 'expired') && qrImageUrl && (
              <img src={qrImageUrl} alt="微信扫码登录"
                style={{ width: '100%', height: '100%', objectFit: 'contain', padding: 8 }}
                onError={(e) => { (e.target as HTMLImageElement).style.display = 'none'; }} />
            )}
            {status === 'expired' && (
              <div style={{ position: 'absolute', inset: 0, background: 'rgba(0,0,0,0.6)', display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
                <span style={{ color: '#ff7675', fontSize: 13, fontWeight: 600 }}>已过期</span>
              </div>
            )}
            {status === 'success' && (
              <motion.div initial={{ scale: 0 }} animate={{ scale: 1 }} transition={{ type: 'spring', stiffness: 200 }}>
                <Shield size={56} color="#00d4aa" />
              </motion.div>
            )}
          </div>

          {/* Status text */}
          {status === 'scanning' && (
            <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', gap: 6, marginBottom: 12, color: '#07c160', fontSize: 12 }}>
              <motion.div animate={{ opacity: [1, 0.3, 1] }} transition={{ duration: 1.5, repeat: Infinity }}
                style={{ width: 6, height: 6, borderRadius: '50%', background: '#07c160' }} />
              等待扫码确认...
            </div>
          )}

          {/* Action Buttons */}
          <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
            {/* 开发模式: 模拟扫码按钮 */}
            {isDev && status === 'scanning' && (
              <button onClick={handleDevScan}
                style={{ padding: '11px', borderRadius: 10, border: 'none', background: 'linear-gradient(135deg, #07c160, #06ad56)', color: '#fff', fontSize: 14, fontWeight: 700, cursor: 'pointer', display: 'flex', alignItems: 'center', justifyContent: 'center', gap: 6 }}>
                <Smartphone size={15} /> 模拟微信扫码确认
              </button>
            )}

            {/* 生产模式: 打开微信 */}
            {!isDev && status === 'scanning' && (
              <button onClick={handleProdWeChat}
                style={{ padding: '11px', borderRadius: 10, border: '1px solid rgba(7,193,96,0.3)', background: 'rgba(7,193,96,0.1)', color: '#07c160', fontSize: 13, fontWeight: 600, cursor: 'pointer', display: 'flex', alignItems: 'center', justifyContent: 'center', gap: 6 }}>
                <ExternalLink size={14} /> 在微信中打开
              </button>
            )}

            {(status === 'expired' || status === 'error') && (
              <button onClick={handleRetry}
                style={{ padding: '10px', borderRadius: 10, border: '1px solid rgba(108,92,231,0.3)', background: 'rgba(108,92,231,0.1)', color: '#6c5ce7', fontSize: 13, fontWeight: 600, cursor: 'pointer', display: 'flex', alignItems: 'center', justifyContent: 'center', gap: 4 }}>
                <RefreshCw size={13} /> 刷新二维码
              </button>
            )}

            <button onClick={onClose}
              style={{ padding: '8px', borderRadius: 10, background: 'transparent', border: '1px solid rgba(255,255,255,0.05)', color: '#8892b0', fontSize: 12, cursor: 'pointer' }}>
              返回邮箱登录
            </button>
          </div>

          <p style={{ fontSize: 10, color: '#5a6a8a', marginTop: 14, lineHeight: 1.5 }}>
            {isDev ? '开发模式：点击「模拟扫码」完成登录' : '使用微信扫描上方二维码完成登录'}
            <br />首次登录自动创建账户
          </p>
        </motion.div>
      </motion.div>
    </AnimatePresence>
  );
}
