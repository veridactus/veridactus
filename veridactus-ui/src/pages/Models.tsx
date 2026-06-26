import { useState, useEffect } from 'react';
import { motion } from 'framer-motion';
import GlassCard from '../components/ui/GlassCard';
import { useI18n } from '../i18n';
import { Cpu, Plus, Edit, Trash2, Loader, Check, X, ArrowLeft } from 'lucide-react';
import { getModelsConfig, createModel, updateModel, deleteModel } from '../api';
import type { ModelConfig } from '../types';

interface EditingModel {
  id: string;
  name: string;
  upstream_url: string;
  upstream_model: string;
  is_default: boolean;
  supported_versions: string;
  status: string;
  api_key: string;
  api_key_header: string;
  use_proxy: boolean;
  proxy_url: string;
}

export default function Models() {
  const { t } = useI18n();
  const [models, setModels] = useState<ModelConfig[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [editingModel, setEditingModel] = useState<EditingModel | null>(null);
  const [isCreating, setIsCreating] = useState(false);
  const [formData, setFormData] = useState({
    name: '',
    upstream_url: '',
    upstream_model: '',
    is_default: false,
    supported_versions: '0.1,0.2',
    status: 'active',
    api_key: '',
    api_key_header: '',
    use_proxy: false,
    proxy_url: '',
  });

  const loadModels = async () => {
    try {
      setLoading(true);
      setError(null);
      const data = await getModelsConfig();
      setModels(data);
    } catch (err) {
      setError('Failed to load models');
      console.error(err);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadModels();
  }, []);

  const handleCreate = async () => {
    try {
      await createModel({
        name: formData.name,
        upstream_url: formData.upstream_url,
        upstream_model: formData.upstream_model || formData.name,
        is_default: formData.is_default,
        supported_versions: formData.supported_versions.split(',').map(v => v.trim()),
        status: formData.status,
        api_key: formData.api_key || undefined,
        api_key_header: formData.api_key_header || undefined,
        use_proxy: formData.use_proxy,
        proxy_url: formData.use_proxy ? formData.proxy_url : undefined,
      });
      setIsCreating(false);
      resetForm();
      await loadModels();
    } catch (err) {
      alert('Failed to create model');
      console.error(err);
    }
  };

  const handleUpdate = async () => {
    if (!editingModel) return;
    try {
      await updateModel(editingModel.id, {
        name: editingModel.name,
        upstream_url: editingModel.upstream_url,
        upstream_model: editingModel.upstream_model,
        is_default: editingModel.is_default,
        supported_versions: editingModel.supported_versions.split(',').map(v => v.trim()),
        status: editingModel.status,
        api_key: editingModel.api_key || undefined,
        api_key_header: editingModel.api_key_header || undefined,
        use_proxy: editingModel.use_proxy,
        proxy_url: editingModel.use_proxy ? editingModel.proxy_url : undefined,
      });
      setEditingModel(null);
      await loadModels();
    } catch (err) {
      alert('Failed to update model');
      console.error(err);
    }
  };

  const handleDelete = async (model: ModelConfig) => {
    if (!confirm(t('models.confirm_delete'))) return;
    try {
      await deleteModel(model.id);
      await loadModels();
    } catch (err) {
      alert('Failed to delete model');
      console.error(err);
    }
  };

  const resetForm = () => {
    setFormData({
      name: '',
      upstream_url: '',
      upstream_model: '',
      is_default: false,
      supported_versions: '0.1,0.2',
      status: 'active',
      api_key: '',
      api_key_header: '',
      use_proxy: false,
      proxy_url: '',
    });
  };

  const startEdit = (model: ModelConfig) => {
    const supportedVersions = Array.isArray(model.supported_versions)
      ? model.supported_versions.join(',')
      : model.supported_versions || '0.1,0.2';
    setEditingModel({
      id: model.id,
      name: model.name,
      upstream_url: model.upstream_url,
      upstream_model: model.upstream_model,
      is_default: model.is_default,
      supported_versions: supportedVersions,
      status: model.status,
      api_key: model.api_key || '',
      api_key_header: model.api_key_header || '',
      use_proxy: model.use_proxy || false,
      proxy_url: model.proxy_url || '',
    });
  };

  return (
    <motion.div initial={{ opacity: 0 }} animate={{ opacity: 1 }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 24 }}>
        <div>
          <h1 style={{ fontSize: 24, fontWeight: 700, color: 'var(--text-primary)' }}>{t('models.title')}</h1>
          <p style={{ color: 'var(--text-secondary)', fontSize: 13, marginTop: 4 }}>{t('models.subtitle')}</p>
        </div>
        <button className="btn-primary" onClick={() => setIsCreating(true)}>
          <Plus size={16} /> {t('models.add')}
        </button>
      </div>

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

      {!loading && !error && models.length === 0 && (
        <GlassCard style={{ padding: 40, textAlign: 'center' }}>
          <Cpu size={40} color="var(--text-tertiary)" style={{ margin: '0 auto 16px' }} />
          <p style={{ color: 'var(--text-secondary)', fontSize: 14 }}>{t('models.no_models')}</p>
          <p style={{ color: 'var(--text-tertiary)', fontSize: 12, marginTop: 8 }}>{t('models.add_first')}</p>
          <button className="btn-primary" style={{ marginTop: 16 }} onClick={() => setIsCreating(true)}>
            <Plus size={14} /> {t('models.add')}
          </button>
        </GlassCard>
      )}

      <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fill, minmax(350px, 1fr))', gap: 16 }}>
        {models.map(m => (
          <GlassCard key={m.id} style={{ padding: 20 }}>
            {editingModel?.id === m.id ? (
              <div>
                <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 16 }}>
                  <ArrowLeft size={16} onClick={() => setEditingModel(null)} style={{ cursor: 'pointer' }} />
                  <span style={{ fontSize: 15, fontWeight: 600, color: 'var(--text-primary)' }}>{t('models.edit')}</span>
                </div>
                <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
                  <div>
                    <label style={{ display: 'block', fontSize: 11, color: 'var(--text-tertiary)', marginBottom: 4 }}>{t('models.name')}</label>
                    <input
                      className="input-field"
                      value={editingModel.name}
                      onChange={e => setEditingModel({ ...editingModel, name: e.target.value })}
                    />
                  </div>
                  <div>
                    <label style={{ display: 'block', fontSize: 11, color: 'var(--text-tertiary)', marginBottom: 4 }}>{t('models.upstream_url')}</label>
                    <input
                      className="input-field"
                      value={editingModel.upstream_url}
                      onChange={e => setEditingModel({ ...editingModel, upstream_url: e.target.value })}
                    />
                  </div>
                  <div>
                    <label style={{ display: 'block', fontSize: 11, color: 'var(--text-tertiary)', marginBottom: 4 }}>{t('models.upstream_model')}</label>
                    <input
                      className="input-field"
                      value={editingModel.upstream_model}
                      onChange={e => setEditingModel({ ...editingModel, upstream_model: e.target.value })}
                    />
                  </div>
                  <div>
                    <label style={{ display: 'block', fontSize: 11, color: 'var(--text-tertiary)', marginBottom: 4 }}>{t('models.supported_versions')}</label>
                    <input
                      className="input-field"
                      value={editingModel.supported_versions}
                      onChange={e => setEditingModel({ ...editingModel, supported_versions: e.target.value })}
                      placeholder="0.1,0.2"
                    />
                  </div>
                  <div>
                    <label style={{ display: 'block', fontSize: 11, color: 'var(--text-tertiary)', marginBottom: 4 }}>API Key</label>
                    <input
                      className="input-field"
                      type="password"
                      value={editingModel.api_key}
                      onChange={e => setEditingModel({ ...editingModel, api_key: e.target.value })}
                      placeholder="Your API key"
                    />
                  </div>
                  <div>
                    <label style={{ display: 'block', fontSize: 11, color: 'var(--text-tertiary)', marginBottom: 4 }}>API Key Header</label>
                    <input
                      className="input-field"
                      value={editingModel.api_key_header}
                      onChange={e => setEditingModel({ ...editingModel, api_key_header: e.target.value })}
                      placeholder="X-goog-api-key or Authorization"
                    />
                  </div>
                  <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                    <input
                      type="checkbox"
                      id={`use-proxy-${m.id}`}
                      checked={editingModel.use_proxy}
                      onChange={e => setEditingModel({ ...editingModel, use_proxy: e.target.checked })}
                    />
                    <label htmlFor={`use-proxy-${m.id}`} style={{ fontSize: 12, color: 'var(--text-secondary)' }}>Use Proxy</label>
                  </div>
                  <div>
                    <label style={{ display: 'block', fontSize: 11, color: 'var(--text-tertiary)', marginBottom: 4 }}>Proxy URL</label>
                    <input
                      className="input-field"
                      value={editingModel.proxy_url}
                      onChange={e => setEditingModel({ ...editingModel, proxy_url: e.target.value })}
                      placeholder=""
                      disabled={!editingModel.use_proxy}
                    />
                  </div>
                  <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                    <input
                      type="checkbox"
                      id={`default-${m.id}`}
                      checked={editingModel.is_default}
                      onChange={e => setEditingModel({ ...editingModel, is_default: e.target.checked })}
                    />
                    <label htmlFor={`default-${m.id}`} style={{ fontSize: 12, color: 'var(--text-secondary)' }}>{t('models.is_default')}</label>
                  </div>
                  <div style={{ display: 'flex', gap: 8, marginTop: 8 }}>
                    <button className="btn-primary" onClick={handleUpdate}><Check size={14} /> {t('models.save')}</button>
                    <button className="btn-secondary" onClick={() => setEditingModel(null)}><X size={14} /> {t('models.cancel')}</button>
                  </div>
                </div>
              </div>
            ) : (
              <div>
                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start' }}>
                  <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
                    <Cpu size={18} color={m.is_default ? '#00d4aa' : '#74b9ff'} />
                    <div>
                      <span style={{ fontSize: 15, fontWeight: 600, color: 'var(--text-primary)' }}>{m.name}</span>
                      {m.is_default && (
                        <span className="badge" style={{ marginLeft: 8, background: 'rgba(0,212,170,0.15)', color: '#00d4aa' }}>{t('models.is_default')}</span>
                      )}
                    </div>
                  </div>
                  <div style={{ display: 'flex', gap: 4 }}>
                    <button className="btn-secondary" style={{ padding: '4px 8px' }} onClick={() => startEdit(m)} title={t('models.edit')}>
                      <Edit size={12} />
                    </button>
                    <button className="btn-secondary" style={{ padding: '4px 8px', color: '#ff7675' }} onClick={() => handleDelete(m)} title={t('models.delete')}>
                      <Trash2 size={12} />
                    </button>
                  </div>
                </div>
                <div style={{ marginTop: 12, fontSize: 12, color: 'var(--text-secondary)' }}>
                  <p><span style={{ color: 'var(--text-tertiary)' }}>{t('models.upstream_url')}:</span> {m.upstream_url}</p>
                  <p><span style={{ color: 'var(--text-tertiary)' }}>{t('models.upstream_model')}:</span> {m.upstream_model}</p>
                  <p><span style={{ color: 'var(--text-tertiary)' }}>{t('models.supported_versions')}:</span> {Array.isArray(m.supported_versions) ? m.supported_versions.join(', ') : m.supported_versions}</p>
                </div>
                <div style={{ marginTop: 12 }}>
                  <span className="badge" style={{
                    background: m.status === 'active' ? 'rgba(0,212,170,0.15)' : 'rgba(253,203,110,0.15)',
                    color: m.status === 'active' ? '#00d4aa' : '#fdcb6e'
                  }}>
                    {m.status === 'active' ? t('models.active') : t('models.inactive')}
                  </span>
                </div>
              </div>
            )}
          </GlassCard>
        ))}
      </div>

      {isCreating && (
        <div style={{
          position: 'fixed', top: 0, left: 0, right: 0, bottom: 0,
          background: 'rgba(0,0,0,0.7)', display: 'flex', alignItems: 'center', justifyContent: 'center',
          zIndex: 1000
        }}>
          <GlassCard style={{ padding: 24, width: 450, maxWidth: '90vw' }}>
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 20 }}>
              <span style={{ fontSize: 16, fontWeight: 600, color: 'var(--text-primary)' }}>{t('models.add')}</span>
              <X size={18} onClick={() => { setIsCreating(false); resetForm(); }} style={{ cursor: 'pointer', color: 'var(--text-tertiary)' }} />
            </div>
            <div style={{ display: 'flex', flexDirection: 'column', gap: 16 }}>
              <div>
                <label style={{ display: 'block', fontSize: 12, fontWeight: 500, color: 'var(--text-secondary)', marginBottom: 6 }}>{t('models.name')}</label>
                <input
                  className="input-field"
                  value={formData.name}
                  onChange={e => setFormData({ ...formData, name: e.target.value })}
                  placeholder="deepseek-r1:14b"
                />
              </div>
              <div>
                <label style={{ display: 'block', fontSize: 12, fontWeight: 500, color: 'var(--text-secondary)', marginBottom: 6 }}>{t('models.upstream_url')}</label>
                <input
                  className="input-field"
                  value={formData.upstream_url}
                  onChange={e => setFormData({ ...formData, upstream_url: e.target.value })}
                />
              </div>
              <div>
                <label style={{ display: 'block', fontSize: 12, fontWeight: 500, color: 'var(--text-secondary)', marginBottom: 6 }}>{t('models.upstream_model')}</label>
                <input
                  className="input-field"
                  value={formData.upstream_model}
                  onChange={e => setFormData({ ...formData, upstream_model: e.target.value })}
                  placeholder={formData.name || 'model-name'}
                />
              </div>
              <div>
                <label style={{ display: 'block', fontSize: 12, fontWeight: 500, color: 'var(--text-secondary)', marginBottom: 6 }}>{t('models.supported_versions')}</label>
                <input
                  className="input-field"
                  value={formData.supported_versions}
                  onChange={e => setFormData({ ...formData, supported_versions: e.target.value })}
                  placeholder="0.1,0.2"
                />
              </div>
              <div>
                <label style={{ display: 'block', fontSize: 12, fontWeight: 500, color: 'var(--text-secondary)', marginBottom: 6 }}>API Key</label>
                <input
                  className="input-field"
                  type="password"
                  value={formData.api_key}
                  onChange={e => setFormData({ ...formData, api_key: e.target.value })}
                  placeholder="Your API key"
                />
              </div>
              <div>
                <label style={{ display: 'block', fontSize: 12, fontWeight: 500, color: 'var(--text-secondary)', marginBottom: 6 }}>API Key Header</label>
                <input
                  className="input-field"
                  value={formData.api_key_header}
                  onChange={e => setFormData({ ...formData, api_key_header: e.target.value })}
                  placeholder="X-goog-api-key or Authorization"
                />
              </div>
              <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                <input
                  type="checkbox"
                  id="new-use-proxy"
                  checked={formData.use_proxy}
                  onChange={e => setFormData({ ...formData, use_proxy: e.target.checked })}
                />
                <label htmlFor="new-use-proxy" style={{ fontSize: 13, color: 'var(--text-secondary)' }}>Use Proxy</label>
              </div>
              <div>
                <label style={{ display: 'block', fontSize: 12, fontWeight: 500, color: 'var(--text-secondary)', marginBottom: 6 }}>Proxy URL</label>
                <input
                  className="input-field"
                  value={formData.proxy_url}
                  onChange={e => setFormData({ ...formData, proxy_url: e.target.value })}
                  placeholder=""
                  disabled={!formData.use_proxy}
                />
              </div>
              <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                <input
                  type="checkbox"
                  id="new-is-default"
                  checked={formData.is_default}
                  onChange={e => setFormData({ ...formData, is_default: e.target.checked })}
                />
                <label htmlFor="new-is-default" style={{ fontSize: 13, color: 'var(--text-secondary)' }}>{t('models.is_default')}</label>
              </div>
              <div style={{ display: 'flex', gap: 10, marginTop: 8 }}>
                <button className="btn-primary" onClick={handleCreate}><Check size={14} /> {t('models.save')}</button>
                <button className="btn-secondary" onClick={() => { setIsCreating(false); resetForm(); }}>{t('models.cancel')}</button>
              </div>
            </div>
          </GlassCard>
        </div>
      )}

      <style>{`
        @keyframes spin { to { transform: rotate(360deg); } }
        .spin { animation: spin 0.8s linear infinite; }
      `}</style>
    </motion.div>
  );
}