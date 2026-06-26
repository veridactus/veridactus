import { NavLink } from 'react-router-dom';
import { motion } from 'framer-motion';
import { LayoutDashboard, GitBranch, Activity, Puzzle, Key, Settings, Shield, Boxes, Globe, Sun, Moon, Cpu, MessageCircle, Database, Eye, FileText, Palette, Building } from 'lucide-react';
import { useI18n } from '../../i18n';
import { useThemeStore } from '../../store';
import { getUserPlan } from '../../auth/AuthGuard';

export default function Sidebar() {
  const { t, locale, setLocale } = useI18n();
  const { theme, toggleTheme } = useThemeStore();
  const plan = getUserPlan();
  const isEnterprise = plan === 'enterprise';

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
    { to: '/audit', icon: Activity, label: t('nav.audit'), id: 'audit' },
    { to: '/playground', icon: Eye, label: 'Dev Hub', id: 'playground' },
    { to: '/settings', icon: Settings, label: t('nav.settings'), id: 'settings' },
  ];

  const navItems = [...baseItems, ...(isEnterprise ? enterpriseItems : personalItems)];

  return (
    <aside className="sidebar" style={{ backdropFilter: 'blur(20px)' }}>
      <div className="sidebar-header">
        <div style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
          <div style={{
            width: 36, height: 36, borderRadius: 10,
            background: 'linear-gradient(135deg, #6c5ce7, #00d4aa)',
            display: 'flex', alignItems: 'center', justifyContent: 'center',
            flexShrink: 0,
          }}>
            <Boxes size={20} color="white" />
          </div>
          <div>
            <h1 style={{ fontSize: 16, fontWeight: 700, color: 'var(--text-primary)', letterSpacing: '-0.01em' }}>{t('app.title')}</h1>
            <p style={{ fontSize: 10, color: 'var(--text-tertiary)', marginTop: 1 }}>{t('app.subtitle')}</p>
          </div>
        </div>
      </div>

      <nav className="sidebar-nav">
        {navItems.map((item: any, idx: number) => {
          if (item.type === 'divider') {
            return <div key={`div-${idx}`} style={{ height: 1, background: 'rgba(255,255,255,0.06)', margin: '8px 16px' }} />;
          }
          const Icon = item.icon;
          return (
            <NavLink key={item.id} to={item.to as string} className={({ isActive }: any) => `nav-item ${isActive ? 'active' : ''}`}>
              {({ isActive }: any) => (
                <>
                  {isActive && (
                    <motion.div layoutId="nav-active-glow" style={{
                      position: 'absolute', left: 0, width: 4, height: '60%', top: '20%',
                      background: 'linear-gradient(180deg, #6c5ce7, #00d4aa)',
                      borderRadius: '0 4px 4px 0',
                      boxShadow: '0 0 12px rgba(108,92,231,0.6)',
                    }} />
                  )}
                  <Icon size={18} />
                  <span style={{ fontSize: 13, flex: 1 }}>{item.label}</span>
                </>
              )}
            </NavLink>
          );
        })}
      </nav>

      <div className="sidebar-footer">
        <div style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
          <div style={{ display: 'flex', gap: 6 }}>
            <button onClick={() => setLocale(locale === 'zh' ? 'en' : 'zh')}
              className="btn-secondary" style={{ flex: 1, padding: '6px 8px', fontSize: 11, justifyContent: 'center' }}>
              <Globe size={12} /> {t('app.switch_lang')}
            </button>
            <button onClick={toggleTheme}
              className="btn-secondary" style={{ width: 32, padding: '6px', justifyContent: 'center' }}
              title={theme === 'dark' ? t('app.light_mode') : t('app.dark_mode')}>
              {theme === 'dark' ? <Sun size={13} /> : <Moon size={13} />}
            </button>
          </div>
          <div style={{ display: 'flex', alignItems: 'center', gap: 6, justifyContent: 'center', opacity: 0.6 }}>
            <Shield size={10} color="#00d4aa" />
            <span style={{ fontSize: 9, color: 'var(--text-tertiary)' }}>{t('app.protocol')}</span>
          </div>
        </div>
      </div>
    </aside>
  );
}
