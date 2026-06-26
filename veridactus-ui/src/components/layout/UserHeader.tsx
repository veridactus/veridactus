// 顶部用户栏 — 头像、名称、Plan、退出
import { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { motion, AnimatePresence } from 'framer-motion';
import { LogOut, User, ChevronDown, Settings, Shield, Building } from 'lucide-react';
import { getToken, clearToken, getUserPlan } from '../../auth/AuthGuard';

interface UserInfo { id: string; email: string; display_name: string; plan: string; }

function parseUser(): UserInfo | null {
  try { const raw = localStorage.getItem('veridactus_user'); if (!raw) return null; const u = JSON.parse(raw); const plan = getUserPlan(); return { id: u.id, email: u.email, display_name: u.display_name || u.email, plan: plan || u.plan || 'personal' }; } catch { return null; }
}

export default function UserHeader() {
  const navigate = useNavigate(); const [open, setOpen] = useState(false);
  const user = parseUser(); const token = getToken();
  if (!token || !user) return null;
  const initial = (user.display_name || user.email)[0].toUpperCase();
  const isEnterprise = user.plan === 'enterprise';

  return (
    <div className="fixed top-0 right-0 z-[100] py-2 px-5">
      <div className="relative">
        <button onClick={() => setOpen(!open)}
          className="flex items-center gap-2.5 py-1.5 pl-2 pr-3.5 rounded-3xl border cursor-pointer transition-all"
          style={{ borderColor: 'rgba(108,92,231,0.2)', background: 'rgba(19,22,51,0.9)', backdropFilter: 'blur(12px)' }}>
          <div className="w-8 h-8 rounded-full flex items-center justify-center text-white text-sm font-bold"
            style={{ background: isEnterprise ? 'linear-gradient(135deg, #6c5ce7, #00d4aa)' : 'linear-gradient(135deg, #6c5ce7, #a29bfe)' }}>{initial}</div>
          <div className="text-left leading-tight">
            <div className="text-xs font-semibold text-[#e0e6f0] max-w-[120px] truncate">{user.display_name}</div>
            <div className="text-[10px] font-semibold flex items-center gap-0.5" style={{ color: isEnterprise ? '#00d4aa' : '#8892b0' }}>
              {isEnterprise ? <Building size={10} /> : <User size={10} />}{isEnterprise ? 'Enterprise' : 'Personal'}
            </div>
          </div>
          <ChevronDown size={14} color="#8892b0" style={{ transform: open ? 'rotate(180deg)' : '', transition: 'transform 0.2s' }} />
        </button>

        <AnimatePresence>
          {open && (
            <motion.div initial={{ opacity: 0, y: -8, scale: 0.95 }} animate={{ opacity: 1, y: 0, scale: 1 }} exit={{ opacity: 0, y: -8, scale: 0.95 }} transition={{ duration: 0.15 }}
              className="absolute top-full right-0 mt-1.5 rounded-2xl border min-w-[200px] overflow-hidden" style={{ background: 'rgba(19,22,51,0.98)', borderColor: 'rgba(108,92,231,0.2)', boxShadow: '0 16px 48px rgba(0,0,0,0.5)', backdropFilter: 'blur(16px)' }}>
              <div className="py-3 px-4 border-b border-[rgba(255,255,255,0.06)]">
                <div className="text-[13px] font-semibold text-[#e0e6f0]">{user.display_name}</div>
                <div className="text-[11px] text-[#8892b0] mt-0.5">{user.email}</div>
              </div>
              <div className="py-1">
                <MenuItem icon={<User size={14} />} label="个人中心" onClick={() => { setOpen(false); navigate('/settings'); }} />
                <MenuItem icon={<Settings size={14} />} label="设置" onClick={() => { setOpen(false); navigate('/settings'); }} />
                {isEnterprise && <MenuItem icon={<Shield size={14} />} label="企业管控" onClick={() => { setOpen(false); navigate('/audit-center'); }} />}
                <div className="h-px mx-0 my-1" style={{ background: 'rgba(255,255,255,0.06)' }} />
                <MenuItem icon={<LogOut size={14} />} label="退出登录" onClick={() => { clearToken(); navigate('/login', { replace: true }); }} style={{ color: '#ff7675' }} />
              </div>
            </motion.div>
          )}
        </AnimatePresence>
        {open && <div onClick={() => setOpen(false)} className="fixed inset-0 z-[-1]" />}
      </div>
    </div>
  );
}

function MenuItem({ icon, label, onClick, style }: { icon: React.ReactNode; label: string; onClick: () => void; style?: React.CSSProperties; }) {
  return (
    <button onClick={onClick}
      className="w-full flex items-center gap-2.5 py-2 px-4 border-none bg-transparent text-[13px] cursor-pointer text-left transition-colors hover:bg-[rgba(108,92,231,0.1)]"
      style={{ color: style?.color || '#e0e6f0' }}>
      {icon}<span>{label}</span>
    </button>
  );
}