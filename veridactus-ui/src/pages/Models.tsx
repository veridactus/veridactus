import { useState, useEffect } from 'react';
import { motion } from 'framer-motion';
import GlassCard from '../components/ui/GlassCard';
import { useI18n } from '../i18n';
import { Cpu, Plus, Edit, Trash2, Loader, Check, X, ArrowLeft } from 'lucide-react';
import { getModelsConfig, createModel, updateModel, deleteModel } from '../api';
import type { ModelConfig } from '../types';
import { ConfirmDialog } from '../components/ui/Dialog';
import { toast } from '../components/ui/Toast';

interface EditingModel { id: string; name: string; upstream_url: string; upstream_model: string; is_default: boolean; supported_versions: string; status: string; api_key: string; api_key_header: string; use_proxy: boolean; proxy_url: string; }

export default function Models() {
  const { t } = useI18n();
  const [models, setModels] = useState<ModelConfig[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [modelToDelete, setModelToDelete] = useState<ModelConfig | null>(null);
  const [editingModel, setEditingModel] = useState<EditingModel | null>(null);
  const [isCreating, setIsCreating] = useState(false);
  const [formData, setFormData] = useState({ name: '', upstream_url: '', upstream_model: '', is_default: false, supported_versions: '0.1,0.2', status: 'active', api_key: '', api_key_header: '', use_proxy: false, proxy_url: '' });

  const loadModels = async () => { try { setLoading(true); setError(null); setModels(await getModelsConfig()); } catch (err) { setError('Failed to load models'); } finally { setLoading(false); } };
  useEffect(() => { loadModels(); }, []);

  const handleCreate = async () => { try { await createModel({ ...formData, upstream_model: formData.upstream_model || formData.name, supported_versions: formData.supported_versions.split(',').map(v => v.trim()), api_key: formData.api_key || undefined, api_key_header: formData.api_key_header || undefined, proxy_url: formData.use_proxy ? formData.proxy_url : undefined }); setIsCreating(false); resetForm(); loadModels(); } catch (err) { toast.error('Failed to create model'); } };
  const handleUpdate = async () => { if (!editingModel) return; try { await updateModel(editingModel.id, { ...editingModel, supported_versions: editingModel.supported_versions.split(',').map(v => v.trim()), api_key: editingModel.api_key || undefined, api_key_header: editingModel.api_key_header || undefined, proxy_url: editingModel.use_proxy ? editingModel.proxy_url : undefined, upstream_model: editingModel.upstream_model }); setEditingModel(null); loadModels(); } catch (err) { toast.error('Failed to update model'); } };
  const handleConfirmDelete = async () => { if (!modelToDelete) return; try { await deleteModel(modelToDelete.id); loadModels(); toast.success('模型已删除'); } catch (err) { toast.error('Failed to delete model'); } finally { setModelToDelete(null); } };
  const resetForm = () => setFormData({ name: '', upstream_url: '', upstream_model: '', is_default: false, supported_versions: '0.1,0.2', status: 'active', api_key: '', api_key_header: '', use_proxy: false, proxy_url: '' });
  const startEdit = (m: ModelConfig) => setEditingModel({ id: m.id, name: m.name, upstream_url: m.upstream_url, upstream_model: m.upstream_model, is_default: m.is_default, supported_versions: Array.isArray(m.supported_versions) ? m.supported_versions.join(',') : m.supported_versions || '0.1,0.2', status: m.status, api_key: m.api_key || '', api_key_header: m.api_key_header || '', use_proxy: m.use_proxy || false, proxy_url: m.proxy_url || '' });

  const formFields = [
    { key: 'name' as const, label: t('models.name'), placeholder: 'deepseek-r1:14b' },
    { key: 'upstream_url' as const, label: t('models.upstream_url'), placeholder: '' },
    { key: 'upstream_model' as const, label: t('models.upstream_model'), placeholder: 'model-name' },
    { key: 'supported_versions' as const, label: t('models.supported_versions'), placeholder: '0.1,0.2' },
    { key: 'api_key' as const, label: 'API Key', placeholder: 'Your API key', type: 'password' },
    { key: 'api_key_header' as const, label: 'API Key Header', placeholder: 'X-goog-api-key or Authorization' },
  ];

  const Label = ({ label }: { label: string }) => <label className="block text-[11px] text-[var(--text-tertiary)] mb-1">{label}</label>;
  const FieldLabel = ({ label }: { label: string }) => <label className="block text-xs font-medium text-[var(--text-secondary)] mb-1.5">{label}</label>;
  const CheckRow = ({ id, checked, onChange, label }: { id: string; checked: boolean; onChange: (e: React.ChangeEvent<HTMLInputElement>) => void; label: string }) => (
    <div className="flex items-center gap-2"><input type="checkbox" id={id} checked={checked} onChange={onChange} /><label htmlFor={id} className="text-xs text-[var(--text-secondary)]">{label}</label></div>
  );

  return (
    <motion.div initial={{ opacity: 0 }} animate={{ opacity: 1 }}>
      <div className="flex justify-between items-center mb-6">
        <div>
          <h1 className="text-2xl font-bold text-[var(--text-primary)]">{t('models.title')}</h1>
          <p className="text-sm text-[var(--text-secondary)] mt-1">{t('models.subtitle')}</p>
        </div>
        <button className="btn-primary" onClick={() => setIsCreating(true)}><Plus size={16} /> {t('models.add')}</button>
      </div>

      {loading && <div className="flex justify-center py-10"><Loader size={24} className="animate-spin text-[var(--text-secondary)]" /></div>}

      {error && <GlassCard className="p-5 mb-5" style={{ background: 'rgba(255,118,117,0.1)', border: '1px solid rgba(255,118,117,0.3)' }}><p className="text-sm text-[#ff7675]">{error}</p></GlassCard>}

      {!loading && !error && models.length === 0 && (
        <GlassCard className="text-center py-10">
          <Cpu size={40} className="mx-auto mb-4 text-[var(--text-tertiary)]" />
          <p className="text-sm text-[var(--text-secondary)]">{t('models.no_models')}</p>
          <p className="text-xs text-[var(--text-tertiary)] mt-2">{t('models.add_first')}</p>
          <button className="btn-primary mt-4" onClick={() => setIsCreating(true)}><Plus size={14} /> {t('models.add')}</button>
        </GlassCard>
      )}

      <div className="grid gap-4" style={{ gridTemplateColumns: 'repeat(auto-fill, minmax(350px, 1fr))' }}>
        {models.map(m => (
          <GlassCard key={m.id} className="p-5">
            {editingModel?.id === m.id ? (
              <div>
                <div className="flex items-center gap-2 mb-4">
                  <ArrowLeft size={16} onClick={() => setEditingModel(null)} className="cursor-pointer" />
                  <span className="text-[15px] font-semibold text-[var(--text-primary)]">{t('models.edit')}</span>
                </div>
                <div className="flex flex-col gap-3">
                  {formFields.map(f => (
                    <div key={f.key}>
                      <Label label={f.label} />
                      <input className="input-field" value={editingModel[f.key]} placeholder={f.placeholder}
                        type={f.type === 'password' ? 'password' : 'text'}
                        onChange={e => setEditingModel({ ...editingModel, [f.key]: e.target.value })} />
                    </div>
                  ))}
                  <CheckRow id={`use-proxy-${m.id}`} checked={editingModel.use_proxy} onChange={e => setEditingModel({ ...editingModel, use_proxy: e.target.checked })} label="Use Proxy" />
                  <div>
                    <Label label="Proxy URL" />
                    <input className="input-field" value={editingModel.proxy_url} disabled={!editingModel.use_proxy}
                      onChange={e => setEditingModel({ ...editingModel, proxy_url: e.target.value })} />
                  </div>
                  <CheckRow id={`default-${m.id}`} checked={editingModel.is_default} onChange={e => setEditingModel({ ...editingModel, is_default: e.target.checked })} label={t('models.is_default')} />
                  <div className="flex gap-2 mt-2">
                    <button className="btn-primary" onClick={handleUpdate}><Check size={14} /> {t('models.save')}</button>
                    <button className="btn-secondary" onClick={() => setEditingModel(null)}><X size={14} /> {t('models.cancel')}</button>
                  </div>
                </div>
              </div>
            ) : (
              <div>
                <div className="flex justify-between items-start">
                  <div className="flex items-center gap-2.5">
                    <Cpu size={18} color={m.is_default ? '#00d4aa' : '#74b9ff'} />
                    <div>
                      <span className="text-[15px] font-semibold text-[var(--text-primary)]">{m.name}</span>
                      {m.is_default && <span className="badge ml-2" style={{ background: 'rgba(0,212,170,0.15)', color: '#00d4aa' }}>{t('models.is_default')}</span>}
                    </div>
                  </div>
                  <div className="flex gap-1">
                    <button className="btn-secondary !py-1 !px-2" onClick={() => startEdit(m)} title={t('models.edit')}><Edit size={12} /></button>
                    <button className="btn-secondary !py-1 !px-2" onClick={() => setModelToDelete(m)} title={t('models.delete')} style={{ color: '#ff7675' }}><Trash2 size={12} /></button>
                  </div>
                </div>
                <div className="mt-3 text-xs text-[var(--text-secondary)]">
                  <p><span className="text-[var(--text-tertiary)]">{t('models.upstream_url')}:</span> {m.upstream_url}</p>
                  <p><span className="text-[var(--text-tertiary)]">{t('models.upstream_model')}:</span> {m.upstream_model}</p>
                  <p><span className="text-[var(--text-tertiary)]">{t('models.supported_versions')}:</span> {Array.isArray(m.supported_versions) ? m.supported_versions.join(', ') : m.supported_versions}</p>
                </div>
                <div className="mt-3">
                  <span className="badge" style={{ background: m.status === 'active' ? 'rgba(0,212,170,0.15)' : 'rgba(253,203,110,0.15)', color: m.status === 'active' ? '#00d4aa' : '#fdcb6e' }}>{m.status === 'active' ? t('models.active') : t('models.inactive')}</span>
                </div>
              </div>
            )}
          </GlassCard>
        ))}
      </div>

      {isCreating && (
        <div className="fixed inset-0 z-[1000] flex items-center justify-center" style={{ background: 'rgba(0,0,0,0.7)' }}>
          <GlassCard className="p-6 w-[450px] max-w-[90vw]">
            <div className="flex justify-between items-center mb-5">
              <span className="text-base font-semibold text-[var(--text-primary)]">{t('models.add')}</span>
              <X size={18} onClick={() => { setIsCreating(false); resetForm(); }} className="cursor-pointer text-[var(--text-tertiary)]" />
            </div>
            <div className="flex flex-col gap-4">
              {formFields.map(f => (
                <div key={f.key}>
                  <FieldLabel label={f.label} />
                  <input className="input-field" value={formData[f.key]} placeholder={f.placeholder}
                    type={f.type === 'password' ? 'password' : 'text'}
                    onChange={e => setFormData({ ...formData, [f.key]: e.target.value })} />
                </div>
              ))}
              <CheckRow id="new-use-proxy" checked={formData.use_proxy} onChange={e => setFormData({ ...formData, use_proxy: e.target.checked })} label="Use Proxy" />
              <div>
                <FieldLabel label="Proxy URL" />
                <input className="input-field" value={formData.proxy_url} disabled={!formData.use_proxy} onChange={e => setFormData({ ...formData, proxy_url: e.target.value })} />
              </div>
              <CheckRow id="new-is-default" checked={formData.is_default} onChange={e => setFormData({ ...formData, is_default: e.target.checked })} label={t('models.is_default')} />
              <div className="flex gap-2.5 mt-2">
                <button className="btn-primary" onClick={handleCreate}><Check size={14} /> {t('models.save')}</button>
                <button className="btn-secondary" onClick={() => { setIsCreating(false); resetForm(); }}>{t('models.cancel')}</button>
              </div>
            </div>
          </GlassCard>
        </div>
      )}

      <ConfirmDialog open={!!modelToDelete} onClose={() => setModelToDelete(null)} onConfirm={handleConfirmDelete} title="删除模型" message={`确定删除 ${modelToDelete?.name || ''}？删除后无法恢复。`} confirmText="删除" danger />
    </motion.div>
  );
}