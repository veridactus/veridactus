import { useState, useCallback, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import { motion } from 'framer-motion';
import GlassCard from '../components/ui/GlassCard';
import { useI18n } from '../i18n';
import { Settings as SettingsIcon, Bell, Globe, Database, Shield as ShieldIcon, RefreshCw, CheckCircle, Cpu, ArrowRight } from 'lucide-react';
import { getModelsConfig } from '../api';

type Field = {
  key: string;
  labelKey: string;
  value: string;
  type: string;
  placeholder?: string;
  options?: string[];
};

export default function SettingsPage() {
  const { t } = useI18n();
  const navigate = useNavigate();
  const [activeSection, setActiveSection] = useState('general');
  const [saved, setSaved] = useState(false);
  const [modelsCount, setModelsCount] = useState(0);

  useEffect(() => {
    getModelsConfig().then(models => setModelsCount(models.length)).catch(() => setModelsCount(0));
  }, []);

  const handleSave = useCallback(() => {
    setSaved(true);
    setTimeout(() => setSaved(false), 2000);
  }, []);

  const handleReset = useCallback(() => {
    setSaved(false);
  }, []);

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
                          <select className="input-field" defaultValue={field.value} style={{ cursor: 'pointer' }}>
                            {field.options.map(o => <option key={o} value={o}>{o}</option>)}
                          </select>
                        ) : (
                          <input className="input-field" defaultValue={field.value} placeholder={field.placeholder || ''} />
                        )}
                      </div>
                    ))}
                  </div>
                  <div style={{ marginTop: 20, display: 'flex', gap: 10, alignItems: 'center' }}>
                    <button className="btn-primary" onClick={handleSave}><RefreshCw size={14} /> {t('settings.save')}</button>
                    <button className="btn-secondary" onClick={handleReset}>{t('settings.reset')}</button>
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