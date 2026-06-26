import { useEffect, useState, useRef } from 'react';
import { useNavigate } from 'react-router-dom';
import { motion, AnimatePresence } from 'framer-motion';
import GlassCard from '../components/ui/GlassCard';
import { useI18n } from '../i18n';
import { getPipelines, createPipeline, deletePipeline } from '../api';
import type { Pipeline } from '../types';
import { ConfirmDialog } from '../components/ui/Dialog';
import { toast } from '../components/ui/Toast';
import {
  GitBranch, Plus, Play, CheckCircle, Zap, Clock, Shield,
  ChevronRight, Trash2, Copy, Star, MoreVertical, Sparkles,
  ArrowUpRight, ArrowDownRight, AlertCircle, Loader2, Settings
} from 'lucide-react';

const examplePipelines = [
  {
    tenant: 'acme-corp',
    stages: [
      { placement: 'pre_request', parallel: false, plugins: [
        { name: 'Budget Guard', type: 'native', config: '{"limit_usd":0.10}', enabled: true },
        { name: 'Auth Validator', type: 'native', config: '{}', enabled: true },
      ]},
      { placement: 'streaming', parallel: true, plugins: [
        { name: 'Keyword Filter', type: 'wasm', config: '{"patterns":["violence","hate"]}', enabled: true },
        { name: 'PII Masking', type: 'wasm', config: '{"level":"masked"}', enabled: true },
      ]},
      { placement: 'post_response', parallel: false, plugins: [
        { name: 'Trace Finalizer', type: 'native', config: '{}', enabled: true },
      ]},
      { placement: 'async', parallel: true, plugins: [
        { name: 'Drift Detector', type: 'grpc', config: '{"threshold":0.7}', enabled: true },
        { name: 'TEE Attestation', type: 'grpc', config: '{"platform":"tdx"}', enabled: true },
      ]},
    ],
  },
  {
    tenant: 'acme-corp',
    stages: [
      { placement: 'pre_request', parallel: false, plugins: [
        { name: 'Route Selector', type: 'native', config: '{"default":"deepseek-r1:14b"}', enabled: true },
      ]},
      { placement: 'streaming', parallel: false, plugins: [
        { name: 'PII Masking', type: 'wasm', config: '{}', enabled: true },
      ]},
      { placement: 'async', parallel: false, plugins: [
        { name: 'C-SafeGen', type: 'grpc', config: '{"methodology":"C-SafeGen_v1.0"}', enabled: true },
      ]},
    ],
  },
];

interface PipelineCardProps {
  pipeline: Pipeline;
  index: number;
  onEdit: (id: string) => void;
  onAdvancedEdit: (id: string) => void;
  onDelete: (id: string) => void;
}

function PipelineCard({ pipeline, index, onEdit, onAdvancedEdit, onDelete }: PipelineCardProps) {
  const [showMenu, setShowMenu] = useState(false);
  const [isHovered, setIsHovered] = useState(false);
  const menuRef = useRef<HTMLDivElement>(null);

  const stageCount = pipeline.stages?.length || 0;
  const pluginCount = pipeline.stages?.reduce((acc: number, s: any) => acc + (s.plugins?.length || 0), 0) || 0;

  const stageColors: Record<string, string> = {
    pre_request: '#00d4aa',
    streaming: '#74b9ff',
    post_response: '#a29bfe',
    async: '#fdcb6e',
  };

  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
        setShowMenu(false);
      }
    };
    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, []);

  return (
    <motion.div
      initial={{ opacity: 0, y: 20 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ delay: index * 0.1, duration: 0.4, ease: [0.23, 1, 0.32, 1] }}
      onHoverStart={() => setIsHovered(true)}
      onHoverEnd={() => setIsHovered(false)}
      style={{ position: 'relative' }}
    >
      <GlassCard
        style={{
          padding: 0,
          overflow: 'hidden',
          border: isHovered ? '1px solid rgba(108,92,231,0.4)' : '1px solid var(--border-default)',
          boxShadow: isHovered
            ? '0 0 40px rgba(108,92,231,0.2), 0 8px 32px rgba(0,0,0,0.3)'
            : '0 4px 20px rgba(0,0,0,0.2)',
          transition: 'all 0.3s ease',
        }}
      >
        {/* Gradient top border */}
        <div style={{
          height: 3,
          background: 'linear-gradient(90deg, #6c5ce7, #a29bfe, #f093fb)',
        }} />

        <div style={{ padding: 20 }}>
          {/* Header */}
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start', marginBottom: 16 }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
              <motion.div
                whileHover={{ scale: 1.1, rotate: 5 }}
                style={{
                  width: 48, height: 48, borderRadius: 14,
                  background: 'linear-gradient(135deg, rgba(108,92,231,0.2) 0%, rgba(162,155,254,0.2) 100%)',
                  border: '1px solid rgba(108,92,231,0.3)',
                  display: 'flex', alignItems: 'center', justifyContent: 'center',
                }}
              >
                <GitBranch size={22} style={{ color: '#6c5ce7' }} />
              </motion.div>
              <div>
                <h3 style={{ fontSize: 16, fontWeight: 700, color: 'var(--text-primary)', marginBottom: 2 }}>
                  {pipeline.plan_id?.slice(0, 12) || 'Unnamed Pipeline'}
                </h3>
                <p style={{ fontSize: 12, color: 'var(--text-tertiary)' }}>
                  {pipeline.tenant || 'default'} • {stageCount} stages
                </p>
              </div>
            </div>

            <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
              <span style={{
                padding: '4px 10px', borderRadius: 20, fontSize: 10, fontWeight: 700,
                background: 'rgba(0,212,170,0.1)', color: '#00d4aa',
                border: '1px solid rgba(0,212,170,0.2)',
              }}>
                <span style={{ display: 'flex', alignItems: 'center', gap: 4 }}>
                  <CheckCircle size={10} /> Active
                </span>
              </span>

              <div ref={menuRef} style={{ position: 'relative' }}>
                <motion.button
                  whileHover={{ scale: 1.1 }}
                  whileTap={{ scale: 0.9 }}
                  onClick={() => setShowMenu(!showMenu)}
                  style={{
                    background: 'rgba(255,255,255,0.05)', border: '1px solid var(--border-default)',
                    borderRadius: 8, padding: '6px 8px', cursor: 'pointer', color: 'var(--text-secondary)',
                  }}
                >
                  <MoreVertical size={16} />
                </motion.button>

                <AnimatePresence>
                  {showMenu && (
                    <motion.div
                      initial={{ opacity: 0, scale: 0.9, y: -10 }}
                      animate={{ opacity: 1, scale: 1, y: 0 }}
                      exit={{ opacity: 0, scale: 0.9, y: -10 }}
                      style={{
                        position: 'absolute', right: 0, top: '100%', marginTop: 8, zIndex: 100,
                        background: 'rgba(19, 22, 51, 0.98)',
                        backdropFilter: 'blur(16px)',
                        border: '1px solid rgba(108,92,231,0.3)',
                        borderRadius: 12, padding: 8, minWidth: 160,
                        boxShadow: '0 0 30px rgba(108,92,231,0.2), 0 8px 32px rgba(0,0,0,0.5)',
                      }}
                    >
                      <button
                        onClick={() => { onEdit(pipeline.plan_id!); setShowMenu(false); }}
                        style={{
                          width: '100%', display: 'flex', alignItems: 'center', gap: 10, padding: '10px 12px',
                          background: 'transparent', border: 'none', cursor: 'pointer', borderRadius: 8,
                          color: 'var(--text-primary)', fontSize: 13, transition: 'background 0.15s',
                        }}
                        onMouseEnter={e => e.currentTarget.style.background = 'rgba(108,92,231,0.1)'}
                        onMouseLeave={e => e.currentTarget.style.background = 'transparent'}
                      >
                        <Sparkles size={14} style={{ color: '#6c5ce7' }} /> 可视化设计
                      </button>
                      <button
                        onClick={() => { onAdvancedEdit(pipeline.plan_id!); setShowMenu(false); }}
                        style={{
                          width: '100%', display: 'flex', alignItems: 'center', gap: 10, padding: '10px 12px',
                          background: 'transparent', border: 'none', cursor: 'pointer', borderRadius: 8,
                          color: 'var(--text-primary)', fontSize: 13, transition: 'background 0.15s',
                        }}
                        onMouseEnter={e => e.currentTarget.style.background = 'rgba(108,92,231,0.1)'}
                        onMouseLeave={e => e.currentTarget.style.background = 'transparent'}
                      >
                        <Settings size={14} style={{ color: '#74b9ff' }} /> 高级配置
                      </button>
                      <button
                        onClick={() => { setShowMenu(false); onDelete(pipeline.plan_id!); }}
                        style={{
                          width: '100%', display: 'flex', alignItems: 'center', gap: 10, padding: '10px 12px',
                          background: 'transparent', border: 'none', cursor: 'pointer', borderRadius: 8,
                          color: 'var(--text-primary)', fontSize: 13, transition: 'background 0.15s',
                        }}
                        onMouseEnter={e => e.currentTarget.style.background = 'rgba(255,118,117,0.1)'}
                        onMouseLeave={e => e.currentTarget.style.background = 'transparent'}
                      >
                        <Trash2 size={14} style={{ color: '#ff7675' }} /> 删除
                      </button>
                    </motion.div>
                  )}
                </AnimatePresence>
              </div>
            </div>
          </div>

          {/* Stats */}
          <div style={{ display: 'flex', gap: 16, marginBottom: 16 }}>
            <div style={{
              flex: 1, padding: '12px 14px', borderRadius: 10,
              background: 'linear-gradient(135deg, rgba(108,92,231,0.08) 0%, rgba(108,92,231,0.04) 100%)',
              border: '1px solid rgba(108,92,231,0.1)',
            }}>
              <div style={{ display: 'flex', alignItems: 'center', gap: 6, marginBottom: 4 }}>
                <Zap size={12} style={{ color: '#6c5ce7' }} />
                <span style={{ fontSize: 10, color: 'var(--text-tertiary)', fontWeight: 600, letterSpacing: '0.05em' }}>STAGES</span>
              </div>
              <p style={{ fontSize: 20, fontWeight: 700, color: 'var(--text-primary)' }}>{stageCount}</p>
            </div>

            <div style={{
              flex: 1, padding: '12px 14px', borderRadius: 10,
              background: 'linear-gradient(135deg, rgba(0,212,170,0.08) 0%, rgba(0,212,170,0.04) 100%)',
              border: '1px solid rgba(0,212,170,0.1)',
            }}>
              <div style={{ display: 'flex', alignItems: 'center', gap: 6, marginBottom: 4 }}>
                <Shield size={12} style={{ color: '#00d4aa' }} />
                <span style={{ fontSize: 10, color: 'var(--text-tertiary)', fontWeight: 600, letterSpacing: '0.05em' }}>PLUGINS</span>
              </div>
              <p style={{ fontSize: 20, fontWeight: 700, color: 'var(--text-primary)' }}>{pluginCount}</p>
            </div>

            <div style={{
              flex: 1, padding: '12px 14px', borderRadius: 10,
              background: 'linear-gradient(135deg, rgba(253,203,110,0.08) 0%, rgba(253,203,110,0.04) 100%)',
              border: '1px solid rgba(253,203,110,0.1)',
            }}>
              <div style={{ display: 'flex', alignItems: 'center', gap: 6, marginBottom: 4 }}>
                <Clock size={12} style={{ color: '#fdcb6e' }} />
                <span style={{ fontSize: 10, color: 'var(--text-tertiary)', fontWeight: 600, letterSpacing: '0.05em' }}>STATUS</span>
              </div>
              <p style={{ fontSize: 14, fontWeight: 700, color: '#00d4aa' }}>Active</p>
            </div>
          </div>

          {/* Stage badges */}
          <div style={{ display: 'flex', flexWrap: 'wrap', gap: 6, marginBottom: 16 }}>
            {pipeline.stages?.map((s: any, i: number) => (
              <span
                key={i}
                style={{
                  padding: '4px 10px', borderRadius: 6, fontSize: 10, fontWeight: 600,
                  background: `${stageColors[s.placement] || '#6c5ce7'}15`,
                  color: stageColors[s.placement] || '#6c5ce7',
                  border: `1px solid ${stageColors[s.placement] || '#6c5ce7'}30`,
                }}
              >
                {s.placement} ({s.plugins?.length || 0})
              </span>
            ))}
          </div>

          {/* Actions */}
          <div style={{ display: 'flex', gap: 8 }}>
            <motion.button
              className="btn-primary"
              whileHover={{ scale: 1.02 }}
              whileTap={{ scale: 0.98 }}
              onClick={() => onEdit(pipeline.plan_id!)}
              style={{ flex: 1, display: 'flex', gap: 8, alignItems: 'center', justifyContent: 'center' }}
            >
              <Sparkles size={14} />
              编辑流水线
            </motion.button>
            <motion.button
              whileHover={{ scale: 1.05 }}
              whileTap={{ scale: 0.95 }}
              style={{
                padding: '10px 16px', borderRadius: 10,
                background: 'linear-gradient(135deg, rgba(0,212,170,0.1) 0%, rgba(0,212,170,0.05) 100%)',
                border: '1px solid rgba(0,212,170,0.3)',
                color: '#00d4aa', cursor: 'pointer', fontSize: 13, fontWeight: 600,
              }}
            >
              <Play size={14} />
            </motion.button>
          </div>
        </div>
      </GlassCard>
    </motion.div>
  );
}

export default function Pipelines() {
  const navigate = useNavigate();
  const { t } = useI18n();
  const [pipelines, setPipelines] = useState<Pipeline[]>([]);
  const [loading, setLoading] = useState(true);
  const [creating, setCreating] = useState(false);
  const [deleteId, setDeleteId] = useState<string | null>(null);
  const seeded = useRef(false);

  useEffect(() => {
    (async () => {
      try {
        let list = await getPipelines();
        if (list.length === 0 && !seeded.current) {
          seeded.current = true;
          for (const ex of examplePipelines) {
            try { await createPipeline(ex); } catch {}
          }
          list = await getPipelines();
        }
        setPipelines(list);
      } catch {} finally {
        setLoading(false);
      }
    })();
  }, []);

  const handleCreate = () => {
    navigate('/pipelines/new');
  };

  const handleEdit = (id: string) => {
    navigate(`/pipelines/design/${id}`);
  };

  const handleAdvancedEdit = (id: string) => {
    navigate(`/pipelines/edit/${id}`);
  };

  const handleDelete = async (id: string) => {
    try {
      await deletePipeline(id);
      setPipelines(prev => prev.filter(p => p.plan_id !== id));
      toast.success('流水线已删除');
    } catch (err) {
      toast.error('删除失败');
      console.error(err);
    }
  };

  return (
    <motion.div
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      style={{ position: 'relative', zIndex: 1 }}
    >
      {/* Background decoration */}
      <div style={{
        position: 'absolute', top: -100, right: -100, width: 400, height: 400,
        borderRadius: '50%', background: 'radial-gradient(circle, rgba(108,92,231,0.1) 0%, transparent 70%)',
        pointerEvents: 'none',
      }} />
      <div style={{
        position: 'absolute', bottom: -50, left: -50, width: 300, height: 300,
        borderRadius: '50%', background: 'radial-gradient(circle, rgba(0,212,170,0.08) 0%, transparent 70%)',
        pointerEvents: 'none',
      }} />

      {/* Header */}
      <motion.div
        initial={{ y: -20, opacity: 0 }}
        animate={{ y: 0, opacity: 1 }}
        style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 32 }}
      >
        <div>
          <h1 style={{ fontSize: 28, fontWeight: 700, color: 'var(--text-primary)', letterSpacing: '-0.02em' }}>
            治理流水线
            <span style={{
              marginLeft: 12, fontSize: 14, fontWeight: 600, padding: '4px 12px',
              background: 'rgba(108,92,231,0.1)', color: '#6c5ce7', borderRadius: 20,
              verticalAlign: 'middle',
            }}>
              v0.2.1
            </span>
          </h1>
          <p style={{ color: 'var(--text-secondary)', fontSize: 14, marginTop: 6 }}>
            设计、部署和管理你的 AI 治理流水线
          </p>
        </div>

        <motion.button
          className="btn-primary"
          onClick={handleCreate}
          disabled={creating}
          whileHover={{ scale: 1.02, boxShadow: '0 0 40px rgba(108,92,231,0.5)' }}
          whileTap={{ scale: 0.98 }}
          style={{ display: 'flex', gap: 10, alignItems: 'center', padding: '12px 24px' }}
        >
          {creating ? (
            <motion.div
              animate={{ rotate: 360 }}
              transition={{ duration: 1, repeat: Infinity, ease: 'linear' }}
            >
              <Loader2 size={18} />
            </motion.div>
          ) : (
            <Plus size={18} />
          )}
          {creating ? '创建中...' : '新建流水线'}
        </motion.button>
      </motion.div>

      {/* Quick stats */}
      <motion.div
        initial={{ y: 20, opacity: 0 }}
        animate={{ y: 0, opacity: 1 }}
        transition={{ delay: 0.1 }}
        style={{ display: 'flex', gap: 16, marginBottom: 32 }}
      >
        <GlassCard style={{ flex: 1, padding: '20px 24px', display: 'flex', alignItems: 'center', gap: 16 }}>
          <div style={{
            width: 52, height: 52, borderRadius: 14,
            background: 'linear-gradient(135deg, rgba(108,92,231,0.2) 0%, rgba(162,155,254,0.2) 100%)',
            display: 'flex', alignItems: 'center', justifyContent: 'center',
          }}>
            <GitBranch size={24} style={{ color: '#6c5ce7' }} />
          </div>
          <div>
            <p style={{ fontSize: 12, color: 'var(--text-tertiary)', marginBottom: 2, fontWeight: 600, letterSpacing: '0.05em' }}>TOTAL PIPELINES</p>
            <p style={{ fontSize: 28, fontWeight: 700, color: 'var(--text-primary)' }}>{pipelines.length}</p>
          </div>
        </GlassCard>

        <GlassCard style={{ flex: 1, padding: '20px 24px', display: 'flex', alignItems: 'center', gap: 16 }}>
          <div style={{
            width: 52, height: 52, borderRadius: 14,
            background: 'linear-gradient(135deg, rgba(0,212,170,0.2) 0%, rgba(0,212,170,0.1) 100%)',
            display: 'flex', alignItems: 'center', justifyContent: 'center',
          }}>
            <CheckCircle size={24} style={{ color: '#00d4aa' }} />
          </div>
          <div>
            <p style={{ fontSize: 12, color: 'var(--text-tertiary)', marginBottom: 2, fontWeight: 600, letterSpacing: '0.05em' }}>ACTIVE</p>
            <p style={{ fontSize: 28, fontWeight: 700, color: '#00d4aa' }}>{pipelines.length}</p>
          </div>
        </GlassCard>

        <GlassCard style={{ flex: 1, padding: '20px 24px', display: 'flex', alignItems: 'center', gap: 16 }}>
          <div style={{
            width: 52, height: 52, borderRadius: 14,
            background: 'linear-gradient(135deg, rgba(253,203,110,0.2) 0%, rgba(253,203,110,0.1) 100%)',
            display: 'flex', alignItems: 'center', justifyContent: 'center',
          }}>
            <Shield size={24} style={{ color: '#fdcb6e' }} />
          </div>
          <div>
            <p style={{ fontSize: 12, color: 'var(--text-tertiary)', marginBottom: 2, fontWeight: 600, letterSpacing: '0.05em' }}>PLUGINS ACTIVE</p>
            <p style={{ fontSize: 28, fontWeight: 700, color: '#fdcb6e' }}>
              {pipelines.reduce((acc: number, p: Pipeline) => acc + (p.stages?.reduce((a: number, s: any) => a + (s.plugins?.length || 0), 0) || 0), 0)}
            </p>
          </div>
        </GlassCard>
      </motion.div>

      {/* Pipeline list */}
      {loading ? (
        <GlassCard style={{ textAlign: 'center', padding: 80 }}>
          <motion.div
            animate={{ rotate: 360 }}
            transition={{ duration: 2, repeat: Infinity, ease: 'linear' }}
            style={{ display: 'flex', justifyContent: 'center', marginBottom: 16 }}
          >
            <Loader2 size={48} style={{ color: '#6c5ce7', opacity: 0.5 }} />
          </motion.div>
          <p style={{ color: 'var(--text-secondary)', fontSize: 15 }}>加载中...</p>
        </GlassCard>
      ) : pipelines.length === 0 ? (
        <GlassCard style={{ textAlign: 'center', padding: 80 }}>
          <motion.div
            animate={{ y: [0, -10, 0] }}
            transition={{ duration: 2, repeat: Infinity, ease: 'easeInOut' }}
          >
            <GitBranch size={64} style={{ opacity: 0.2, margin: '0 auto 20px', display: 'block', color: '#6c5ce7' }} />
          </motion.div>
          <h3 style={{ fontSize: 22, fontWeight: 700, marginBottom: 8, color: 'var(--text-primary)' }}>
            还没有流水线
          </h3>
          <p style={{ color: 'var(--text-secondary)', fontSize: 14, marginBottom: 24, maxWidth: 400, margin: '0 auto 24px' }}>
            创建你的第一个 AI 治理流水线，设计同步和异步治理流程，保护你的 LLM 应用
          </p>
          <motion.button
            className="btn-primary"
            onClick={handleCreate}
            whileHover={{ scale: 1.05 }}
            whileTap={{ scale: 0.95 }}
            style={{ display: 'inline-flex', gap: 8, alignItems: 'center', padding: '12px 28px' }}
          >
            <Plus size={18} />
            创建第一个流水线
          </motion.button>
        </GlassCard>
      ) : (
        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fill, minmax(360px, 1fr))', gap: 20 }}>
          {pipelines.map((p, idx) => (
            <PipelineCard
              key={p.plan_id}
              pipeline={p}
              index={idx}
              onEdit={handleEdit}
              onAdvancedEdit={handleAdvancedEdit}
              onDelete={(id: string) => setDeleteId(id)}
            />
          ))}
        </div>
      )}

      {/* Footer info */}
      <motion.div
        initial={{ opacity: 0 }}
        animate={{ opacity: 1 }}
        transition={{ delay: 0.5 }}
        style={{
          marginTop: 32, padding: '16px 20px', borderRadius: 12,
          background: 'rgba(108,92,231,0.05)', border: '1px solid rgba(108,92,231,0.1)',
          display: 'flex', alignItems: 'center', gap: 12,
        }}
      >
        <Sparkles size={16} style={{ color: '#6c5ce7' }} />
        <p style={{ fontSize: 12, color: 'var(--text-secondary)' }}>
          VERIDACTUS 流水线遵循 <strong style={{ color: 'var(--text-primary)' }}>Protocol v0.2.1</strong>，
          支持同步快速路径治理和异步可信路径验证，确保 AI 应用的合规性和安全性
        </p>
      <ConfirmDialog
        open={!!deleteId}
        onClose={() => setDeleteId(null)}
        onConfirm={() => { if (deleteId) { handleDelete(deleteId); setDeleteId(null); } }}
        title="删除流水线"
        message="确定要删除这个流水线吗？删除后无法恢复。"
        confirmText="删除"
        danger
      />
      </motion.div>
    </motion.div>
  );
}