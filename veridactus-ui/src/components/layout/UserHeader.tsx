// 顶部用户栏 — 头像、名称、Plan、退出
import { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { motion, AnimatePresence } from 'framer-motion';
import { LogOut, User, ChevronDown, Settings, Shield, Building } from 'lucide-react';
import { getToken, clearToken, getUserPlan } from '../../auth/AuthGuard';

interface UserInfo {
  id: string;
  email: string;
  display_name: string;
  plan: string;
}

function parseUser(): UserInfo | null {
  try {
    const raw = localStorage.getItem('veridactus_user');
    if (!raw) return null;
    const u = JSON.parse(raw);
    const plan = getUserPlan();
    return { id: u.id, email: u.email, display_name: u.display_name || u.email, plan: plan || u.plan || 'personal' };
  } catch { return null; }
}

export default function UserHeader() {
  const navigate = useNavigate();
  const [open, setOpen] = useState(false);
  const user = parseUser();
  const token = getToken();

  if (!token || !user) return null;

  const initial = (user.display_name || user.email)[0].toUpperCase();
  const isEnterprise = user.plan === 'enterprise';

  const handleLogout = () => {
    clearToken();
    navigate('/login', { replace: true });
  };

  return (
    <div style={{
      position: 'fixed', top: 0, right: 0, zIndex: 100,
      padding: '8px 20px',
    }}>
      <div style={{ position: 'relative' }}>
        <button
          onClick={() => setOpen(!open)}
          style={{
            display: 'flex', alignItems: 'center', gap: 10, padding: '6px 14px 6px 8px',
            borderRadius: 24, border: '1px solid rgba(108,92,231,0.2)',
            background: 'rgba(19,22,51,0.9)', backdropFilter: 'blur(12px)',
            cursor: 'pointer', transition: 'all 0.15s',
          }}
        >
          {/* Avatar */}
          <div style={{
            width: 32, height: 32, borderRadius: '50%',
            background: isEnterprise
              ? 'linear-gradient(135deg, #6c5ce7, #00d4aa)'
              : 'linear-gradient(135deg, #6c5ce7, #a29bfe)',
            display: 'flex', alignItems: 'center', justifyContent: 'center',
            color: '#fff', fontSize: 14, fontWeight: 700,
          }}>
            {initial}
          </div>

          {/* Name + Plan */}
          <div style={{ textAlign: 'left', lineHeight: 1.2 }}>
            <div style={{ fontSize: 12, fontWeight: 600, color: '#e0e6f0', maxWidth: 120, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
              {user.display_name}
            </div>
            <div style={{
              fontSize: 10, fontWeight: 600,
              color: isEnterprise ? '#00d4aa' : '#8892b0',
              display: 'flex', alignItems: 'center', gap: 3,
            }}>
              {isEnterprise ? <Building size={10} /> : <User size={10} />}
              {isEnterprise ? 'Enterprise' : 'Personal'}
            </div>
          </div>

          <ChevronDown size={14} color="#8892b0"
            style={{ transform: open ? 'rotate(180deg)' : '', transition: 'transform 0.2s' }} />
        </button>

        {/* Dropdown */}
        <AnimatePresence>
          {open && (
            <motion.div
              initial={{ opacity: 0, y: -8, scale: 0.95 }}
              animate={{ opacity: 1, y: 0, scale: 1 }}
              exit={{ opacity: 0, y: -8, scale: 0.95 }}
              transition={{ duration: 0.15 }}
              style={{
                position: 'absolute', top: '100%', right: 0, marginTop: 6,
                background: 'rgba(19,22,51,0.98)', borderRadius: 14,
                border: '1px solid rgba(108,92,231,0.2)', minWidth: 200,
                boxShadow: '0 16px 48px rgba(0,0,0,0.5)',
                overflow: 'hidden', backdropFilter: 'blur(16px)',
              }}
            >
              {/* User info */}
              <div style={{ padding: '12px 16px', borderBottom: '1px solid rgba(255,255,255,0.06)' }}>
                <div style={{ fontSize: 13, fontWeight: 600, color: '#e0e6f0' }}>{user.display_name}</div>
                <div style={{ fontSize: 11, color: '#8892b0', marginTop: 2 }}>{user.email}</div>
              </div>

              {/* Menu items */}
              <div style={{ padding: '4px 0' }}>
                <MenuItem icon={<User size={14} />} label="个人中心" onClick={() => { setOpen(false); navigate('/settings'); }} />
                <MenuItem icon={<Settings size={14} />} label="设置" onClick={() => { setOpen(false); navigate('/settings'); }} />
                {isEnterprise && (
                  <MenuItem icon={<Shield size={14} />} label="企业管控" onClick={() => { setOpen(false); navigate('/audit-center'); }} />
                )}
                <div style={{ height: 1, background: 'rgba(255,255,255,0.06)', margin: '4px 0' }} />
                <MenuItem
                  icon={<LogOut size={14} />} label="退出登录" onClick={handleLogout}
                  style={{ color: '#ff7675' }}
                />
              </div>
            </motion.div>
          )}
        </AnimatePresence>
      </div>

      {/* Click outside to close */}
      {open && <div onClick={() => setOpen(false)} style={{ position: 'fixed', inset: 0, zIndex: -1 }} />}
    </div>
  );
}

function MenuItem({ icon, label, onClick, style }: {
  icon: React.ReactNode; label: string; onClick: () => void; style?: React.CSSProperties;
}) {
  return (
    <button onClick={onClick}
      style={{
        width: '100%', display: 'flex', alignItems: 'center', gap: 10,
        padding: '9px 16px', border: 'none', background: 'transparent',
        color: style?.color || '#e0e6f0', fontSize: 13, cursor: 'pointer',
        transition: 'all 0.1s', textAlign: 'left',
      }}
      onMouseEnter={e => { (e.target as HTMLElement).style.background = 'rgba(108,92,231,0.1)'; }}
      onMouseLeave={e => { (e.target as HTMLElement).style.background = 'transparent'; }}
    >
      {icon}
      <span>{label}</span>
    </button>
  );
}
