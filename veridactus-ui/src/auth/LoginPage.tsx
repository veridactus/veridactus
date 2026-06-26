/// <reference types="vite/client" />
// VERIDACTUS 登录/注册 — 生产级多通道认证
// Email + Phone + 微信 + GitHub + Google 全通道支持
import { useState, useEffect, useRef } from 'react';
import { useNavigate, useLocation } from 'react-router-dom';
import { motion } from 'framer-motion';
import { Shield, Github, Mail, Lock, User, Eye, EyeOff, AlertTriangle, Check, Phone, Smartphone, Building, User as UserIcon, ArrowRight, Sparkles } from 'lucide-react';
import { setToken, isAuthenticated } from './AuthGuard';
import WeChatQRModal from './WeChatQRModal';

// API 地址：生产环境使用相对路径（同域），开发环境可设置 VITE_API_URL
const API = (typeof window !== 'undefined' && (window as any).__VERIDACTUS_API__) ||
  (import.meta as any)?.env?.VITE_API_URL || '';

type AuthTab = 'login' | 'register';
type Method = 'email' | 'phone';

export default function LoginPage() {
  const navigate = useNavigate();
  const location = useLocation();
  const from = (location.state as any)?.from || '/chat';

  useEffect(() => { if (isAuthenticated()) navigate(from, { replace: true }); }, []);

  const [tab, setTab] = useState<AuthTab>('register');
  const [method, setMethod] = useState<Method>('email');
  const [plan, setPlan] = useState<'personal' | 'enterprise'>('personal');
  const [email, setEmail] = useState('');
  const [password, setPassword] = useState('');
  const [displayName, setDisplayName] = useState('');
  const [orgName, setOrgName] = useState('');
  const [phone, setPhone] = useState('');
  const [phoneCode, setPhoneCode] = useState('');
  const [phoneSent, setPhoneSent] = useState(false);
  const [phoneVerified, setPhoneVerified] = useState(false);
  const [showPassword, setShowPassword] = useState(false);
  const [error, setError] = useState('');
  const [loading, setLoading] = useState(false);
  const [githubAvailable, setGithubAvailable] = useState(false);
  const [githubUrl, setGithubUrl] = useState('');
  const [googleAvailable, setGoogleAvailable] = useState(false);
  const [googleUrl, setGoogleUrl] = useState('');
  const [showWeChat, setShowWeChat] = useState(false);
  const phoneTimerRef = useRef<NodeJS.Timeout | null>(null);
  const [phoneCountdown, setPhoneCountdown] = useState(0);

  useEffect(() => {
    const params = new URLSearchParams(location.search);
    // 生产微信回调: HTML页面重定向回来时携带 wechat_token
    const wxToken = params.get('wechat_token');
    if (wxToken) {
      setToken(wxToken);
      // 清除 URL 中的 token 参数
      window.history.replaceState({}, '', '/login');
      navigate(from, { replace: true });
      return;
    }
    // 开发模式微信回调: 直接带 code 参数
    const wxCode = params.get('code');
    if (wxCode) {
      handleWeChatCallback(wxCode);
      return;
    }
    fetch(`${API}/api/v1/auth/login/github`)
      .then(r => r.json())
      .then(d => { if (d.auth_url && !d.error) { setGithubAvailable(true); setGithubUrl(d.auth_url); } })
      .catch(() => {});
    fetch(`${API}/api/v1/auth/login/google`)
      .then(r => r.json())
      .then(d => { if (d.auth_url && !d.error) { setGoogleAvailable(true); setGoogleUrl(d.auth_url); } })
      .catch(() => {});
    return () => { if (phoneTimerRef.current) clearInterval(phoneTimerRef.current); };
  }, []);

  const handleWeChatCallback = async (code: string) => {
    setLoading(true); setError('');
    try {
      const res = await fetch(`${API}/api/v1/auth/callback/wechat?code=${code}`);
      const data = await res.json();
      if (!res.ok) { setError(data.message || '微信登录失败'); return; }
      if (data.access_token) {
        setToken(data.access_token);
        localStorage.setItem('veridactus_user', JSON.stringify(data.user || {}));
        if (data.need_bind_phone) {
          navigate('/bind-phone', { replace: true });
        } else {
          navigate(from, { replace: true });
        }
      }
    } catch { setError('微信服务连接失败'); }
    finally { setLoading(false); }
  };

  const passwordStrength = (pw: string): { score: number; label: string; color: string } => {
    if (!pw) return { score: 0, label: '', color: '#8892b0' };
    let s = 0;
    if (pw.length >= 8) s++;
    if (/[A-Z]/.test(pw)) s++;
    if (/[a-z]/.test(pw)) s++;
    if (/[0-9]/.test(pw)) s++;
    if (/[!@#$%^&*()_+\-=\[\]{};:,.<>?~]/.test(pw)) s++;
    if (s <= 2) return { score: s, label: '弱', color: '#ff7675' };
    if (s <= 3) return { score: s, label: '一般', color: '#fdcb6e' };
    if (s <= 4) return { score: s, label: '安全', color: '#00d4aa' };
    return { score: s, label: '非常安全', color: '#6c5ce7' };
  };
  const pwdSt = passwordStrength(password);

  const handleSendCode = async () => {
    if (!phone || loading) return;
    setLoading(true); setError('');
    try {
      const res = await fetch(`${API}/api/v1/auth/phone/send`, {
        method: 'POST', headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ phone }),
      });
      if (res.ok) {
        setPhoneSent(true); setPhoneCountdown(60);
        const t = setInterval(() => setPhoneCountdown(c => { if (c <= 1) { clearInterval(t); return 0; } return c - 1; }), 1000);
      } else {
        const data = await res.json();
        setError(data.message || '短信发送失败，请确认已配置短信服务商');
      }
    } catch { setError('服务连接失败'); }
    finally { setLoading(false); }
  };

  const handleVerifyCode = async () => {
    if (!phone || !phoneCode) return;
    setLoading(true);
    try {
      const res = await fetch(`${API}/api/v1/auth/phone/verify`, {
        method: 'POST', headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ phone, code: phoneCode }),
      });
      if (res.ok) setPhoneVerified(true);
      else setError('验证码错误');
    } catch { setError('验证失败'); }
    finally { setLoading(false); }
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault(); setError('');
    if (method === 'email') {
      if (!email || !password) { setError('请填写邮箱和密码'); return; }
      if (tab === 'register' && !displayName) { setError('请填写显示名称'); return; }
      if (tab === 'register' && pwdSt.score < 3) { setError('密码强度不够，请使用包含大写+小写+数字+特殊字符的密码'); return; }
    } else {
      if (!phone || !phoneVerified) { setError('请先验证手机号'); return; }
    }

    setLoading(true);
    try {
      let res: Response;
      if (method === 'email' && tab === 'register') {
        res = await fetch(`${API}/api/v1/auth/register`, {
          method: 'POST', headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ email, password, display_name: displayName, plan, org_name: plan === 'enterprise' ? orgName : undefined }),
        });
      } else if (method === 'email' && tab === 'login') {
        res = await fetch(`${API}/api/v1/auth/login`, {
          method: 'POST', headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ email, password }),
        });
      } else if (method === 'phone') {
        // 手机号登录/注册：已验证后通过 phone/verify 完成
        res = await fetch(`${API}/api/v1/auth/phone/verify`, {
          method: 'POST', headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ phone, code: phoneCode }),
        });
      } else {
        setError('请选择登录方式'); setLoading(false); return;
      }

      const data = await res.json();
      if (!res.ok) { setError(data.error || data.message || '操作失败'); setLoading(false); return; }
      if (data.access_token) {
        setToken(data.access_token);
        localStorage.setItem('veridactus_user', JSON.stringify(data.user || {}));
        navigate(from, { replace: true });
      }
    } catch { setError('无法连接服务器'); setLoading(false); }
  };

  return (
    <div style={{ minHeight: '100vh', display: 'flex', fontFamily: 'system-ui, -apple-system, sans-serif', background: '#0B0F19' }}>
      {/* 左侧品牌区 */}
      <div style={{ flex: 1, display: 'flex', flexDirection: 'column', justifyContent: 'center', padding: '0 80px', background: 'radial-gradient(ellipse at 30% 50%, rgba(108,92,231,0.12) 0%, transparent 70%)', position: 'relative', overflow: 'hidden' }}>
        <div style={{ position: 'absolute', top: -200, left: -200, width: 600, height: 600, borderRadius: '50%', background: 'radial-gradient(circle, rgba(0,212,170,0.08) 0%, transparent 70%)', filter: 'blur(60px)' }} />
        <motion.div initial={{ opacity: 0, x: -40 }} animate={{ opacity: 1, x: 0 }} transition={{ duration: 0.8 }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: 12, marginBottom: 24 }}>
            <motion.div animate={{ scale: [1, 1.08, 1], rotate: [0, 0, 0] }} transition={{ duration: 3, repeat: Infinity }}>
              <Shield size={52} color="#6c5ce7" style={{ filter: 'drop-shadow(0 0 24px rgba(108,92,231,0.5))' }} />
            </motion.div>
            <h1 style={{ fontSize: 42, fontWeight: 900, background: 'linear-gradient(135deg, #6c5ce7 30%, #00d4aa 70%)', WebkitBackgroundClip: 'text', WebkitTextFillColor: 'transparent', margin: 0 }}>VERIDACTUS</h1>
          </div>
          <h2 style={{ fontSize: 22, color: '#e0e6f0', fontWeight: 600, marginBottom: 12, maxWidth: 480 }}>
            AI 治理的可信基础设施
          </h2>
          <p style={{ fontSize: 15, color: '#8892b0', lineHeight: 1.8, maxWidth: 460, marginBottom: 40 }}>
            每条 AI 交互都经过密码学签名审计，不可篡改、可独立验证。
            L0 → L2A → L2B 三层证明链，符合 EU AI Act 和 GDPR 合规要求。
          </p>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
            {[
              { icon: <Shield size={16} color="#00d4aa" />, text: 'L0 密码学签名审计' },
              { icon: <Sparkles size={16} color="#6c5ce7" />, text: 'OWASP ASI Top 10 全覆盖' },
              { icon: <Building size={16} color="#74b9ff" />, text: '企业版 SSO + 审计报告' },
            ].map((f, i) => (
              <div key={i} style={{ display: 'flex', alignItems: 'center', gap: 10, color: '#a0aec0', fontSize: 14 }}>
                {f.icon} {f.text}
              </div>
            ))}
          </div>
        </motion.div>
      </div>

      {/* 右侧表单区 */}
      <div style={{ width: 500, display: 'flex', alignItems: 'center', justifyContent: 'center', padding: 40, background: 'linear-gradient(180deg, #131633 0%, #0B0F19 100%)', borderLeft: '1px solid rgba(108,92,231,0.1)' }}>
        <motion.div key={tab + method} initial={{ opacity: 0, y: 16 }} animate={{ opacity: 1, y: 0 }} transition={{ duration: 0.3 }} style={{ width: '100%', maxWidth: 400 }}>
          <div style={{ marginBottom: 32 }}>
            <h3 style={{ fontSize: 24, fontWeight: 700, color: '#fff', marginBottom: 4 }}>
              {tab === 'register' ? '创建账户' : '欢迎回来'}
            </h3>
            <p style={{ fontSize: 13, color: '#8892b0' }}>
              {tab === 'register' ? '开始保护您的 AI 交互' : '登录以继续'}
            </p>
          </div>

          {/* Tab 切换 */}
          <div style={{ display: 'flex', gap: 1, background: 'rgba(108,92,231,0.08)', borderRadius: 10, padding: 3, marginBottom: 8 }}>
            {(['login', 'register'] as AuthTab[]).map(t => (
              <button key={t} onClick={() => { setTab(t); setError(''); }}
                style={{ flex: 1, padding: '9px 0', border: 'none', borderRadius: 8, fontSize: 13, fontWeight: 600, cursor: 'pointer', background: tab === t ? '#6c5ce7' : 'transparent', color: tab === t ? '#fff' : '#8892b0', transition: 'all 0.2s' }}>
                {t === 'login' ? '登录' : '注册'}
              </button>
            ))}
          </div>

          {/* Method 切换 */}
          {tab === 'register' && (
            <div style={{ display: 'flex', gap: 8, marginBottom: 20 }}>
              {([
                { id: 'email' as Method, icon: <Mail size={14} />, label: '邮箱' },
                { id: 'phone' as Method, icon: <Smartphone size={14} />, label: '手机号' },
              ]).map(m => (
                <button key={m.id} onClick={() => setMethod(m.id)}
                  style={{ flex: 1, display: 'flex', alignItems: 'center', justifyContent: 'center', gap: 6, padding: '7px', borderRadius: 8, border: '1px solid ' + (method === m.id ? '#6c5ce7' : 'rgba(255,255,255,0.08)'), background: method === m.id ? 'rgba(108,92,231,0.1)' : 'transparent', color: method === m.id ? '#6c5ce7' : '#8892b0', fontSize: 12, fontWeight: 500, cursor: 'pointer' }}>
                  {m.icon} {m.label}
                </button>
              ))}
            </div>
          )}

          {/* Error */}
          {error && (
            <div style={{ padding: '10px 14px', borderRadius: 10, marginBottom: 16, background: 'rgba(255,118,117,0.1)', border: '1px solid rgba(255,118,117,0.3)', display: 'flex', alignItems: 'center', gap: 8, fontSize: 13, color: '#ff7675' }}>
              <AlertTriangle size={16} /> {error}
            </div>
          )}

          <form onSubmit={handleSubmit} style={{ display: 'flex', flexDirection: 'column', gap: 14 }}>
            {/* Email method */}
            {method === 'email' && (
              <>
                {tab === 'register' && (
                  <div style={{ position: 'relative' }}>
                    <User size={15} style={iconPos} />
                    <input type="text" placeholder="显示名称" value={displayName} onChange={e => setDisplayName(e.target.value)} style={inpStyle} />
                  </div>
                )}
                <div style={{ position: 'relative' }}>
                  <Mail size={15} style={iconPos} />
                  <input type="email" placeholder="邮箱地址" value={email} onChange={e => setEmail(e.target.value)} style={inpStyle} />
                </div>
                <div style={{ position: 'relative' }}>
                  <Lock size={15} style={iconPos} />
                  <input type={showPassword ? 'text' : 'password'} placeholder="密码" value={password} onChange={e => setPassword(e.target.value)} style={inpStyle} />
                  <button type="button" onClick={() => setShowPassword(!showPassword)} style={{ position: 'absolute', right: 12, top: '50%', transform: 'translateY(-50%)', background: 'none', border: 'none', color: '#8892b0', cursor: 'pointer' }}>{showPassword ? <EyeOff size={15} /> : <Eye size={15} />}</button>
                </div>
                {tab === 'register' && password && (
                  <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginTop: -8 }}>
                    <div style={{ flex: 1, height: 3, borderRadius: 2, background: 'rgba(255,255,255,0.06)', overflow: 'hidden' }}>
                      <motion.div initial={{ width: 0 }} animate={{ width: `${(pwdSt.score / 5) * 100}%` }} style={{ height: '100%', borderRadius: 2, background: pwdSt.color }} />
                    </div>
                    <span style={{ fontSize: 11, color: pwdSt.color, fontWeight: 600, minWidth: 60 }}>{pwdSt.label}</span>
                  </div>
                )}
              </>
            )}

            {/* Phone method */}
            {method === 'phone' && (
              <>
                <div style={{ position: 'relative' }}>
                  <Smartphone size={15} style={iconPos} />
                  <input type="tel" placeholder="手机号 (如 +8613800138000)" value={phone} onChange={e => setPhone(e.target.value)} style={inpStyle} />
                </div>
                {!phoneVerified ? (
                  <div style={{ display: 'flex', gap: 10 }}>
                    {phoneSent ? (
                      <>
                        <div style={{position:'relative',flex:1}}><input type="text" placeholder="验证码" value={phoneCode} onChange={e=>setPhoneCode(e.target.value)} style={{...inpStyle,padding:'12px 14px'}}/></div>
                        <button type="button" onClick={handleVerifyCode} disabled={loading} style={{...btnStyle,background:'#00d4aa',border:'none',padding:'0 20px',fontSize:13,fontWeight:600,color:'#000',cursor:'pointer',borderRadius:10}}>验证</button>
                      </>
                    ) : (
                      <button type="button" onClick={handleSendCode} disabled={loading || phoneCountdown > 0} style={{...btnStyle,background:phoneCountdown>0?'rgba(108,92,231,0.1)':'#6c5ce7',border:'none',padding:'12px 20px',fontSize:13,fontWeight:600,color:'#fff',cursor:phoneCountdown>0?'default':'pointer',borderRadius:10,width:'100%'}}>
                        {phoneCountdown > 0 ? `重新发送 (${phoneCountdown}s)` : '发送验证码'}
                      </button>
                    )}
                  </div>
                ) : (
                  <div style={{ display:'flex',alignItems:'center',gap:6,padding:'8px 12px',borderRadius:8,background:'rgba(0,212,170,0.1)',border:'1px solid rgba(0,212,170,0.2)',fontSize:13,color:'#00d4aa'}}>
                    <Check size={14} /> 手机号已验证: {phone}
                  </div>
                )}
              </>
            )}

            {/* Plan 选择 */}
            {tab === 'register' && (
              <div style={{ display: 'flex', gap: 8, marginTop: 4 }}>
                {([
                  { id: 'personal' as const, icon: <UserIcon size={14} />, label: '个人版', desc: '免费', features: 'L0 审计 + 1个工作空间' },
                  { id: 'enterprise' as const, icon: <Building size={14} />, label: '企业版', desc: '联系我们', features: 'SSO + 合规报告 + 白标' },
                ]).map(p => (
                  <button key={p.id} type="button" onClick={() => setPlan(p.id)}
                    style={{ flex: 1, padding: '12px', borderRadius: 12, border: '2px solid ' + (plan === p.id ? '#6c5ce7' : 'rgba(255,255,255,0.06)'), background: plan === p.id ? 'rgba(108,92,231,0.1)' : 'rgba(255,255,255,0.02)', cursor: 'pointer', textAlign: 'left', transition: 'all 0.2s' }}>
                    <div style={{ display: 'flex', alignItems: 'center', gap: 6, color: plan === p.id ? '#6c5ce7' : '#8892b0', fontSize: 13, fontWeight: 600, marginBottom: 4 }}>{p.icon} {p.label}</div>
                    <div style={{ fontSize: 11, color: '#8892b0' }}>{p.desc}</div>
                    <div style={{ fontSize: 10, color: '#5a6a8a', marginTop: 2 }}>{p.features}</div>
                  </button>
                ))}
              </div>
            )}

            {tab === 'register' && plan === 'enterprise' && (
              <div style={{ position: 'relative' }}>
                <Building size={15} style={iconPos} />
                <input type="text" placeholder="企业/组织名称" value={orgName} onChange={e => setOrgName(e.target.value)} style={inpStyle} />
              </div>
            )}

            <motion.button type="submit" disabled={loading}
              whileHover={{ scale: 1.01 }} whileTap={{ scale: 0.98 }}
              style={{
                padding: '13px', borderRadius: 10, border: 'none', cursor: loading ? 'not-allowed' : 'pointer',
                background: 'linear-gradient(135deg, #6c5ce7, #00d4aa)', color: '#000', fontSize: 15, fontWeight: 700, marginTop: 8, opacity: loading ? 0.7 : 1,
              }}>
              {loading ? '处理中...' : tab === 'register' ? '创建账户' : '登录'}
              {!loading && <ArrowRight size={16} style={{ verticalAlign: -3, marginLeft: 4 }} />}
            </motion.button>
          </form>

          {/* WeChat 扫码登录 + GitHub */}
          <div style={{ display: 'flex', alignItems: 'center', gap: 10, margin: '20px 0' }}>
            <div style={{ flex: 1, height: 1, background: 'rgba(255,255,255,0.06)' }} />
            <span style={{ fontSize: 11, color: '#5a6a8a' }}>快捷方式</span>
            <div style={{ flex: 1, height: 1, background: 'rgba(255,255,255,0.06)' }} />
          </div>

          <div style={{ display: 'flex', gap: 10 }}>
            {/* WeChat — 第一优先级 */}
            <motion.button
              whileHover={{ scale: 1.02 }} whileTap={{ scale: 0.98 }}
              onClick={() => setShowWeChat(true)}
              disabled={loading}
              style={{
                flex: 1, display: 'flex', alignItems: 'center', justifyContent: 'center', gap: 8,
                padding: '11px', borderRadius: 10, border: '1px solid rgba(7,193,96,0.3)',
                background: 'linear-gradient(135deg, rgba(7,193,96,0.15), rgba(7,193,96,0.05))',
                color: '#fff', fontSize: 13, fontWeight: 600, cursor: 'pointer',
              }}
            >
              <span style={{ fontSize: 18 }}>💬</span> 微信登录
            </motion.button>

            {/* GitHub */}
            {githubAvailable && (
              <motion.a href={githubUrl} whileHover={{ scale: 1.02 }} whileTap={{ scale: 0.98 }}
                style={{
                  flex: 1, display: 'flex', alignItems: 'center', justifyContent: 'center', gap: 8,
                  padding: '11px', borderRadius: 10, background: '#24292e',
                  border: '1px solid rgba(255,255,255,0.08)', color: '#fff',
                  fontSize: 13, fontWeight: 600, textDecoration: 'none',
                }}>
                <Github size={16} /> GitHub
              </motion.a>
            )}

            {/* Google */}
            {googleAvailable && (
              <motion.a href={googleUrl} whileHover={{ scale: 1.02 }} whileTap={{ scale: 0.98 }}
                style={{
                  flex: 1, display: 'flex', alignItems: 'center', justifyContent: 'center', gap: 8,
                  padding: '11px', borderRadius: 10, background: '#fff',
                  border: '1px solid rgba(0,0,0,0.1)', color: '#333',
                  fontSize: 13, fontWeight: 600, textDecoration: 'none',
                }}>
                <span style={{ fontSize: 15, color: '#4285F4', fontWeight: 700 }}>G</span> Google
              </motion.a>
            )}
          </div>

          <p style={{ textAlign: 'center', fontSize: 10, color: '#5a6a8a', marginTop: 12, lineHeight: 1.6 }}>
            微信扫码即可登录，首次登录自动创建账户<br />
            登录后可绑定手机号，开启完整功能
          </p>

          <p style={{ textAlign: 'center', fontSize: 10, color: '#5a6a8a', marginTop: 20 }}>
            VERIDACTUS v0.3.0 · 注册即表示同意《服务条款》和《隐私政策》
          </p>
        </motion.div>
      </div>

      {/* 微信扫码 Modal */}
      <WeChatQRModal
        isOpen={showWeChat}
        onClose={() => setShowWeChat(false)}
        onSuccess={(needBindPhone) => {
          if (needBindPhone) {
            navigate('/bind-phone', { replace: true });
          } else {
            navigate(from, { replace: true });
          }
        }}
      />
    </div>
  );
}

const iconPos: React.CSSProperties = { position: 'absolute', left: 14, top: '50%', transform: 'translateY(-50%)', color: '#8892b0', zIndex: 1 };

const inpStyle: React.CSSProperties = {
  width: '100%', padding: '12px 14px 12px 40px', borderRadius: 10,
  background: 'rgba(255,255,255,0.04)', border: '1px solid rgba(255,255,255,0.08)',
  color: '#e2e8f0', fontSize: 14, outline: 'none', boxSizing: 'border-box',
};

const btnStyle: React.CSSProperties = { border: 'none', cursor: 'pointer', fontWeight: 600, borderRadius: 10 };
