import { useEffect, useState } from 'react';
import { motion } from 'framer-motion';
import GlassCard from '../components/ui/GlassCard';
import { useI18n } from '../i18n';
import { getPlugins, registerPlugin } from '../api';
import type { PluginMeta } from '../types';
import { Puzzle, Download, CheckCircle, Cpu, Zap, Plug } from 'lucide-react';

const defaultPlugins = [
  { name: 'PII Detector', type: 'native', version: '0.2.1', description: '生产级PII检测插件 - 检测并遮蔽身份证、信用卡、电话、邮箱等敏感信息', config: '{"enabled":true,"action_on_detect":"mask","detect_types":["china_id_card","credit_card","phone_number","email"]}' },
  { name: 'Budget Guard', type: 'native', version: '1.0.0', description: 'Rate limiting and budget enforcement for AI API calls', config: '{"limit_usd":10.0,"window":"daily"}' },
  { name: 'Auth Validator', type: 'native', version: '1.0.0', description: 'API key and delegation token validation', config: '{}' },
  { name: 'Route Selector', type: 'native', version: '1.0.0', description: 'Intelligent model routing and failover', config: '{"default":"deepseek-r1:14b"}' },
  { name: 'Keyword Guardrail', type: 'wasm', version: '0.2.0', description: 'Wasm-powered real-time content filtering', config: '{"patterns":["violence","hate","illegal"]}' },
  { name: 'Drift Detector', type: 'grpc', version: '0.2.0', description: 'Embedding drift analysis for semantic consistency', config: '{"threshold":0.7}' },
  { name: 'C-SafeGen', type: 'grpc', version: '1.0.0', description: 'Certified guarantee computation with conformal analysis', config: '{"methodology":"C-SafeGen_v1.0"}' },
  { name: 'TEE Attestation', type: 'grpc', version: '0.2.0', description: 'L1 TEE proof generation and verification', config: '{"platform":"tdx"}' },
  { name: 'Trace Finalizer', type: 'native', version: '1.0.0', description: 'L0 signature computation and trace finalization', config: '{}' },
  { name: 'Semantic Analyzer', type: 'grpc', version: '0.1.0', description: 'Advanced semantic analysis for output verification', config: '{}' },
];

const typeIcons: Record<string, any> = { native: Cpu, wasm: Zap, grpc: Plug };

export default function PluginsPage() {
  const { t } = useI18n();
  const [plugins, setPlugins] = useState<PluginMeta[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    getPlugins().then(setPlugins).catch(() => {}).finally(() => setLoading(false));
  }, []);

  const handleInstall = async (dp: typeof defaultPlugins[0]) => {
    try {
      const p = await registerPlugin({ name: dp.name, type: dp.type, version: dp.version, description: dp.description, config: dp.config });
      setPlugins(prev => [...prev, p]);
    } catch {}
  };

  return (
    <motion.div initial={{ opacity: 0 }} animate={{ opacity: 1 }}>
      <div style={{ marginBottom: 24 }}>
        <h1 style={{ fontSize: 24, fontWeight: 700, color: 'var(--text-primary)' }}>{t('plugin.title')}</h1>
        <p style={{ color: 'var(--text-secondary)', fontSize: 13, marginTop: 4 }}>{t('plugin.subtitle')}</p>
      </div>

      {loading ? (
        <div style={{ textAlign: 'center', padding: 60, color: 'var(--text-tertiary)' }}>{t('app.loading')}</div>
      ) : (
        <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
          {defaultPlugins.map((dp, i) => {
            const installed = plugins.some(p => p.name === dp.name);
            const Icon = typeIcons[dp.type] || Puzzle;
            const bgColor = dp.type === 'native' ? 'rgba(108,92,231,0.12)' : dp.type === 'wasm' ? 'rgba(0,212,170,0.12)' : 'rgba(116,185,255,0.12)';
            const accent = dp.type === 'native' ? '#6c5ce7' : dp.type === 'wasm' ? '#00d4aa' : '#74b9ff';

            return (
              <GlassCard key={dp.name} style={{ padding: 20 }}>
                <div style={{ display: 'flex', alignItems: 'center', gap: 16 }}>
                  <div style={{ width: 44, height: 44, borderRadius: 12, background: bgColor, display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
                    <Icon size={20} color={accent} />
                  </div>
                  <div style={{ flex: 1 }}>
                    <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
                      <span style={{ fontSize: 15, fontWeight: 600, color: 'var(--text-primary)' }}>{dp.name}</span>
                      <span className="badge" style={{ background: bgColor, color: accent, fontSize: 10 }}>{dp.type}</span>
                      <span style={{ fontSize: 11, color: 'var(--text-tertiary)' }}>v{dp.version}</span>
                    </div>
                    <p style={{ fontSize: 12, color: 'var(--text-secondary)', marginTop: 4 }}>{dp.description}</p>
                  </div>
                  <button
                    className={installed ? 'btn-secondary' : 'btn-primary'}
                    style={{ padding: '8px 16px', fontSize: 12, whiteSpace: 'nowrap' }}
                    onClick={() => !installed && handleInstall(dp)}
                    disabled={installed}
                  >
                    {installed ? <><CheckCircle size={12} color="#00d4aa" /> {t('plugin.installed')}</> : <><Download size={12} /> {t('plugin.install')}</>}
                  </button>
                </div>
              </GlassCard>
            );
          })}
        </div>
      )}
    </motion.div>
  );
}
