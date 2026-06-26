/// <reference types="vite/client" />
// VERIDACTUS 登录/注册 — 生产级多通道认证（Tailwind + 响应式）
import { useState, useEffect, useRef } from 'react';
import { useNavigate, useLocation } from 'react-router-dom';
import { motion } from 'framer-motion';
import { Shield, Github, Mail, Lock, User, Eye, EyeOff, AlertTriangle, Check, Phone, Smartphone, Building, User as UserIcon, ArrowRight, Sparkles } from 'lucide-react';
import { setToken, isAuthenticated } from './AuthGuard';
import WeChatQRModal from './WeChatQRModal';

const API = (import.meta as any)?.env?.VITE_API_URL || '';
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
    const wxToken = params.get('wechat_token');
    if (wxToken) { setToken(wxToken); window.history.replaceState({}, '', '/login'); navigate(from, { replace: true }); return; }
    const wxCode = params.get('code');
    if (wxCode) { handleWeChatCallback(wxCode); return; }
    fetch(`${API}/api/v1/auth/login/github`).then(r=>r.json()).then(d=>{if(d.auth_url&&!d.error){setGithubAvailable(true);setGithubUrl(d.auth_url)}}).catch(()=>{});
    fetch(`${API}/api/v1/auth/login/google`).then(r=>r.json()).then(d=>{if(d.auth_url&&!d.error){setGoogleAvailable(true);setGoogleUrl(d.auth_url)}}).catch(()=>{});
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
        navigate(data.need_bind_phone ? '/bind-phone' : from, { replace: true });
      }
    } catch { setError('微信服务连接失败'); }
    finally { setLoading(false); }
  };

  const passwordStrength = (pw: string) => {
    if (!pw) return { score: 0, label: '', color: '#8892b0' };
    let s = 0;
    if (pw.length >= 8) s++; if (/[A-Z]/.test(pw)) s++; if (/[a-z]/.test(pw)) s++;
    if (/[0-9]/.test(pw)) s++; if (/[!@#$%^&*()_+\-=\[\]{};:,.<>?~]/.test(pw)) s++;
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
      const res = await fetch(`${API}/api/v1/auth/phone/send`, { method:'POST', headers:{'Content-Type':'application/json'}, body: JSON.stringify({phone}) });
      if (res.ok) { setPhoneSent(true); setPhoneCountdown(60); const t=setInterval(()=>setPhoneCountdown(c=>{if(c<=1){clearInterval(t);return 0}return c-1}),1000); }
      else { const d=await res.json(); setError(d.message||'短信发送失败'); }
    } catch { setError('服务连接失败'); }
    finally { setLoading(false); }
  };

  const handleVerifyCode = async () => {
    if (!phone || !phoneCode) return;
    setLoading(true);
    try {
      const res = await fetch(`${API}/api/v1/auth/phone/verify`, { method:'POST', headers:{'Content-Type':'application/json'}, body:JSON.stringify({phone,code:phoneCode}) });
      if (res.ok) setPhoneVerified(true); else setError('验证码错误');
    } catch { setError('验证失败'); }
    finally { setLoading(false); }
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault(); setError('');
    if (method==='email') {
      if (!email||!password) { setError('请填写邮箱和密码'); return; }
      if (tab==='register' && !displayName) { setError('请填写显示名称'); return; }
      if (tab==='register' && pwdSt.score<3) { setError('密码强度不够'); return; }
    } else { if (!phone||!phoneVerified) { setError('请先验证手机号'); return; } }
    setLoading(true);
    try {
      let res: Response;
      if (method==='email' && tab==='register') res = await fetch(`${API}/api/v1/auth/register`, { method:'POST', headers:{'Content-Type':'application/json'}, body:JSON.stringify({email,password,display_name:displayName,plan,org_name:plan==='enterprise'?orgName:undefined}) });
      else if (method==='email' && tab==='login') res = await fetch(`${API}/api/v1/auth/login`, { method:'POST', headers:{'Content-Type':'application/json'}, body:JSON.stringify({email,password}) });
      else if (method==='phone') res = await fetch(`${API}/api/v1/auth/phone/verify`, { method:'POST', headers:{'Content-Type':'application/json'}, body:JSON.stringify({phone,code:phoneCode}) });
      else { setError('请选择登录方式'); setLoading(false); return; }
      const data = await res.json();
      if (!res.ok) { setError(data.error||data.message||'操作失败'); setLoading(false); return; }
      if (data.access_token) { setToken(data.access_token); localStorage.setItem('veridactus_user',JSON.stringify(data.user||{})); navigate(from,{replace:true}); }
    } catch { setError('无法连接服务器'); setLoading(false); }
  };

  return (
    <div className="min-h-screen flex font-sans bg-[#0B0F19] flex-col lg:flex-row">
      {/* 左侧品牌区 — 移动端隐藏 */}
      <div className="hidden lg:flex flex-1 flex-col justify-center px-20 relative overflow-hidden"
        style={{ background: 'radial-gradient(ellipse at 30% 50%, rgba(108,92,231,0.12) 0%, transparent 70%)' }}>
        <div className="absolute -top-[200px] -left-[200px] w-[600px] h-[600px] rounded-full blur-[60px]"
          style={{ background: 'radial-gradient(circle, rgba(0,212,170,0.08) 0%, transparent 70%)' }} />
        <motion.div initial={{ opacity: 0, x: -40 }} animate={{ opacity: 1, x: 0 }} transition={{ duration: 0.8 }}>
          <div className="flex items-center gap-3 mb-6">
            <motion.div animate={{ scale: [1, 1.08, 1] }} transition={{ duration: 3, repeat: Infinity }}>
              <Shield size={52} color="#6c5ce7" style={{ filter: 'drop-shadow(0 0 24px rgba(108,92,231,0.5))' }} />
            </motion.div>
            <h1 className="text-[42px] font-black bg-gradient-to-r from-[#6c5ce7] to-[#00d4aa] bg-clip-text text-transparent">VERIDACTUS</h1>
          </div>
          <h2 className="text-[22px] text-[#e0e6f0] font-semibold mb-3 max-w-[480px]">AI 治理的可信基础设施</h2>
          <p className="text-[15px] text-[#8892b0] leading-relaxed max-w-[460px] mb-10">
            每条 AI 交互都经过密码学签名审计，不可篡改、可独立验证。L0 → L2A → L2B 三层证明链，符合 EU AI Act 和 GDPR 合规要求。
          </p>
          <div className="flex flex-col gap-3">
            {[
              { icon: <Shield size={16} color="#00d4aa" />, text: 'L0 密码学签名审计' },
              { icon: <Sparkles size={16} color="#6c5ce7" />, text: 'OWASP ASI Top 10 全覆盖' },
              { icon: <Building size={16} color="#74b9ff" />, text: '企业版 SSO + 审计报告' },
            ].map((f, i) => (
              <div key={i} className="flex items-center gap-2.5 text-sm text-[#a0aec0]">{f.icon} {f.text}</div>
            ))}
          </div>
        </motion.div>
      </div>

      {/* 右侧表单区 — 移动端全宽 */}
      <div className="w-full lg:w-[500px] flex items-center justify-center p-8 lg:p-10 flex-shrink-0"
        style={{ background: 'linear-gradient(180deg, #131633 0%, #0B0F19 100%)', borderLeft: '1px solid rgba(108,92,231,0.1)' }}>
        <motion.div key={tab+method} initial={{opacity:0,y:16}} animate={{opacity:1,y:0}} transition={{duration:0.3}} className="w-full max-w-[400px]">
          <div className="mb-8">
            <h3 className="text-2xl font-bold text-white mb-1">{tab==='register'?'创建账户':'欢迎回来'}</h3>
            <p className="text-sm text-[#8892b0]">{tab==='register'?'开始保护您的 AI 交互':'登录以继续'}</p>
          </div>

          {/* Tab 切换 */}
          <div className="flex gap-px bg-[rgba(108,92,231,0.08)] rounded-btn p-[3px] mb-2">
            {(['login','register'] as AuthTab[]).map(t => (
              <button key={t} onClick={()=>{setTab(t);setError('')}}
                className={`flex-1 py-2.5 border-none rounded-lg text-sm font-semibold cursor-pointer transition-all ${tab===t?'bg-[#6c5ce7] text-white':'bg-transparent text-[#8892b0]'}`}>
                {t==='login'?'登录':'注册'}
              </button>
            ))}
          </div>

          {/* Method 切换 */}
          {tab==='register' && (
            <div className="flex gap-2 mb-5">
              {([{id:'email' as Method, icon:<Mail size={14}/>, label:'邮箱'},{id:'phone' as Method, icon:<Smartphone size={14}/>, label:'手机号'}]).map(m=>(
                <button key={m.id} onClick={()=>setMethod(m.id)}
                  className={`flex-1 flex items-center justify-center gap-1.5 py-1.5 rounded-lg border text-xs font-medium cursor-pointer transition-all ${method===m.id?'border-[#6c5ce7] bg-[rgba(108,92,231,0.1)] text-[#6c5ce7]':'border-white/10 bg-transparent text-[#8892b0]'}`}>
                  {m.icon}{m.label}
                </button>
              ))}
            </div>
          )}

          {/* Error */}
          {error && (
            <div className="p-2.5 rounded-btn mb-4 bg-[rgba(255,118,117,0.1)] border border-[rgba(255,118,117,0.3)] flex items-center gap-2 text-sm text-[#ff7675]">
              <AlertTriangle size={16}/> {error}
            </div>
          )}

          <form onSubmit={handleSubmit} className="flex flex-col gap-3.5">
            {method==='email' && (<>
              {tab==='register' && (
                <div className="relative"><User size={15} className="absolute left-3.5 top-1/2 -translate-y-1/2 text-[#8892b0] z-10"/>
                  <input type="text" placeholder="显示名称" value={displayName} onChange={e=>setDisplayName(e.target.value)} className="input-field pl-10"/>
                </div>
              )}
              <div className="relative"><Mail size={15} className="absolute left-3.5 top-1/2 -translate-y-1/2 text-[#8892b0] z-10"/>
                <input type="email" placeholder="邮箱地址" value={email} onChange={e=>setEmail(e.target.value)} className="input-field pl-10"/>
              </div>
              <div className="relative"><Lock size={15} className="absolute left-3.5 top-1/2 -translate-y-1/2 text-[#8892b0] z-10"/>
                <input type={showPassword?'text':'password'} placeholder="密码" value={password} onChange={e=>setPassword(e.target.value)} className="input-field pl-10"/>
                <button type="button" onClick={()=>setShowPassword(!showPassword)} className="absolute right-3 top-1/2 -translate-y-1/2 bg-transparent border-none text-[#8892b0] cursor-pointer">{showPassword?<EyeOff size={15}/>:<Eye size={15}/>}</button>
              </div>
              {tab==='register' && password && (
                <div className="flex items-center gap-2 -mt-2">
                  <div className="flex-1 h-[3px] rounded-sm bg-white/[0.06] overflow-hidden">
                    <motion.div initial={{width:0}} animate={{width:`${(pwdSt.score/5)*100}%`}} className="h-full rounded-sm" style={{background:pwdSt.color}}/>
                  </div>
                  <span className="text-xs font-semibold min-w-[60px]" style={{color:pwdSt.color}}>{pwdSt.label}</span>
                </div>
              )}
            </>)}

            {method==='phone' && (<>
              <div className="relative"><Smartphone size={15} className="absolute left-3.5 top-1/2 -translate-y-1/2 text-[#8892b0] z-10"/>
                <input type="tel" placeholder="手机号 (如 +8613800138000)" value={phone} onChange={e=>setPhone(e.target.value)} className="input-field pl-10"/>
              </div>
              {!phoneVerified?(
                <div className="flex gap-2.5">
                  {phoneSent?(<>
                    <div className="relative flex-1"><input type="text" placeholder="验证码" value={phoneCode} onChange={e=>setPhoneCode(e.target.value)} className="input-field"/></div>
                    <button type="button" onClick={handleVerifyCode} disabled={loading} className="bg-[#00d4aa] border-none px-5 text-sm font-semibold text-black cursor-pointer rounded-btn disabled:opacity-50">验证</button>
                  </>):(
                    <button type="button" onClick={handleSendCode} disabled={loading||phoneCountdown>0}
                      className={`py-3 px-5 text-sm font-semibold text-white cursor-pointer rounded-btn w-full border-none ${phoneCountdown>0?'bg-[rgba(108,92,231,0.1)] cursor-default':'bg-[#6c5ce7]'}`}>
                      {phoneCountdown>0?`重新发送 (${phoneCountdown}s)`:'发送验证码'}
                    </button>
                  )}
                </div>
              ):(
                <div className="flex items-center gap-1.5 p-2 rounded-lg bg-[rgba(0,212,170,0.1)] border border-[rgba(0,212,170,0.2)] text-sm text-[#00d4aa]">
                  <Check size={14}/> 手机号已验证: {phone}
                </div>
              )}
            </>)}

            {tab==='register' && (
              <div className="flex gap-2 mt-1">
                {([{id:'personal' as const,icon:<UserIcon size={14}/>,label:'个人版',desc:'免费'},{id:'enterprise' as const,icon:<Building size={14}/>,label:'企业版',desc:'联系我们'}]).map(p=>(
                  <button key={p.id} type="button" onClick={()=>setPlan(p.id)}
                    className={`flex-1 p-3 rounded-xl border-2 cursor-pointer text-left transition-all ${plan===p.id?'border-[#6c5ce7] bg-[rgba(108,92,231,0.1)]':'border-white/[0.06] bg-white/[0.02]'}`}>
                    <div className={`flex items-center gap-1.5 text-sm font-semibold mb-1 ${plan===p.id?'text-[#6c5ce7]':'text-[#8892b0]'}`}>{p.icon}{p.label}</div>
                    <div className="text-[11px] text-[#8892b0]">{p.desc}</div>
                  </button>
                ))}
              </div>
            )}
            {tab==='register' && plan==='enterprise' && (
              <div className="relative"><Building size={15} className="absolute left-3.5 top-1/2 -translate-y-1/2 text-[#8892b0] z-10"/>
                <input type="text" placeholder="企业/组织名称" value={orgName} onChange={e=>setOrgName(e.target.value)} className="input-field pl-10"/>
              </div>
            )}

            <motion.button type="submit" disabled={loading} whileHover={{scale:1.01}} whileTap={{scale:0.98}}
              className="py-3.5 rounded-btn border-none bg-gradient-to-r from-[#6c5ce7] to-[#00d4aa] text-black text-[15px] font-bold mt-2 disabled:opacity-70 disabled:cursor-not-allowed cursor-pointer">
              {loading?'处理中...':tab==='register'?'创建账户':'登录'}
              {!loading && <ArrowRight size={16} className="inline align-middle ml-1"/>}
            </motion.button>
          </form>

          {/* OAuth 快捷方式 */}
          <div className="flex items-center gap-2.5 my-5">
            <div className="flex-1 h-px bg-white/[0.06]"/>
            <span className="text-[11px] text-[#5a6a8a]">快捷方式</span>
            <div className="flex-1 h-px bg-white/[0.06]"/>
          </div>

          <div className="flex gap-2.5 flex-wrap">
            <motion.button whileHover={{scale:1.02}} whileTap={{scale:0.98}} onClick={()=>setShowWeChat(true)} disabled={loading}
              className="flex-1 flex items-center justify-center gap-2 py-[11px] rounded-btn border border-[rgba(7,193,96,0.3)] bg-gradient-to-r from-[rgba(7,193,96,0.15)] to-[rgba(7,193,96,0.05)] text-white text-sm font-semibold cursor-pointer">
              <span className="text-lg">💬</span>微信登录
            </motion.button>
            {githubAvailable && (
              <motion.a href={githubUrl} whileHover={{scale:1.02}} whileTap={{scale:0.98}}
                className="flex-1 flex items-center justify-center gap-2 py-[11px] rounded-btn bg-[#24292e] border border-white/10 text-white text-sm font-semibold no-underline">
                <Github size={16}/> GitHub
              </motion.a>
            )}
            {googleAvailable && (
              <motion.a href={googleUrl} whileHover={{scale:1.02}} whileTap={{scale:0.98}}
                className="flex-1 flex items-center justify-center gap-2 py-[11px] rounded-btn bg-white border border-black/10 text-[#333] text-sm font-semibold no-underline">
                <span className="text-[15px] text-[#4285F4] font-bold">G</span> Google
              </motion.a>
            )}
          </div>

          <p className="text-center text-[10px] text-[#5a6a8a] mt-3 leading-relaxed">微信扫码即可登录，首次登录自动创建账户<br/>登录后可绑定手机号，开启完整功能</p>
          <p className="text-center text-[10px] text-[#5a6a8a] mt-5">VERIDACTUS v0.3.0 · 注册即表示同意《服务条款》和《隐私政策》</p>
        </motion.div>
      </div>

      <WeChatQRModal isOpen={showWeChat} onClose={()=>setShowWeChat(false)}
        onSuccess={(needBindPhone)=>{ navigate(needBindPhone?'/bind-phone':from,{replace:true}); }}/>
    </div>
  );
}
