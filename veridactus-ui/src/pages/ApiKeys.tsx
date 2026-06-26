import { useState, useEffect } from 'react';
import { motion } from 'framer-motion';
import GlassCard from '../components/ui/GlassCard';
import { useI18n } from '../i18n';
import { Key, Shield, Plus, Copy, Eye, EyeOff, RotateCcw, Trash2, Loader } from 'lucide-react';
import { getApiKeys, createApiKey, deleteApiKey, rotateApiKey } from '../api';
import type { ApiKey } from '../types';
import { PromptDialog, ConfirmDialog } from '../components/ui/Dialog';
import { toast } from '../components/ui/Toast';

export default function ApiKeys() {
  const { t } = useI18n();
  const [keys, setKeys] = useState<ApiKey[]>([]);
  const [loading, setLoading] = useState(true);
  const [visibleKeys, setVisibleKeys] = useState<Set<string>>(new Set());
  const [error, setError] = useState<string | null>(null);
  // 对话框状态
  const [promptOpen, setPromptOpen] = useState(false);
  const [confirmAction, setConfirmAction] = useState<null | { type: 'rotate' | 'delete'; key: ApiKey }>(null);

  const loadKeys = async () => {
    try {
      setLoading(true);
      setError(null);
      const data = await getApiKeys();
      setKeys(data);
    } catch (err) {
      setError('Failed to load API keys');
      console.error(err);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadKeys();
  }, []);

  const toggleVisible = (id: string) => {
    setVisibleKeys(prev => {
      const n = new Set(prev);
      if (n.has(id)) n.delete(id);
      else n.add(id);
      return n;
    });
  };

  const handleGenerate = () => { setPromptOpen(true); };

  const doGenerateKey = async (name: string) => {
    try {
      await createApiKey(name);
      await loadKeys();
      toast.success('API Key 创建成功');
    } catch (err) {
      toast.error('Failed to generate key');
      console.error(err);
    }
  };

  const handleRotate = async (key: ApiKey) => {
    try {
      await rotateApiKey(key.id);
      await loadKeys();
      toast.success('API Key 轮换成功');
    } catch (err) {
      toast.error('Failed to rotate key');
      console.error(err);
    }
  };

  const handleDelete = async (key: ApiKey) => {
    try {
      await deleteApiKey(key.id);
      await loadKeys();
      toast.success('API Key 已删除');
    } catch (err) {
      toast.error('Failed to delete key');
      console.error(err);
    }
  };

  const copyKey = (key: string) => {
    navigator.clipboard.writeText(key);
  };

  const formatDate = (dateStr: string) => {
    try {
      return new Date(dateStr).toLocaleDateString();
    } catch {
      return dateStr;
    }
  };

  return (
    <motion.div initial={{ opacity: 0 }} animate={{ opacity: 1 }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 24 }}>
        <div>
          <h1 style={{ fontSize: 24, fontWeight: 700, color: 'var(--text-primary)' }}>{t('apikey.title')}</h1>
          <p style={{ color: 'var(--text-secondary)', fontSize: 13, marginTop: 4 }}>{t('apikey.subtitle')}</p>
        </div>
        <button className="btn-primary" onClick={handleGenerate}><Plus size={16} /> {t('apikey.generate')}</button>
      </div>

      <GlassCard style={{ padding: 24, marginBottom: 20 }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
          <Shield size={20} color="#fdcb6e" />
          <div>
            <h3 style={{ fontSize: 14, fontWeight: 600, color: 'var(--text-primary)' }}>{t('apikey.security_title')}</h3>
            <p style={{ fontSize: 12, color: 'var(--text-secondary)' }}>{t('apikey.security_desc')}</p>
          </div>
        </div>
      </GlassCard>

      {loading && (
        <div style={{ display: 'flex', justifyContent: 'center', padding: 40 }}>
          <Loader size={24} className="spin" style={{ color: 'var(--text-secondary)' }} />
        </div>
      )}

      {error && (
        <GlassCard style={{ padding: 20, marginBottom: 20, background: 'rgba(255,118,117,0.1)', border: '1px solid rgba(255,118,117,0.3)' }}>
          <p style={{ color: '#ff7675', fontSize: 14 }}>{error}</p>
        </GlassCard>
      )}

      {!loading && !error && keys.length === 0 && (
        <GlassCard style={{ padding: 40, textAlign: 'center' }}>
          <Key size={40} color="var(--text-tertiary)" style={{ margin: '0 auto 16px' }} />
          <p style={{ color: 'var(--text-secondary)', fontSize: 14 }}>{t('apikey.subtitle')}</p>
          <button className="btn-primary" style={{ marginTop: 16 }} onClick={handleGenerate}>
            <Plus size={14} /> {t('apikey.generate')}
          </button>
        </GlassCard>
      )}

      <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
        {keys.map(k => (
          <GlassCard key={k.id} style={{ padding: 20 }}>
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start' }}>
              <div>
                <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
                  <Key size={16} color="#74b9ff" />
                  <span style={{ fontSize: 15, fontWeight: 600, color: 'var(--text-primary)' }}>{k.name}</span>
                  <span className="badge" style={{
                    background: k.status === 'active' ? 'rgba(0,212,170,0.15)' : 'rgba(253,203,110,0.15)',
                    color: k.status === 'active' ? '#00d4aa' : '#fdcb6e'
                  }}>{k.status}</span>
                </div>
                <p style={{ fontSize: 12, color: 'var(--text-tertiary)', marginTop: 4 }}>
                  {t('apikey.tenant')}: {k.tenant_id} · {t('apikey.created')}: {formatDate(k.created_at)}
                </p>
              </div>
              <div style={{ display: 'flex', gap: 8 }}>
                <button className="btn-secondary" style={{ padding: '6px 10px' }} onClick={() => toggleVisible(k.id)} title={visibleKeys.has(k.id) ? t('apikey.hide') : t('apikey.show')}>
                  {visibleKeys.has(k.id) ? <EyeOff size={14} /> : <Eye size={14} />}
                </button>
                <button className="btn-secondary" style={{ padding: '6px 10px' }} onClick={() => copyKey(k.key)} title={t('apikey.copy')}>
                  <Copy size={14} />
                </button>
                <button className="btn-secondary" style={{ padding: '6px 10px' }} onClick={() => setConfirmAction({ type: 'rotate', key: k })} title={t('apikey.rotate')}>
                  <RotateCcw size={14} />
                </button>
                <button className="btn-secondary" style={{ padding: '6px 10px', color: '#ff7675' }} onClick={() => setConfirmAction({ type: 'delete', key: k })} title={t('models.delete')}>
                  <Trash2 size={14} />
                </button>
              </div>
            </div>
            <div style={{ marginTop: 12, padding: '10px 14px', borderRadius: 8, background: 'rgba(0,0,0,0.2)', fontFamily: "'JetBrains Mono', monospace", fontSize: 12, color: visibleKeys.has(k.id) ? '#00d4aa' : 'var(--text-tertiary)' }}>
              {visibleKeys.has(k.id) ? k.key : k.key.slice(0, 12) + '••••••••••••••••'}
            </div>
          </GlassCard>
        ))}
      </div>

      <style>{`
        @keyframes spin { to { transform: rotate(360deg); } }
        .spin { animation: spin 0.8s linear infinite; }
      `}</style>

      {/* 对话框组件 */}
      <PromptDialog
        open={promptOpen}
        onClose={() => setPromptOpen(false)}
        onSubmit={doGenerateKey}
        title="创建 API Key"
        placeholder="输入 Key 名称"
      />
      <ConfirmDialog
        open={confirmAction?.type === 'rotate'}
        onClose={() => setConfirmAction(null)}
        onConfirm={() => { if (confirmAction?.key) handleRotate(confirmAction.key); }}
        title="轮换 API Key"
        message="轮换后旧 Key 立即失效，是否继续？"
        confirmText="轮换"
      />
      <ConfirmDialog
        open={confirmAction?.type === 'delete'}
        onClose={() => setConfirmAction(null)}
        onConfirm={() => { if (confirmAction?.key) handleDelete(confirmAction.key); }}
        title="删除 API Key"
        message="删除后无法恢复，是否继续？"
        confirmText="删除"
        danger
      />
    </motion.div>
  );
}