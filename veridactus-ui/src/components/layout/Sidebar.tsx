import { useState } from 'react';
import { NavLink, useNavigate } from 'react-router-dom';
import { motion, AnimatePresence } from 'framer-motion';
import { LayoutDashboard, GitBranch, Activity, Key, Settings, Shield, Boxes, Globe, Sun, Moon, Cpu, MessageCircle, Database, Eye, Palette, Menu, X, User, Building, LogOut, ChevronRight } from 'lucide-react';
import { useI18n } from '../../i18n';
import { useThemeStore, useNavSidebarStore } from '../../store';
import { getUserPlan, getToken, clearToken } from '../../auth/AuthGuard';

function parseUser() {
  try { const raw = localStorage.getItem('veridactus_user'); if (!raw) return null; const u = JSON.parse(raw); const plan = getUserPlan(); return { id: u.id, email: u.email, display_name: u.display_name || u.email, plan: plan || u.plan || 'personal' }; } catch { return null; }
}

export default function Sidebar() {
  const { t, locale, setLocale } = useI18n();
  const { theme, toggleTheme } = useThemeStore();
  const { navCollapsed } = useNavSidebarStore();
  const navigate = useNavigate();
  const plan = getUserPlan();
  const isEnterprise = plan === 'enterprise';
  const [mobileOpen, setMobileOpen] = useState(false);
  const user = parseUser();
  const token = getToken();
  const [userMenuOpen, setUserMenuOpen] = useState(false);

  const baseItems = [
    { to: '/chat', icon: MessageCircle, label: 'Chat 沙箱', id: 'chat' },
    { to: '/vault', icon: Database, label: 'Trace Vault', id: 'vault' },
    { type: 'divider' as const },
    { to: '/dashboard', icon: LayoutDashboard, label: t('nav.dashboard'), id: 'dashboard' },
    { to: '/pipelines', icon: GitBranch, label: t('nav.pipelines'), id: 'pipelines' },
    { to: '/api-keys', icon: Key, label: t('nav.api-keys'), id: 'api-keys' },
    { to: '/models', icon: Cpu, label: t('models.title'), id: 'models' },
  ];

  const enterpriseItems = [
    { type: 'divider' as const },
    { to: '/playground', icon: Eye, label: 'Dev Hub', id: 'playground' },
    { to: '/audit-center', icon: Activity, label: '审计中心', id: 'audit-center' },
    { to: '/brand', icon: Palette, label: '品牌定制', id: 'brand' },
    { to: '/settings', icon: Settings, label: 'SSO & 企业配置', id: 'settings' },
  ];

  const personalItems = [
    { to: '/playground', icon: Eye, label: 'Dev Hub', id: 'playground' },
    { to: '/settings', icon: Settings, label: t('nav.settings'), id: 'settings' },
  ];

  const navItems = [...baseItems, ...(isEnterprise ? enterpriseItems : personalItems)];

  const sidebarContent = (
    <>
      <div className="sidebar-header">
        <div className="flex items-center gap-3">
          <div className="w-9 h-9 rounded-btn flex items-center justify-center flex-shrink-0" style={{ background: 'linear-gradient(135deg, #6c5ce7, #00d4aa)' }}>
            <Boxes size={20} color="white" />
          </div>
          <div>
            <h1 className="text-base font-bold text-[var(--text-primary)] tracking-tight">{t('app.title')}</h1>
            <p className="text-[10px] text-[var(--text-tertiary)] mt-0.5">{t('app.subtitle')}</p>
          </div>
        </div>
      </div>

      <nav className="sidebar-nav">
        {navItems.map((item: any, idx: number) => {
          if (item.type === 'divider') return <div key={`div-${idx}`} className="h-px mx-4 my-2" style={{ background: 'rgba(255,255,255,0.06)' }} />;
          const Icon = item.icon;
          return (
            <NavLink key={item.id} to={item.to as string} onClick={() => setMobileOpen(false)} className={({ isActive }: any) => `nav-item ${isActive ? 'active' : ''}`}>
              {({ isActive }: any) => (<>
                {isActive && <motion.div layoutId="nav-active-glow" className="absolute left-0 w-1 h-[60%] top-[20%] rounded-r" style={{ background: 'linear-gradient(180deg, #6c5ce7, #00d4aa)', boxShadow: '0 0 12px rgba(108,92,231,0.6)' }} />}
                <Icon size={18} />
                <span className="text-[13px] flex-1">{item.label}</span>
              </>)}
            </NavLink>
          );
        })}
      </nav>

      {/* User Section — 整合用户信息到侧边栏（替代右上角浮层）*/}
      {user && token && (
        <div className="px-3 pb-3">
          <div className="flex items-center gap-2.5 p-2.5 rounded-xl cursor-pointer transition-colors hover:bg-[var(--bg-glass)]" onClick={() => setUserMenuOpen(!userMenuOpen)}>
            <div className="w-8 h-8 rounded-full flex items-center justify-center text-white text-xs font-bold flex-shrink-0"
              style={{ background: isEnterprise ? 'linear-gradient(135deg, #6c5ce7, #00d4aa)' : 'linear-gradient(135deg, #6c5ce7, #a29bfe)' }}>
              {(user.display_name || user.email)[0].toUpperCase()}
            </div>
            <div className="flex-1 min-w-0">
              <div className="text-xs font-semibold text-[var(--text-primary)] truncate">{user.display_name}</div>
              <div className="text-[10px] text-[var(--text-tertiary)] flex items-center gap-1">
                {isEnterprise ? <Building size={9} /> : <User size={9} />}
                {isEnterprise ? 'Enterprise' : 'Personal'}
              </div>
            </div>
            <ChevronRight size={12} className="text-[var(--text-tertiary)] transition-transform" style={{ transform: userMenuOpen ? 'rotate(90deg)' : '' }} />
          </div>

          <AnimatePresence>
            {userMenuOpen && (
              <motion.div initial={{ opacity: 0, height: 0 }} animate={{ opacity: 1, height: 'auto' }} exit={{ opacity: 0, height: 0 }}
                className="overflow-hidden mt-1">
                <div className="py-1 px-2 rounded-xl" style={{ background: 'rgba(255,255,255,0.02)' }}>
                  <div className="py-2 px-2 border-b border-[rgba(255,255,255,0.04)] mb-1">
                    <div className="text-[11px] font-semibold text-[var(--text-primary)]">{user.display_name}</div>
                    <div className="text-[10px] text-[var(--text-tertiary)] mt-0.5">{user.email}</div>
                  </div>
                  <button onClick={() => { setUserMenuOpen(false); navigate('/settings'); }}
                    className="w-full flex items-center gap-2 py-2 px-2 rounded-lg text-[12px] text-[var(--text-secondary)] hover:bg-[var(--bg-glass)] hover:text-[var(--text-primary)] transition-colors">
                    <Settings size={13} /> 账户设置
                  </button>
                  <button onClick={() => { clearToken(); navigate('/login', { replace: true }); }}
                    className="w-full flex items-center gap-2 py-2 px-2 rounded-lg text-[12px] text-[#ff7675] hover:bg-[rgba(255,118,117,0.06)] transition-colors">
                    <LogOut size={13} /> 退出登录
                  </button>
                </div>
              </motion.div>
            )}
          </AnimatePresence>
        </div>
      )}

      <div className="sidebar-footer">
        <div className="flex flex-col gap-1.5">
          <div className="flex gap-1.5">
            <button onClick={() => setLocale(locale === 'zh' ? 'en' : 'zh')} className="btn-secondary flex-1 py-1.5 px-2 text-[11px] justify-center">
              <Globe size={12} /> {t('app.switch_lang')}
            </button>
            <button onClick={toggleTheme} className="btn-secondary w-8 py-1.5 justify-center" title={theme === 'dark' ? t('app.light_mode') : t('app.dark_mode')}>
              {theme === 'dark' ? <Sun size={13} /> : <Moon size={13} />}
            </button>
          </div>
          <div className="flex items-center gap-1.5 justify-center opacity-60">
            <Shield size={10} color="#00d4aa" />
            <span className="text-[9px] text-[var(--text-tertiary)]">{t('app.protocol')}</span>
          </div>
        </div>
      </div>
    </>
  );

  return (
    <>
      {/* Mobile hamburger */}
      <button onClick={() => setMobileOpen(true)} className="lg:hidden fixed top-4 left-4 z-[100] p-2 rounded-lg bg-[var(--bg-secondary)] border border-[var(--border-default)] text-[var(--text-primary)]">
        <Menu size={20} />
      </button>

      {/* Mobile overlay */}
      <AnimatePresence>
        {mobileOpen && (
          <motion.div initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }}
            className="lg:hidden fixed inset-0 z-[90]" style={{ background: 'rgba(0,0,0,0.5)' }}
            onClick={() => setMobileOpen(false)} />
        )}
      </AnimatePresence>

      {/* Desktop sidebar — icon-only when collapsed */}
      <aside className={`sidebar hidden lg:flex transition-all duration-300 ${navCollapsed?'!w-[64px]':''}`} style={{ backdropFilter: 'blur(20px)' }}>
        {navCollapsed ? (
          <div className="flex flex-col items-center pt-3 gap-1 w-full">
            <div className="w-9 h-9 rounded-btn flex items-center justify-center mb-2" style={{ background: 'linear-gradient(135deg, #6c5ce7, #00d4aa)' }}>
              <Boxes size={18} color="white" />
            </div>
            {navItems.map((item: any, idx: number) => {
              if (item.type === 'divider') return <div key={`div-${idx}`} className="w-8 h-px my-1" style={{ background: 'rgba(255,255,255,0.06)' }} />;
              const Icon = item.icon;
              return (
                <NavLink key={item.id} to={item.to as string} onClick={() => setMobileOpen(false)} className="nav-item !p-2 !justify-center w-full" title={item.label}>
                  {({ isActive }: any) => <Icon size={18} color={isActive ? '#00d4aa' : undefined} />}
                </NavLink>
              );
            })}
            <div className="mt-auto flex flex-col items-center gap-2 py-4">
              <button onClick={() => setLocale(locale === 'zh' ? 'en' : 'zh')} className="p-1.5 rounded-lg hover:bg-[var(--bg-glass)] text-[var(--text-tertiary)]" title={t('app.switch_lang')}><Globe size={14} /></button>
              <button onClick={toggleTheme} className="p-1.5 rounded-lg hover:bg-[var(--bg-glass)] text-[var(--text-tertiary)]" title={theme === 'dark' ? t('app.light_mode') : t('app.dark_mode')}>{theme === 'dark' ? <Sun size={14} /> : <Moon size={14} />}</button>
            </div>
          </div>
        ) : sidebarContent}
      </aside>

      {/* Mobile sidebar */}
      <AnimatePresence>
        {mobileOpen && (
          <motion.aside initial={{ x: -280 }} animate={{ x: 0 }} exit={{ x: -280 }} transition={{ duration: 0.25, ease: 'easeOut' }}
            className="sidebar flex lg:hidden fixed left-0 top-0 bottom-0 z-[95]" style={{ backdropFilter: 'blur(20px)' }}>
            <button onClick={() => setMobileOpen(false)} className="absolute top-4 right-4 p-1 rounded text-[var(--text-tertiary)] hover:text-[var(--text-primary)]">
              <X size={18} />
            </button>
            {sidebarContent}
          </motion.aside>
        )}
      </AnimatePresence>
    </>
  );
}
