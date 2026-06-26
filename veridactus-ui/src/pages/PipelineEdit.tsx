import { useEffect, useState } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import { motion } from 'framer-motion';
import GlassCard from '../components/ui/GlassCard';
import { useI18n } from '../i18n';
import { toast } from '../components/ui/Toast';
import { getPipeline, updatePipeline, createPipeline } from '../api';
import type { Pipeline } from '../types';
import {
  GitBranch, ArrowLeft, Save, Plus, Trash2, Settings,
  CheckCircle, XCircle, Loader2, ChevronDown
} from 'lucide-react';

interface StageConfig {
  placement: string;
  parallel: boolean;
  plugins: { name: string; type: string; config: string; enabled: boolean }[];
}

const placements = [
  { value: 'pre_request', label: '请求前处理' },
  { value: 'streaming', label: '流式处理' },
  { value: 'post_response', label: '响应后处理' },
  { value: 'async', label: '异步处理' },
];

const pluginTypes = ['native', 'wasm', 'grpc'];

export default function PipelineEdit() {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const { t } = useI18n();
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [pipeline, setPipeline] = useState<Pipeline | null>(null);
  const [stages, setStages] = useState<StageConfig[]>([]);
  const [tenant, setTenant] = useState('');
  const [showStageMenu, setShowStageMenu] = useState<string | null>(null);

  useEffect(() => {
    if (id) {
      loadPipeline();
    } else {
      setStages([{ placement: 'pre_request', parallel: false, plugins: [] }]);
      setTenant('default');
      setLoading(false);
    }
  }, [id]);

  async function loadPipeline() {
    try {
      const data = await getPipeline(id!);
      setPipeline(data);
      setTenant(data.tenant || 'default');
      setStages(data.stages || [{ placement: 'pre_request', parallel: false, plugins: [] }]);
    } catch (err) {
      console.error('Failed to load pipeline:', err);
    } finally {
      setLoading(false);
    }
  }

  const addStage = () => {
    setStages(prev => [...prev, { placement: 'pre_request', parallel: false, plugins: [] }]);
  };

  const removeStage = (index: number) => {
    if (stages.length <= 1) return;
    setStages(prev => prev.filter((_, i) => i !== index));
  };

  const updateStage = (index: number, updates: Partial<StageConfig>) => {
    setStages(prev => prev.map((s, i) => i === index ? { ...s, ...updates } : s));
  };

  const addPlugin = (stageIndex: number) => {
    setStages(prev => prev.map((s, i) => {
      if (i === stageIndex) {
        return { ...s, plugins: [...s.plugins, { name: '', type: 'native', config: '{}', enabled: true }] };
      }
      return s;
    }));
  };

  const removePlugin = (stageIndex: number, pluginIndex: number) => {
    setStages(prev => prev.map((s, i) => {
      if (i === stageIndex) {
        return { ...s, plugins: s.plugins.filter((_, j) => j !== pluginIndex) };
      }
      return s;
    }));
  };

  const updatePlugin = (stageIndex: number, pluginIndex: number, updates: Partial<{ name: string; type: string; config: string; enabled: boolean }>) => {
    setStages(prev => prev.map((s, i) => {
      if (i === stageIndex) {
        return {
          ...s,
          plugins: s.plugins.map((p, j) => j === pluginIndex ? { ...p, ...updates } : p)
        };
      }
      return s;
    }));
  };

  const handleSave = async () => {
    setSaving(true);
    try {
      const payload = { tenant, stages };
      if (id) {
        await updatePipeline(id, payload);
      } else {
        await createPipeline(payload);
      }
      navigate('/pipelines');
    } catch (err) {
      toast.error('保存失败');
      console.error(err);
    } finally {
      setSaving(false);
    }
  };

  if (loading) return <div className="flex justify-center items-center h-[50vh]"><Loader2 size={32} className="animate-spin text-[#6c5ce7]" /></div>;

  return (
    <motion.div initial={{ opacity: 0 }} animate={{ opacity: 1 }} className="relative z-[1]">
      <div className="absolute -top-[100px] -right-[100px] w-[400px] h-[400px] rounded-full pointer-events-none" style={{ background: 'radial-gradient(circle, rgba(108,92,231,0.1) 0%, transparent 70%)' }} />

      <motion.div initial={{ y: -20, opacity: 0 }} animate={{ y: 0, opacity: 1 }} className="flex items-center gap-5 mb-8">
        <motion.button onClick={() => navigate('/pipelines')} whileHover={{ scale: 1.05 }} whileTap={{ scale: 0.95 }}
          className="w-11 h-11 rounded-xl flex items-center justify-center cursor-pointer" style={{ background: 'rgba(255,255,255,0.1)', border: '1px solid var(--border-default)' }}>
          <ArrowLeft size={20} />
        </motion.button>
        <div>
          <h1 className="text-3xl font-bold text-[var(--text-primary)] tracking-tight">{id ? '编辑流水线' : '新建流水线'}</h1>
          <p className="text-sm text-[var(--text-secondary)] mt-1">配置治理流水线的各个阶段和插件</p>
        </div>
        <motion.button className="btn-primary flex gap-2 items-center ml-auto" onClick={handleSave} disabled={saving} whileHover={{ scale: 1.02 }} whileTap={{ scale: 0.98 }}>
          {saving ? <Loader2 size={18} className="animate-spin" /> : <Save size={18} />}
          {saving ? '保存中...' : '保存'}
        </motion.button>
      </motion.div>

      <GlassCard className="p-6 mb-6">
        <h3 className="text-sm font-semibold text-[var(--text-primary)] mb-4 flex items-center gap-2"><Settings size={16} className="text-[#6c5ce7]" /> 基本信息</h3>
        <div className="flex gap-6">
          <div className="flex-1">
            <label className="text-xs text-[var(--text-tertiary)] block mb-1.5">租户 ID</label>
            <input className="input-field" value={tenant} onChange={e => setTenant(e.target.value)} placeholder="输入租户ID" />
          </div>
          <div className="flex-1">
            <label className="text-xs text-[var(--text-tertiary)] block mb-1.5">流水线 ID</label>
            <input className="input-field" value={pipeline?.plan_id || '新建'} disabled style={{ background: 'rgba(0,0,0,0.1)', color: 'var(--text-secondary)' }} />
          </div>
        </div>
      </GlassCard>

      <GlassCard className="p-0 overflow-hidden">
        <div className="p-6 border-b border-[var(--border-default)] flex justify-between items-center">
          <h3 className="text-sm font-semibold text-[var(--text-primary)] flex items-center gap-2"><GitBranch size={16} className="text-[#6c5ce7]" /> 流水线阶段</h3>
          <motion.button onClick={addStage} whileHover={{ scale: 1.05 }} whileTap={{ scale: 0.95 }}
            className="py-2 px-4 rounded-lg text-xs font-semibold flex items-center gap-1.5 cursor-pointer" style={{ background: 'rgba(108,92,231,0.1)', color: '#6c5ce7', border: '1px solid rgba(108,92,231,0.3)' }}>
            <Plus size={14} /> 添加阶段
          </motion.button>
        </div>

        <div className="p-6 flex flex-col gap-4">
          {stages.map((stage, stageIndex) => (
            <motion.div
              key={stageIndex}
              initial={{ opacity: 0, y: 10 }}
              animate={{ opacity: 1, y: 0 }}
              style={{
                padding: 20,
                background: 'rgba(255,255,255,0.03)',
                borderRadius: 12,
                border: '1px solid var(--border-default)',
              }}
            >
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start', marginBottom: 16 }}>
                <div style={{ display: 'flex', gap: 12 }}>
                  <div style={{
                    width: 36, height: 36, borderRadius: 10,
                    background: `linear-gradient(135deg, rgba(${stageIndex % 4 === 0 ? '0,212,170' : stageIndex % 4 === 1 ? '116,185,255' : stageIndex % 4 === 2 ? '162,155,254' : '253,203,110'},0.2) 0%, rgba(0,0,0,0) 100%)`,
                    display: 'flex', alignItems: 'center', justifyContent: 'center',
                  }}>
                    <span style={{ fontSize: 12, fontWeight: 700, color: stageIndex % 4 === 0 ? '#00d4aa' : stageIndex % 4 === 1 ? '#74b9ff' : stageIndex % 4 === 2 ? '#a29bfe' : '#fdcb6e' }}>
                      {stageIndex + 1}
                    </span>
                  </div>
                  <div>
                    <div style={{ display: 'flex', gap: 12, alignItems: 'center' }}>
                      <div style={{ position: 'relative' }}>
                        <button
                          onClick={() => setShowStageMenu(showStageMenu === `stage-${stageIndex}` ? null : `stage-${stageIndex}`)}
                          style={{
                            padding: '8px 16px',
                            background: 'rgba(255,255,255,0.05)',
                            border: '1px solid var(--border-default)',
                            borderRadius: 8,
                            color: 'var(--text-primary)',
                            fontSize: 14,
                            fontWeight: 500,
                            display: 'flex', alignItems: 'center', gap: 8,
                            cursor: 'pointer',
                          }}
                        >
                          {placements.find(p => p.value === stage.placement)?.label || stage.placement}
                          <ChevronDown size={14} />
                        </button>
                        {showStageMenu === `stage-${stageIndex}` && (
                          <motion.div
                            initial={{ opacity: 0, y: 5 }}
                            animate={{ opacity: 1, y: 0 }}
                            style={{
                              position: 'absolute',
                              top: '100%',
                              left: 0,
                              marginTop: 4,
                              background: 'var(--bg-primary)',
                              border: '1px solid var(--border-default)',
                              borderRadius: 8,
                              padding: 4,
                              minWidth: 150,
                              boxShadow: '0 8px 32px rgba(0,0,0,0.2)',
                              zIndex: 10,
                            }}
                          >
                            {placements.map(p => (
                              <button
                                key={p.value}
                                onClick={() => {
                                  updateStage(stageIndex, { placement: p.value });
                                  setShowStageMenu(null);
                                }}
                                style={{
                                  width: '100%',
                                  padding: '8px 12px',
                                  textAlign: 'left',
                                  background: 'transparent',
                                  border: 'none',
                                  borderRadius: 6,
                                  color: 'var(--text-primary)',
                                  fontSize: 13,
                                  cursor: 'pointer',
                                }}
                              >
                                {p.label}
                              </button>
                            ))}
                          </motion.div>
                        )}
                      </div>
                      <label style={{ display: 'flex', alignItems: 'center', gap: 6, cursor: 'pointer' }}>
                        <input
                          type="checkbox"
                          checked={stage.parallel}
                          onChange={(e) => updateStage(stageIndex, { parallel: e.target.checked })}
                          style={{ width: 16, height: 16 }}
                        />
                        <span style={{ fontSize: 13, color: 'var(--text-secondary)' }}>并行执行</span>
                      </label>
                    </div>
                    <p style={{ fontSize: 12, color: 'var(--text-tertiary)', marginTop: 4 }}>
                      {stage.plugins.length} 个插件
                    </p>
                  </div>
                </div>
                {stages.length > 1 && (
                  <motion.button
                    onClick={() => removeStage(stageIndex)}
                    whileHover={{ scale: 1.1, color: '#ff6b6b' }}
                    whileTap={{ scale: 0.9 }}
                    style={{
                      padding: 8,
                      background: 'rgba(255,107,107,0.1)',
                      border: 'none',
                      borderRadius: 8,
                      color: '#ff6b6b',
                      cursor: 'pointer',
                    }}
                  >
                    <Trash2 size={16} />
                  </motion.button>
                )}
              </div>

              <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
                {stage.plugins.map((plugin, pluginIndex) => (
                  <div
                    key={pluginIndex}
                    style={{
                      padding: 16,
                      background: 'rgba(0,0,0,0.1)',
                      borderRadius: 10,
                      display: 'flex', gap: 16,
                    }}
                  >
                    <div style={{ flex: 1 }}>
                      <input
                        type="text"
                        value={plugin.name}
                        onChange={(e) => updatePlugin(stageIndex, pluginIndex, { name: e.target.value })}
                        style={{
                          width: '100%', padding: '10px 14px',
                          background: 'rgba(255,255,255,0.05)',
                          border: '1px solid var(--border-default)',
                          borderRadius: 8,
                          color: 'var(--text-primary)',
                          fontSize: 13,
                          marginBottom: 8,
                        }}
                        placeholder="插件名称"
                      />
                      <div style={{ display: 'flex', gap: 12 }}>
                        <select
                          value={plugin.type}
                          onChange={(e) => updatePlugin(stageIndex, pluginIndex, { type: e.target.value })}
                          style={{
                            padding: '8px 12px',
                            background: 'rgba(255,255,255,0.05)',
                            border: '1px solid var(--border-default)',
                            borderRadius: 8,
                            color: 'var(--text-primary)',
                            fontSize: 13,
                          }}
                        >
                          {pluginTypes.map(t => (
                            <option key={t} value={t}>{t}</option>
                          ))}
                        </select>
                        <div style={{ flex: 1 }}>
                          <input
                            type="text"
                            value={plugin.config}
                            onChange={(e) => updatePlugin(stageIndex, pluginIndex, { config: e.target.value })}
                            style={{
                              width: '100%', padding: '8px 12px',
                              background: 'rgba(255,255,255,0.05)',
                              border: '1px solid var(--border-default)',
                              borderRadius: 8,
                              color: 'var(--text-primary)',
                              fontSize: 13,
                              fontFamily: 'monospace',
                            }}
                            placeholder='{"key": "value"}'
                          />
                        </div>
                        <button
                          onClick={() => updatePlugin(stageIndex, pluginIndex, { enabled: !plugin.enabled })}
                          style={{
                            padding: 8,
                            background: plugin.enabled ? 'rgba(0,212,170,0.1)' : 'rgba(255,107,107,0.1)',
                            border: 'none',
                            borderRadius: 8,
                            color: plugin.enabled ? '#00d4aa' : '#ff6b6b',
                            cursor: 'pointer',
                          }}
                        >
                          {plugin.enabled ? <CheckCircle size={18} /> : <XCircle size={18} />}
                        </button>
                      </div>
                    </div>
                    <button
                      onClick={() => removePlugin(stageIndex, pluginIndex)}
                      style={{
                        padding: 8,
                        background: 'rgba(255,107,107,0.1)',
                        border: 'none',
                        borderRadius: 8,
                        color: '#ff6b6b',
                        cursor: 'pointer',
                      }}
                    >
                      <Trash2 size={16} />
                    </button>
                  </div>
                ))}
                <motion.button
                  onClick={() => addPlugin(stageIndex)}
                  whileHover={{ scale: 1.02 }}
                  whileTap={{ scale: 0.98 }}
                  style={{
                    padding: '10px 16px',
                    background: 'transparent',
                    border: '1px dashed var(--border-default)',
                    borderRadius: 8,
                    color: 'var(--text-secondary)',
                    fontSize: 13,
                    display: 'flex', alignItems: 'center', gap: 6,
                    cursor: 'pointer',
                  }}
                >
                  <Plus size={14} /> 添加插件
                </motion.button>
              </div>
            </motion.div>
          ))}
        </div>
      </GlassCard>
    </motion.div>
  );
}