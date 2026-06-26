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
  const [keys, setKeys] = useState<ApiKey[]>([]); const [loading, setLoading] = useState(true);
  const [visibleKeys, setVisibleKeys] = useState<Set<string>>(new Set()); const [error, setError] = useState<string | null>(null);
  const [promptOpen, setPromptOpen] = useState(false);
  const [confirmAction, setConfirmAction] = useState<null | { type: 'rotate' | 'delete'; key: ApiKey }>(null);

  const loadKeys = async () => { try { setLoading(true); setError(null); setKeys(await getApiKeys()); } catch (err) { setError('Failed to load API keys'); } finally { setLoading(false); } };
  useEffect(() => { loadKeys(); }, []);

  const toggleVisible = (id: string) => setVisibleKeys(p => { const n = new Set(p); n.has(id) ? n.delete(id) : n.add(id); return n; });
  const doGenerateKey = async (name: string) => { try { await createApiKey(name); loadKeys(); toast.success('API Key 创建成功'); } catch (err) { toast.error('Failed to generate key'); } };
  const handleRotate = async (key: ApiKey) => { try { await rotateApiKey(key.id); loadKeys(); toast.success('API Key 轮换成功'); } catch (err) { toast.error('Failed to rotate key'); } };
  const handleDelete = async (key: ApiKey) => { try { await deleteApiKey(key.id); loadKeys(); toast.success('API Key 已删除'); } catch (err) { toast.error('Failed to delete key'); } };

  const formatDate = (ds: string) => { try { return new Date(ds).toLocaleDateString(); } catch { return ds; } };

  return (
    <motion.div initial={{ opacity: 0 }} animate={{ opacity: 1 }}>
      <div className="flex justify-between items-center mb-6">
        <div>
          <h1 className="text-2xl font-bold text-[var(--text-primary)]">{t('apikey.title')}</h1>
          <p className="text-sm text-[var(--text-secondary)] mt-1">{t('apikey.subtitle')}</p>
        </div>
        <button className="btn-primary" onClick={() => setPromptOpen(true)}><Plus size={16} /> {t('apikey.generate')}</button>
      </div>

      <GlassCard className="p-6 mb-5">
        <div className="flex items-center gap-3">
          <Shield size={20} color="#fdcb6e" />
          <div>
            <h3 className="text-sm font-semibold text-[var(--text-primary)]">{t('apikey.security_title')}</h3>
            <p className="text-xs text-[var(--text-secondary)]">{t('apikey.security_desc')}</p>
          </div>
        </div>
      </GlassCard>

      {loading && <div className="flex justify-center py-10"><Loader size={24} className="animate-spin text-[var(--text-secondary)]" /></div>}

      {error && <GlassCard className="p-5 mb-5" style={{ background: 'rgba(255,118,117,0.1)', border: '1px solid rgba(255,118,117,0.3)' }}><p className="text-sm text-[#ff7675]">{error}</p></GlassCard>}

      {!loading && !error && keys.length === 0 && (
        <GlassCard className="text-center py-10">
          <Key size={40} className="mx-auto mb-4 text-[var(--text-tertiary)]" />
          <p className="text-sm text-[var(--text-secondary)]">{t('apikey.subtitle')}</p>
          <button className="btn-primary mt-4" onClick={() => setPromptOpen(true)}><Plus size={14} /> {t('apikey.generate')}</button>
        </GlassCard>
      )}

      <div className="flex flex-col gap-3">
        {keys.map(k => (
          <GlassCard key={k.id} className="p-5">
            <div className="flex justify-between items-start">
              <div>
                <div className="flex items-center gap-2.5">
                  <Key size={16} color="#74b9ff" />
                  <span className="text-[15px] font-semibold text-[var(--text-primary)]">{k.name}</span>
                  <span className="badge" style={{ background: k.status === 'active' ? 'rgba(0,212,170,0.15)' : 'rgba(253,203,110,0.15)', color: k.status === 'active' ? '#00d4aa' : '#fdcb6e' }}>{k.status}</span>
                </div>
                <p className="text-xs text-[var(--text-tertiary)] mt-1">{t('apikey.tenant')}: {k.tenant_id} · {t('apikey.created')}: {formatDate(k.created_at)}</p>
              </div>
              <div className="flex gap-2">
                {([
                  [() => toggleVisible(k.id), visibleKeys.has(k.id) ? EyeOff : Eye, visibleKeys.has(k.id) ? t('apikey.hide') : t('apikey.show')],
                  [() => { navigator.clipboard.writeText(k.key); toast.info('已复制到剪贴板'); }, Copy, t('apikey.copy')],
                  [() => setConfirmAction({ type: 'rotate', key: k }), RotateCcw, t('apikey.rotate')],
                ] as [() => void, typeof Key, string][]).map(([onClick, Icon, title]) => (
                  <button key={title} className="btn-secondary !py-1.5 !px-2.5" onClick={onClick} title={title}><Icon size={14} /></button>
                ))}
                <button className="btn-secondary !py-1.5 !px-2.5" onClick={() => setConfirmAction({ type: 'delete', key: k })} title={t('models.delete')} style={{ color: '#ff7675' }}><Trash2 size={14} /></button>
              </div>
            </div>
            <div className="mt-3 py-2.5 px-3.5 rounded-lg font-mono text-xs" style={{ background: 'rgba(0,0,0,0.2)', color: visibleKeys.has(k.id) ? '#00d4aa' : 'var(--text-tertiary)' }}>
              {visibleKeys.has(k.id) ? k.key : k.key.slice(0, 12) + '••••••••••••••••'}
            </div>
          </GlassCard>
        ))}
      </div>

      <PromptDialog open={promptOpen} onClose={() => setPromptOpen(false)} onSubmit={doGenerateKey} title="创建 API Key" placeholder="输入 Key 名称" />
      <ConfirmDialog open={confirmAction?.type === 'rotate'} onClose={() => setConfirmAction(null)} onConfirm={() => { if (confirmAction?.key) handleRotate(confirmAction.key); }} title="轮换 API Key" message="轮换后旧 Key 立即失效，是否继续？" confirmText="轮换" />
      <ConfirmDialog open={confirmAction?.type === 'delete'} onClose={() => setConfirmAction(null)} onConfirm={() => { if (confirmAction?.key) handleDelete(confirmAction.key); }} title="删除 API Key" message="删除后无法恢复，是否继续？" confirmText="删除" danger />
    </motion.div>
  );
}