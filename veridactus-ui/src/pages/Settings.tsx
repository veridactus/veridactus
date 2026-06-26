import { useState, useCallback, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import { motion } from 'framer-motion';
import GlassCard from '../components/ui/GlassCard';
import { useI18n } from '../i18n';
import { Settings as SettingsIcon, Bell, Globe, Database, Shield as ShieldIcon, RefreshCw, CheckCircle, Cpu, ArrowRight, AlertTriangle } from 'lucide-react';
import { getModelsConfig, getSystemSettings, updateSystemSettings } from '../api';

type Field = {
  key: string;
  labelKey: string;
  value: string;
  type: string;
  placeholder?: string;
  options?: string[];
};

/** 本地持久化的 settings key */
const SETTINGS_STORAGE_KEY = 'veridactus-settings';

function loadPersistedSettings(): Record<string, string> {
  try {
    const raw = localStorage.getItem(SETTINGS_STORAGE_KEY);
    return raw ? JSON.parse(raw) : {};
  } catch {
    return {};
  }
}

function persistSettings(data: Record<string, string>) {
  try {
    localStorage.setItem(SETTINGS_STORAGE_KEY, JSON.stringify(data));
  } catch {
    // localStorage 不可用时静默失败
  }
}

export default function SettingsPage() {
  const { t } = useI18n();
  const navigate = useNavigate();
  const [activeSection, setActiveSection] = useState('general');
  const [saved, setSaved] = useState(false);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [modelsCount, setModelsCount] = useState(0);
  const [formValues, setFormValues] = useState<Record<string, string>>({});

  // 加载持久化的设置值
  useEffect(() => {
    const persisted = loadPersistedSettings();
    setFormValues(prev => ({ ...prev, ...persisted }));
  }, []);

  useEffect(() => {
    getModelsConfig()
      .then(models => setModelsCount(models.length))
      .catch(() => setModelsCount(0));
  }, []);

  const handleFieldChange = useCallback((key: string, value: string) => {
    setFormValues(prev => ({ ...prev, [key]: value }));
    setSaved(false);
    setError(null);
  }, []);

  const handleSave = useCallback(async () => {
    setSaving(true);
    setError(null);
    try {
      // 1. 本地持久化
      persistSettings(formValues);

      // 2. 尝试同步到控制面
      try {
        await updateSystemSettings(formValues);
      } catch {
        // 控制面不可用时，本地保存仍生效
        console.warn('Settings saved locally but failed to sync to control plane');
      }

      setSaved(true);
      setTimeout(() => setSaved(false), 3000);
    } catch (err) {
      setError(err instanceof Error ? err.message : '保存失败');
    } finally {
      setSaving(false);
    }
  }, [formValues]);

  const handleReset = useCallback(() => {
    try {
      localStorage.removeItem(SETTINGS_STORAGE_KEY);
    } catch {
      // 静默失败
    }
    setFormValues({});
    setSaved(false);
    setError(null);
  }, []);

  // 获取字段当前值（优先使用 formValues，回退到默认值）
  const getFieldValue = useCallback((field: Field) => {
    return formValues[field.key] !== undefined ? formValues[field.key] : field.value;
  }, [formValues]);

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
      <div style={{ marginBottom: 24 }}>
        <h1 style={{ fontSize: 24, fontWeight: 700, color: 'var(--text-primary)' }}>{t('settings.title')}</h1>
        <p style={{ color: 'var(--text-secondary)', fontSize: 13, marginTop: 4 }}>{t('settings.subtitle')}</p>
      </div>

      <div style={{ display: 'flex', gap: 20 }}>
        <div style={{ width: 220, flexShrink: 0 }}>
          <GlassCard style={{ padding: 12 }}>
            {sections.map(s => (
              <div key={s.id}
                style={{ display: 'flex', alignItems: 'center', gap: 10, padding: '10px 12px', borderRadius: 8, cursor: 'pointer', fontSize: 13,
                  background: activeSection === s.id ? 'rgba(108,92,231,0.15)' : 'transparent',
                  color: activeSection === s.id ? '#00d4aa' : 'var(--text-secondary)', transition: 'all 0.2s' }}
                onClick={() => setActiveSection(s.id)}
              >
                <s.icon size={16} /> {t(s.labelKey)}
                {s.id === 'llm_models' && modelsCount > 0 && (
                  <span style={{ marginLeft: 'auto', background: 'rgba(0,212,170,0.2)', color: '#00d4aa', fontSize: 10, padding: '2px 6px', borderRadius: 10 }}>
                    {modelsCount}
                  </span>
                )}
              </div>
            ))}
          </GlassCard>
        </div>

        <div style={{ flex: 1 }}>
          {sections.filter(s => s.id === activeSection).map(section => (
            <GlassCard key={section.id} style={{ padding: 24 }}>
              {section.id === 'llm_models' ? (
                <div>
                  <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 20 }}>
                    <div>
                      <h3 style={{ fontSize: 16, fontWeight: 600, color: 'var(--text-primary)' }}>{t('settings.llm_models')}</h3>
                      <p style={{ fontSize: 13, color: 'var(--text-secondary)', marginTop: 4 }}>{t('settings.llm_models_desc')}</p>
                    </div>
                    <span style={{ background: 'rgba(0,212,170,0.15)', color: '#00d4aa', fontSize: 12, padding: '4px 12px', borderRadius: 12 }}>
                      {modelsCount} {t('models.title')}
                    </span>
                  </div>
                  <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
                    <GlassCard style={{ padding: 16, background: 'rgba(108,92,231,0.08)', border: '1px solid rgba(108,92,231,0.2)' }}>
                      <div style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
                        <Cpu size={24} color="#6c5ce7" />
                        <div style={{ flex: 1 }}>
                          <h4 style={{ fontSize: 14, fontWeight: 600, color: 'var(--text-primary)' }}>{t('models.title')}</h4>
                          <p style={{ fontSize: 12, color: 'var(--text-secondary)' }}>{t('models.subtitle')}</p>
                        </div>
                        <button className="btn-primary" onClick={() => navigate('/models')}>
                          {t('models.title')} <ArrowRight size={14} />
                        </button>
                      </div>
                    </GlassCard>
                    <GlassCard style={{ padding: 16, background: 'rgba(0,212,170,0.08)', border: '1px solid rgba(0,212,170,0.2)' }}>
                      <div style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
                        <ShieldIcon size={24} color="#00d4aa" />
                        <div style={{ flex: 1 }}>
                          <h4 style={{ fontSize: 14, fontWeight: 600, color: 'var(--text-primary)' }}>{t('apikey.title')}</h4>
                          <p style={{ fontSize: 12, color: 'var(--text-secondary)' }}>{t('apikey.subtitle')}</p>
                        </div>
                        <button className="btn-secondary" onClick={() => navigate('/api-keys')}>
                          {t('apikey.title')} <ArrowRight size={14} />
                        </button>
                      </div>
                    </GlassCard>
                  </div>
                </div>
              ) : (
                <div>
                  <h3 style={{ fontSize: 16, fontWeight: 600, color: 'var(--text-primary)', marginBottom: 20 }}>{t(section.labelKey)}</h3>
                  <div style={{ display: 'flex', flexDirection: 'column', gap: 16 }}>
                    {section.fields.map(field => (
                      <div key={field.key}>
                        <label style={{ display: 'block', fontSize: 12, fontWeight: 500, color: 'var(--text-secondary)', marginBottom: 6 }}>{t(field.labelKey)}</label>
                        {field.type === 'select' && field.options ? (
                          <select
                            className="input-field"
                            value={getFieldValue(field)}
                            onChange={e => handleFieldChange(field.key, e.target.value)}
                            style={{ cursor: 'pointer' }}
                          >
                            {field.options.map(o => <option key={o} value={o}>{o}</option>)}
                          </select>
                        ) : (
                          <input
                            className="input-field"
                            value={getFieldValue(field)}
                            onChange={e => handleFieldChange(field.key, e.target.value)}
                            placeholder={field.placeholder || ''}
                          />
                        )}
                      </div>
                    ))}
                  </div>
                  {error && (
                    <div style={{
                      marginTop: 16, padding: '10px 14px', borderRadius: 8,
                      background: 'rgba(255,118,117,0.1)', border: '1px solid rgba(255,118,117,0.3)',
                      display: 'flex', alignItems: 'center', gap: 8, fontSize: 13, color: '#ff7675',
                    }}>
                      <AlertTriangle size={16} />
                      {error}
                    </div>
                  )}
                  <div style={{ marginTop: 20, display: 'flex', gap: 10, alignItems: 'center' }}>
                    <button className="btn-primary" onClick={handleSave} disabled={saving}>
                      {saving ? <RefreshCw size={14} style={{ animation: 'spin 1s linear infinite' }} /> : <RefreshCw size={14} />}
                      {' '}{saving ? '保存中...' : t('settings.save')}
                    </button>
                    <button className="btn-secondary" onClick={handleReset} disabled={saving}>{t('settings.reset')}</button>
                    {saved && (
                      <span style={{ display: 'flex', alignItems: 'center', gap: 4, fontSize: 13, color: '#00d4aa' }}>
                        <CheckCircle size={14} /> {t('settings.saved')}
                      </span>
                    )}
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