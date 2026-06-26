import { useState, useCallback, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import { motion } from 'framer-motion';
import GlassCard from '../components/ui/GlassCard';
import { useI18n } from '../i18n';
import { Settings as SettingsIcon, Bell, Globe, Database, Shield as ShieldIcon, RefreshCw, CheckCircle, Cpu, ArrowRight, AlertTriangle } from 'lucide-react';
import { getModelsConfig, getSystemSettings, updateSystemSettings } from '../api';

type Field = { key: string; labelKey: string; value: string; type: string; placeholder?: string; options?: string[]; };
const SETTINGS_STORAGE_KEY = 'veridactus-settings';

function loadPersistedSettings(): Record<string, string> { try { const r = localStorage.getItem(SETTINGS_STORAGE_KEY); return r ? JSON.parse(r) : {}; } catch { return {}; } }
function persistSettings(data: Record<string, string>) { try { localStorage.setItem(SETTINGS_STORAGE_KEY, JSON.stringify(data)); } catch {} }

export default function SettingsPage() {
  const { t } = useI18n(); const navigate = useNavigate();
  const [activeSection, setActiveSection] = useState('general');
  const [saved, setSaved] = useState(false); const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null); const [modelsCount, setModelsCount] = useState(0);
  const [formValues, setFormValues] = useState<Record<string, string>>({});

  useEffect(() => { setFormValues(prev => ({ ...prev, ...loadPersistedSettings() })); }, []);
  useEffect(() => { getModelsConfig().then(m => setModelsCount(m.length)).catch(() => setModelsCount(0)); }, []);

  const handleFieldChange = useCallback((key: string, value: string) => { setFormValues(p => ({ ...p, [key]: value })); setSaved(false); setError(null); }, []);
  const handleSave = useCallback(async () => {
    setSaving(true); setError(null);
    try { persistSettings(formValues); try { await updateSystemSettings(formValues); } catch {}; setSaved(true); setTimeout(() => setSaved(false), 3000); }
    catch (err) { setError(err instanceof Error ? err.message : '保存失败'); } finally { setSaving(false); }
  }, [formValues]);
  const handleReset = useCallback(() => { try { localStorage.removeItem(SETTINGS_STORAGE_KEY); } catch {}; setFormValues({}); setSaved(false); setError(null); }, []);
  const getFieldValue = useCallback((field: Field) => formValues[field.key] !== undefined ? formValues[field.key] : field.value, [formValues]);

  const sections: { id: string; icon: any; labelKey: string; fields: Field[] }[] = [
    { id: 'general', icon: Globe, labelKey: 'settings.general', fields: [
      { key: 'tenant_name', labelKey: 'settings.tenant_name', value: 'Acme Corp', type: 'text' },
      { key: 'protocol_version', labelKey: 'settings.protocol_version', value: '0.2.1', type: 'text' },
    ]},
    { id: 'notifications', icon: Bell, labelKey: 'settings.notifications', fields: [
      { key: 'webhook_url', labelKey: 'settings.webhook_url', value: '', type: 'text', placeholder: 'https://hooks.example.com/veridactus' },
      { key: 'alert_email', labelKey: 'settings.alert_email', value: 'admin@acme.com', type: 'text' },
    ]},
    { id: 'storage', icon: Database, labelKey: 'settings.storage', fields: [
      { key: 'trace_retention', labelKey: 'settings.trace_retention', value: '90', type: 'text' },
      { key: 'cold_storage', labelKey: 'settings.cold_storage', value: 'S3/MinIO', type: 'text' },
    ]},
    { id: 'security', icon: ShieldIcon, labelKey: 'settings.security', fields: [
      { key: 'key_rotation', labelKey: 'settings.key_rotation', value: '90', type: 'text' },
      { key: 'audit_log', labelKey: 'settings.audit_log', value: '365 days', type: 'text' },
    ]},
    { id: 'llm_models', icon: Cpu, labelKey: 'settings.llm_models', fields: [] },
  ];

  return (
    <motion.div initial={{ opacity: 0 }} animate={{ opacity: 1 }}>
      <div className="mb-6">
        <h1 className="text-2xl font-bold text-[var(--text-primary)]">{t('settings.title')}</h1>
        <p className="text-sm text-[var(--text-secondary)] mt-1">{t('settings.subtitle')}</p>
      </div>

      <div className="flex gap-5">
        {/* 左侧导航 */}
        <div className="w-[220px] flex-shrink-0">
          <GlassCard className="p-3">
            {sections.map(s => (
              <div key={s.id}
                className="flex items-center gap-2.5 py-2.5 px-3 rounded-lg cursor-pointer text-sm transition-colors"
                style={{ background: activeSection === s.id ? 'rgba(108,92,231,0.15)' : 'transparent', color: activeSection === s.id ? '#00d4aa' : 'var(--text-secondary)' }}
                onClick={() => setActiveSection(s.id)}>
                <s.icon size={16} /> {t(s.labelKey)}
                {s.id === 'llm_models' && modelsCount > 0 && (
                  <span className="ml-auto text-[10px] py-0.5 px-1.5 rounded-btn" style={{ background: 'rgba(0,212,170,0.2)', color: '#00d4aa' }}>{modelsCount}</span>
                )}
              </div>
            ))}
          </GlassCard>
        </div>

        {/* 右侧内容 */}
        <div className="flex-1">
          {sections.filter(s => s.id === activeSection).map(section => (
            <GlassCard key={section.id} className="p-6">
              {section.id === 'llm_models' ? (
                <div>
                  <div className="flex justify-between items-center mb-5">
                    <div>
                      <h3 className="text-base font-semibold text-[var(--text-primary)]">{t('settings.llm_models')}</h3>
                      <p className="text-sm text-[var(--text-secondary)] mt-1">{t('settings.llm_models_desc')}</p>
                    </div>
                    <span className="text-xs py-1 px-3 rounded-xl" style={{ background: 'rgba(0,212,170,0.15)', color: '#00d4aa' }}>{modelsCount} {t('models.title')}</span>
                  </div>
                  <div className="flex flex-col gap-3">
                    {[
                      [Cpu, '#6c5ce7', t('models.title'), t('models.subtitle'), '/models', 'btn-primary'],
                      [ShieldIcon, '#00d4aa', t('apikey.title'), t('apikey.subtitle'), '/api-keys', 'btn-secondary'],
                    ].map(([Icon, color, title, desc, to, btnClass]) => (
                      <GlassCard key={title as string} className="p-4 hover:brightness-110 transition-all" style={{ background: `${color}14`, border: `1px solid ${color}33` }}>
                        <div className="flex items-center gap-3">
                          <Icon size={24} />
                          <div className="flex-1">
                            <h4 className="text-sm font-semibold text-[var(--text-primary)]">{title as string}</h4>
                            <p className="text-xs text-[var(--text-secondary)]">{desc as string}</p>
                          </div>
                          <button className={btnClass as string} onClick={() => navigate(to as string)}>
                            {title as string} <ArrowRight size={14} />
                          </button>
                        </div>
                      </GlassCard>
                    ))}
                  </div>
                </div>
              ) : (
                <div>
                  <h3 className="text-base font-semibold text-[var(--text-primary)] mb-5">{t(section.labelKey)}</h3>
                  <div className="flex flex-col gap-4">
                    {section.fields.map(field => (
                      <div key={field.key}>
                        <label className="block text-xs font-medium text-[var(--text-secondary)] mb-1.5">{t(field.labelKey)}</label>
                        {field.type === 'select' && field.options ? (
                          <select className="input-field cursor-pointer" value={getFieldValue(field)} onChange={e => handleFieldChange(field.key, e.target.value)}>
                            {field.options.map(o => <option key={o} value={o}>{o}</option>)}
                          </select>
                        ) : (
                          <input className="input-field" value={getFieldValue(field)} onChange={e => handleFieldChange(field.key, e.target.value)} placeholder={field.placeholder || ''} />
                        )}
                      </div>
                    ))}
                  </div>
                  {error && (
                    <div className="mt-4 py-2.5 px-3.5 rounded-lg flex items-center gap-2 text-sm" style={{ background: 'rgba(255,118,117,0.1)', border: '1px solid rgba(255,118,117,0.3)', color: '#ff7675' }}>
                      <AlertTriangle size={16} />{error}
                    </div>
                  )}
                  <div className="mt-5 flex gap-2.5 items-center">
                    <button className="btn-primary" onClick={handleSave} disabled={saving}>
                      {saving ? <RefreshCw size={14} className="animate-spin" /> : <RefreshCw size={14} />}
                      {' '}{saving ? '保存中...' : t('settings.save')}
                    </button>
                    <button className="btn-secondary" onClick={handleReset} disabled={saving}>{t('settings.reset')}</button>
                    {saved && <span className="flex items-center gap-1 text-sm text-[#00d4aa]"><CheckCircle size={14} /> {t('settings.saved')}</span>}
                  </div>
                </div>
              )}
            </GlassCard>
          ))}
        </div>
      </div>
    </motion.div>
  );
}